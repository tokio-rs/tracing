use tokio_trace::{
    field,
    span::{self, Id},
    subscriber::Subscriber,
    Event, Metadata,
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
        F: Filter + 'static,
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
        O: Observe + 'static,
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
        R: RegisterSpan + 'static,
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
    F: Filter + 'static,
    O: Observe + 'static,
    R: RegisterSpan + 'static,
{
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.filter.enabled(metadata) && self.observer.filter().enabled(metadata)
    }

    fn new_span(&self, attrs: &span::Attributes) -> Id {
        self.registry.new_id(attrs)
    }

    fn record(&self, _span: &Id, _values: &span::Record) {
        unimplemented!()
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.registry.add_follows_from(span, follows)
    }

    fn event(&self, _event: &Event) {
        unimplemented!()
    }

    fn enter(&self, id: &Id) {
        self.registry.with_span(id, |span| {
            self.observer.enter(span);
        });
    }

    fn exit(&self, id: &Id) {
        self.registry.with_span(id, |span| {
            self.observer.exit(span);
        });
    }

    fn clone_span(&self, id: &Id) -> Id {
        self.registry.clone_span(id)
    }

    fn drop_span(&self, id: Id) {
        self.registry.drop_span(id)
    }
}
