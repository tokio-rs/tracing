//! An appender that writes the same log line to multiple writers at once.
//!
//! This appender wraps other writers and writes out the same log line to each
//! wrapped writer in a loop. This appender can only accept writers of the same
//! concrete type.
//!
//! `MultiAppender` implements [`MakeWriter`][make_writer], which integrates with `tracing_subscriber`.
//!
//! [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
use std::io::Result as IOResult;
use std::io::Write;
use std::sync::Arc;
use tracing_subscriber::fmt::MakeWriter;

/// A writer that wraps other writers and writes log lines to all wrapped writers at the
/// same time.
///
/// This struct will wrap multiple other writers of the same type (ie, a group of
/// [`NonBlocking`][non_blocking] or a group of [`RollingFileAppender`][rolling]) and write the log line to all the writers in
/// in a loop. It is the spiritual equivalent of using `tee(1)` to have your log output printed
/// to both stdout and a file at the same time.
///
/// [non_blocking]: https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/struct.NonBlocking.html
/// [rolling]: https://docs.rs/tracing-appender/latest/tracing_appender/rolling/struct.RollingFileAppender.html
#[derive(Debug, Clone)]
pub struct MultiAppender<S> {
    writers: Arc<Vec<S>>,
}

impl<S> MultiAppender<S>
where
    S: MakeWriter + std::io::Write + Send + Sync,
{
    /// Returns a new `MultiAppender` wrapping a provided `Vec` of [`MakeWriter`][make_writer].
    ///
    /// [make_writer]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/trait.MakeWriter.html
    pub fn from(writers: Vec<S>) -> Self {
        MultiAppender {
            writers: Arc::new(writers),
        }
    }
}

impl<S> Write for MultiAppender<S>
where
    S: MakeWriter + std::io::Write + Send + Sync,
{
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        let size = buf.len();
        for writer in &*self.writers {
            let _ = writer.make_writer().write(buf)?;
        }

        Ok(size)
    }

    fn flush(&mut self) -> IOResult<()> {
        for writer in &*self.writers {
            writer.make_writer().flush()?;
        }

        Ok(())
    }
}

impl<S> MakeWriter for MultiAppender<S>
where
    S: MakeWriter + std::io::Write + Send + Sync,
{
    type Writer = MultiAppender<S>;

    fn make_writer(&self) -> Self::Writer {
        MultiAppender {
            writers: self.writers.clone(),
        }
    }
}
