use sharded_slab::{Guard, Slab};

use super::stack::SpanStack;
use crate::{
    registry::{
        extensions::{Extensions, ExtensionsInner, ExtensionsMut},
        LookupSpan, SpanData,
    },
    sync::RwLock,
};
use std::{
    cell::{Cell, RefCell},
    sync::atomic::{fence, AtomicUsize, Ordering},
};
use tracing_core::{
    dispatcher::{self, Dispatch},
    span::{self, Current, Id},
    Event, Interest, Metadata, Subscriber,
};

/// A shared, reusable store for spans.
///
/// A `Registry` is a [`Subscriber`] around which multiple [`Layer`]s
/// implementing various behaviors may be [added]. Unlike other types
/// implementing `Subscriber` `Registry` does not actually record traces itself:
/// instead, it collects and stores span data that is exposed to any `Layer`s
/// wrapping it through implementations of the [`LookupSpan`] and
/// [`LookupMetadata`] traits. The `Registry` is responsible for storing span
/// metadata, recording relationships between spans, and tracking which spans
/// are active and whicb are closed. In addition, it provides a mechanism
/// `Layer`s to store user-defined per-span data, called [extensions], in the
/// registry. This allows `Layer`-specific data to benefit from the `Registry`'s
/// high-performance concurrent storage.
///
/// This registry is implemented using a [lock-free sharded slab][slab], and is
/// highly optimized for concurrent access.
///
/// [slab]: https://docs.rs/crate/sharded-slab/
/// [`Subscriber`]:
///     https://docs.rs/crate/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html
/// [`Layer`]: ../trait.Layer.html
/// [added]: ../trait.Layer.html#method.with_subscriber
/// [`LookupSpan`]: trait.LookupSpan.html
/// [`LookupMetadata`]: trait.LookupMetadata.html
/// [extensions]: extensions/index.html
#[derive(Debug)]
pub struct Registry {
    spans: Slab<DataInner>,
}

/// Span data stored in a [`Registry`].
///
/// The registry stores well-known data defined by tracing: span relationships,
/// metadata and reference counts. Additional user-defined data provided by
/// [`Layer`s], such as formatted fields, metrics, or distributed traces should
/// be stored in the [extensions] typemap.
///
/// [`Registry`]: struct.Registry.html
/// [`Layer`s]: ../trait.Layer.html
/// [extensions]: extensions/index.html
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

/// A guard that tracks how many [`Registry`]-backed `Layer`s have
/// processed an `on_close` event.
///
/// This is needed to enable a [`Registry`]-backed Layer to access span
/// data after the `Layer` has recieved the `on_close` callback.
///
/// Once all `Layer`s have processed this event, the [`Registry`] knows
/// that is able to safely remove the span tracked by `id`. `CloseGuard`
/// accomplishes this through a two-step process:
/// 1. Whenever a [`Registry`]-backed `Layer::on_close` method is
///    called, `Registry::start_close` is closed.
///    `Registry::start_close` increments a thread-local `CLOSE_COUNT`
///    by 1 and returns a `CloseGuard`.
/// 2. The `CloseGuard` is dropped at the end of `Layer::on_close`. On
///    drop, `CloseGuard` checks thread-local `CLOSE_COUNT`. If
///    `CLOSE_COUNT` is 0, the `CloseGuard` removes the span with the
///    `id` from the registry, as all `Layers` that might have seen the
///    `on_close` notification have processed it. If `CLOSE_COUNT` is
///    greater than 0, `CloseGuard` decrements the counter by one and
///    _does not_ remove the span from the [`Registry`].
///
/// [`Registry`]: ./struct.Registry.html
pub(crate) struct CloseGuard<'a> {
    id: Id,
    registry: &'a Registry,
    is_closing: bool,
}

impl Registry {
    fn insert(&self, s: DataInner) -> Option<usize> {
        self.spans.insert(s)
    }

    fn get(&self, id: &Id) -> Option<Guard<'_, DataInner>> {
        self.spans.get(id_to_idx(id))
    }

    /// Returns a guard which tracks how many `Layer`s have
    /// processed an `on_close` notification via the `CLOSE_COUNT` thread-local.
    /// For additional details, see [`CloseGuard`].
    ///
    /// [`CloseGuard`]: ./struct.CloseGuard.html
    pub(crate) fn start_close(&self, id: Id) -> CloseGuard<'_> {
        CLOSE_COUNT.with(|count| {
            let c = count.get();
            count.set(c + 1);
        });
        CloseGuard {
            id,
            registry: &self,
            is_closing: false,
        }
    }
}

thread_local! {
    /// `CLOSE_COUNT` is the thread-local counter used by `CloseGuard` to
    /// track how many layers have processed the close.
    /// For additional details, see [`CloseGuard`].
    ///
    /// [`CloseGuard`]: ./struct.CloseGuard.html
    static CLOSE_COUNT: Cell<usize> = Cell::new(0);
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

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

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
        let span = self
            .get(&id)
            .unwrap_or_else(|| panic!("tried to clone {:?}, but no span exists with that ID", id));
        // Like `std::sync::Arc`, adds to the ref count (on clone) don't require
        // a strong ordering; if we call` clone_span`, the reference count must
        // always at least 1. The only synchronization necessary is between
        // calls to `try_close`:  we have to ensure that all threads have
        // dropped their refs to the span before the span is closed.
        let refs = span.ref_count.fetch_add(1, Ordering::Relaxed);
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

        // Synchronize if we are actually removing the span (stolen
        // from std::Arc); this ensures that all other `try_close` calls on
        // other threads happen-before we actually remove the span.
        fence(Ordering::Acquire);
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
    // A span is not considered closed until all of its children have closed.
    // Therefore, each span's `DataInner` holds a "reference" to the parent
    // span, keeping the parent span open until all its children have closed.
    // When we close a span, we must then decrement the parent's ref count
    // (potentially, allowing it to close, if this child is the last reference
    // to that span).
    fn drop(&mut self) {
        // We have to actually unpack the option inside the `get_default`
        // closure, since it is a `FnMut`, but testing that there _is_ a value
        // here lets us avoid the thread-local access if we don't need the
        // dispatcher at all.
        if self.parent.is_some() {
            // Note that --- because `Layered::try_close` works by calling
            // `try_close` on the inner subscriber and using the return value to
            // determine whether to call the `Layer`'s `on_close` callback ---
            // we must call `try_close` on the entire subscriber stack, rather
            // than just on the registry. If the registry called `try_close` on
            // itself directly, the layers wouldn't see the close notification.
            let subscriber = dispatcher::get_default(Dispatch::clone);
            if let Some(parent) = self.parent.take() {
                let _ = subscriber.try_close(parent);
            }
        }
    }
}

impl<'a> CloseGuard<'a> {
    pub(crate) fn is_closing(&mut self) {
        self.is_closing = true;
    }
}

impl<'a> Drop for CloseGuard<'a> {
    fn drop(&mut self) {
        // If this returns with an error, we are already panicking. At
        // this point, there's nothing we can really do to recover
        // except by avoiding a double-panic.
        let _ = CLOSE_COUNT.try_with(|count| {
            let c = count.get();
            // Decrement the count to indicate that _this_ guard's
            // `on_close` callback has completed.
            //
            // Note that we *must* do this before we actually remove the span
            // from the registry, since dropping the `DataInner` may trigger a
            // new close, if this span is the last reference to a parent span.
            count.set(c - 1);

            // If the current close count is 1, this stack frame is the last
            // `on_close` call. If the span is closing, it's okay to remove the
            // span.
            if c == 1 && self.is_closing {
                self.registry.spans.remove(id_to_idx(&self.id));
            }
        });
    }
}

// === impl Data ===

impl<'a> SpanData<'a> for Data<'a> {
    fn id(&self) -> Id {
        idx_to_id(self.inner.key())
    }

    fn metadata(&self) -> &'static Metadata<'static> {
        (*self).inner.metadata
    }

    fn parent(&self) -> Option<&Id> {
        self.inner.parent.as_ref()
    }

    fn extensions(&self) -> Extensions<'_> {
        Extensions::new(self.inner.extensions.read().expect("Mutex poisoned"))
    }

    fn extensions_mut(&self) -> ExtensionsMut<'_> {
        ExtensionsMut::new(self.inner.extensions.write().expect("Mutex poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use super::Registry;
    use crate::{layer::Context, registry::LookupSpan, Layer};
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
    };
    use tracing::{self, subscriber::with_default};
    use tracing_core::{
        dispatcher,
        span::{Attributes, Id},
        Subscriber,
    };

    struct AssertionLayer;
    impl<S> Layer<S> for AssertionLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_close(&self, id: Id, ctx: Context<'_, S>) {
            dbg!(format_args!("closing {:?}", id));
            assert!(&ctx.span(&id).is_some());
        }
    }

    #[test]
    fn single_layer_can_access_closed_span() {
        let subscriber = AssertionLayer.with_subscriber(Registry::default());

        with_default(subscriber, || {
            let span = tracing::debug_span!("span");
            drop(span);
        });
    }

    #[test]
    fn multiple_layers_can_access_closed_span() {
        let subscriber = AssertionLayer
            .and_then(AssertionLayer)
            .with_subscriber(Registry::default());

        with_default(subscriber, || {
            let span = tracing::debug_span!("span");
            drop(span);
        });
    }

    struct CloseInOrder {
        inner: Mutex<VecDeque<&'static str>>,
        okay: Arc<AtomicBool>,
    }

    struct ClosingLayer {
        span1_removed: Arc<AtomicBool>,
        span2_removed: Arc<AtomicBool>,
    }

    impl<S> Layer<S> for ClosingLayer
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn new_span(&self, _: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
            let span = ctx.span(id).expect("Missing span; this is a bug");
            let is_removed = match dbg!(span.name()) {
                "span1" => self.span1_removed.clone(),
                "span2" => self.span2_removed.clone(),
                _ => return,
            };
            let mut extensions = span.extensions_mut();
            extensions.insert(ClosingSpan { is_removed });
        }

        fn on_close(&self, id: Id, ctx: Context<'_, S>) {
            dbg!(&id);
            assert!(&ctx.span(&id).is_some());
            let span = &ctx.span(&id).unwrap();
            match dbg!(span.name()) {
                "span1" | "span2" => {}
                _ => return,
            };
            let extensions = span.extensions();
            assert!(extensions.get::<ClosingSpan>().is_some());
        }
    }

    struct ClosingSpan {
        is_removed: Arc<AtomicBool>,
    }

    impl Drop for ClosingSpan {
        fn drop(&mut self) {
            self.is_removed.store(true, Ordering::Release)
        }
    }

    impl<S> Layer<S> for CloseInOrder
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        fn on_close(&self, id: Id, ctx: Context<'_, S>) {
            let span = &ctx.span(&id).unwrap();
            let name = span.name();
            if let Ok(mut lock) = self.inner.lock() {
                if let Some(next) = lock.pop_front() {
                    if next != name {
                        println!(
                            "expected span \"{}\" to close next, but \"{}\" closed instead!",
                            next, name
                        );
                        self.okay.store(false, Ordering::Release);
                    }
                } else {
                    println!(
                        "span \"{}\" closed when no more spans were expected to close",
                        name
                    );
                    self.okay.store(false, Ordering::Release);
                }
            }
        }
    }

    impl CloseInOrder {
        fn new(i: impl IntoIterator<Item = &'static str>) -> (Self, Arc<AtomicBool>) {
            let closes = i.into_iter().collect::<VecDeque<_>>();
            let inner = Mutex::new(closes);
            let okay = Arc::new(AtomicBool::new(true));
            let handle = okay.clone();
            (Self { inner, okay }, handle)
        }
    }

    #[test]
    fn spans_are_removed_from_registry() {
        let span1_removed = Arc::new(AtomicBool::new(false));
        let span2_removed = Arc::new(AtomicBool::new(false));

        let span1_removed2 = span1_removed.clone();
        let span2_removed2 = span2_removed.clone();

        let subscriber = AssertionLayer
            .and_then(ClosingLayer {
                span1_removed,
                span2_removed,
            })
            .with_subscriber(Registry::default());

        // Create a `Dispatch` (which is internally reference counted) so that
        // the subscriber lives to the end of the test. Otherwise, if we just
        // passed the subscriber itself to `with_default`, we could see the span
        // be dropped when the subscriber itself is dropped, destroying the
        // registry.
        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span = tracing::debug_span!("span1");
            drop(span);
            let span = tracing::info_span!("span2");
            drop(span);
        });

        assert!(span1_removed2.load(Ordering::Acquire));
        assert!(span2_removed2.load(Ordering::Acquire));

        // Ensure the registry itself outlives the span.
        drop(dispatch);
    }

    #[test]
    fn spans_are_only_closed_when_the_last_ref_drops() {
        let span1_removed = Arc::new(AtomicBool::new(false));
        let span2_removed = Arc::new(AtomicBool::new(false));

        let span1_removed2 = span1_removed.clone();
        let span2_removed2 = span2_removed.clone();

        let subscriber = AssertionLayer
            .and_then(ClosingLayer {
                span1_removed,
                span2_removed,
            })
            .with_subscriber(Registry::default());

        // Create a `Dispatch` (which is internally reference counted) so that
        // the subscriber lives to the end of the test. Otherwise, if we just
        // passed the subscriber itself to `with_default`, we could see the span
        // be dropped when the subscriber itself is dropped, destroying the
        // registry.
        let dispatch = dispatcher::Dispatch::new(subscriber);

        let span2 = dispatcher::with_default(&dispatch, || {
            let span = tracing::debug_span!("span1");
            drop(span);
            let span2 = tracing::info_span!("span2");
            let span2_clone = span2.clone();
            drop(span2);
            span2_clone
        });

        assert!(span1_removed2.load(Ordering::Acquire));
        assert!(!span2_removed2.load(Ordering::Acquire));

        drop(span2);
        assert!(span2_removed2.load(Ordering::Acquire));

        // Ensure the registry itself outlives the span.
        drop(dispatch);
    }

    #[test]
    fn span_enter_guards_are_dropped_out_of_order() {
        let span1_removed = Arc::new(AtomicBool::new(false));
        let span2_removed = Arc::new(AtomicBool::new(false));

        let subscriber = AssertionLayer
            .and_then(ClosingLayer {
                span1_removed: span1_removed.clone(),
                span2_removed: span2_removed.clone(),
            })
            .with_subscriber(Registry::default());

        // Create a `Dispatch` (which is internally reference counted) so that
        // the subscriber lives to the end of the test. Otherwise, if we just
        // passed the subscriber itself to `with_default`, we could see the span
        // be dropped when the subscriber itself is dropped, destroying the
        // registry.
        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span1 = tracing::debug_span!("span1");
            let span2 = tracing::info_span!("span2");

            let enter1 = span1.enter();
            let enter2 = span2.enter();

            drop(enter1);
            drop(span1);

            assert!(span1_removed.load(Ordering::Acquire));
            assert!(!span2_removed.load(Ordering::Acquire));

            drop(enter2);
            drop(span2);
            assert!(span2_removed.load(Ordering::Acquire));
        });
    }

    #[test]
    fn child_closes_parent() {
        let span1_removed = Arc::new(AtomicBool::new(false));
        let span2_removed = Arc::new(AtomicBool::new(false));
        let (order_layer, closed_in_order) = CloseInOrder::new(vec!["span2", "span1"]);

        let subscriber = order_layer
            .and_then(ClosingLayer {
                span1_removed: span1_removed.clone(),
                span2_removed: span2_removed.clone(),
            })
            .with_subscriber(Registry::default());

        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span1 = tracing::info_span!("span1");
            let span2 = tracing::info_span!(parent: &span1, "span2");

            assert!(!span1_removed.load(Ordering::Acquire));
            assert!(!span2_removed.load(Ordering::Acquire));

            drop(span1);

            assert!(
                !span1_removed.load(Ordering::Acquire),
                "span1 must not have closed yet (span2 is keeping it open)"
            );
            assert!(!span2_removed.load(Ordering::Acquire));

            drop(span2);

            assert!(span2_removed.load(Ordering::Acquire));
            assert!(span1_removed.load(Ordering::Acquire));
        });

        assert!(
            closed_in_order.load(Ordering::Acquire),
            "spans closed out of order!"
        );
    }

    #[test]
    fn child_closes_grandparent() {
        let span1_removed = Arc::new(AtomicBool::new(false));
        let span2_removed = Arc::new(AtomicBool::new(false));
        let (order_layer, closed_in_order) = CloseInOrder::new(vec!["span3", "span2", "span1"]);

        let subscriber = order_layer
            .and_then(ClosingLayer {
                span1_removed: span1_removed.clone(),
                span2_removed: span2_removed.clone(),
            })
            .with_subscriber(Registry::default());

        let dispatch = dispatcher::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatch, || {
            let span1 = tracing::info_span!("span1");
            let span2 = tracing::info_span!(parent: &span1, "span2");
            let span3 = tracing::info_span!(parent: &span2, "span3");

            assert!(!span1_removed.load(Ordering::Acquire));
            assert!(!span2_removed.load(Ordering::Acquire));

            drop(span1);
            drop(span2);

            assert!(
                !span1_removed.load(Ordering::Acquire),
                "span1 must not have closed yet (span2 is keeping it open)"
            );
            assert!(
                !span2_removed.load(Ordering::Acquire),
                "span2 must not have closed yet (span3 is keeping it open)"
            );

            drop(span3);

            assert!(
                span2_removed.load(Ordering::Acquire),
                "closing span3 should have closed span2"
            );
            assert!(
                span1_removed.load(Ordering::Acquire),
                "closing span2 should have closed span1"
            );
        });

        assert!(
            closed_in_order.load(Ordering::Acquire),
            "spans closed out of order!"
        );
    }
}
