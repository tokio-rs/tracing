pub mod field;
pub mod level;
pub use self::level::LevelFilter;
mod directive;
use self::directive::Directive;
pub use self::directive::ParseError;

use crate::{
    layer::{Context, Layer},
    thread,
};
use crossbeam_utils::sync::ShardedLock;
use std::{collections::HashMap, env, error::Error, fmt, str::FromStr};
use tracing_core::{
    callsite,
    field::Field,
    span,
    subscriber::{Interest, Subscriber},
    Metadata,
};

#[derive(Debug)]
pub struct Filter {
    // TODO: eventually, this should be exposed by the registry.
    scope: thread::Local<Vec<LevelFilter>>,

    statics: directive::Statics,
    dynamics: directive::Dynamics,

    by_id: ShardedLock<HashMap<span::Id, directive::SpanMatcher>>,
    by_cs: ShardedLock<HashMap<callsite::Identifier, directive::CallsiteMatcher>>,
}

type FieldMap<T> = HashMap<Field, T>;

#[cfg(feature = "smallvec")]
type FilterVec<T> = smallvec::SmallVec<[T; 8]>;
#[cfg(not(feature = "smallvec"))]
type FilterVec<T> = Vec<T>;

#[derive(Debug)]
pub struct FromEnvError {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    Parse(ParseError),
    Env(env::VarError),
}

impl Filter {
    pub const DEFAULT_ENV: &'static str = "RUST_LOG";

    /// Returns a new `Filter` from the value of the `RUST_LOG` environment
    /// variable, ignoring any invalid filter directives.
    pub fn from_default_env() -> Self {
        Self::from_env(Self::DEFAULT_ENV)
    }

    /// Returns a new `Filter` from the value of the given environment
    /// variable, ignoring any invalid filter directives.
    pub fn from_env<A: AsRef<str>>(env: A) -> Self {
        env::var(env.as_ref()).map(Self::new).unwrap_or_default()
    }

    /// Returns a new `Filter` from the directives in the given string,
    /// ignoring any that are invalid.
    pub fn new<S: AsRef<str>>(dirs: S) -> Self {
        let directives = dirs.as_ref().split(',').filter_map(|s| match s.parse() {
            Ok(d) => Some(d),
            Err(err) => {
                eprintln!("ignoring `{}`: {}", s, err);
                None
            }
        });
        Self::from_directives(directives)
    }

    /// Returns a new `Filter` from the directives in the given string,
    /// or an error if any are invalid.
    pub fn try_new<S: AsRef<str>>(dirs: S) -> Result<Self, ParseError> {
        let directives = dirs
            .as_ref()
            .split(',')
            .map(|s| s.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::from_directives(directives))
    }

    /// Returns a new `Filter` from the value of the `RUST_LOG` environment
    /// variable, or an error if the environment variable contains any invalid
    /// filter directives.
    pub fn try_from_default_env() -> Result<Self, FromEnvError> {
        Self::try_from_env(Self::DEFAULT_ENV)
    }

    /// Returns a new `Filter` from the value of the given environment
    /// variable, or an error if the environment variable is unset or contains
    /// any invalid filter directives.
    pub fn try_from_env<A: AsRef<str>>(env: A) -> Result<Self, FromEnvError> {
        env::var(env.as_ref())?.parse().map_err(Into::into)
    }

    fn from_directives(directives: impl IntoIterator<Item = Directive>) -> Self {
        let (dynamics, mut statics) = Directive::make_tables(directives);

        if statics.is_empty() && dynamics.is_empty() {
            statics.add(directive::StaticDirective::default());
        }

        Self {
            scope: thread::Local::new(),
            statics,
            dynamics,
            by_id: ShardedLock::new(HashMap::new()),
            by_cs: ShardedLock::new(HashMap::new()),
        }
    }

    fn cares_about_span(&self, span: &span::Id) -> bool {
        let spans = try_lock!(self.by_id.read(), else return false);
        spans.contains_key(span)
    }

    fn base_interest(&self) -> Interest {
        if self.dynamics.is_empty() {
            Interest::never()
        } else {
            Interest::sometimes()
        }
    }
}

impl<S: Subscriber> Layer<S> for Filter {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if metadata.is_span() {
            // If this metadata describes a span, first, check if there is a
            // dynamic filter that should be constructed for it. If so, it
            // should always be enabled, since it influences filtering.
            if let Some(matcher) = self.dynamics.matcher(metadata) {
                let mut by_cs = self.by_cs.write().unwrap();
                let _i = by_cs.insert(metadata.callsite(), matcher);
                debug_assert_eq!(_i, None, "register_callsite called twice since reset");
                return Interest::always();
            }
        }

        // Otherwise, check if any of our static filters enable this metadata.
        if self.statics.enabled(metadata) {
            Interest::always()
        } else {
            self.base_interest()
        }
    }

    fn enabled(&self, metadata: &Metadata, _: Context<S>) -> bool {
        let level = metadata.level();
        for filter in self.scope.get().iter() {
            if filter >= level {
                return true;
            }
        }

        // TODO: other filters...

        false
    }

    fn new_span(&self, attrs: &span::Attributes, id: &span::Id, _: Context<S>) {
        let by_cs = self.by_cs.read().unwrap();
        if let Some(cs) = by_cs.get(&attrs.metadata().callsite()) {
            let span = cs.to_span_match(attrs);
            self.by_id.write().unwrap().insert(id.clone(), span);
        }
    }

    fn on_record(&self, id: &span::Id, values: &span::Record, _: Context<S>) {
        if let Some(span) = self.by_id.read().unwrap().get(id) {
            span.record_update(values);
        }
    }

    fn on_enter(&self, id: &span::Id, _: Context<S>) {
        // XXX: This is where _we_ could push IDs to the stack instead, and use
        // that to allow changing the filter while a span is already entered.
        // But that might be much less efficient...
        if let Some(span) = try_lock!(self.by_id.read()).get(id) {
            self.scope.get().push(span.level());
        }
    }

    fn on_exit(&self, id: &span::Id, _: Context<S>) {
        if self.cares_about_span(id) {
            self.scope.get().pop();
        }
    }

    fn on_close(&self, id: span::Id, _: Context<S>) {
        // If we don't need to acquire a write lock, avoid doing so.
        if !self.cares_about_span(&id) {
            return;
        }

        let mut spans = try_lock!(self.by_id.write());
        spans.remove(&id);
    }
}

impl FromStr for Filter {
    type Err = ParseError;

    fn from_str(spec: &str) -> Result<Self, Self::Err> {
        Self::try_new(spec)
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::from_directives(std::iter::empty())
    }
}

// ===== impl FromEnvError =====

impl From<ParseError> for FromEnvError {
    fn from(p: ParseError) -> Self {
        Self {
            kind: ErrorKind::Parse(p),
        }
    }
}

impl From<env::VarError> for FromEnvError {
    fn from(v: env::VarError) -> Self {
        Self {
            kind: ErrorKind::Env(v),
        }
    }
}

impl fmt::Display for FromEnvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Parse(ref p) => p.fmt(f),
            ErrorKind::Env(ref e) => e.fmt(f),
        }
    }
}

impl Error for FromEnvError {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::Parse(ref p) => p.description(),
            ErrorKind::Env(ref e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.kind {
            ErrorKind::Parse(ref p) => Some(p),
            ErrorKind::Env(ref e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_core::field::FieldSet;
    use tracing_core::*;

    struct NoSubscriber;
    impl Subscriber for NoSubscriber {
        #[inline]
        fn register_callsite(&self, _: &'static Metadata<'static>) -> subscriber::Interest {
            subscriber::Interest::always()
        }
        fn new_span(&self, _: &span::Attributes) -> span::Id {
            span::Id::from_u64(0xDEAD)
        }
        fn event(&self, _event: &Event) {}
        fn record(&self, _span: &span::Id, _values: &span::Record) {}
        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

        #[inline]
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true
        }
        fn enter(&self, _span: &span::Id) {}
        fn exit(&self, _span: &span::Id) {}
    }

    struct Cs;
    impl Callsite for Cs {
        fn set_interest(&self, _interest: Interest) {}
        fn metadata(&self) -> &Metadata {
            unimplemented!()
        }
    }

    #[test]
    fn callsite_enabled_no_span_directive() {
        let filter = Filter::new("app=debug").with_subscriber(NoSubscriber);
        static META: &'static Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&[], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(META);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_off() {
        let filter = Filter::new("app=off").with_subscriber(NoSubscriber);
        static META: &'static Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::ERROR,
            None,
            None,
            None,
            FieldSet::new(&[], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(&META);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_enabled_includes_span_directive() {
        let filter = Filter::new("app[mySpan]=debug").with_subscriber(NoSubscriber);
        static META: &'static Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&[], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(&META);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_enabled_includes_span_directive_field() {
        let filter =
            Filter::new("app[mySpan{field=\"value\"}]=debug").with_subscriber(NoSubscriber);
        static META: &'static Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&["field"], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(&META);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_enabled_includes_span_directive_multiple_fields() {
        let filter = Filter::new("app[mySpan{field=\"value\",field2=2}]=debug")
            .with_subscriber(NoSubscriber);
        static META: &'static Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&["field"], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(&META);
        assert!(interest.is_never());
    }
}
