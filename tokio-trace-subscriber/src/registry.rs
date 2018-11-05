use tokio_trace::{
    span::{Data, Id},
    subscriber::{AddValueError, FollowsError},
    value::{IntoValue, OwnedValue},
};

use std::{
    cmp,
    collections::HashMap,
    hash::{Hash, Hasher},
    iter,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};

/// The span registration portion of the [`Subscriber`] trait.
///
/// Implementations of this trait represent the logic run on span creation. They
/// handle span ID generation.
pub trait RegisterSpan {
    type PriorSpans: Iterator<Item = Id>;

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
    fn new_span(&self, new_span: Data) -> Id;

    fn add_value(
        &self,
        span: &Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError>;

    /// Adds an indication that `span` follows from the span with the id
    /// `follows`.
    ///
    /// This relationship differs somewhat from the parent-child relationship: a
    /// span may have any number of prior spans, rather than a single one; and
    /// spans are not considered to be executing _inside_ of the spans they
    /// follow from. This means that a span may close even if subsequent spans
    /// that follow from it are still open, and time spent inside of a
    /// subsequent span should not be included in the time its precedents were
    /// executing. This is used to model causal relationships such as when a
    /// single future spawns several related background tasks, et cetera.
    ///
    /// If the registry has spans corresponding to the given IDs, it should
    /// record this relationship in whatever way it deems necessary. Otherwise,
    /// if one or both of the given span IDs do not correspond to spans that the
    /// registry knows about, or if a cyclical relationship would be created
    /// (i.e., some span _a_ which proceeds some other span _b_ may not also
    /// follow from _b_), it should return a `FollowsError`.
    fn add_follows_from(&self, span: &Id, follows: Id) -> Result<(), FollowsError>;

    /// Queries the registry for an iterator over the IDs of the spans that
    /// `span` follows from.
    fn prior_spans(&self, span: &Id) -> Self::PriorSpans;

    fn with_span<F>(&self, id: &Id, f: F)
    where
        F: for<'a> Fn(&'a SpanRef<'a>);

    // TODO: Should the registry also be informed of span closure?
}

#[derive(Debug)]
pub struct SpanRef<'a> {
    pub id: &'a Id,
    pub data: Option<&'a Data>,
    // TODO: the registry can still have a concept of span states...
}

impl<'a> Hash for SpanRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'a, 'b> cmp::PartialEq<SpanRef<'b>> for SpanRef<'a> {
    fn eq(&self, other: &SpanRef<'b>) -> bool {
        self.id == other.id
    }
}

impl<'a> cmp::Eq for SpanRef<'a> {}

impl<'a> IntoIterator for &'a SpanRef<'a> {
    type Item = (&'a str, &'a OwnedValue);
    type IntoIter = Box<Iterator<Item = Self::Item> + 'a>; // TODO: unbox
    fn into_iter(self) -> Self::IntoIter {
        self.data
            .map(|data| {
                // This is necessary because of type inference.
                let iter: Box<Iterator<Item = Self::Item> + 'a> = Box::new(data.fields());
                iter
            }).unwrap_or_else(|| Box::new(::std::iter::empty()))
    }
}
// /// Registers new span IDs with an increasing `usize` counter.
// ///
// /// This may overflow on 32-bit machines.
// pub fn increasing_counter(_new_span: Data) -> Id {
//     static NEXT_ID: AtomicUsize = ATOMIC_USIZE_INIT;
//     let next = NEXT_ID.fetch_add(1, Ordering::SeqCst);
//     Id::from_u64(next as u64)
// }

#[derive(Default)]
pub struct IncreasingCounter {
    next_id: AtomicUsize,
    spans: Mutex<HashMap<Id, Data>>,
}

pub fn increasing_counter() -> IncreasingCounter {
    IncreasingCounter::default()
}

impl RegisterSpan for IncreasingCounter {
    type PriorSpans = iter::Empty<Id>;

    fn new_span(&self, new_span: Data) -> Id {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let id = Id::from_u64(id as u64);
        if let Ok(mut spans) = self.spans.lock() {
            spans.insert(id.clone(), new_span);
        }
        id
    }

    fn add_value(
        &self,
        span: &Id,
        name: &'static str,
        value: &dyn IntoValue,
    ) -> Result<(), AddValueError> {
        let mut spans = self.spans.lock().expect("mutex poisoned!");
        let span = spans.get_mut(span).ok_or(AddValueError::NoSpan)?;
        span.add_value(name, value)
    }

    fn add_follows_from(&self, _span: &Id, _follows: Id) -> Result<(), FollowsError> {
        // unimplemented
        Ok(())
    }

    fn prior_spans(&self, _span: &Id) -> Self::PriorSpans {
        unimplemented!();
    }

    fn with_span<F>(&self, id: &Id, f: F)
    where
        F: for<'a> Fn(&'a SpanRef<'a>),
    {
        let spans = self.spans.lock().expect("mutex poisoned!");
        let data = spans.get(id);
        let span = SpanRef { id, data };
        f(&span);
    }
}
