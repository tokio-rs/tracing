//! Storage for span data shared by multiple [`Layer`]s.
//!
//! ## Using the Span Registry
//!
//! This module provides the [`Registry`] type, a [`Subscriber`] implementation
//! which tracks per-span data and exposes it to [`Layer`]s. When a `Registry`
//! is used as the base `Subscriber` of a `Layer` stack, the
//! [`layer::Context`][ctx] type will provide methods allowing `Layer`s to
//! [look up span data][lookup] stored in the registry. While [`Registry`] is a
//! reasonable default for storing spans and events, other stores that implement
//! [`LookupSpan`] and [`Subscriber`] themselves (with [`SpanData`] implemented
//! by the per-span data they store) can be used as a drop-in replacement.
//!
//! For example, we might create a `Registry` and add multiple `Layer`s like so:
//! ```rust
//! use tracing_subscriber::{registry::Registry, Layer, prelude::*};
//! # use tracing_core::Subscriber;
//! # pub struct FooLayer {}
//! # pub struct BarLayer {}
//! # impl<S: Subscriber> Layer<S> for FooLayer {}
//! # impl<S: Subscriber> Layer<S> for BarLayer {}
//! # impl FooLayer {
//! # fn new() -> Self { Self {} }
//! # }
//! # impl BarLayer {
//! # fn new() -> Self { Self {} }
//! # }
//!
//! let subscriber = Registry::default()
//!     .with(FooLayer::new())
//!     .with(BarLayer::new());
//! ```
//!
//! If a type implementing `Layer` depends on the functionality of a `Registry`
//! implementation, it should bound its `Subscriber` type parameter with the
//! [`LookupSpan`] trait, like so:
//!
//! ```rust
//! use tracing_subscriber::{registry, Layer};
//! use tracing_core::Subscriber;
//!
//! pub struct MyLayer {
//!     // ...
//! }
//!
//! impl<S> Layer<S> for MyLayer
//! where
//!     S: Subscriber + for<'a> registry::LookupSpan<'a>,
//! {
//!     // ...
//! }
//! ```
//! When this bound is added, the `Layer` implementation will be guaranteed
//! access to the [`Context`][ctx] methods, such as [`Context::span`][lookup], that
//! require the root subscriber to be a registry.
//!
//! [`Layer`]: ../layer/trait.Layer.html
//! [`Subscriber`]:
//!     https://docs.rs/tracing-core/latest/tracing_core/subscriber/trait.Subscriber.html
//! [`Registry`]: struct.Registry.html
//! [ctx]: ../layer/struct.Context.html
//! [lookup]: ../layer/struct.Context.html#method.span
//! [`LookupSpan`]: trait.LookupSpan.html
//! [`SpanData`]: trait.SpanData.html
use std::fmt::Debug;

use tracing_core::{field::FieldSet, span::Id, Metadata};

/// A module containing a type map of span extensions.
mod extensions;
#[cfg(feature = "registry")]
mod sharded;
#[cfg(feature = "registry")]
mod stack;

pub use extensions::{Extensions, ExtensionsMut};
#[cfg(feature = "registry")]
#[cfg_attr(docsrs, doc(cfg(feature = "registry")))]
pub use sharded::Data;
#[cfg(feature = "registry")]
#[cfg_attr(docsrs, doc(cfg(feature = "registry")))]
pub use sharded::Registry;

/// Provides access to stored span data.
///
/// Subscribers which store span data and associate it with span IDs should
/// implement this trait; if they do, any [`Layer`]s wrapping them can look up
/// metadata via the [`Context`] type's [`span()`] method.
///
/// [`Layer`]: ../layer/trait.Layer.html
/// [`Context`]: ../layer/struct.Context.html
/// [`span()`]: ../layer/struct.Context.html#method.span
pub trait LookupSpan<'a> {
    /// The type of span data stored in this registry.
    type Data: SpanData<'a>;

    /// Returns the [`SpanData`] for a given `Id`, if it exists.
    ///
    /// <div class="information">
    ///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
    /// </div>
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: users of the <code>LookupSpan</code> trait should
    /// typically call the <a href="#method.span"><code>span</code></a> method rather
    /// than this method. The <code>span</code> method is implemented by
    /// <em>calling</em> <code>span_data</code>, but returns a reference which is
    /// capable of performing more sophisiticated queries.
    /// </pre></div>
    ///
    /// [`SpanData`]: trait.SpanData.html
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
        let data = self.span_data(&id)?;
        Some(SpanRef {
            registry: self,
            data,
        })
    }
}

/// A stored representation of data associated with a span.
pub trait SpanData<'a> {
    /// Returns this span's ID.
    fn id(&self) -> Id;

    /// Returns a reference to the span's `Metadata`.
    fn metadata(&self) -> &'static Metadata<'static>;

    /// Returns a reference to the ID
    fn parent(&self) -> Option<&Id>;

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

/// An iterator over the parents of a span, ordered from leaf to root.
///
/// This is returned by the [`SpanRef::scope`] method.
#[derive(Debug)]
pub struct Scope<'a, R> {
    registry: &'a R,
    next: Option<Id>,
}

impl<'a, R> Scope<'a, R>
where
    R: LookupSpan<'a>,
{
    /// Flips the order of the iterator, so that it is ordered from root to leaf.
    ///
    /// The iterator will first return the root span, then that span's immediate child,
    /// and so on until it finally returns the span that [`SpanRef::scope`] was called on.
    ///
    /// If any items were consumed from the [`Scope`] before calling this method then they
    /// will *not* be returned from the [`ScopeFromRoot`].
    ///
    /// **Note**: this will allocate if there are many spans remaining, or if the
    /// "smallvec" feature flag is not enabled.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_root(self) -> ScopeFromRoot<'a, R> {
        #[cfg(feature = "smallvec")]
        type Buf<T> = smallvec::SmallVec<T>;
        #[cfg(not(feature = "smallvec"))]
        type Buf<T> = Vec<T>;
        ScopeFromRoot {
            spans: self.collect::<Buf<_>>().into_iter().rev(),
        }
    }
}

impl<'a, R> Iterator for Scope<'a, R>
where
    R: LookupSpan<'a>,
{
    type Item = SpanRef<'a, R>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.registry.span(self.next.as_ref()?)?;
        self.next = curr.parent_id().cloned();
        Some(curr)
    }
}

/// An iterator over the parents of a span, ordered from root to leaf.
///
/// This is returned by the [`Scope::from_root`] method.
pub struct ScopeFromRoot<'a, R>
where
    R: LookupSpan<'a>,
{
    #[cfg(feature = "smallvec")]
    spans: std::iter::Rev<smallvec::IntoIter<SpanRefVecArray<'a, R>>>,
    #[cfg(not(feature = "smallvec"))]
    spans: std::iter::Rev<std::vec::IntoIter<SpanRef<'a, R>>>,
}

impl<'a, R> Iterator for ScopeFromRoot<'a, R>
where
    R: LookupSpan<'a>,
{
    type Item = SpanRef<'a, R>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.spans.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.spans.size_hint()
    }
}

impl<'a, R> Debug for ScopeFromRoot<'a, R>
where
    R: LookupSpan<'a>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad("ScopeFromRoot { .. }")
    }
}

/// An iterator over the parents of a span.
///
/// This is returned by the [`SpanRef::parents`] method.
///
/// [`SpanRef::parents`]: struct.SpanRef.html#method.parents
#[deprecated(note = "replaced by Scope")]
pub type Parents<'a, R> = Scope<'a, R>;

/// An iterator over a span's parents, starting with the root of the trace
/// tree.
///
/// For additonal details, see [`SpanRef::from_root`].
///
/// [`Span::from_root`]: struct.SpanRef.html#method.from_root
#[deprecated(note = "replaced by ScopeFromRoot")]
pub type FromRoot<'a, R> = ScopeFromRoot<'a, R>;

#[cfg(feature = "smallvec")]
type SpanRefVecArray<'span, L> = [SpanRef<'span, L>; 16];

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

    /// Returns an iterator over all parents of this span, starting with this span,
    /// ordered from leaf to root.
    ///
    /// The iterator will first return the span, then the span's immediate parent,
    /// followed by that span's parent,  and so on, until it reaches a root span.
    pub fn scope(&self) -> Scope<'a, R> {
        Scope {
            registry: self.registry,
            next: Some(self.id()),
        }
    }

    /// Returns an iterator over all parents of this span, starting with the
    /// immediate parent.
    ///
    /// The iterator will first return the span's immediate parent, followed by
    /// that span's parent, followed by _that_ span's parent, and so on, until a
    /// it reaches a root span.
    #[deprecated(
        note = "equivalent to self.scope().skip(1), but consider whether the skip is actually intended"
    )]
    #[allow(deprecated)]
    pub fn parents(&self) -> Parents<'a, R> {
        let mut scope = self.scope();
        scope.next();
        scope
    }

    /// Returns an iterator over all parents of this span, starting with the
    /// root of the trace tree.
    ///
    /// The iterator will return the root of the trace tree, followed by the
    /// next span, and then the next, until this span's immediate parent is
    /// returned.
    ///
    /// **Note**: this will allocate if there are many spans remaining, or if the
    /// "smallvec" feature flag is not enabled.
    #[deprecated(
        note = "equivalent to self.scope().skip(1).from_root(), but consider whether the skip is actually intended"
    )]
    #[allow(deprecated)]
    pub fn from_root(&self) -> FromRoot<'a, R> {
        self.parents().from_root()
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
