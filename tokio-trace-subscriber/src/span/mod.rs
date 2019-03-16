pub use tokio_trace_core::span::*;

use std::borrow::{Borrow, BorrowMut};

#[cfg(feature = "store")]
pub mod store;

mod data;
pub use self::data::Data;

pub trait Registry<'a>: 'static {
    type Span: Ref<'a>;
    type SpanMut: RefMut<'a>;

    fn new_span<F>(&self, attrs: &Attributes, f: F) -> Id
    where
        F: FnOnce(&Attributes, &mut Data);

    fn clone_span(&self, id: &Id);

    fn drop_span(&self, id: Id);

    fn get(&self, id: &Id) -> Option<Self::Span>;

    fn get_mut(&self, id: &Id) -> Option<Self::SpanMut>;

    fn current(&self) -> Option<Self::Span>;

    fn current_mut(&self) -> Option<Self::SpanMut>;
}

pub trait Ref<'a>: Borrow<Data> {
    fn name(&self) -> &'static str;

    fn parent(&self) -> Option<&Id>;

    fn data(&self) -> &Data {
        self.borrow()
    }
}

pub trait RefMut<'a>: Ref<'a> + BorrowMut<Data> {
    fn data_mut(&mut self) -> &mut Data {
        self.borrow_mut()
    }
}

pub struct Context<'registry, R> {
    registry: &'registry R,
}
