pub mod field;
pub mod level;
pub mod span;
pub use self::level::LevelFilter;

use crate::thread;
use crossbeam_utils::sync::ShardedLock;
use std::collections::HashMap;
use tracing_core::{callsite, subscriber::Interest, Metadata};

pub struct Filter {
    scope: thread::Local<Vec<LevelFilter>>,

    targets: Vec<(String, LevelFilter)>,
    dynamic: Vec<Directive>,

    by_cs: ShardedLock<HashMap<callsite::Identifier, span::Match>>,
}

#[derive(Debug)]
pub struct Directive {
    target: Option<String>,
    in_span: Option<String>,
    // TODO: this can probably be a `SmallVec` someday, since a span won't have
    // over 32 fields.
    fields: Vec<field::Match>,
    level: LevelFilter,
}

enum MatchResult {
    Static(Interest),
    Dynamic(span::Match),
}

impl Directive {
    pub fn is_dynamic(&self) -> bool {
        self.in_span.is_some() || !self.fields.is_empty()
    }
}

impl Filter {
    fn dyn_directives<'a>(
        &'a self,
        metadata: &'a Metadata<'a>,
    ) -> impl Iterator<Item = &'a Directive> + 'a {
        self.dynamic
            .iter()
            .rev()
            .filter(move |d| d.cares_about(metadata))
    }
}

impl Directive {
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
}
