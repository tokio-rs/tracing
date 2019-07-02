use tracing_core::{subscriber::Interest, Metadata};

pub trait Layer: 'static {
    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata) -> bool;
}

#[derive(Clone, Debug)]
pub struct EnabledFn<F>(F);

#[derive(Clone, Debug)]
pub struct InterestFn<F>(F);

pub fn enabled_fn<F>(f: F) -> EnabledFn<F>
where
    F: Fn(&Metadata) -> bool + 'static,
{
    EnabledFn::from(f)
}

pub fn callsite_fn<F>(f: F) -> InterestFn<F>
where
    F: Fn(&Metadata) -> Interest + 'static,
{
    InterestFn::from(f)
}

// === impl Layer ===

impl<F: Layer, S> crate::layer::Layer<S> for F {
    fn register_callsite(&self, metadata: &'static Metadata<'static>, prev: Interest) -> Interest {
        let my_interest = self.callsite_enabled(metadata);
        if my_interest.is_always() {
            prev
        } else {
            my_interest
        }
    }

    fn enabled(&self, metadata: &Metadata, prev: bool) -> bool {
        Layer::enabled(self, metadata) && prev
    }
}

// === impl EnabledFn ===

impl<F> Layer for EnabledFn<F>
where
    F: Fn(&Metadata) -> bool + 'static,
{
    fn enabled(&self, metadata: &Metadata) -> bool {
        (self.0)(metadata)
    }
}

impl<F> From<F> for EnabledFn<F>
where
    F: Fn(&Metadata) -> bool + 'static,
{
    fn from(f: F) -> Self {
        Self(f)
    }
}

// === impl InterestFn ===

impl<F> Layer for InterestFn<F>
where
    F: Fn(&Metadata) -> Interest + 'static,
{
    fn enabled(&self, metadata: &Metadata) -> bool {
        let my_interest = (self.0)(metadata);
        my_interest.is_always() || my_interest.is_sometimes()
    }

    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        (self.0)(metadata)
    }
}

impl<F> From<F> for InterestFn<F>
where
    F: Fn(&'static Metadata<'static>) -> Interest + 'static,
{
    fn from(f: F) -> Self {
        Self(f)
    }
}
