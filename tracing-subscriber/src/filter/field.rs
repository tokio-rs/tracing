use matchers::Pattern;
use std::{error::Error, fmt, str::FromStr};
use tracing_core::field::Field;

#[derive(Debug)]
pub struct Match {
    pub(crate) name: String, // TODO: allow match patterns for names?
    pub(crate) value: Option<ValueMatch>,
}

#[derive(Debug)]
pub struct Keyed {
    pub(crate) field: Field,
    pub(crate) value: ValueMatch,
}

#[derive(Debug)]
pub(crate) enum ValueMatch {
    Bool(bool),
    U64(u64),
    I64(i64),
    Pat(Pattern),
}

#[derive(Clone, Debug)]
struct BadName {
    name: String,
}

// === impl Match ===

impl FromStr for Match {
    type Err = Box<dyn Error + Send + Sync>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('=');
        let name = parts
            .next()
            .ok_or_else(|| BadName {
                name: "".to_string(),
            })?
            // TODO: validate field name
            .to_string();
        let value = parts.next().map(ValueMatch::from_str).transpose()?;
        Ok(Match { name, value })
    }
}

// === impl ValueMatch ===

impl FromStr for ValueMatch {
    type Err = matchers::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<bool>()
            .map(ValueMatch::Bool)
            .or_else(|_| s.parse::<u64>().map(ValueMatch::U64))
            .or_else(|_| s.parse::<i64>().map(ValueMatch::I64))
            .or_else(|_| s.parse::<Pattern>().map(ValueMatch::Pat))
    }
}

impl Error for BadName {}

impl fmt::Display for BadName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid field name `{}`", self.name)
    }
}
