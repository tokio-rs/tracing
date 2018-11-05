use std::{cell::Cell, thread::LocalKey};
use {Dispatch, Meta, Span, Subscriber};

#[derive(Debug)]
pub struct Callsite(pub(crate) &'static LocalKey<Cache<'static>>);

#[doc(hidden)]
pub struct Cache<'a> {
    pub(crate) last_filtered_by: Cell<usize>,
    pub(crate) cached_filter: Cell<Option<bool>>,
    pub(crate) meta: &'a Meta<'a>,
}

impl Callsite {
    #[doc(hidden)]
    pub fn new(cache: &'static LocalKey<Cache<'static>>) -> Self {
        Callsite(cache)
    }

    #[inline]
    pub(crate) fn metadata(&self) -> &'static Meta<'static> {
        self.0.with(|cache| cache.meta)
    }
}

impl<'a> Cache<'a> {
    pub fn new(meta: &'a Meta<'a>) -> Self {
        Self {
            last_filtered_by: Cell::new(0),
            cached_filter: Cell::new(None),
            meta,
        }
    }

    #[inline]
    pub fn metadata(&self) -> &'a Meta<'a> {
        self.meta
    }
}
