use matchers::Pattern;
use std::{error::Error, fmt, str::FromStr};
use tracing_core::field::Field;

#[derive(Debug, Eq, PartialEq)]
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

impl Match {
    pub(crate) fn is_dynamic(&self) -> bool {
        self.value.is_some()
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

impl PartialEq<Self> for ValueMatch {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueMatch::Bool(a), ValueMatch::Bool(b)) => a == b,
            (ValueMatch::I64(a), ValueMatch::I64(b)) => a == b,
            (ValueMatch::U64(a), ValueMatch::U64(b)) => a == b,
            // TODO: T_T
            (ValueMatch::Pat(_), ValueMatch::Pat(_)) => false,
            (_, _) => false,
        }
    }
}

impl Eq for ValueMatch {}

// === impl BadName ===
impl Error for BadName {}

impl fmt::Display for BadName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid field name `{}`", self.name)
    }
}
