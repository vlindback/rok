// sinks/file.rs
//
// Writes records to a log file using a BufWriter for efficiency.
// The logger thread batches writes and calls flush() periodically
// and on shutdown, so records are not lost.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use rok_abi::log::LogLevel;
use rok_abi::log::LogRecord;

use crate::sink::Sink;

pub struct FileSink {
    writer: BufWriter<File>,
    min_level: LogLevel,
}

impl FileSink {
    /// Open (or create and truncate) a log file at `path`.
    /// Returns None and logs to stderr if the file cannot be opened.
    pub fn new(path: &Path, min_level: LogLevel) -> Option<Self> {
        match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
        {
            Ok(file) => Some(Self {
                writer: BufWriter::new(file),
                min_level,
            }),
            Err(e) => {
                // We cannot use the logger here (we are constructing a sink).
                // Fall back to eprintln which is acceptable at init time.
                eprintln!("[rok-log] failed to open log file {:?}: {}", path, e);
                None
            }
        }
    }
}

impl Sink for FileSink {
    fn min_level(&self) -> LogLevel {
        self.min_level
    }

    fn write(&mut self, record: &LogRecord) {
        // SAFETY: file is always a valid null-terminated static string.
        let file = unsafe { std::ffi::CStr::from_ptr(record.file) }
            .to_str()
            .unwrap_or("<invalid utf8>");

        let message = std::str::from_utf8(&record.message[..record.message_len as usize])
            .unwrap_or("<invalid utf8>");

        let level = match record.level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO ",
            LogLevel::Warning => "WARN ",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        };

        // Ignore write errors — a stalled disk should not panic the game.
        let _ = write!(
            self.writer,
            "[{}] {}:{} — {}\n",
            level, file, record.line, message,
        );
    }

    fn flush(&mut self) {
        let _ = self.writer.flush();
    }
}
