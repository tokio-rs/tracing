use crate::fmt::{format::Writer, time::FormatTime};
use chrono::Local;

pub struct LocalTime;

impl FormatTime for LocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        w.write_str(&Local::now().to_rfc3339())
    }
}

pub struct Utc;

impl FormatTime for Utc {
    fn format_time(&self, w: &mut Writer<'_>) -> alloc::fmt::Result {
        w.write_str(&chrono::Utc::now().to_rfc3339())
    }
}
