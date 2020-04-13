use crate::registry::{self, LookupSpan};
use std::any::TypeId;
use tracing_core::{span, subscriber::Interest, Event, Metadata};

/// A type-erased [`Subscriber`].
pub struct Subscriber(Box<dyn tracing_core::Subscriber + Send + Sync + 'static>);

/// A type-erased [`Subscriber`] that implements [`LookupSpan`].
pub struct Registry<D>(Box<dyn for<'a> ErasableRegistry<'a, Data = D> + Send + Sync + 'static>);

pub fn subscriber(s: impl tracing_core::Subscriber + Send + Sync + 'static) -> Subscriber {
    Subscriber(Box::new(s))
}

pub fn registry<R, D>(r: R) -> Registry<D>
where
    R: tracing_core::Subscriber + Send + Sync + 'static,
    R: for<'a> LookupSpan<'a, Data = D>,
{
    Registry(Box::new(r))
}

impl tracing_core::Subscriber for Subscriber {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.0.register_callsite(metadata)
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.0.enabled(metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        self.0.new_span(span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.0.record(span, values)
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.0.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event<'_>) {
        self.0.event(event)
    }

    fn enter(&self, span: &span::Id) {
        self.0.enter(span)
    }

    fn exit(&self, span: &span::Id) {
        self.0.exit(span)
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.0.clone_span(id)
    }

    fn try_close(&self, id: span::Id) -> bool {
        self.0.try_close(id)
    }

    fn current_span(&self) -> span::Current {
        self.0.current_span()
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        self.0.downcast_raw(id)
    }
}

impl<D: for<'a> registry::SpanData<'a> + 'static> tracing_core::Subscriber for Registry<D> {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.0.register_callsite(metadata)
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.0.enabled(metadata)
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        self.0.new_span(span)
    }

    fn record(&self, span: &span::Id, values: &span::Record<'_>) {
        self.0.record(span, values)
    }

    fn record_follows_from(&self, span: &span::Id, follows: &span::Id) {
        self.0.record_follows_from(span, follows)
    }

    fn event(&self, event: &Event<'_>) {
        self.0.event(event)
    }

    fn enter(&self, span: &span::Id) {
        self.0.enter(span)
    }

    fn exit(&self, span: &span::Id) {
        self.0.exit(span)
    }

    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.0.clone_span(id)
    }

    fn try_close(&self, id: span::Id) -> bool {
        self.0.try_close(id)
    }

    fn current_span(&self) -> span::Current {
        self.0.current_span()
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        self.0.downcast_raw(id)
    }
}

impl<'data, D: for<'a> registry::SpanData<'a>> LookupSpan<'data> for Registry<D> {
    type Data = D;

    fn span_data(&'data self, id: &span::Id) -> Option<Self::Data> {
        self.0.span_data(id)
    }
}

trait ErasableRegistry<'a>: tracing_core::Subscriber + LookupSpan<'a> {}
impl<'a, T> ErasableRegistry<'a> for T where T: tracing_core::Subscriber + LookupSpan<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{prelude::*, Layer};

    struct NopLayer;
    impl<S: tracing_core::Subscriber> Layer<S> for NopLayer {}

    struct NopLayer2;
    impl<S: tracing_core::Subscriber> Layer<S> for NopLayer2 {}

    fn is_subscriber<S: tracing_core::Subscriber>(_s: S) {}
    fn is_lookup_span<S: for<'a> LookupSpan<'a>>(_s: S) {}

    #[test]
    fn simple_subscriber_works() {
        let s = subscriber(crate::FmtSubscriber::new());
        is_subscriber(s);
    }

    #[test]
    fn complicated_subscriber_works() {
        let s = crate::registry().with(NopLayer).with(NopLayer);
        let erased = subscriber(s);
        is_subscriber(erased);
    }

    #[test]
    fn simple_registry_works() {
        let s = registry(crate::registry());
        is_subscriber(s);
        is_lookup_span(s);
    }

    #[test]
    fn complicated_registry_works() {
        let subscriber = crate::registry().with(NopLayer).with(NopLayer);
        let erased = registry(subscriber);
        is_subscriber(erased);
        is_lookup_span(erased);
    }

    #[test]
    fn downcasting_also_works() {
        use tracing_core::Subscriber as _;
        #[derive(Debug, Eq, PartialEq)]
        struct StringLayer1(String);
        impl<S: tracing_core::Subscriber> Layer<S> for StringLayer1 {}

        #[derive(Debug, Eq, PartialEq)]
        struct StringLayer2(String);
        impl<S: tracing_core::Subscriber> Layer<S> for StringLayer2 {}

        #[derive(Debug, Eq, PartialEq)]
        struct StringLayer3(String);
        impl<S: tracing_core::Subscriber> Layer<S> for StringLayer3 {}

        let string1 = "The old pond;";
        let string2 = "A frog jumps in —";
        let string3 = "The sound of the water.";
        // —- Matsuo Basho, translated by R. H. Blyth

        let subscriber = subscriber(
            crate::registry()
                .with(StringLayer1(String::from(string1)))
                .with(StringLayer2(String::from(string2)))
                .with(StringLayer3(String::from(string3))),
        );

        assert_eq!(
            tracing_core::Subscriber::downcast_ref::<StringLayer1>(&subscriber),
            Some(&StringLayer1(String::from(string1)))
        );
        assert_eq!(
            tracing_core::Subscriber::downcast_ref::<StringLayer2>(&subscriber),
            Some(&StringLayer2(String::from(string2)))
        );
        assert_eq!(
            tracing_core::Subscriber::downcast_ref::<StringLayer3>(&subscriber),
            Some(&StringLayer3(String::from(string3)))
        );

        let registry = registry(
            crate::registry()
                .with(StringLayer1(String::from(string1)))
                .with(StringLayer2(String::from(string2)))
                .with(StringLayer3(String::from(string3))),
        );

        assert_eq!(
            tracing_core::Subscriber::downcast_ref::<StringLayer1>(&registry),
            Some(&StringLayer1(String::from(string1)))
        );
        assert_eq!(
            tracing_core::Subscriber::downcast_ref::<StringLayer2>(&registry),
            Some(&StringLayer2(String::from(string2)))
        );
        assert_eq!(
            tracing_core::Subscriber::downcast_ref::<StringLayer3>(&registry),
            Some(&StringLayer3(String::from(string3)))
        );
    }
}
