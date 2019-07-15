use matchers::Pattern;
use std::{error::Error, fmt, str::FromStr};

pub struct FieldMatch {
    name: String, // TODO: allow match patterns for names?
    value: Option<ValueMatch>,
}

enum ValueMatch {
    Bool(bool),
    U64(u64),
    I64(i64),
    Pat(Pattern),
}

#[derive(Clone, Debug)]
struct BadName {
    name: String,
}

// === impl FieldMatch ===

impl FromStr for FieldMatch {
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
        Ok(FieldMatch { name, value })
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
