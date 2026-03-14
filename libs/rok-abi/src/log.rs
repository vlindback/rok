// log.rs
//
// Logging shared types
//
// LogLevel and LogRecord are defined here so they are visible to every
// crate (Host, Engine, Target) without pulling in the rok-log implementation.
//
// LogLevel was previously in host_api.rs — it is re-exported from there
// for backwards compatibility.
//
// All types are #[repr(C)] with no heap allocation. A LogRecord must be
// safe to push into a lock-free ArrayQueue from any thread with zero alloc.

use core::ffi::c_char;

// ---------------------------------------------------------------------------
// LogLevel
// ---------------------------------------------------------------------------

/// Log severity. repr(u32) for stable FFI across compiler/DLL boundaries.
///
/// Ordered so that level >= LogLevel::Warning works as a filter.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warning = 3,
    Error = 4,
    Fatal = 5,
}

// ---------------------------------------------------------------------------
// LogRecord
// ---------------------------------------------------------------------------

/// Maximum byte length of an encoded log message.
/// Messages exceeding this are truncated. Chosen to cover the vast majority
/// of real log lines while keeping LogRecord stack-allocatable.
pub const LOG_MESSAGE_CAPACITY: usize = 512;

/// A single log record. Fixed size, no pointers into the heap.
///
/// `file` points to a static string literal produced by the `file!()`
/// macro — it is always `'static` and never needs to be freed.
///
/// # Safety
/// `file` is a raw pointer to a `'static` string. `LogRecord` is manually
/// declared `Send` because raw pointers are not `Send` by default, but
/// this pointer is always valid for the lifetime of the process.
#[repr(C)]
pub struct LogRecord {
    /// Monotonic nanosecond timestamp.
    pub timestamp_ns: u64,

    /// Severity level.
    pub level: LogLevel,

    /// Source file. Null-terminated static string from `file!()`. Never null.
    pub file: *const c_char,

    /// Source line number from `line!()`.
    pub line: u32,

    /// Number of valid bytes in `message`. Never exceeds LOG_MESSAGE_CAPACITY.
    pub message_len: u16,

    /// UTF-8 message bytes. Not null-terminated. Valid bytes: `..message_len`.
    pub message: [u8; LOG_MESSAGE_CAPACITY],
}

// SAFETY: `file` always points to a `'static` string literal. It is never
// written to and is valid for the entire process lifetime. Safe to send
// across thread boundaries.
unsafe impl Send for LogRecord {}
