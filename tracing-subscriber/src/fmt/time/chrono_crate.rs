use crate::fmt::format::Writer;
use crate::fmt::time::FormatTime;

/// Formats [local time]s and [UTC time]s with `FormatTime` implementations
/// that use the [`chrono` crate].
///
/// [local time]: [`chrono::offset::Local`]
/// [UTC time]: [`chrono::offset::Utc`]
/// [`chrono` crate]: [`chrono`]

/// The RFC 3339 format is used by default but a custom format string
/// can be used. See [`chrono::format::strftime`]for details on
/// the supported syntax.
///
/// [`chrono::format::strftime`]: https://docs.rs/chrono/0.4.9/chrono/format/strftime/index.html
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Eq, PartialEq)]
enum ChronoFmtType {
    /// Format according to the RFC 3339 convention.
    Rfc3339,
    /// Format according to a custom format string.
    Custom(String),
}

impl Default for ChronoFmtType {
    fn default() -> Self {
        ChronoFmtType::Rfc3339
    }
}

/// Retrieve and print the current local time.
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ChronoLocal {
    format: ChronoFmtType,
}

impl FormatTime for ChronoLocal {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        let t = chrono::Local::now();
        match &self.format {
            ChronoFmtType::Rfc3339 => w.write_str(&t.to_rfc3339()),
            ChronoFmtType::Custom(fmt) => w.write_str(&format!("{}", t.format(fmt))),
        }
    }
}

/// Retrieve and print the current UTC time.
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ChronoUtc {
    format: ChronoFmtType,
}

impl FormatTime for ChronoUtc {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        let t = chrono::Utc::now();
        match &self.format {
            ChronoFmtType::Rfc3339 => w.write_str(&t.to_rfc3339()),
            ChronoFmtType::Custom(fmt) => w.write_str(&format!("{}", t.format(fmt))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::format::Writer;
    use crate::fmt::time::FormatTime;

    use super::ChronoFmtType;
    use super::ChronoLocal;
    use super::ChronoUtc;

    #[test]
    fn test_chrono_format_time_utc_default() {
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&ChronoUtc::default(), &mut dst).is_ok());
        // e.g. `buf` contains "2023-08-18T19:05:08.662499+00:00"
    }

    #[test]
    fn test_chrono_format_time_utc_custom() {
        let fmt = ChronoUtc {
            format: ChronoFmtType::Custom("%a %b %e %T %Y".to_owned()),
        };
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&fmt, &mut dst).is_ok());
        // e.g. `buf` contains "Wed Aug 23 15:53:23 2023"
    }

    #[test]
    fn test_chrono_format_time_local_default() {
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&ChronoLocal::default(), &mut dst).is_ok());
        // e.g. `buf` contains "2023-08-18T14:59:08.662499-04:00".
    }

    #[test]
    fn test_chrono_format_time_local_custom() {
        let fmt = ChronoLocal {
            format: ChronoFmtType::Custom("%a %b %e %T %Y".to_owned()),
        };
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&fmt, &mut dst).is_ok());
        // e.g. `buf` contains "Wed Aug 23 15:55:46 2023".
    }
}
