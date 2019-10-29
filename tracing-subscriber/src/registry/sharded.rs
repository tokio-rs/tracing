use sharded_slab::{Guard, Slab};

use crate::{
    fmt::span::SpanStack,
    registry::{LookupSpan, SpanData},
};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    convert::TryInto,
    fmt,
    sync::{
        atomic::{fence, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tracing_core::{
    dispatcher,
    field::{FieldSet, Visit},
    span::{self, Id},
    Dispatch, Event, Field, Interest, Metadata, Subscriber,
};

#[derive(Debug)]
pub struct Registry {
    spans: Arc<Slab<Data>>,
}

thread_local! {
    static CONTEXT: RefCell<SpanStack> = RefCell::new(SpanStack::new());
}

#[derive(Debug)]
pub struct Data {
    metadata: &'static Metadata<'static>,
    parent: Option<Id>,
    children: Vec<Id>,
    ref_count: AtomicUsize,
    // TODO(david): get rid of these
    values: Mutex<HashMap<&'static str, Value>>,
}

impl Drop for Data {
    fn drop(&mut self) {
        // We have to actually unpack the option inside the `get_default`
        // closure, since it is a `FnMut`, but testing that there _is_ a value
        // here lets us avoid the thread-local access if we don't need the
        // dispatcher at all.
        if self.parent.is_some() {
            dispatcher::get_default(|subscriber| {
                if let Some(parent) = self.parent.take() {
                    let _ = subscriber.try_close(parent);
                }
            })
        }
    }
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
    fn insert(&self, s: Data) -> Option<usize> {
        self.spans.insert(s)
    }

    fn get(&self, id: &Id) -> Option<Guard<'_, Data>> {
        self.spans.get(id_to_idx(id))
    }

    /// Decrements the reference count of the span with the given `id`, and
    /// removes the span if it is zero.
    ///
    /// The allocated span slot will be reused when a new span is created.
    fn drop_span(&self, id: Id) -> bool {
        if !self.drop_ref(&id) {
            return false;
        }

        // Synchronize only if we are actually removing the span (stolen
        // from std::Arc);
        fence(Ordering::Acquire);
        self.spans.remove(id_to_idx(&id))
    }

    fn drop_ref(&self, id: &Id) -> bool {
        let idx = id_to_idx(id);
        let span = self.spans.get(idx).expect("Span not found");

        let refs = span.ref_count.fetch_sub(1, Ordering::Relaxed);
        assert!(
            if std::thread::panicking() {
                // don't cause a double panic, even if the ref-count is wrong...
                true
            } else {
                refs != std::usize::MAX
            },
            "reference count overflow!"
        );
        refs == 1
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
        let parent = attrs.parent().map(|id| self.clone_span(id));
        let s = Data {
            metadata: attrs.metadata(),
            parent,
            children: vec![],
            ref_count: AtomicUsize::new(1),
            values: Mutex::new(values),
        };
        let id = self.insert(s).expect("Unable to allocate another span");
        idx_to_id(id)
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

    fn enter(&self, id: &span::Id) {
        CURRENT_SPANS.with(|spans| {
            spans.borrow_mut().push(id.clone());
            self.clone_span(id);
        })
    }

    fn exit(&self, id: &span::Id) {
        let _ = CURRENT_SPANS.with(|spans| spans.borrow_mut().pop(id));
        dispatcher::get_default(|dispatch| dispatch.try_close(id.clone()));
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        if let Some(data) = self.get(&id) {
            let refs = data.ref_count.fetch_add(1, Ordering::Release);
            if refs == 0 {
                panic!("tried to clone a span that already closed");
            }
        } else {
            if !std::thread::panicking() {
                panic!("tried to clone {:?}, but no span exists with that ID", id)
            }
        }
        id.clone()
    }

    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        self.drop_span(id)
    }
}

impl<'a> LookupSpan<'a> for Registry {
    type Data = Guard<'a, Data>;

    fn span_data(&'a self, id: &Id) -> Option<Self::Data> {
        self.get(id)
    }
}

// === impl Data ===

impl Data {
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
        F: FnMut(&Id, Guard<'_, Data>) -> Result<(), E>,
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

impl<'a> SpanData<'a> for Guard<'a, Data> {
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
