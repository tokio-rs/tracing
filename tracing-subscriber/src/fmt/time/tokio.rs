use crate::fmt::format::Writer;

/// Retrieve and print the relative elapsed tokio time since an epoch.
///
/// In non-paused tokio environments and when used outside of a tokio runtime,
/// the epoch is the current time. See [`tokio::time::Instant`] for more info.
///
/// The `Default` implementation for `Uptime` makes the epoch the current time.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TokioUptime {
    epoch: tokio::time::Instant,
}

impl Default for TokioUptime {
    fn default() -> Self {
        TokioUptime {
            epoch: tokio::time::Instant::now(),
        }
    }
}

impl From<tokio::time::Instant> for TokioUptime {
    fn from(epoch: tokio::time::Instant) -> Self {
        TokioUptime { epoch }
    }
}

impl super::FormatTime for TokioUptime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let e = self.epoch.elapsed();
        write!(w, "{:4}.{:09}s", e.as_secs(), e.subsec_nanos())
    }
}

/// Returns a new [`TokioUptime`] timestamp provider.
///
/// With this timer, timestamps will be formatted with the amount of time
/// elapsed in the tokio runtime since the timestamp provider was constructed.
///
/// This can then be configured further to determine how timestamps should be
/// configured.
///
/// This is equivalent to calling
/// ```no_run
/// tracing_subscriber::fmt::time::TokioUptime::default()
/// ```
pub fn tokio_uptime() -> TokioUptime {
    TokioUptime::default()
}
