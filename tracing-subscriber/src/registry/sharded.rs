use sharded_slab::{Guard, Slab};

use crate::{
    fmt::span::SpanStack,
    registry::{extensions::Extensions, LookupMetadata, LookupSpan, SpanData},
    sync::RwLock,
};
use std::{
    cell::RefCell,
    sync::atomic::{fence, AtomicUsize, Ordering},
};
use tracing_core::{
    dispatcher,
    field::FieldSet,
    span::{self, Current, Id},
    Event, Interest, Metadata, Subscriber,
};

#[derive(Debug)]
pub struct Registry {
    spans: Slab<Data>,
}

#[derive(Debug)]
pub struct Data {
    metadata: &'static Metadata<'static>,
    parent: Option<Id>,
    children: Vec<Id>,
    ref_count: AtomicUsize,
    extensions: RwLock<Extensions>,
}

// === impl Registry ===

impl Default for Registry {
    fn default() -> Self {
        Self { spans: Slab::new() }
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
        let parent = attrs.parent().map(|id| self.clone_span(id));
        let s = Data {
            metadata: attrs.metadata(),
            parent,
            children: vec![],
            ref_count: AtomicUsize::new(1),
            extensions: RwLock::new(Extensions::new()),
        };
        let id = self.insert(s).expect("Unable to allocate another span");
        idx_to_id(id)
    }

    #[inline]
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {
        // TODO(david)—implement this.
    }

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {
        // TODO(david)—implement this.
    }

    fn event(&self, _: &Event<'_>) {
        // TODO(david)—implement this.
    }

    fn enter(&self, id: &span::Id) {
        CURRENT_SPANS.with(|spans| {
            spans.borrow_mut().push(self.clone_span(id));
        })
    }

    fn exit(&self, id: &span::Id) {
        if let Some(id) = CURRENT_SPANS.with(|spans| spans.borrow_mut().pop(id)) {
            dispatcher::get_default(|dispatch| dispatch.try_close(id.clone()));
        }
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        let refs = self
            .get(&id)
            .unwrap_or_else(|| panic!("tried to clone {:?}, but no span exists with that ID", id))
            .ref_count
            .fetch_add(1, Ordering::Relaxed);
        assert!(refs != 0, "tried to clone a span that already closed");
        id.clone()
    }

    fn current_span(&self) -> Current {
        CURRENT_SPANS
            .with(|spans| {
                let spans = spans.borrow();
                let id = spans.current()?;
                let span = self.get(id)?;
                Some(Current::new(id.clone(), span.metadata))
            })
            .unwrap_or_else(Current::none)
    }

    /// Decrements the reference count of the span with the given `id`, and
    /// removes the span if it is zero.
    ///
    /// The allocated span slot will be reused when a new span is created.
    #[inline]
    fn try_close(&self, id: span::Id) -> bool {
        let span = match self.get(&id) {
            Some(span) => span,
            None if std::thread::panicking() => return false,
            None => panic!("tried to drop a ref to {:?}, but no such span exists!", id),
        };

        let refs = span.ref_count.fetch_sub(1, Ordering::Release);
        if !std::thread::panicking() {
            assert!(refs < std::usize::MAX, "reference count overflow!");
        }
        if refs > 1 {
            return false;
        }

        // Synchronize only if we are actually removing the span (stolen
        // from std::Arc);
        fence(Ordering::Acquire);
        self.spans.remove(id_to_idx(&id));
        true
    }
}

impl<'a> LookupSpan<'a> for Registry {
    type Data = Guard<'a, Data>;

    fn span_data(&'a self, id: &Id) -> Option<Self::Data> {
        self.get(id)
    }

    // TODO(david): move this somewhere more appropriate; rewrite in terms of `SpanData`.
    fn visit_parents<E, F>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&Id) -> Result<(), E>,
    {
        CURRENT_SPANS.with(|spans| {
            let spans = spans.borrow();
            if let Some(id) = spans.current().clone() {
                // TODO(david): make this a less unpleasant loop.
                let mut span = self.span(&id);
                loop {
                    if let Some(s) = span {
                        let id = s.id();
                        f(&id);
                        span = s.parent()
                    } else {
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}

impl LookupMetadata for Registry {
    fn metadata(&self, id: &Id) -> Option<&'static Metadata<'static>> {
        if let Some(span) = self.get(id) {
            Some(span.metadata())
        } else {
            None
        }
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

impl<'a> SpanData<'a> for Guard<'a, Data> {
    type Children = std::slice::Iter<'a, Id>; // not yet implemented...
    type Follows = std::slice::Iter<'a, Id>;

    fn id(&self) -> Id {
        idx_to_id(self.idx())
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
