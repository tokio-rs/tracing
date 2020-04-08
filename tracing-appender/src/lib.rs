use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};

pub mod file_appender;

mod inner;
mod worker;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Rotation(RotationKind);

#[derive(Clone, Eq, PartialEq, Debug)]
enum RotationKind {
    Hourly,
    Daily,
    Never,
}

impl Rotation {
    pub const HOURLY: Self = Self(RotationKind::Hourly);
    pub const DAILY: Self = Self(RotationKind::Daily);
    pub const NEVER: Self = Self(RotationKind::Never);

    fn next_date(&self, current_date: &DateTime<Utc>) -> DateTime<Utc> {
        let unrounded_next_date = match *self {
            Rotation::HOURLY => *current_date + chrono::Duration::hours(1),
            Rotation::DAILY => *current_date + chrono::Duration::days(1),
            Rotation::NEVER => Utc.ymd(9999, 1, 1).and_hms(1, 0, 0),
        };
        self.round_date(&unrounded_next_date)
    }

    fn round_date(&self, date: &DateTime<Utc>) -> DateTime<Utc> {
        match *self {
            Rotation::HOURLY => {
                Utc.ymd(date.year(), date.month(), date.day())
                    .and_hms(date.hour(), 0, 0)
            }
            Rotation::DAILY => Utc
                .ymd(date.year(), date.month(), date.day())
                .and_hms(0, 0, 0),
            Rotation::NEVER => {
                Utc.ymd(date.year(), date.month(), date.day())
                    .and_hms(date.hour(), 0, 0)
            }
        }
    }

    fn join_date(&self, filename: &str, date: &DateTime<Utc>) -> String {
        match *self {
            Rotation::HOURLY => format!("{}.{}", filename, date.format("%F-%H")),
            Rotation::DAILY => format!("{}.{}", filename, date.format("%F")),
            Rotation::NEVER => filename.to_string(),
        }
    }
}
