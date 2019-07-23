use crate::{format_trace, AsTrace};
use log;
use tracing_core::dispatcher;

/// A simple "logger" that converts all log records into `tracing` `Event`s.
///
/// Can be initialized with:
///
/// * [`init`] if you want to convert all logs and do the filtering in a subscriber
/// * [`init_with_filter`] if you know in advance a log level you want to filter
///
/// [`init`]: ../fn.init.html
/// [`init_with_filter`]: ../fn.init_with_filter.html
#[derive(Debug)]
pub struct LogTracer {
    ignore_crates: Box<[String]>,
}

#[derive(Debug)]
pub struct Builder {
    ignore_crates: Vec<String>,
    filter: log::LevelFilter,
}

// ===== impl LogTracer =====

impl LogTracer {
    /// Returns a builder that allows customizing a `LogTracer` and setting it
    /// the default logger.
    ///
    /// For example:
    /// ```rust
    /// # use std::error::Error;
    /// use tracing_log::LogTracer;
    /// use log;
    ///
    /// # fn main() -> Result<(), Box<Error>> {
    /// LogTracer::builder()
    ///     .ignore_crate("foo") // suppose the `foo` crate is using `tracing`'s log feature
    ///     .with_max_level(log::LevelFilter::Info)
    ///     .init()?;
    ///
    /// // will be available for Subscribers as a tracing Event
    /// log::info!("an example info log");
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Creates a new `LogTracer` that can then be used as a logger for the `log` crate.
    ///
    /// It is generally simpler to use the [`init`] or [`init_with_filter`] methods
    /// which will create the `LogTracer` and set it as the global logger.
    ///
    /// Logger setup without the initialization methods can be done with:
    ///
    /// ```rust
    /// # use std::error::Error;
    /// use tracing_log::LogTracer;
    /// use log;
    ///
    /// # fn main() -> Result<(), Box<Error>> {
    /// let logger = LogTracer::new();
    /// log::set_boxed_logger(Box::new(logger))?;
    /// log::set_max_level(log::LevelFilter::Trace);
    ///
    /// // will be available for Subscribers as a tracing Event
    /// log::trace!("an example trace log");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`init`]: #method.init
    /// [`init_with_filter`]: .#method.init_with_filter
    pub fn new() -> Self {
        Self {
            ignore_crates: Vec::new().into_boxed_slice(),
        }
    }

    /// Sets up `LogTracer` as global logger for the `log` crate,
    /// with the given level as max level filter.
    ///
    /// Setting a global logger can only be done once.
    ///
    /// The [`builder`] function can be used to customize the `LogTracer` before
    /// initializing it.
    ///
    /// [`builder`]: #method.builder
    pub fn init_with_filter(level: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        Self::builder().with_max_level(level).init()
    }

    /// Sets a `LogTracer` as the global logger for the `log` crate.
    ///
    /// Setting a global logger can only be done once.
    ///
    /// ```rust
    /// # use std::error::Error;
    /// use tracing_log::LogTracer;
    /// use log;
    ///
    /// # fn main() -> Result<(), Box<Error>> {
    /// LogTracer::init()?;
    ///
    /// // will be available for Subscribers as a tracing Event
    /// log::trace!("an example trace log");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This will forward all logs to `tracing` and lets the current `Subscriber`
    /// determine if they are enabled.
    ///
    /// The [`builder`] function can be used to customize the `LogTracer` before
    /// initializing it.
    ///
    /// If you know in advance you want to filter some log levels,
    /// use [`builder`] or [`init_with_filter`] instead.
    ///
    /// [`init_with_filter`]: #method.init_with_filter
    /// [`builder`]: #method.builder
    pub fn init() -> Result<(), log::SetLoggerError> {
        Self::builder().init()
    }
}

impl Default for LogTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl log::Log for LogTracer {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if self.ignore_crates.is_empty() {
            return true;
        }

        // If we are ignoring certain module paths, ensure that the metadata
        // does not start with one of those paths.
        let target = metadata.target();
        !self
            .ignore_crates
            .iter()
            .any(|ignored| target.starts_with(ignored))
    }

    fn log(&self, record: &log::Record) {
        let enabled = dispatcher::get_default(|dispatch| {
            // TODO: can we cache this for each log record, so we can get
            // similar to the callsite cache?
            dispatch.enabled(&record.as_trace())
        });

        if enabled {
            // TODO: if the record is enabled, we'll get the current dispatcher
            // twice --- once to check if enabled, and again to dispatch the event.
            // If we could construct events without dispatching them, we could
            // re-use the dispatcher reference...
            format_trace(record).unwrap();
        }
    }

    fn flush(&self) {}
}

// ===== impl Builder =====

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a global maximum level for `log` records.
    ///
    /// Log records whose level is more verbose than the provided level will be
    /// disabled.
    ///
    /// By default, all `log` records will be enabled.
    pub fn with_max_level(self, filter: impl Into<log::LevelFilter>) -> Self {
        let filter = filter.into();
        Self { filter, ..self }
    }

    /// Configures the `LogTracer` to ignore all log records whose target
    /// starts with the given string.
    ///
    /// This should be used when a crate enables the `tracing/log` feature to
    /// emit log records for tracing events. Otherwise, those events will be
    /// recorded twice.
    pub fn ignore_crate(mut self, name: impl Into<String>) -> Self {
        self.ignore_crates.push(name.into());
        self
    }

    /// Configures the `LogTracer` to ignore all log records whose target
    /// starts with any of the given the given strings.
    ///
    /// This should be used when a crate enables the `tracing/log` feature to
    /// emit log records for tracing events. Otherwise, those events will be
    /// recorded twice.
    pub fn ignore_all<I>(self, crates: impl IntoIterator<Item = I>) -> Self
    where
        I: Into<String>,
    {
        crates.into_iter().fold(self, Self::ignore_crate)
    }

    /// Constructs a new `LogTracer` with the provided configuration and sets it
    /// as the default logger.
    ///
    /// Setting a global logger can only be done once.
    pub fn init(self) -> Result<(), log::SetLoggerError> {
        let ignore_crates = self.ignore_crates.into_boxed_slice();
        let logger = Box::new(LogTracer { ignore_crates });
        log::set_boxed_logger(logger)?;
        log::set_max_level(self.filter);
        Ok(())
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            ignore_crates: Vec::new(),
            filter: log::LevelFilter::max(),
        }
    }
}
