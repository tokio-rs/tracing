use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

pub trait Filterable {
    fn register_filter(&self) -> FilterId;
    fn disable_span(&self, filter: FilterId, span: &span::Id);
    fn is_enabled_by(&self, filter: FilterId, span: &span::Id) -> bool;
}

pub struct Filtered<F, L, I, S = I> {
    layer: L,
    next: I,
    filter: F,
    id: usize,
    _s: PhantomData<fn(S)>,
}

#[derive(Copy, Clone, Debug)]
pub struct FilterId(u8);

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
        let bit = 1 << (filter % Self::SHARDS);
        self.bitmap[idx] |= bit;
    }

    pub fn is_enabled(&self, FilterId(filter): FilterId) -> bool {
        let idx = filter as usize / 8;
        let bit = 1 << (filter % Self::SHARDS);
        self.bitmap[idx] & bit == 0
    }
}
