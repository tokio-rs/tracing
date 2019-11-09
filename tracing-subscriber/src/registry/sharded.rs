use sharded_slab::{Guard, Slab};

use crate::{
    registry::{
        extensions::{Extensions, ExtensionsInner, ExtensionsMut},
        LookupMetadata, LookupSpan, SpanData,
    },
    sync::RwLock,
};
use super::stack::SpanStack;
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

/// A shared, reusable store for spans.
///
/// This registry is implemented using a [lock-free sharded slab][1], and is
/// highly optimized for concurrent access.
///
/// [1]: https://docs.rs/crate/sharded-slab/0.0.5
#[derive(Debug)]
pub struct Registry {
    spans: Slab<DataInner>,
}

/// Span data stored in a [`Registry`].
///
/// This definition  is intentionally kept small—only data pertaining to span
/// relationships, span metadata, and active references are stored. Additional
/// data, such as formatted fields, may be stored in the [extensions] typemap.
///
/// [`Registry`]: ../struct.Registry.html
/// [extensions]: ../extensions/index.html
#[derive(Debug)]
pub struct Data<'a> {
    inner: Guard<'a, DataInner>,
}

#[derive(Debug)]
struct DataInner {
    metadata: &'static Metadata<'static>,
    parent: Option<Id>,
    ref_count: AtomicUsize,
    pub(crate) extensions: RwLock<ExtensionsInner>,
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
    fn insert(&self, s: DataInner) -> Option<usize> {
        self.spans.insert(s)
    }

    fn get(&self, id: &Id) -> Option<Guard<'_, DataInner>> {
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
        let parent = if attrs.is_root() {
            None
        } else if attrs.is_contextual() {
            self.current_span().id().map(|id| self.clone_span(id))
        } else {
            attrs.parent().map(|id| self.clone_span(id))
        };

        let s = DataInner {
            metadata: attrs.metadata(),
            parent,
            ref_count: AtomicUsize::new(1),
            extensions: RwLock::new(ExtensionsInner::new()),
        };
        let id = self.insert(s).expect("Unable to allocate another span");
        idx_to_id(id)
    }

    /// This is intentionally not implemented, as recording fields
    /// on a span is the responsibility of layers atop of this registry.
    #[inline]
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {}

    /// This is intentionally not implemented, as recording events
    /// is the responsibility of layers atop of this registry.
    fn event(&self, _: &Event<'_>) {}

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
    type Data = Data<'a>;

    fn span_data(&'a self, id: &Id) -> Option<Self::Data> {
        let inner = self.get(id)?;
        Some(Data { inner })
    }
}

// === impl DataInner ===

impl Drop for DataInner {
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

// === impl Data ===

impl<'a> Data<'a> {
    /// Gets the name of a span.
    pub fn name(&self) -> &'static str {
        self.inner.metadata.name()
    }

    /// Gets the fields of a span.
    pub fn fields(&self) -> &FieldSet {
        self.inner.metadata.fields()
    }
}

impl<'a> SpanData<'a> for Data<'a> {
    type Follows = std::slice::Iter<'a, Id>;

    fn id(&self) -> Id {
        idx_to_id(self.inner.key())
    }

    fn metadata(&self) -> &'static Metadata<'static> {
        (*self).inner.metadata
    }

    fn parent(&self) -> Option<&Id> {
        self.inner.parent.as_ref()
    }

    fn follows_from(&self) -> Self::Follows {
        unimplemented!("david: add this to `BigSpan`")
    }

    fn extensions(&self) -> Extensions<'_> {
        Extensions::new(self.inner.extensions.read().expect("Mutex poisoned"))
    }

    fn extensions_mut(&self) -> ExtensionsMut<'_> {
        ExtensionsMut::new(self.inner.extensions.write().expect("Mutex poisoned"))
    }
}
