//! Storage for span data shared by multiple [`Layer`]s.
//!
//! [`Layer`]: ../layer/struct.Layer.html
use tracing_core::{span::Id, Metadata, field::FieldSet};

/// A module containing a type map of span extensions.
pub mod extensions;
mod sharded;
mod stack;

pub use extensions::{Extensions, ExtensionsMut};
pub use sharded::{Data, Registry};

/// Provides access to stored span metadata.
///
/// Subscribers which store span metadata and associate it with span IDs should
/// implement this trait; if they do, any [`Layer`]s wrapping them can look up
/// metadata via the [`Context`] type's [`metadata()`] method.
///
/// [`Layer`]: ../layer/struct.Layer.html
/// [`Context`]: ../layer/struct.Context.html
/// [`metadata()`]: ../layer/struct.Context.html#method.metadata
pub trait LookupMetadata {
    /// Returns metadata for tne span with the given `id`, if it exists.
    ///
    /// If no span exists for the provided ID (e.g. the span has closed and been
    /// removed from the registry, or the ID is invalid), this should return `None`.
    fn metadata(&self, id: &Id) -> Option<&'static Metadata<'static>>;

    /// Returns `true` if a span with the given `id` exists, false otherwise.
    ///
    /// **Note**: The default implementation of this method is simply:
    ///```rust,ignore
    /// fn exists(&self, id: &span::Id) -> bool {
    ///     self.metadata(id).is_some()
    /// }
    ///```
    /// If the subscriber has a faster way of determining whether a span exists
    /// for a given ID (e.g., if the ID is greater than the current value of an
    /// increasing ID counter, etc), this method may be overridden as an optimization.
    fn exists(&self, id: &Id) -> bool {
        self.metadata(id).is_some()
    }
}

/// Provides access to stored span data.
///
/// Subscribers which store span data and associate it with span IDs should
/// implement this trait; if they do, any [`Layer`]s wrapping them can look up
/// metadata via the [`Context`] type's [`span()`] method.
///
/// [`Layer`]: ../layer/struct.Layer.html
/// [`Context`]: ../layer/struct.Context.html
/// [`span()`]: ../layer/struct.Context.html#method.metadata
pub trait LookupSpan<'a> {
    /// The type of span data stored in this registry.
    type Data: SpanData<'a>;

    /// Returns the [`SpanData`] for a given `Id`, if it exists.
    ///
    /// **Note**: users of the `LookupSpan` trait should typically call the
    /// [`span`] method rather than this method. The `span` method is
    /// implemented by calling `span_data`, but returns a reference which is
    /// capable of performing more sophisiticated queries.
    ///
    /// [`SpanData`]: trait.SpanData.html
    /// [`span`]: #method.span
    fn span_data(&'a self, id: &Id) -> Option<Self::Data>;

    /// Returns a [`SpanRef`] for the span with the given `Id`, if it exists.
    ///
    /// A `SpanRef` is similar to [`SpanData`], but it allows performing
    /// additional lookups against the registryr that stores the wrapped data.
    ///
    /// In general, _users_ of the `LookupSpan` trait should use this method
    /// rather than the [`span_data`] method; while _implementors_ of this trait
    /// should only implement `span_data`.
    ///
    /// [`SpanRef`]: struct.SpanRef.html
    /// [`SpanData`]: trait.SpanData.html
    /// [`span_data`]: #method.span_data
    fn span(&'a self, id: &Id) -> Option<SpanRef<'_, Self>>
    where
        Self: Sized,
    {
        let data = self.span_data(id)?;
        Some(SpanRef {
            registry: self,
            data,
        })
    }
}

/// A stored representation of data associated with a span.
pub trait SpanData<'a> {
    /// An iterator of all the spans this span succeeds.
    type Follows: Iterator<Item = &'a Id>;

    /// Returns this span's ID.
    fn id(&self) -> Id;

    /// Returns a reference to the span's `Metadata`.
    fn metadata(&self) -> &'static Metadata<'static>;

    /// Returns a reference to the ID
    fn parent(&self) -> Option<&Id>;

    /// Returns the an iterator of the spans this span succeeds.
    fn follows_from(&'a self) -> Self::Follows;

    /// Returns a reference to this span's `Extensions`.
    ///
    /// The extensions may be used by `Layer`s to store additional data
    /// describing the span.
    fn extensions(&self) -> Extensions<'_>;

    /// Returns a mutable reference to this span's `Extensions`.
    ///
    /// The extensions may be used by `Layer`s to store additional data
    /// describing the span.
    fn extensions_mut(&self) -> ExtensionsMut<'_>;
}

/// A reference to [span data] and the associated [registry].
///
/// This type implements all the same methods as [`SpanData`][span data], and
/// provides additional methods for querying the registry based on values from
/// the span.
///
/// [span data]: trait.SpanData.html
/// [registry]: trait.LookupSpan.html
#[derive(Debug)]
pub struct SpanRef<'a, R: LookupSpan<'a>> {
    registry: &'a R,
    data: R::Data,
}

/// An iterator over the parents of a span.
///
/// This is returned by the [`SpanRef::parents`] method.
///
/// [`SpanRef::parents`]: struct.SpanRef.html#method.parents
#[derive(Debug)]
pub struct Parents<'a, R> {
    registry: &'a R,
    next: Option<Id>,
}

impl<'a, R> SpanRef<'a, R>
where
    R: LookupSpan<'a>,
{
    /// Returns this span's ID.
    pub fn id(&self) -> Id {
        self.data.id()
    }

    /// Returns a static reference to the span's metadata.
    pub fn metadata(&self) -> &'static Metadata<'static> {
        self.data.metadata()
    }

    /// Returns the span's name,
    pub fn name(&self) -> &'static str {
        self.data.metadata().name()
    }

    /// Returns a list of [fields] defined by the span.
    ///
    /// [fields]: https://docs.rs/tracing-core/latest/tracing_core/field/index.html
    pub fn fields(&self) -> &FieldSet {
        self.data.metadata().fields()
    }

    /// Returns the ID of this span's parent, or `None` if this span is the root
    /// of its trace tree.
    pub fn parent_id(&self) -> Option<&Id> {
        self.data.parent()
    }

    /// Returns a `SpanRef` describing this span's parent, or `None` if this
    /// span is the root of its trace tree.
    pub fn parent(&self) -> Option<Self> {
        let id = self.data.parent()?;
        let data = self.registry.span_data(id)?;
        Some(Self {
            registry: self.registry,
            data,
        })
    }

    /// Returns an iterator over all parents of this span.
    ///
    /// The iterator will first return the span's immediate parent, followed by
    /// that span's parent, followed by _that_ span's parent, and so on, until a
    /// it reaches a root span.
    pub fn parents(&self) -> Parents<'_, R> {
        Parents {
            registry: self.registry,
            next: self.parent().map(|parent| parent.id()),
        }
    }

    /// Returns a reference to this span's `Extensions`.
    ///
    /// The extensions may be used by `Layer`s to store additional data
    /// describing the span.
    pub fn extensions(&self) -> Extensions<'_> {
        self.data.extensions()
    }

    /// Returns a mutable reference to this span's `Extensions`.
    ///
    /// The extensions may be used by `Layer`s to store additional data
    /// describing the span.
    pub fn extensions_mut(&self) -> ExtensionsMut<'_> {
        self.data.extensions_mut()
    }
}

impl<'a, R> Iterator for Parents<'a, R>
where
    R: LookupSpan<'a>,
{
    type Item = SpanRef<'a, R>;
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next.take()?;
        let span = self.registry.span(&id)?;
        self.next = span.parent().map(|parent| parent.id());
        Some(span)
    }
}

impl<L> LookupMetadata for L
where
    L: for<'a> LookupSpan<'a>,
{
    fn metadata(&self, id: &Id) -> Option<&'static Metadata<'static>> {
        self.span_data(id).map(|data| data.metadata())
    }
}
