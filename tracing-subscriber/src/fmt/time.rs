//! Formatters for event timestamps.
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
///
/// The full list of provided implementations can be found in [`time`].
///
/// [`time`]: ./index.html
pub trait FormatTime {
    /// Measure and write out the current time.
    ///
    /// When `format_time` is called, implementors should get the current time using their desired
    /// mechanism, and write it out to the given `fmt::Write`. Implementors must insert a trailing
    /// space themselves if they wish to separate the time from subsequent log message text.
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result;
}

/// Returns a new `SystemTime` timestamp provider.
///
/// This can then be configured further to determine how timestamps should be
/// configured.
///
/// This is equivalent to calling
/// ```rust
/// # fn timer() -> tracing_subscriber::fmt::time::SystemTime {
/// tracing_subscriber::fmt::time::SystemTime::default()
/// # }
/// ```
pub fn time() -> SystemTime {
    SystemTime::default()
}

/// Returns a new `Uptime` timestamp provider.
///
/// With this timer, timestamps will be formatted with the amount of time
/// elapsed since the timestamp provider was constructed.
///
/// This can then be configured further to determine how timestamps should be
/// configured.
///
/// This is equivalent to calling
/// ```rust
/// # fn timer() -> tracing_subscriber::fmt::time::Uptime {
/// tracing_subscriber::fmt::time::Uptime::default()
/// # }
/// ```
pub fn uptime() -> Uptime {
    Uptime::default()
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
        write!(w, "{}", chrono::Local::now().format("%b %d %H:%M:%S%.3f"))
    }
}

#[cfg(not(feature = "chrono"))]
impl FormatTime for SystemTime {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "{}", DateTime::from(std::time::SystemTime::now()))
    }
}

impl FormatTime for Uptime {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let e = self.epoch.elapsed();
        write!(w, "{:4}.{:09}s", e.as_secs(), e.subsec_nanos())
    }
}

/// The RFC 3339 format is used by default and using
/// this struct allows chrono to bypass the parsing
/// used when a custom format string is provided
#[cfg(feature = "chrono")]
#[derive(Debug, Clone, Eq, PartialEq)]
enum ChronoFmtType {
    Rfc3339,
    Custom(String),
}

#[cfg(feature = "chrono")]
impl Default for ChronoFmtType {
    fn default() -> Self {
        ChronoFmtType::Rfc3339
    }
}

/// Retrieve and print the current UTC time.
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ChronoUtc {
    format: ChronoFmtType,
}

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl ChronoUtc {
    /// Format the time using the [`RFC 3339`] format
    /// (a subset of [`ISO 8601`]).
    ///
    /// [`RFC 3339`]: https://tools.ietf.org/html/rfc3339
    /// [`ISO 8601`]: https://en.wikipedia.org/wiki/ISO_8601
    pub fn rfc3339() -> Self {
        ChronoUtc {
            format: ChronoFmtType::Rfc3339,
        }
    }

    /// Format the time using the given format string.
    ///
    /// See [`chrono::format::strftime`]
    /// for details on the supported syntax.
    ///
    /// [`chrono::format::strftime`]: https://docs.rs/chrono/0.4.9/chrono/format/strftime/index.html
    pub fn with_format(format_string: String) -> Self {
        ChronoUtc {
            format: ChronoFmtType::Custom(format_string),
        }
    }
}

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl FormatTime for ChronoUtc {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let time = chrono::Utc::now();
        match self.format {
            ChronoFmtType::Rfc3339 => write!(w, "{}", time.to_rfc3339()),
            ChronoFmtType::Custom(ref format_str) => write!(w, "{}", time.format(format_str)),
        }
    }
}

/// Retrieve and print the current local time.
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ChronoLocal {
    format: ChronoFmtType,
}

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl ChronoLocal {
    /// Format the time using the [`RFC 3339`] format
    /// (a subset of [`ISO 8601`]).
    ///
    /// [`RFC 3339`]: https://tools.ietf.org/html/rfc3339
    /// [`ISO 8601`]: https://en.wikipedia.org/wiki/ISO_8601
    pub fn rfc3339() -> Self {
        ChronoLocal {
            format: ChronoFmtType::Rfc3339,
        }
    }

    /// Format the time using the given format string.
    ///
    /// See [`chrono::format::strftime`]
    /// for details on the supported syntax.
    ///
    /// [`chrono::format::strftime`]: https://docs.rs/chrono/0.4.9/chrono/format/strftime/index.html
    pub fn with_format(format_string: String) -> Self {
        ChronoLocal {
            format: ChronoFmtType::Custom(format_string),
        }
    }
}

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
impl FormatTime for ChronoLocal {
    fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let time = chrono::Local::now();
        match self.format {
            ChronoFmtType::Rfc3339 => write!(w, "{} ", time.to_rfc3339()),
            ChronoFmtType::Custom(ref format_str) => write!(w, "{} ", time.format(format_str)),
        }
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
    }
    writer.write_char(' ')?;
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

/// A date/time type which exists primarily to convert `SystemTime` timestamps into an ISO 8601
/// formatted string.
///
/// Yes, this exists. Before you have a heart attack, understand that the meat of this is musl's
/// [`__secs_to_tm`][1] converted to Rust via [c2rust][2] and then cleaned up by hand. All existing
/// `strftime`-like APIs I found were unable to handle the full range of timestamps representable
/// by `SystemTime`, including `strftime` itself, since tm.tm_year is an int.
///
/// TODO: figure out how to properly attribute the MIT licensed musl project.
///
/// [1] http://git.musl-libc.org/cgit/musl/tree/src/time/__secs_to_tm.c
/// [2] https://c2rust.com/
///
/// This is directly copy-pasted from https://github.com/danburkert/kudu-rs/blob/c9660067e5f4c1a54143f169b5eeb49446f82e54/src/timestamp.rs#L5-L18
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    nanos: u32,
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.year > 9999 {
            write!(f, "+{}", self.year)?;
        } else if self.year < 0 {
            write!(f, "{:05}", self.year)?;
        } else {
            write!(f, "{:04}", self.year)?;
        }

        write!(
            f,
            "-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            self.nanos / 1_000
        )
    }
}

impl From<std::time::SystemTime> for DateTime {
    fn from(timestamp: std::time::SystemTime) -> DateTime {
        let (t, nanos) = match timestamp.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                debug_assert!(duration.as_secs() <= i64::MAX as u64);
                (duration.as_secs() as i64, duration.subsec_nanos())
            }
            Err(error) => {
                let duration = error.duration();
                debug_assert!(duration.as_secs() <= i64::MAX as u64);
                let (secs, nanos) = (duration.as_secs() as i64, duration.subsec_nanos());
                if nanos == 0 {
                    (-secs, 0)
                } else {
                    (-secs - 1, 1_000_000_000 - nanos)
                }
            }
        };

        // 2000-03-01 (mod 400 year, immediately after feb29
        const LEAPOCH: i64 = 946_684_800 + 86400 * (31 + 29);
        const DAYS_PER_400Y: i32 = 365 * 400 + 97;
        const DAYS_PER_100Y: i32 = 365 * 100 + 24;
        const DAYS_PER_4Y: i32 = 365 * 4 + 1;
        static DAYS_IN_MONTH: [i8; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

        // Note(dcb): this bit is rearranged slightly to avoid integer overflow.
        let mut days: i64 = (t / 86_400) - (LEAPOCH / 86_400);
        let mut remsecs: i32 = (t % 86_400) as i32;
        if remsecs < 0i32 {
            remsecs += 86_400;
            days -= 1
        }

        let mut qc_cycles: i32 = (days / i64::from(DAYS_PER_400Y)) as i32;
        let mut remdays: i32 = (days % i64::from(DAYS_PER_400Y)) as i32;
        if remdays < 0 {
            remdays += DAYS_PER_400Y;
            qc_cycles -= 1;
        }

        let mut c_cycles: i32 = remdays / DAYS_PER_100Y;
        if c_cycles == 4 {
            c_cycles -= 1;
        }
        remdays -= c_cycles * DAYS_PER_100Y;

        let mut q_cycles: i32 = remdays / DAYS_PER_4Y;
        if q_cycles == 25 {
            q_cycles -= 1;
        }
        remdays -= q_cycles * DAYS_PER_4Y;

        let mut remyears: i32 = remdays / 365;
        if remyears == 4 {
            remyears -= 1;
        }
        remdays -= remyears * 365;

        let mut years: i64 = i64::from(remyears)
            + 4 * i64::from(q_cycles)
            + 100 * i64::from(c_cycles)
            + 400 * i64::from(qc_cycles);

        let mut months: i32 = 0;
        while i32::from(DAYS_IN_MONTH[months as usize]) <= remdays {
            remdays -= i32::from(DAYS_IN_MONTH[months as usize]);
            months += 1
        }

        if months >= 10 {
            months -= 12;
            years += 1;
        }

        DateTime {
            year: years + 2000,
            month: (months + 3) as u8,
            day: (remdays + 1) as u8,
            hour: (remsecs / 3600) as u8,
            minute: (remsecs / 60 % 60) as u8,
            second: (remsecs % 60) as u8,
            nanos,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::i32;
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;

    #[test]
    fn test_datetime() {
        let case = |expected: &str, secs: i64, micros: u32| {
            let timestamp = if secs >= 0 {
                UNIX_EPOCH + Duration::new(secs as u64, micros * 1_000)
            } else {
                (UNIX_EPOCH - Duration::new(!secs as u64 + 1, 0)) + Duration::new(0, micros * 1_000)
            };
            assert_eq!(
                expected,
                format!("{}", DateTime::from(timestamp)),
                "secs: {}, micros: {}",
                secs,
                micros
            )
        };

        // Mostly generated with:
        //  - date -jur <secs> +"%Y-%m-%dT%H:%M:%S.000000Z"
        //  - http://unixtimestamp.50x.eu/

        case("1970-01-01T00:00:00.000000Z", 0, 0);

        case("1970-01-01T00:00:00.000001Z", 0, 1);
        case("1970-01-01T00:00:00.500000Z", 0, 500_000);
        case("1970-01-01T00:00:01.000001Z", 1, 1);
        case("1970-01-01T00:01:01.000001Z", 60 + 1, 1);
        case("1970-01-01T01:01:01.000001Z", 60 * 60 + 60 + 1, 1);
        case(
            "1970-01-02T01:01:01.000001Z",
            24 * 60 * 60 + 60 * 60 + 60 + 1,
            1,
        );

        case("1969-12-31T23:59:59.000000Z", -1, 0);
        case("1969-12-31T23:59:59.000001Z", -1, 1);
        case("1969-12-31T23:59:59.500000Z", -1, 500_000);
        case("1969-12-31T23:58:59.000001Z", -60 - 1, 1);
        case("1969-12-31T22:58:59.000001Z", -60 * 60 - 60 - 1, 1);
        case(
            "1969-12-30T22:58:59.000001Z",
            -24 * 60 * 60 - 60 * 60 - 60 - 1,
            1,
        );

        case("2038-01-19T03:14:07.000000Z", i32::MAX as i64, 0);
        case("2038-01-19T03:14:08.000000Z", i32::MAX as i64 + 1, 0);
        case("1901-12-13T20:45:52.000000Z", i32::MIN as i64, 0);
        case("1901-12-13T20:45:51.000000Z", i32::MIN as i64 - 1, 0);
        case("+292277026596-12-04T15:30:07.000000Z", i64::MAX, 0);
        case("+292277026596-12-04T15:30:06.000000Z", i64::MAX - 1, 0);
        case("-292277022657-01-27T08:29:53.000000Z", i64::MIN + 1, 0);

        case("1900-01-01T00:00:00.000000Z", -2208988800, 0);
        case("1899-12-31T23:59:59.000000Z", -2208988801, 0);
        case("0000-01-01T00:00:00.000000Z", -62167219200, 0);
        case("-0001-12-31T23:59:59.000000Z", -62167219201, 0);

        case("1234-05-06T07:08:09.000000Z", -23215049511, 0);
        case("-1234-05-06T07:08:09.000000Z", -101097651111, 0);
        case("2345-06-07T08:09:01.000000Z", 11847456541, 0);
        case("-2345-06-07T08:09:01.000000Z", -136154620259, 0);
    }
}