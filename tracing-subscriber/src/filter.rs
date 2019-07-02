
use crate::layer::Layer;
use tracing_core::{subscriber::Interest, Metadata};

pub trait Filter: 'static {
    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        if self.enabled(metadata) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata) -> bool;
}

pub trait FilterExt: Filter {
    fn or<B>(self, b: B) -> Or<Self, B>
    where
        Self: Sized,
    {
        Or { a: self, b }
    }

    fn and<B>(self, b: B) -> And<Self, B>
    where
        Self: Sized,
    {
        And { a: self, b }
    }
}

#[derive(Clone, Debug)]
pub struct EnabledFn<F>(F);

#[derive(Clone, Debug)]
pub struct InterestFn<F>(F);

#[derive(Clone, Debug)]
pub struct Or<A, B> {
    a: A,
    b: B,
}

#[derive(Clone, Debug)]
pub struct And<A, B> {
    a: A,
    b: B,
}

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

impl<F: Filter, S> Layer<S> for F {
    fn register_callsite(&self, metadata: &'static Metadata<'static>, prev: Interest) -> Interest {
        let my_interest = self.callsite_enabled(metadata);
        if my_interest.is_always() {
            prev
        } else {
            my_interest
        }
    }

    fn enabled(&self, metadata: &Metadata, prev: bool) -> bool {
        Filter::enabled(self, metadata) && prev
    }
}

// === impl EnabledFn ===

impl<F> Filter for EnabledFn<F>
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

impl<F> Filter for InterestFn<F>
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

// === impl FilterExt ===
impl<F: Filter> FilterExt for F {}

// === impl And ===

impl<A, B> Filter for And<A, B>
where
    A: Filter,
    B: Filter,
{
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.a.enabled(metadata) && self.b.enabled(metadata)
    }

    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        let a = self.a.callsite_enabled(metadata);
        let b = self.b.callsite_enabled(metadata);
        match () {
            _ if a.is_sometimes() && b.is_sometimes() => Interest::sometimes(),
            _ if a.is_always() && !b.is_never() => Interest::always(),
            _ if b.is_always() && !a.is_never() => Interest::always(),
            _ => Interest::never(),
        }
    }
}

// === impl Or ===

impl<A, B> Filter for Or<A, B>
where
    A: Filter,
    B: Filter,
{
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.a.enabled(metadata) || self.b.enabled(metadata)
    }

    fn callsite_enabled(&self, metadata: &'static Metadata<'static>) -> Interest {
        let a = self.a.callsite_enabled(metadata);
        let b = self.b.callsite_enabled(metadata);
        match () {
            _ if a.is_always() || b.is_always() => Interest::always(),
            _ if a.is_sometimes() || b.is_sometimes() => Interest::sometimes(),
            _ => Interest::never(),
        }
    }
}
