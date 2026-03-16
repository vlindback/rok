// rok-log/src/lib.rs
//
// Public API surface for rok-log.
//
// The Host calls:
//   rok_log::init(sinks)   — once, early in startup
//   rok_log::shutdown()    — once, on process exit
//   rok_log::register_sink — to add sinks at runtime (e.g. in-game console)
//
// Everyone calls the macros:
//   log_trace!("msg {}", val)
//   log_debug!("msg {}", val)
//   log_info!("msg {}", val)
//   log_warn!("msg {}", val)
//   log_error!("msg {}", val)
//   log_fatal!("msg {}", val)
//
// The macros work before init() — records go straight to stderr and into
// the pre-init ring buffer which is replayed into all sinks on init().

use rok_abi::log::LOG_MESSAGE_CAPACITY;

pub mod logger;
pub mod sink;

mod file_sink;
mod stderr_sink;

pub use file_sink::FileSink;
pub use logger::{init, init_remote, register_sink, shutdown};
pub use sink::Sink;
pub use stderr_sink::StderrSink;

use crate::logger::make_record;

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

/// Monotonic nanosecond timestamp. Used by the log macros.
#[inline]
pub fn timestamp_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

// ---------------------------------------------------------------------------
// Internal submit — called by macros only
// ---------------------------------------------------------------------------

#[doc(hidden)]
pub fn __submit(level: rok_abi::log::LogLevel, file: &str, line: u32, args: std::fmt::Arguments) {
    let mut buf = [0u8; LOG_MESSAGE_CAPACITY];
    let message_len = {
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        let _ = std::io::Write::write_fmt(&mut cursor, args);
        cursor.position() as usize
    };

    let ts = timestamp_ns();
    let message = &buf[..message_len];

    logger::log_record(make_record(ts, level, file, line, message));
}

// ---------------------------------------------------------------------------
// Logging macros
// ---------------------------------------------------------------------------

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::__submit(
            rok_abi::log::LogLevel::Trace,
            file!(),
            line!(),
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::__submit(
            rok_abi::log::LogLevel::Debug,
            file!(),
            line!(),
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::__submit(
            rok_abi::log::LogLevel::Info,
            file!(),
            line!(),
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::__submit(
            rok_abi::log::LogLevel::Warning,
            file!(),
            line!(),
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::__submit(
            rok_abi::log::LogLevel::Error,
            file!(),
            line!(),
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! log_fatal {
    ($($arg:tt)*) => {
        $crate::__submit(
            rok_abi::log::LogLevel::Fatal,
            file!(),
            line!(),
            format_args!($($arg)*),
        )
    };
}
