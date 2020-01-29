use matchers::Pattern;
use std::{
    cmp::Ordering,
    error::Error,
    fmt,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering::*},
        Arc,
    },
};

use super::{FieldMap, LevelFilter, TargetFilter};
use tracing_core::field::{Field, Visit};

#[derive(Debug, Eq, PartialEq, Ord)]
pub(crate) struct Match {
    pub(crate) name: String, // TODO: allow match patterns for names?
    pub(crate) value: Option<ValueMatch>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CallsiteMatch {
    pub(crate) fields: FieldMap<ValueMatch>,
    pub(crate) level: LevelFilter,
    pub(crate) target: TargetFilter,
}

#[derive(Debug)]
pub(crate) struct SpanMatch {
    fields: FieldMap<(ValueMatch, AtomicBool)>,
    level: LevelFilter,
    target: TargetFilter,
    has_matched: AtomicBool,
}

pub(crate) struct MatchVisitor<'a> {
    inner: &'a SpanMatch,
}

#[derive(Debug, Clone, PartialOrd, Ord, Eq, PartialEq)]
pub(crate) enum ValueMatch {
    Bool(bool),
    U64(u64),
    I64(i64),
    Pat(Box<MatchPattern>),
}

#[derive(Debug, Clone)]
pub(crate) struct MatchPattern {
    pub(crate) matcher: Pattern,
    pattern: Arc<str>,
}

/// Indicates that a field name specified in a filter directive was invalid.
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
    pub(crate) fn has_value(&self) -> bool {
        self.value.is_some()
    }

    // TODO: reference count these strings?
    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)?;
        if let Some(ref value) = self.value {
            write!(f, "={}", value)?;
        }
        Ok(())
    }
}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Ordering for `Match` directives is based first on _whether_ a value
        // is matched or not. This is semantically meaningful --- we would
        // prefer to check directives that match values first as they are more
        // specific.
        let has_value = match (self.value.as_ref(), other.value.as_ref()) {
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            _ => Ordering::Equal,
        };
        // If both directives match a value, we fall back to the field names in
        // length + lexicographic ordering, and if these are equal as well, we
        // compare the match directives.
        //
        // This ordering is no longer semantically meaningful but is necessary
        // so that the directives can be stored in the `BTreeMap` in a defined
        // order.
        Some(
            has_value
                .then_with(|| self.name.cmp(&other.name))
                .then_with(|| self.value.cmp(&other.value)),
        )
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
            .or_else(|_| {
                s.parse::<MatchPattern>()
                    .map(|p| ValueMatch::Pat(Box::new(p)))
            })
    }
}

impl fmt::Display for ValueMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueMatch::Bool(ref inner) => fmt::Display::fmt(inner, f),
            ValueMatch::I64(ref inner) => fmt::Display::fmt(inner, f),
            ValueMatch::U64(ref inner) => fmt::Display::fmt(inner, f),
            ValueMatch::Pat(ref inner) => fmt::Display::fmt(inner, f),
        }
    }
}

// === impl MatchPattern ===

impl FromStr for MatchPattern {
    type Err = matchers::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let matcher = s.parse::<Pattern>()?;
        Ok(Self {
            matcher,
            pattern: s.to_owned().into(),
        })
    }
}

impl fmt::Display for MatchPattern {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.pattern, f)
    }
}

impl AsRef<str> for MatchPattern {
    #[inline]
    fn as_ref(&self) -> &str {
        self.pattern.as_ref()
    }
}

impl MatchPattern {
    #[inline]
    fn str_matches(&self, s: &impl AsRef<str>) -> bool {
        self.matcher.matches(s)
    }

    #[inline]
    fn debug_matches(&self, d: &impl fmt::Debug) -> bool {
        self.matcher.debug_matches(d)
    }
}

impl PartialEq for MatchPattern {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}

impl Eq for MatchPattern {}

impl PartialOrd for MatchPattern {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.pattern.cmp(&other.pattern))
    }
}

impl Ord for MatchPattern {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.pattern.cmp(&other.pattern)
    }
}

// === impl BadName ===

impl Error for BadName {}

impl fmt::Display for BadName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid field name `{}`", self.name)
    }
}

impl CallsiteMatch {
    pub(crate) fn to_span_match(&self) -> SpanMatch {
        let fields = self
            .fields
            .iter()
            .map(|(k, v)| (k.clone(), (v.clone(), AtomicBool::new(false))))
            .collect();
        SpanMatch {
            fields,
            level: self.level.clone(),
            has_matched: AtomicBool::new(false),
            target: self.target.clone(),
        }
    }
}

impl SpanMatch {
    pub(crate) fn visitor(&self) -> MatchVisitor<'_> {
        MatchVisitor { inner: self }
    }

    #[inline]
    pub(crate) fn is_matched(&self) -> bool {
        if self.has_matched.load(Acquire) {
            return true;
        }
        self.is_matched_slow()
    }

    #[inline(never)]
    fn is_matched_slow(&self) -> bool {
        let matched = self
            .fields
            .values()
            .all(|(_, matched)| matched.load(Acquire));
        if matched {
            self.has_matched.store(true, Release);
        }
        matched
    }

    #[inline]
    pub(crate) fn filter(&self) -> Option<LevelFilter> {
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
                matched.store(true, Release);
            }
            Some((ValueMatch::U64(ref e), ref matched)) if Ok(value) == (*e).try_into() => {
                matched.store(true, Release);
            }
            _ => {}
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::U64(ref e), ref matched)) if value == *e => {
                matched.store(true, Release);
            }
            _ => {}
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::Bool(ref e), ref matched)) if value == *e => {
                matched.store(true, Release);
            }
            _ => {}
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::Pat(ref e), ref matched)) if e.str_matches(&value) => {
                matched.store(true, Release);
            }
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match self.inner.fields.get(field) {
            Some((ValueMatch::Pat(ref e), ref matched)) if e.debug_matches(&value) => {
                matched.store(true, Release);
            }
            _ => {}
        }
    }
}
