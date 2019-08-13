//! Filter implementations for determining what spans and events to record.
use crate::span;

use tracing_core::{subscriber::Interest, Metadata};

/// A policy for determining what spans and events should be enabled.
pub trait Filter<N> {
    fn callsite_enabled(&self, metadata: &Metadata<'_>, ctx: &span::Context<'_, N>) -> Interest {
        if self.enabled(metadata, ctx) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, ctx: &span::Context<'_, N>) -> bool;
}

pub mod env;
pub mod reload;

#[doc(inline)]
pub use self::{env::EnvFilter, reload::ReloadFilter};

impl<'a, F, N> Filter<N> for F
where
    F: Fn(&Metadata<'_>, &span::Context<'_, N>) -> bool,
    N: crate::NewVisitor<'a>,
{
    fn enabled(&self, metadata: &Metadata<'_>, ctx: &span::Context<'_, N>) -> bool {
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
    fn enabled(&self, _: &Metadata<'_>, _: &span::Context<'_, N>) -> bool {
        true
    }
}
