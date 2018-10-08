use tokio_trace::span::{Id, NewSpan};

use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

/// The span registration portion of the [`Subscriber`] trait.
///
/// Implementations of this trait represent the logic run on span creation. They
/// handle span ID generation.
pub trait RegisterSpan {
    /// Record the construction of a new [`Span`], returning a a new [span ID] for
    /// the span being constructed.
    ///
    /// Span IDs are used to uniquely identify spans, so span equality will be
    /// based on the returned ID. Thus, if the subscriber wishes for all spans
    /// with the same metadata to be considered equal, it should return the same
    /// ID every time it is given a particular set of metadata. Similarly, if it
    /// wishes for two separate instances of a span with the same metadata to *not*
    /// be equal, it should return a distinct ID every time this function is called,
    /// regardless of the metadata.
    ///
    /// Subscribers which do not rely on the implementations of `PartialEq`,
    /// `Eq`, and `Hash` for `Span`s are free to return span IDs with value 0
    /// from all calls to this function, if they so choose.
    ///
    /// [span ID]: ../span/struct.Id.html
    fn new_span(&self, new_span: &NewSpan) -> Id;
}

/// Registers new span IDs with an increasing `usize` counter.
///
/// This may overflow on 32-bit machines.
pub fn increasing_counter(_new_span: &NewSpan) -> Id {
    static NEXT_ID: AtomicUsize = ATOMIC_USIZE_INIT;
    let next = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    Id::from_u64(next as u64)
}

impl<T> RegisterSpan for T
where
    T: Fn(&NewSpan) -> Id,
{
    fn new_span(&self, new_span: &NewSpan) -> Id {
        self(new_span)
    }
}
