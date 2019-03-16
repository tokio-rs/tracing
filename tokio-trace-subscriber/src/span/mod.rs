pub use tokio_trace_core::span::*;

#[cfg(feature = "store")]
pub mod store;

pub trait Registry: 'static {
    type Span: Ref;

    fn new_span(&self, attrs: &Attributes) -> Id;

    fn record(&self, id: &Id, fields: &Record);

    fn clone_span(&self, id: &Id);

    fn drop_span(&self, id: &Id);

    fn get(&self, id: &Id) -> Option<Self::Span>;

    fn current(&self) -> Option<Self::Span>;
}

pub trait Ref {
    fn name(&self) -> &'static str;

    fn parent(&self) -> Option<&Id>;
}

pub struct Context<'registry, R> {
    registry: &'registry R,
}
