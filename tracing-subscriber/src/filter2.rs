use crate::layer::{Filter, Layer, Layered, Context};
use std::{
    any::TypeId,
    mem,
    sync::atomic::{AtomicUsize, Ordering},
    marker::PhantomData,
};
// use tracing_core::{
    use tracing_core::{Event, span};
    use tracing_core::subscriber::{Interest, Subscriber};
// };
use tracing_core::Metadata;
pub trait Filterable {
    fn register_filter(&mut self) -> FilterId;
    fn disable_span(&self, filter: FilterId, span: &span::Id);
    fn is_enabled_by(&self, filter: FilterId, span: &span::Id) -> bool;
}

pub struct Filtered<F, L, S> {
    pub(crate) layer: L,
    // next: I,
    pub(crate) filter: F,
    pub(crate) id: FilterId,
    pub(crate) _s: PhantomData<S>,
}

impl<S, F, L> Layer<S> for Filtered<F, L, S>
where
    S: Subscriber + Filterable,
    F: Filter<S> + 'static,
    L: Layer<S> + 'static
{
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata, Context::none()) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: Context<'_, S>) -> bool {
        let _ = (metadata, ctx);
        true
    }

    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {}
    // fn on_record(&self, _span: &span::Id, _values: &span::Record<'_>, _ctx: Context<'_, S>) {}

    // fn on_follows_from(&self, _span: &span::Id, _follows: &span::Id, _ctx: Context<'_, S>) {}

    // fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {}

    // fn on_enter(&self, _id: &span::Id, _ctx: Context<'_, S>) {}

    // fn on_exit(&self, _id: &span::Id, _ctx: Context<'_, S>) {}

    // fn on_close(&self, _id: span::Id, _ctx: Context<'_, S>) {}

    // fn on_id_change(&self, _old: &span::Id, _new: &span::Id, _ctx: Context<'_, S>) {}

    fn and_then<L2>(self, layer: L2) -> Layered<L2, Self, S>
    where
        L2: Layer<S>,
        Self: Sized,
    {
        unimplemented!()
    }

    fn with_subscriber(self, mut inner: S) -> Layered<Self, S>
    where
        Self: Sized,
    {
        let id = inner.register_filter();
        Layered {
            layer: Self { id, ..self },
            inner,
            _s: PhantomData,
        }
    }

    #[doc(hidden)]
    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        match id {
            id if id == TypeId::of::<Self>() => Some(self as *const _ as *const ()),
            id if id == TypeId::of::<F>() => Some(&self.filter as *const _ as *const ()),
            _ => self.layer.downcast_raw(id),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FilterId(pub(crate) u8);

#[derive(Default)]
pub(crate) struct FilterMap {
    bitmap: [u8; Self::SHARDS],
}

impl FilterMap {
    pub const MAX_FILTERS: usize = 8 * Self::SHARDS;
    const SHARDS: usize = 32;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn disable(&mut self, FilterId(filter): FilterId) {
        let idx = filter as usize / 8;
        let bit = 1 << (filter % Self::SHARDS as u8);
        self.bitmap[idx] |= bit;
    }

    pub fn is_enabled(&self, FilterId(filter): FilterId) -> bool {
        let idx = filter as usize / 8;
        let bit = 1 << (filter % Self::SHARDS as u8);
        self.bitmap[idx] & bit == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::tests::NopLayer;
    use crate::prelude::*;

    struct NopFilterable {
        next_id: u8,
    }

    impl Subscriber for NopFilterable {
        fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
            Interest::never()
        }

        fn enabled(&self, _: &Metadata<'_>) -> bool {
            false
        }

        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(1)
        }

        fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
        fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &span::Id) {}
        fn exit(&self, _: &span::Id) {}
    }

    impl Filterable for NopFilterable {
        fn register_filter(&mut self) -> FilterId {
            let id = self.next_id;
            self.next_id += 1;
            FilterId(id)
        }

        fn disable_span(&self, filter: FilterId, span: &span::Id) {
            unimplemented!()
        }
        fn is_enabled_by(&self, filter: FilterId, span: &span::Id) -> bool {
            unimplemented!()
        }
    }

    struct NopFilter1;

    impl<S: Subscriber + Filterable> Filter<S> for NopFilter1 {
        /// Filter on a specific span or event's metadata.
        fn filter(&self, metadata: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
            true
        }
    }
    struct NopFilter2;

    impl<S: Subscriber + Filterable> Filter<S> for NopFilter2 {
        /// Filter on a specific span or event's metadata.
        fn filter(&self, metadata: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
            true
        }
    }

    #[test]
    fn with_id_generation() {
        let subscriber = NopFilterable { next_id: 0 }
            .with(NopLayer.with_filter(NopFilter1))
            .with(NopLayer.with_filter(NopFilter2));

        assert_eq!(
            subscriber.layer.id,
            FilterId(1)
        );
        assert_eq!(
            subscriber.inner.layer.id,
            FilterId(0)
        );
    }
}
