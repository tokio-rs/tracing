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
pub struct FilterId(usize);

#[derive(Default)]
pub(crate) struct FilterMap {
    bitmap: [AtomicUsize; BITMAP_SHARDS],
}

impl FilterMap {
    pub const MAX_FILTERS: usize = 512;
    const BITMAP_SHARDS: usize = MAX_FILTERS / BITS_PER_SHARD;
    const BITS_PER_SHARD: usize = mem::size_of::<AtomicUsize>() * 8;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn disable(&self, FilterId(filter): FilterId) {
        let idx = filter / Self::BITMAP_SHARDS;
        let bit = 1 << (filter % Self::BITS_PER_SHARD);
        self.bitmap[idx].fetch_or(1 << bit, Ordering::Release);
    }

    pub fn is_enabled(&self, FilterId(filter): FilterId) {
        let idx = filter / Self::BITMAP_SHARDS;
        let bit = 1 << (filter % Self::BITS_PER_SHARD);
        self.bitmap[idx].load(Ordering::Acquire) & bit != 0
    }
}
