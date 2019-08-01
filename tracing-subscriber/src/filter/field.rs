use matchers::Pattern;
use std::{
    error::Error,
    fmt,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
};

use super::{FieldMap, LevelFilter};
use tracing_core::field::{Field, Visit};

#[derive(Debug, Eq, PartialEq)]
pub struct Match {
    pub(crate) name: String, // TODO: allow match patterns for names?
    pub(crate) value: Option<ValueMatch>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CallsiteMatch {
    pub(crate) fields: FieldMap<ValueMatch>,
    pub(crate) level: LevelFilter,
}

#[derive(Debug)]
pub struct SpanMatch {
    fields: FieldMap<(ValueMatch, AtomicBool)>,
    level: LevelFilter,
    has_matched: AtomicBool,
}

pub struct MatchVisitor<'a> {
    inner: &'a SpanMatch,
}

#[derive(Debug, Clone)]
pub(crate) enum ValueMatch {
    Bool(bool),
    U64(u64),
    I64(i64),
    Pat(Pattern),
}

#[derive(Clone, Debug)]
pub struct BadName {
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
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    // TODO: reference count these strings?
    pub fn name(&self) -> String {
        self.name.clone()
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
            // XXX: we cannot easily implement `PartialEq` for patterns, since the
            // `regex_automata::DenseDFA` types that represent them internally
            // do not implement `PartialEq`. having them never be equal is not
            // technically correct, but it shouldn't matter in practice.
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

impl CallsiteMatch {
    pub fn to_span_match(&self) -> SpanMatch {
        let fields = self
            .fields
            .iter()
            .map(|(k, v)| (k.clone(), (v.clone(), AtomicBool::new(false))))
            .collect();
        SpanMatch {
            fields,
            level: self.level.clone(),
            has_matched: AtomicBool::new(false),
        }
    }
}

impl SpanMatch {
    pub fn visitor<'a>(&'a self) -> MatchVisitor<'a> {
        MatchVisitor { inner: self }
    }

    pub fn level(&self) -> LevelFilter {
        self.level.clone()
    }

    pub fn is_matched(&self) -> bool {
        if self.has_matched.load(Ordering::Acquire) {
            return true;
        }
        self.is_matched_slow()
    }

    fn is_matched_slow(&self) -> bool {
        let matched = self
            .fields
            .values()
            .all(|(_, matched)| matched.load(Ordering::Acquire));
        if matched {
            self.has_matched.store(true, Ordering::Release);
        }
        matched
    }

    pub fn filter(&self) -> Option<LevelFilter> {
        if self.is_matched() {
            Some(self.level.clone())
        } else {
            None
        }
    }
}

impl<'a> Visit for MatchVisitor<'a> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        use std::convert::TryInto;

        match self.inner.fields.get(field) {
            Some((ValueMatch::I64(ref e), ref matched)) if value == *e => {
                matched.store(true, Ordering::Release);
            }
            Some((ValueMatch::U64(ref e), ref matched)) if Ok(value) == (*e).try_into() => {
                matched.store(true, Ordering::Release);
            }
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::U64(ref e), ref matched)) if value == *e => {
                matched.store(true, Ordering::Release);
            }
            _ => {}
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::Bool(ref e), ref matched)) if value == *e => {
                matched.store(true, Ordering::Release);
            }
            _ => {}
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::Pat(ref e), ref matched)) if e.matches(&value) => {
                matched.store(true, Ordering::Release);
            }
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::Pat(ref e), ref matched)) if e.debug_matches(&value) => {
                matched.store(true, Ordering::Release);
            }
            _ => {}
        }
    }
}
