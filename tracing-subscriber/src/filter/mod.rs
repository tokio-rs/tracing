pub mod field;
pub mod level;
pub use self::level::LevelFilter;
mod directive;
pub use self::directive::ParseError;

use crate::{
    layer::{Context, Layer},
    thread,
};
use crossbeam_utils::sync::ShardedLock;
use std::collections::HashMap;
use tracing_core::{
    callsite,
    field::Field,
    span,
    subscriber::{Interest, Subscriber},
    Metadata,
};

pub struct Filter {
    // TODO: eventually, this should be exposed by the registry.
    scope: thread::Local<Vec<LevelFilter>>,

    statics: directive::Statics,
    dynamics: directive::Dynamics,

    by_id: ShardedLock<HashMap<span::Id, directive::SpanMatch>>,
    by_cs: ShardedLock<HashMap<callsite::Identifier, directive::CallsiteMatch>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Directive {
    target: Option<String>,
    in_span: Option<String>,
    // TODO: this can probably be a `SmallVec` someday, since a span won't have
    // over 32 fields.
    fields: Vec<field::Match>,
    level: LevelFilter,
}

type FieldMap<T> = HashMap<Field, T>;

// enum MatchResult {
//     Static(Interest),
//     Dynamic(SpanMatch),
// }

impl Filter {
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
        if self.statics.enabled(metadata) {
            return Interest::always();
        }

        if let Some(matcher) = self.dynamics.matcher(metadata) {
            let mut by_cs = self.by_cs.write().unwrap();
            let _i = by_cs.insert(metadata.callsite(), matcher);
            debug_assert_eq!(_i, None, "register_callsite called twice since reset");
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
