use sharded_slab::{Guard, Slab};

use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt, io,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct Registry {
    spans: Arc<Slab<BigSpan>>,
    local_spans: RwLock<SpanStack>,
}

#[derive(Debug)]
pub struct BigSpan {
    metadata: &'static Metadata<'static>,
    values: Mutex<HashMap<&'static str, Value>>,
    events: Mutex<Vec<BigEvent>>,
}

// === impl Registry ===

impl Default for Registry {
    fn default() -> Self {
        Self {
            spans: Arc::new(Slab::new()),
            local_spans: RwLock::new(SpanStack::new()),
        }
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

    fn get(&self, id: &Id) -> Option<Guard<BigSpan>> {
        self.spans.get(id_to_idx(id))
    }

    fn take(&self, id: Id) -> Option<BigSpan> {
        self.spans.take(id_to_idx(&id))
    }
}

thread_local! {
    static CONTEXT: RefCell<SpanStack> = RefCell::new(SpanStack::new());
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
        let s = BigSpan {
            metadata: attrs.metadata(),
            values: Mutex::new(values),
            events: Mutex::new(vec![]),
        };
        let id = (self.insert(s).expect("Unable to allocate another span") + 1) as u64;
        Id::from_u64(id.try_into().unwrap())
    }

    #[inline]
    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        dbg!(span);
        dbg!(values);
        // self.spans.record(span, values, &self.fmt_fields)
        // unimplemented!()
    }

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
        // TODO: implement this please
    }

    fn enter(&self, id: &span::Id) {
        let id = id.into_u64();
        self.local_spans
            .write()
            .expect("Mutex poisoned")
            .push(span::Id::from_u64(id));
    }

    fn event(&self, event: &Event<'_>) {
        let id = match event.parent() {
            Some(id) => Some(id.clone()),
            None => {
                if event.is_contextual() {
                    self.local_spans
                        .read()
                        .expect("Mutex poisoned")
                        .current()
                        .map(|id| id.clone())
                } else {
                    None
                }
            }
        };
        if let Some(id) = id {
            let mut values = HashMap::new();
            let mut visitor = RegistryVisitor(&mut values);
            event.record(&mut visitor);
            let span = self.get(&id).expect("Missing parent span for event");
            let event = BigEvent {
                parent: id,
                metadata: event.metadata(),
                values,
            };
            span.events.lock().expect("Mutex poisoned").push(event);
        }
    }

    fn exit(&self, id: &span::Id) {
        self.local_spans.write().expect("Mutex poisoned").pop(id);
    }

    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        let _ = self.take(id);
        true
    }
}

impl<'a> LookupSpan<'a> for Registry {
    type Span = Guard<'a, BigSpan>;

    fn span(&'a self, id: &Id) -> Option<Self::Span> {
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
            if let Some(parent_id) = span.parent() {
                if Some(parent_id) != last_id {
                    if let Some(parent) = registry.get(parent_id) {
                        parent.with_parent(parent_id, Some(my_id), f, registry)?;
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
