//! **EXPERIMENTAL**: Storage for span data shared by multiple [`Layer`]s.
//!
//! This module is experimental. Although potential breaking changes will be
//! avoided when possible, we reserve the right to make breaking changes to this
//! module until it is no longer experimental.
//!
//! Add the `registry_unstable` feature to your `Cargo.toml` to enable
//! this module:
//!
//! ```toml
//! [dependencies.tracing-subscriber]
//! features = ["registry_unstable"]
//! ```
//!
//! [`Layer`]: ../layer/struct.Layer.html
use std::any::Any;
use tracing_core::{span::Id, Metadata, Subscriber};

pub mod extensions;
pub mod fmt;
pub mod sharded;

pub use fmt::{FmtLayer, FmtLayerBuilder};
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

pub trait LookupSpan<'a> {
    type Data: SpanData<'a>;
    fn span_data(&'a self, id: &Id) -> Option<Self::Data>;

    fn span(&'a self, id: &Id) -> Option<SpanRef<'a, Self>>
    where
        Self: Sized,
    {
        let data = self.span_data(id)?;
        Some(SpanRef {
            registry: self,
            data,
        })
    }

    // TODO(david): move this somewhere more appropriate; rewrite in terms of `SpanData`.
    fn visit_parents<E, F>(&self, f: F) -> Result<(), E>
    where
        F: FnMut(&Id) -> Result<(), E>;
}

pub trait SpanData<'a> {
    type Children: Iterator<Item = &'a Id>;
    type Follows: Iterator<Item = &'a Id>;

    fn id(&self) -> Id;
    fn metadata(&self) -> &'static Metadata<'static>;
    fn parent(&self) -> Option<&Id>;
    fn children(&'a self) -> Self::Children;
    fn follows_from(&'a self) -> Self::Follows;
}

// XXX(eliza): should this have a SpanData bound? The expectation is that there
// would be add'l impls for `T: LookupSpan where T::Span: Extensions`...
//
// XXX(eliza): also, consider having `.extensions`/`.extensions_mut` methods to
// get the extensions, so we can control read locking vs write locking?
pub trait Extensions {
    fn get<T: Any + Send + Sync>(&self) -> Option<&T>;
    fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T>;
    fn insert<T: Any + Send + Sync>(&mut self, t: T) -> Option<T>;
    fn remove<T: Any + Send + Sync>(&mut self) -> Option<T>;
}

// TODO(david): this might require implementing Extensions on
// slab's guard.
impl Extensions for extensions::Extensions {
    fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.get::<T>()
    }
    fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
        self.get_mut::<T>()
    }
    fn insert<T: Any + Send + Sync>(&mut self, t: T) -> Option<T> {
        self.insert::<T>(t)
    }
    fn remove<T: Any + Send + Sync>(&mut self) -> Option<T> {
        self.remove::<T>()
    }
}

#[derive(Debug)]
pub struct SpanRef<'a, R: LookupSpan<'a>> {
    registry: &'a R,
    data: R::Data,
}

#[derive(Debug)]
pub struct Parents<'a, R> {
    registry: &'a R,
    next: Option<Id>,
}

impl<'a, R> SpanRef<'a, R>
where
    R: LookupSpan<'a>,
{
    pub fn id(&self) -> Id {
        self.data.id()
    }

    pub fn parent_id(&self) -> Option<&Id> {
        self.data.parent()
    }

    pub fn parent(&self) -> Option<Self> {
        let id = self.data.parent()?;
        let data = self.registry.span_data(id)?;
        Some(Self {
            registry: self.registry,
            data,
        })
    }

    pub fn parents(&'a self) -> Parents<'a, R> {
        Parents {
            registry: self.registry,
            next: self.parent().map(|parent| parent.id()),
        }
    }

    pub fn child_ids(&'a self) -> <R::Data as SpanData<'a>>::Children {
        self.data.children()
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
