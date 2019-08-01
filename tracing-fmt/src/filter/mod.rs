use crate::span;

use tracing_core::{subscriber::Interest, Metadata};

pub trait Filter<N> {
    fn callsite_enabled(&self, metadata: &Metadata, ctx: &span::Context<N>) -> Interest {
        if self.enabled(metadata, ctx) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata, ctx: &span::Context<N>) -> bool;
}

pub mod env;
pub mod reload;

pub use self::{env::EnvFilter, reload::ReloadFilter};

impl<'a, F, N> Filter<N> for F
where
    F: Fn(&Metadata, &span::Context<N>) -> bool,
    N: crate::NewVisitor<'a>,
{
    fn enabled(&self, metadata: &Metadata, ctx: &span::Context<N>) -> bool {
        (self)(metadata, ctx)
    }
}

pub fn none() -> NoFilter {
    NoFilter { _p: () }
}

#[derive(Clone, Debug)]
pub struct NoFilter {
    _p: (),
}

impl<N> Filter<N> for NoFilter {
    fn enabled(&self, _: &Metadata, _: &span::Context<N>) -> bool {
        true
    }
}
