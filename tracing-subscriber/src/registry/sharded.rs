use sharded_slab::{Guard, Slab};

use crate::{
    fmt::span::SpanStack,
    registry::{LookupSpan, SpanData},
    sync::RwLock,
};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    convert::TryInto,
    fmt,
    sync::{Arc, Mutex},
};
use tracing_core::{
    field::{FieldSet, Visit},
    span::{self, Id},
    Event, Field, Interest, Metadata, Subscriber,
};

#[derive(Debug)]
pub struct Registry {
    spans: Arc<Slab<BigSpan>>,
}

thread_local! {
    static CONTEXT: RefCell<SpanStack> = RefCell::new(SpanStack::new());
}

#[derive(Debug)]
pub struct BigSpan {
    metadata: &'static Metadata<'static>,
    parent: Option<Id>,
    children: Vec<Id>,
    // TODO(david): get rid of these
    values: Mutex<HashMap<&'static str, Value>>,
}

// === impl Registry ===

impl Default for Registry {
    fn default() -> Self {
        Self {
            spans: Arc::new(Slab::new()),
        }
    }
}

struct RegistryVisitor<'a>(&'a mut HashMap<&'static str, Value>);

impl<'a> Visit for RegistryVisitor<'a> {
    // TODO: special visitors for various formats that honeycomb.io supports
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let s = format!("{:?}", value);
        self.0.insert(field.name(), json!(s));
    }
}

#[inline]
fn idx_to_id(idx: usize) -> Id {
    Id::from_u64(idx as u64 + 1)
}

#[inline]
fn id_to_idx(id: &Id) -> usize {
    id.into_u64() as usize - 1
}

impl Registry {
    fn insert(&self, s: BigSpan) -> Option<usize> {
        self.spans.insert(s)
    }

    fn get(&self, id: &Id) -> Option<Guard<'_, BigSpan>> {
        self.spans.get(id_to_idx(id))
    }

    fn set_parent(&self, current: &Id, parent: &Id) {
        // TODO(david): remove `.expect()`; return an error.
        let current_span = self
            .spans
            .get(id_to_idx(current))
            .expect(&format!("Missing current span with id: {:?}", current));
        let parent_span = self
            .spans
            .get(id_to_idx(parent))
            .expect(&format!("Missing parent span with id: {:?}", parent));

        // current_span.parent = Some(parent.clone());
        // parent_span.children.push(current.clone())
    }

    fn take(&self, id: Id) -> Option<BigSpan> {
        self.spans.take(id_to_idx(&id))
    }
}

thread_local! {
    static CURRENT_SPANS: RefCell<SpanStack> = RefCell::new(SpanStack::new());
}

impl Subscriber for Registry {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    #[inline]
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
        let mut values = HashMap::new();
        let mut visitor = RegistryVisitor(&mut values);
        attrs.record(&mut visitor);
        let parent = attrs.parent().map(|id| id.clone());
        let s = BigSpan {
            metadata: attrs.metadata(),
            parent,
            children: vec![],
            values: Mutex::new(values),
        };
        let id = (self.insert(s).expect("Unable to allocate another span") + 1) as u64;
        Id::from_u64(id.try_into().unwrap())
    }

    #[inline]
    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        dbg!(span);
        dbg!(values);
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        let current = self
            .get(span)
            .expect("Current span is missing; this is a bug");

        let current = self
            .span(span)
            .expect("Current span is missing; this is a bug");
    }

    fn enter(&self, id: &span::Id) {
        let id = id.into_u64();
        CURRENT_SPANS.with(|spans| {
            spans.borrow_mut().push(span::Id::from_u64(id));
        })
    }

    fn event(&self, event: &Event<'_>) {
        let id = match event.parent() {
            Some(id) => Some(id.clone()),
            None => {
                if event.is_contextual() {
                    CURRENT_SPANS.with(|spans| spans.borrow().current().map(|id| id.clone()))
                } else {
                    None
                }
            }
        };
        if let Some(id) = id {
            let mut values = HashMap::new();
            let mut visitor = RegistryVisitor(&mut values);
            event.record(&mut visitor);
            let _ = self.get(&id).expect("Missing parent span for event");
        }
    }

    fn exit(&self, id: &span::Id) {
        CURRENT_SPANS.with(|spans| {
            spans.borrow_mut().pop(id);
        })
    }

    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        let _ = self.take(id);
        true
    }
}

impl<'a> LookupSpan<'a> for Registry {
    type Data = Guard<'a, BigSpan>;

    fn span_data(&'a self, id: &Id) -> Option<Self::Data> {
        self.get(id)
    }
}

// === impl BigSpan ===

impl BigSpan {
    pub fn name(&self) -> &'static str {
        self.metadata.name()
    }

    pub fn fields(&self) -> &FieldSet {
        self.metadata.fields()
    }

    #[inline(always)]
    fn with_parent<'registry, F, E>(
        &self,
        my_id: &Id,
        last_id: Option<&Id>,
        f: &mut F,
        registry: &'registry Registry,
    ) -> Result<(), E>
    where
        F: FnMut(&Id, Guard<'_, BigSpan>) -> Result<(), E>,
    {
        if let Some(span) = registry.get(my_id) {
            if let Some(ref parent_id) = span.parent {
                if Some(parent_id) != last_id {
                    if let Some(parent) = registry.get(&parent_id) {
                        parent.with_parent(&parent_id, Some(my_id), f, registry)?;
                    } else {
                        panic!("missing span for {:?}; this is a bug", parent_id);
                    }
                }
            }
            if let Some(span) = registry.get(my_id) {
                f(my_id, span);
            }
        }
        Ok(())
    }
}

impl<'a> SpanData<'a> for Guard<'a, BigSpan> {
    type Children = std::slice::Iter<'a, Id>; // not yet implemented...
    type Follows = std::slice::Iter<'a, Id>;

    fn id(&self) -> Id {
        let id: u64 = self.idx().try_into().unwrap();
        Id::from_u64(id)
    }
    fn metadata(&self) -> &'static Metadata<'static> {
        (*self).metadata
    }
    fn parent(&self) -> Option<&Id> {
        self.parent.as_ref()
    }
    fn children(&'a self) -> Self::Children {
        self.children.iter()
    }
    fn follows_from(&self) -> Self::Follows {
        unimplemented!("david: add this to `BigSpan`")
    }
}
