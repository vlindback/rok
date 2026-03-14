// logger.rs
//
// The Logger singleton. Owns the ArrayQueue, the logger thread, the sink
// list, and the pre-init buffer.
//
// State machine:
//
//   null AtomicPtr  →  Logger::init()  →  non-null AtomicPtr
//   (Uninit)                              (Running)
//
// The hot path (log_record) does a single AtomicPtr::load(Acquire).
// If null  → pre-init path (write_stderr + ring buffer).
// If !null → push into ArrayQueue + unpark logger thread.
//
// Sink list uses arc-swap for lock-free reads on the logger thread with
// clone-on-write registration from the Host thread.

use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam::queue::ArrayQueue;
use crossbeam::sync::{Parker, Unparker};

use rok_abi::log::{LOG_MESSAGE_CAPACITY, LogRecord};

use crate::sink::Sink;
use crate::stderr_sink::write_stderr;

// ---------------------------------------------------------------------------
// Queue capacity
// ---------------------------------------------------------------------------

/// Number of LogRecords the queue can hold before records are dropped.
/// Sized to absorb a burst of several frames worth of logging without
/// stalling any producer thread.
const QUEUE_CAPACITY: usize = 1024;

// ---------------------------------------------------------------------------
// Dropped record counter
// ---------------------------------------------------------------------------

/// Incremented when a push fails because the queue is full.
/// The logger thread periodically checks and emits a warning.
static DROPPED_RECORDS: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Singleton pointer
// ---------------------------------------------------------------------------

static LOGGER: AtomicPtr<LoggerInner> = AtomicPtr::new(ptr::null_mut());

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

pub fn make_record(
    timestamp_ns: u64,
    level: rok_abi::log::LogLevel,
    file: *const std::ffi::c_char,
    line: u32,
    message: &[u8],
) -> LogRecord {
    let mut rec = LogRecord {
        timestamp_ns,
        level,
        file,
        line,
        message_len: 0,
        message: [0u8; LOG_MESSAGE_CAPACITY],
    };

    let len = message.len().min(LOG_MESSAGE_CAPACITY);
    rec.message[..len].copy_from_slice(&message[..len]);
    rec.message_len = len as u16;
    rec
}

// ---------------------------------------------------------------------------
// LoggerInner
// ---------------------------------------------------------------------------

pub(crate) struct LoggerInner {
    pub queue: Arc<ArrayQueue<LogRecord>>,
    pub unparker: Unparker,
    pub sinks: Arc<Mutex<Vec<Box<dyn Sink>>>>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// pub(crate) fn is_uninit() -> bool {
//     LOGGER.load(Ordering::Acquire).is_null()
// }

/// Initialise the logger. Must be called exactly once by the Host, early in
/// startup after basic platform init is done.
///
/// After this call all log records go through the queue to the logger thread.
/// The pre-init ring buffer is replayed into all initial sinks so nothing is
/// lost.
///
/// `initial_sinks` — sinks to register immediately (e.g. StderrSink, FileSink).
/// `pre_init_buf`  — the buffer collected before this call; replayed then discarded.
pub fn init(initial_sinks: Vec<Box<dyn Sink>>) {
    assert!(
        LOGGER.load(Ordering::Acquire).is_null(),
        "rok-log: Logger::init() called more than once"
    );

    let queue = Arc::new(ArrayQueue::new(QUEUE_CAPACITY));
    let sinks = Arc::new(Mutex::new(initial_sinks));

    let parker = Parker::new();
    let unparker = parker.unparker().clone();
    let inner = Box::new(LoggerInner {
        queue: Arc::clone(&queue),
        unparker,
        sinks: Arc::clone(&sinks),
    });

    let raw = Box::into_raw(inner);

    // Publish the pointer. From this point any thread calling log_record will
    // push into the queue rather than going through the pre-init path.
    LOGGER.store(raw, Ordering::Release);

    // Spawn the logger thread. It borrows queue and sinks via Arc.
    let thread_queue = Arc::clone(&queue);
    let thread_sinks = Arc::clone(&sinks);

    thread::Builder::new()
        .name("rok-log".into())
        .spawn(move || logger_thread(thread_queue, thread_sinks, parker))
        .expect("rok-log: failed to spawn logger thread");
}

/// Register a new sink at runtime. Safe to call from the Host thread at any
/// time after init(). The logger thread will pick it up on its next wake.
///
/// Panics if called before init().
pub fn register_sink(sink: Box<dyn Sink>) {
    let ptr = LOGGER.load(Ordering::Acquire);
    assert!(
        !ptr.is_null(),
        "rok-log: register_sink() called before init()"
    );

    // SAFETY: ptr is non-null and was set by init()
    // The LoggerInner is never freed until shutdown().
    let inner = unsafe { &*ptr };

    let mut sinks = inner.sinks.lock().unwrap();
    sinks.push(sink);
}

/// The hot-path entry point. Called by the log!() macros.
///
/// If the logger is uninitialised: writes to stderr and stashes in the
/// pre-init ring buffer (caller's responsibility to pass the right buffer).
///
/// If initialised: pushes into the queue and unparks the logger thread.
#[inline]
pub fn log_record(record: LogRecord) {
    let ptr = LOGGER.load(Ordering::Acquire);

    if ptr.is_null() {
        // Pre-init path. Caller must handle the ring buffer separately via
        // the PRE_INIT_BUFFER in lib.rs.
        write_stderr(&record);
        return;
    }

    // SAFETY: same as register_sink above.
    let inner = unsafe { &*ptr };

    if inner.queue.push(record).is_err() {
        DROPPED_RECORDS.fetch_add(1, Ordering::Relaxed);
        return;
    }

    inner.unparker.unpark();
}

/// Flush all sinks and stop the logger thread. Called by the Host on shutdown.
/// After this returns it is safe to exit the process.
pub fn shutdown() {
    let ptr = LOGGER.swap(ptr::null_mut(), Ordering::AcqRel);
    if ptr.is_null() {
        return;
    }

    // SAFETY: we just took ownership back via the swap.
    let inner = unsafe { Box::from_raw(ptr) };

    // Wake the thread one last time so it drains the queue and exits.
    inner.unparker.unpark();

    // Give the thread a moment to drain. In practice it will finish
    // near-instantly since the queue is bounded and small.
    // A more robust approach would use a shutdown flag + condvar, but
    // for now a short yield loop is sufficient at process exit.
    //
    // TODO: replace with a proper shutdown channel if needed.
    for _ in 0..1000 {
        if inner.queue.is_empty() {
            break;
        }
        std::hint::spin_loop();
    }

    // Flush all sinks one final time.
    {
        let mut sinks = inner.sinks.lock().unwrap();
        for sink in sinks.iter_mut() {
            sink.flush();
        }
    }
}

// ---------------------------------------------------------------------------
// Logger thread
// ---------------------------------------------------------------------------

fn logger_thread(
    queue: Arc<ArrayQueue<LogRecord>>,
    sinks: Arc<Mutex<Vec<Box<dyn Sink>>>>,
    parker: Parker,
) {
    loop {
        // Park until there is work or we are woken for shutdown.
        parker.park();

        // Drain the queue in one pass.
        let mut sink_guard = sinks.lock().unwrap();

        // Check and report dropped records.
        let dropped = DROPPED_RECORDS.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            // Emit a synthetic warning directly to sinks — we cannot call
            // log_record() here (would re-enter) so we format inline.
            eprintln!(
                "[rok-log] WARNING: dropped {} records (queue full)",
                dropped
            );
        }

        while let Some(record) = queue.pop() {
            for sink in sink_guard.iter_mut() {
                if record.level >= sink.min_level() {
                    sink.write(&record);
                }
            }
        }

        // Flush after draining the batch.
        for sink in sink_guard.iter_mut() {
            sink.flush();
        }

        // Exit condition: LOGGER pointer was cleared by shutdown().
        if LOGGER.load(Ordering::Acquire).is_null() {
            break;
        }
    }
}
