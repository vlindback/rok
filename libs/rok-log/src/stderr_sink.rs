// sinks/stderr.rs
//
// Writes every record at or above min_level to stderr.
// Uses the same formatter as the pre-init path for consistency —
// log output looks identical before and after logger initialisation.

use crate::sink::Sink;
use rok_abi::log::{LogLevel, LogRecord};
use std::io::Write;

pub struct StderrSink {
    pub min_level: LogLevel,
}

impl StderrSink {
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level }
    }
}

impl Sink for StderrSink {
    fn min_level(&self) -> LogLevel {
        self.min_level
    }

    fn write(&mut self, record: &LogRecord) {
        write_stderr(record);
    }

    fn flush(&mut self) {
        // stderr is unbuffered — nothing to flush.
    }
}

/// Level tag used in the formatted output line.
fn level_tag(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warning => "WARN",
        LogLevel::Error => "ERROR",
        LogLevel::Fatal => "FATAL",
    }
}

/// Format a LogRecord into a stack-allocated byte buffer and write it to
/// stderr synchronously.
///
/// Format: `[LEVEL] file:line — message\n`
pub fn write_stderr(record: &LogRecord) {
    // SAFETY: file is always a valid null-terminated static string.
    let file =
        std::str::from_utf8(&record.file[..record.file_len as usize]).unwrap_or("<invalid utf8>");

    let message = std::str::from_utf8(&record.message[..record.message_len as usize])
        .unwrap_or("<invalid utf8>");

    // Stack-allocated formatting. No heap.
    // eprintln! internally heap-allocates on some platforms so we use
    // write! directly into stderr which goes through the fd write path.
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();

    #[cfg(windows)]
    const NEWLINE: &str = "\r\n";
    #[cfg(not(windows))]
    const NEWLINE: &str = "\n";

    // Ignore write errors — if stderr is broken there is nothing we can do.
    let _ = write!(
        handle,
        "[ {} ] {}:{} — {}{}",
        level_tag(record.level),
        file,
        record.line,
        message,
        NEWLINE,
    );
}
