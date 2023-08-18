use crate::fmt::format::Writer;
use crate::fmt::time::FormatTime;

/// Formats [local time]s and [UTC time]s with [formatter] implementations
/// that use the [`chrono` crate].
///
/// [local time]: https://docs.rs/chrono/0.4.26/chrono/offset/struct.Local.html
/// [UTC time]: https://docs.rs/chrono/0.4.26/chrono/offset/struct.Utc.html
/// [`chrono` crate]: https://docs.rs/chrono/0.4.26/chrono/
/// [formatter]: https://docs.rs/time/0.3/time/formatting/trait.Formattable.html

/// Tag-type (indicating UTC timezone) enabling static dispatch
/// to `chrono::Local` functions.
#[derive(Debug)]
pub struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        w.write_str(&chrono::Local::now().to_rfc3339())
    }
}

/// Tag-type (indicating the "local" timezone) enabling static
/// dispatch to `chrono::Utc` functions.
#[derive(Debug)]
pub struct Utc;

impl FormatTime for Utc {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        w.write_str(&chrono::Utc::now().to_rfc3339())
    }
}

#[cfg(test)]
mod tests {
    use crate::fmt::format::Writer;
    use crate::fmt::time::FormatTime;

    use super::LocalTime;
    use super::Utc;

    #[test]
    fn test_chrono_format_time_utc() {
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&Utc, &mut dst).is_ok());
        // e.g. `buf` contains "2023-08-18T19:05:08.662499+00:00"
    }

    #[test]
    fn test_chrono_format_time_local() {
        let mut buf = String::new();
        let mut dst: Writer<'_> = Writer::new(&mut buf);
        assert!(FormatTime::format_time(&LocalTime, &mut dst).is_ok());
        // e.g. `buf` contains "2023-08-18T14:59:08.662499-04:00".
    }
}
