use crate::fmt::format::Writer;
use crate::fmt::time::FormatTime;

/// Formats [local time]s and [UTC time]s with `FormatTime` implementations
/// that use the [`chrono` crate].
///
/// [local time]: [`chrono::offset::Local`]
/// [UTC time]: [`chrono::offset::Utc`]
/// [`chrono` crate]: [`chrono`]

/// Retrieve and print the current local time.
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct ChronoLocal;

#[cfg(feature = "chrono")]
impl FormatTime for ChronoLocal {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        w.write_str(&chrono::Local::now().to_rfc3339())
    }
}

/// Retrieve and print the current UTC time.
#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct ChronoUtc;

#[cfg(feature = "chrono")]
impl FormatTime for ChronoUtc {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        w.write_str(&chrono::Utc::now().to_rfc3339())
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::format::Writer;
    use crate::fmt::time::FormatTime;

    #[cfg(feature = "chrono")]
    use super::ChronoLocal;
    #[cfg(feature = "chrono")]
    use super::ChronoUtc;

    #[cfg(feature = "chrono")]
    #[test]
    fn test_chrono_format_time_utc() {
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&ChronoUtc, &mut dst).is_ok());
        // e.g. `buf` contains "2023-08-18T19:05:08.662499+00:00"
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_chrono_format_time_local() {
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&ChronoLocal, &mut dst).is_ok());
        // e.g. `buf` contains "2023-08-18T14:59:08.662499-04:00".
    }
}
