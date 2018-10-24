use tokio_trace::{
    span,
    subscriber::{AddValueError, Subscriber},
    Event, IntoValue, Meta, SpanData,
};
use {filter::NoFilter, observe::NoObserver, Filter, Observe, RegisterSpan};

#[derive(Debug, Clone)]
pub struct Composed<F, O, R> {
    filter: F,
    observer: O,
    registry: R,
}

impl Composed<NoFilter, NoObserver, ()> {
    /// Returns a new instance of `Composed` which can be built using the
    /// [`with_filter`], [`with_observer`], and [`with_registry`] methods.
    pub fn builder() -> Self {
        Composed {
            filter: NoFilter,
            observer: NoObserver,
            registry: (),
        }
    }
}

impl<O, R> Composed<NoFilter, O, R> {
    /// Sets the [filter] to be used by the composed `Subscriber`.
    ///
    /// [filter]: ../trait.Filter.html
    pub fn with_filter<F>(self, filter: F) -> Composed<F, O, R>
    where
        F: Filter,
    {
        Composed {
            filter,
            observer: self.observer,
            registry: self.registry,
        }
    }
}

impl<F, R> Composed<F, NoObserver, R> {
    /// Sets the [observer] to be used by the composed `Subscriber`.
    ///
    /// [observer]: ../trait.Observe.html
    pub fn with_observer<O>(self, observer: O) -> Composed<F, O, R>
    where
        O: Observe,
    {
        Composed {
            filter: self.filter,
            observer,
            registry: self.registry,
        }
    }
}

impl<F, O> Composed<F, O, ()> {
    /// Sets the [span registry] to be used by the composed `Subscriber`.
    ///
    /// [span registry]: ../trait.Register.html
    pub fn with_registry<R>(self, registry: R) -> Composed<F, O, R>
    where
        R: RegisterSpan,
    {
        Composed {
            filter: self.filter,
            observer: self.observer,
            registry,
        }
    }
}

impl<F, O, R> Composed<F, O, R> {
    /// Construct a new composed `Subscriber`, given a [filter], an
    /// [observer], and a [span registry].
    ///
    /// [filter]: ../trait.Filter.html
    /// [observer]: ../trait.Observe.html
    /// [span registry]: ../trait.Register.html
    pub fn new(filter: F, observer: O, registry: R) -> Self {
        Composed {
            filter,
            observer,
            registry,
        }
    }
}

impl<F, O, R> Subscriber for Composed<F, O, R>
where
    F: Filter,
    O: Observe,
    R: RegisterSpan,
{
    fn enabled(&self, metadata: &Meta) -> bool {
        self.filter.enabled(metadata) && self.observer.filter().enabled(metadata)
    }

    fn should_invalidate_filter(&self, metadata: &Meta) -> bool {
        self.filter.should_invalidate_filter(metadata)
            || self.observer.filter().should_invalidate_filter(metadata)
    }

    fn new_span(&self, new_span: SpanData) -> span::Id {
        self.registry.new_span(new_span)
    }

    fn add_value(
        &self,
        span: &span::Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError> {
        self.registry.add_value(span, name, value)
    }

    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        self.observer.observe_event(event)
    }

    fn enter(&self, id: span::Id, state: span::State) {
        self.registry.with_span(&id, state, |span| {
            self.observer.enter(span);
        });
    }

    fn exit(&self, id: span::Id, state: span::State) {
        self.registry.with_span(&id, state, |span| {
            self.observer.exit(span);
        });
    }
}
