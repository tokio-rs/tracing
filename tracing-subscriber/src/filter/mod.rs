pub mod field;
pub mod level;
pub use self::level::LevelFilter;

use crate::{thread, layer::{Layer, Context}};
use crossbeam_utils::sync::ShardedLock;
use std::{cmp::Ordering, collections::HashMap, iter::FromIterator};
use tracing_core::{callsite, subscriber::{Interest, Subscriber}, Level, Metadata, span, Event, field::{Field, FieldMap}};

pub struct Filter {
    // TODO: eventually, this should be exposed by the registry.
    scope: thread::Local<Vec<LevelFilter>>,

    statics: Statics,
    dynamic: Dynamics,

    by_id: ShardedLock<HashMap<span::Id, LevelFilter>>,
    by_cs: ShardedLock<HashMap<callsite::Identifier, SpanMatch>>,
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

#[derive(Debug, PartialEq, Eq)]
struct SpanMatch {
    fields: FieldMap<(field::ValueMatch, AtomicBool)>,
    level: LevelFilter,
}

#[derive(Debug, PartialEq, Eq, Ord)]
struct StaticDirective {
    target: Option<String>,
    field_names: Vec<String>,
    level: LevelFilter,
}

#[derive(Debug)]
struct Dynamics {
    spans: Vec<Directive>,
    max_level: LevelFilter,
    // can_disable: bool,
}

#[derive(Debug)]
struct Statics {
    directives: Vec<StaticDirective>,
    max_level: LevelFilter,
}

enum MatchResult {
    Static(Interest),
    Dynamic(SpanMatch),
}

impl Filter {
    fn cares_about_span(&self, span: &span::Id) -> bool {
        let spans = try_lock!(self.by_id.read(), else return false);
        spans.contains_key(span)
    }
}

impl<S: Subscriber> Layer<S> for Filter {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        unimplemented!()
    }

    fn enabled(&self, metadata: &Metadata, _: Context<S>) -> bool {
        let level = metadata.level();
        for filter in self.scope.iter() {
            if filter >= level {
                return true;
            }
        }

        // TODO: other filters...

        false
    }

    fn new_span(&self, attrs: &span::Attributes, id: &span::Id, ctx: Context<S>) {
        unimplemented!()
    }

    fn on_record(&self, span: &span::Id, values: &span::Record, ctx: Context<S>) {
        unimplemented!()
    }

    #[inline]
    fn on_follows_from(&self, span: &span::Id, follows: &span::Id, ctx: Context<S>) {
        unimplemented!()
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        unimplemented!()
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<S>) {
        if let Some(level) = try_lock!(self.by_id.read()).get(id) {
            self.scope.get().push(level.clone());
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

impl Directive {
    fn has_name(&self) -> bool {
        self.in_span.is_some()
    }

    fn has_target(&self) -> bool {
        self.target.is_some()
    }

    fn has_fields(&self) -> bool {
        self.fields.is_empty()
    }

    fn into_static(self) -> Option<StaticDirective> {
        if self.is_dynamic() {
            None
        } else {
            let field_names = self
                .fields
                .into_iter()
                .map(|f| {
                    debug_assert!(f.value.is_none());
                    f.name
                })
                .collect();
            Some(StaticDirective {
                target: self.target,
                field_names,
                level: self.level,
            })
        }
    }

    fn is_dynamic(&self) -> bool {
        self.has_name() || !self.fields.iter().any(field::Match::is_dynamic)
    }

    fn cares_about(&self, meta: &Metadata) -> bool {
        // Does this directive have a target filter, and does it match the
        // metadata's target?
        if let Some(ref target) = self.target.as_ref() {
            // If so,
            if !meta.target().starts_with(&target[..]) {
                return false;
            }
        }

        // Do we have a name filter, and does it match the metadata's name?
        // TODO: put name globbing here?
        if let Some(ref name) = self.in_span {
            if name != meta.name() {
                return false;
            }
        }

        let fields = meta.fields();
        for field in &self.fields {
            if !fields.field(&field.name).is_some() {
                return false;
            }
        }

        true
    }

    fn make_tables(directives: impl IntoIterator<Item = Directive>) -> (Dynamics, Statics) {
        let (dyns, stats): (Vec<Directive>, Vec<Directive>) =
            directives.into_iter().partition(Directive::is_dynamic);
        (Dynamics::from_iter(dyns), Statics::from_iter(stats))
    }
}

// === impl Dynamics ===

impl Dynamics {
    fn directives_for<'a>(
        &'a self,
        metadata: &'a Metadata<'a>,
    ) -> impl Iterator<Item = &'a Directive> + 'a {
        self.spans
            .iter()
            .rev()
            .filter(move |d| d.cares_about(metadata))
    }
}

impl Default for Dynamics {
    fn default() -> Self {
        Self {
            spans: Vec::new(),
            max_level: LevelFilter::OFF,
            // can_disable: false,
        }
    }
}

impl FromIterator<Directive> for Dynamics {
    fn from_iter<I: IntoIterator<Item = Directive>>(iter: I) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

impl Extend<Directive> for Dynamics {
    fn extend<I: IntoIterator<Item = Directive>>(&mut self, iter: I) {
        let max_level = &mut self.max_level;
        // let can_disable = &mut self.can_disable;
        let ds = iter.into_iter().filter(Directive::is_dynamic).inspect(|d| {
            if &d.level > &*max_level {
                *max_level = d.level.clone();
            }
        });
        self.spans.extend(ds);
        self.spans.sort_unstable();
    }
}

impl PartialOrd for Directive {
    fn partial_cmp(&self, other: &Directive) -> Option<Ordering> {
        match (self.has_name(), other.has_name()) {
            (true, false) => return Some(Ordering::Greater),
            (false, true) => return Some(Ordering::Less),
            _ => {}
        }

        match (self.fields.len(), other.fields.len()) {
            (a, b) if a == b => {}
            (a, b) => return Some(a.cmp(&b)),
        }

        match (self.target.as_ref(), other.target.as_ref()) {
            (Some(a), Some(b)) => Some(a.len().cmp(&b.len())),
            (Some(_), None) => Some(Ordering::Greater),
            (None, Some(_)) => Some(Ordering::Less),
            (None, None) => Some(Ordering::Equal),
        }
    }
}

impl Ord for Directive {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .expect("Directive::partial_cmp should never return `None`")
    }
}

// === impl Statics ===

impl Default for Statics {
    fn default() -> Self {
        Self {
            directives: Vec::new(),
            max_level: LevelFilter::OFF,
        }
    }
}

impl Extend<Directive> for Statics {
    fn extend<I: IntoIterator<Item = Directive>>(&mut self, iter: I) {
        let max_level = &mut self.max_level;
        let ds = iter
            .into_iter()
            .filter_map(Directive::into_static)
            .inspect(|d| {
                if &d.level > &*max_level {
                    *max_level = d.level.clone();
                }
            });
        self.directives.extend(ds);
        self.directives.sort_unstable();
    }
}

impl FromIterator<Directive> for Statics {
    fn from_iter<I: IntoIterator<Item = Directive>>(iter: I) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

impl PartialOrd for StaticDirective {
    fn partial_cmp(&self, other: &StaticDirective) -> Option<Ordering> {
        match (self.field_names.len(), other.field_names.len()) {
            (a, b) if a == b => {}
            (a, b) => return Some(a.cmp(&b)),
        }

        match (self.target.as_ref(), other.target.as_ref()) {
            (Some(a), Some(b)) => Some(a.len().cmp(&b.len())),
            (Some(_), None) => Some(Ordering::Greater),
            (None, Some(_)) => Some(Ordering::Less),
            (None, None) => Some(Ordering::Equal),
        }
    }
}
