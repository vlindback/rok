// sink.rs
//
// The Sink trait. Anything that can receive a LogRecord implements this.
//
// Sinks are owned by the logger thread exclusively — they are never called
// from the hot path. All formatting and I/O happens here, off the
// producer threads.
//
// Implementing a sink:
//   - `write` will be called for every record that passes the sink's level
//     filter. It must not block indefinitely.
//   - `flush` is called on shutdown and may be called periodically. It
//     should ensure all buffered records are committed to their destination.
//   - `min_level` is checked by the logger thread before calling `write`,
//     so implementations do not need to re-check it internally.

use rok_abi::log::LogLevel;
use rok_abi::log::LogRecord;

pub trait Sink: Send + 'static {
    /// The minimum level this sink cares about.
    /// Records below this level are dropped before `write` is called.
    fn min_level(&self) -> LogLevel;

    /// Write a single record to this sink.
    /// Called only from the logger thread. Must not block indefinitely.
    fn write(&mut self, record: &LogRecord);

    /// Flush any internal buffers to the underlying destination.
    fn flush(&mut self);
}
