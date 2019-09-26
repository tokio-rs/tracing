//! Formatters for event timestamps.

#[cfg(feature = "chrono")]
use chrono;

#[cfg(feature = "ansi")]
use ansi_term::Style;

use std::fmt;
use std::time::Instant;

/// A type that can measure and format the current time.
///
/// This trait is used by `Format` to include a timestamp with each `Event` when it is logged.
///
/// Notable default implementations of this trait are `SystemTime` and `()`. The former prints the
/// current time as reported by `std::time::SystemTime`, and the latter does not print the current
/// time at all. `FormatTime` is also automatically implemented for any function pointer with the
/// appropriate signature.
pub trait FormatTime {
    /// Measure and write out the current time.
    ///
    /// When `format_time` is called, implementors should get the current time using their desired
    /// mechanism, and write it out to the given `fmt::Write`. Implementors must insert a trailing
    /// space themselves if they wish to separate the time from subsequent log message text.
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result;
}

impl<'a, F> FormatTime for &'a F
where
    F: FormatTime,
{
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        (*self).format_time(w)
    }
}

impl FormatTime for () {
    fn format_time(&self, _: &mut dyn fmt::Write) -> fmt::Result {
        Ok(())
    }
}

impl FormatTime for fn(&mut dyn fmt::Write) -> fmt::Result {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        (*self)(w)
    }
}

/// Retrieve and print the current wall-clock time.
///
/// If the `chrono` feature is enabled, the current time is printed in a human-readable format like
/// "Jun 25 14:27:12.955". Otherwise the `Debug` implementation of `std::time::SystemTime` is used.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct SystemTime;

/// Retrieve and print the relative elapsed wall-clock time since an epoch.
///
/// The `Default` implementation for `Uptime` makes the epoch the current time.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Uptime {
    epoch: Instant,
}

impl Default for Uptime {
    fn default() -> Self {
        Uptime {
            epoch: Instant::now(),
        }
    }
}

impl From<Instant> for Uptime {
    fn from(epoch: Instant) -> Self {
        Uptime { epoch }
    }
}

#[cfg(feature = "chrono")]
impl FormatTime for SystemTime {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "{} ", chrono::Local::now().format("%b %d %H:%M:%S%.3f"))
    }
}
#[cfg(not(feature = "chrono"))]
impl FormatTime for SystemTime {
    fn format_time(&self, w: &mut fmt::Write) -> fmt::Result {
        write!(w, "{:?} ", std::time::SystemTime::now())
    }
}

impl FormatTime for Uptime {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let e = self.epoch.elapsed();
        write!(w, "{:4}.{:09}s ", e.as_secs(), e.subsec_nanos())
    }
}

#[inline(always)]
#[cfg(feature = "ansi")]
pub(crate) fn write<T>(timer: T, writer: &mut dyn fmt::Write, with_ansi: bool) -> fmt::Result
where
    T: FormatTime,
{
    if with_ansi {
        let style = Style::new().dimmed();
        write!(writer, "{}", style.prefix())?;
        timer.format_time(writer)?;
        write!(writer, "{}", style.suffix())?;
    } else {
        timer.format_time(writer)?;
        write!(writer, " ")?;
    }
    Ok(())
}

#[inline(always)]
#[cfg(not(feature = "ansi"))]
pub(crate) fn write<T>(timer: T, writer: &mut dyn fmt::Write) -> fmt::Result
where
    T: FormatTime,
{
    timer.format_time(writer)?;
    write!(writer, " ")
}
