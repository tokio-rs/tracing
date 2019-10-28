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
use tracing_core::{span::Id, Metadata};

pub mod sharded;

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
    type Span: SpanData<'a>;
    fn span(&'a self, id: &Id) -> Option<Self::Span>;
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
    fn get<T: Any>(&self) -> Option<&T>;
    fn get_mut<T: Any>(&mut self) -> Option<&mut T>;
    fn insert<T: Any>(&mut self, t: T) -> Option<T>;
    fn remove<T: Any>(&mut self) -> Option<T>;
}