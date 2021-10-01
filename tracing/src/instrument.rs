use crate::span::Span;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::{future::Future, marker::Sized};
use pin_project_lite::pin_project;

#[cfg(feature = "std")]
use crate::dispatch::{self, Dispatch};

/// Attaches spans to a [`std::future::Future`].
///
/// Extension trait allowing futures to be
/// instrumented with a `tracing` [span].
///
/// [span]: super::Span
pub trait Instrument: Sized {
    /// Instruments this type with the provided [`Span`], returning an
    /// `Instrumented` wrapper.
    ///
    /// The attached [`Span`] will be [entered] every time the instrumented
    /// [`Future`] is polled.
    ///
    /// # Examples
    ///
    /// Instrumenting a future:
    ///
    /// ```rust
    /// use tracing::Instrument;
    ///
    /// # async fn doc() {
    /// let my_future = async {
    ///     // ...
    /// };
    ///
    /// my_future
    ///     .instrument(tracing::info_span!("my_future"))
    ///     .await
    /// # }
    /// ```
    ///
    /// The [`Span::or_current`] combinator can be used in combination with
    /// `instrument` to ensure that the [current span] is attached to the
    /// future if the span passed to `instrument` is [disabled]:
    ///
    /// ```
    /// use tracing::Instrument;
    /// # mod tokio {
    /// #     pub(super) fn spawn(_: impl std::future::Future) {}
    /// # }
    ///
    /// let my_future = async {
    ///     // ...
    /// };
    ///
    /// let outer_span = tracing::info_span!("outer").entered();
    ///
    /// // If the "my_future" span is enabled, then the spawned task will
    /// // be within both "my_future" *and* "outer", since "outer" is
    /// // "my_future"'s parent. However, if "my_future" is disabled,
    /// // the spawned task will *not* be in any span.
    /// tokio::spawn(
    ///     my_future
    ///         .instrument(tracing::debug_span!("my_future"))
    /// );
    ///
    /// // Using `Span::or_current` ensures the spawned task is instrumented
    /// // with the current span, if the new span passed to `instrument` is
    /// // not enabled. This means that if the "my_future"  span is disabled,
    /// // the spawned task will still be instrumented with the "outer" span:
    /// # let my_future = async {};
    /// tokio::spawn(
    ///    my_future
    ///         .instrument(tracing::debug_span!("my_future").or_current())
    /// );
    /// ```
    ///
    /// [entered]: super::Span::enter()
    /// [`Span::or_current`]: super::Span::or_current()
    /// [current span]: super::Span::current()
    /// [disabled]: super::Span::is_disabled()
    /// [`Future`]: std::future::Future
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented { inner: self, span }
    }

    /// Instruments this type with the [current] [`Span`], returning an
    /// `Instrumented` wrapper.
    ///
    /// The attached [`Span`] will be [entered] every time the instrumented
    /// [`Future`] is polled.
    ///
    /// This can be used to propagate the current span when spawning a new future.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tracing::Instrument;
    ///
    /// # mod tokio {
    /// #     pub(super) fn spawn(_: impl std::future::Future) {}
    /// # }
    /// # async fn doc() {
    /// let span = tracing::info_span!("my_span");
    /// let _enter = span.enter();
    ///
    /// // ...
    ///
    /// let future = async {
    ///     tracing::debug!("this event will occur inside `my_span`");
    ///     // ...
    /// };
    /// tokio::spawn(future.in_current_span());
    /// # }
    /// ```
    ///
    /// [current]: super::Span::current()
    /// [entered]: super::Span::enter()
    /// [`Span`]: crate::Span
    /// [`Future`]: std::future::Future
    #[inline]
    fn in_current_span(self) -> Instrumented<Self> {
        self.instrument(Span::current())
    }
}

/// Extension trait allowing futures to be instrumented with
/// a `tracing` collector.
///

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub trait WithCollector: Sized {
    /// Attaches the provided [collector] to this type, returning a
    /// [`WithDispatch`] wrapper.
    ///
    /// The attached [collector] will be set as the [default] when the returned
    /// [`Future`] is polled.
    ///
    /// # Examples
    ///
    /// ```
    /// # pub struct MyCollector;
    /// # impl tracing::Collect for MyCollector {
    /// #   fn new_span(&self, _: &tracing::span::Attributes) -> tracing::span::Id {
    /// #       tracing::span::Id::from_u64(0)
    /// #   }
    /// #   fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record) {}
    /// #   fn event(&self, _: &tracing::Event<'_>) {}
    /// #   fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    /// #   fn enabled(&self, _: &tracing::Metadata) -> bool { false }
    /// #   fn enter(&self, _: &tracing::span::Id) {}
    /// #   fn exit(&self, _: &tracing::span::Id) {}
    /// #   fn current_span(&self) -> tracing_core::span::Current {
    /// #       tracing_core::span::Current::unknown()
    /// #    }
    /// # }
    /// # impl MyCollector { fn new() -> Self { Self } }
    /// # async fn docs() {
    /// use tracing::instrument::WithCollector;
    ///
    /// // Set the default collector
    /// let _default = tracing::collect::set_default(MyCollector::new());
    ///
    /// tracing::info!("this event will be recorded by the default collector");
    ///
    /// // Create a different collector and attach it to a future.
    /// let other_collector = MyCollector::new();
    /// let future = async {
    ///     tracing::info!("this event will be recorded by the other collector");
    ///     // ...
    /// };
    ///
    /// future
    ///     // Attach the other collector to the future before awaiting it
    ///     .with_collector(other_collector)
    ///     .await;
    ///
    /// // Once the future has completed, we return to the default collector.
    /// tracing::info!("this event will be recorded by the default collector");
    /// # }
    /// ```
    ///
    /// [collector]: super::Collect
    /// [default]: crate::dispatch#setting-the-default-collector
    /// [`Future`]: std::future::Future
    fn with_collector<C>(self, collector: C) -> WithDispatch<Self>
    where
        C: Into<Dispatch>,
    {
        WithDispatch {
            inner: self,
            dispatch: collector.into(),
        }
    }

    /// Attaches the current [default] [collector] to this type, returning a
    /// [`WithDispatch`] wrapper.
    ///
    /// The attached collector will be set as the [default] when the returned
    /// [`Future`] is polled.
    ///
    /// This can be used to propagate the current dispatcher context when
    /// spawning a new future that may run on a different thread.
    ///
    /// # Examples
    ///
    /// ```
    /// # mod tokio {
    /// #     pub(super) fn spawn(_: impl std::future::Future) {}
    /// # }
    /// # pub struct MyCollector;
    /// # impl tracing::Collect for MyCollector {
    /// #   fn new_span(&self, _: &tracing::span::Attributes) -> tracing::span::Id {
    /// #       tracing::span::Id::from_u64(0)
    /// #   }
    /// #   fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record) {}
    /// #   fn event(&self, _: &tracing::Event<'_>) {}
    /// #   fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    /// #   fn enabled(&self, _: &tracing::Metadata) -> bool { false }
    /// #   fn enter(&self, _: &tracing::span::Id) {}
    /// #   fn exit(&self, _: &tracing::span::Id) {}
    /// #   fn current_span(&self) -> tracing_core::span::Current {
    /// #       tracing_core::span::Current::unknown()
    /// #    }
    /// # }
    /// # impl MyCollector { fn new() -> Self { Self } }
    /// # async fn docs() {
    /// use tracing::instrument::WithCollector;
    ///
    /// // Using `set_default` (rather than `set_global_default`) sets the
    /// // default collector for *this* thread only.
    /// let _default = tracing::collect::set_default(MyCollector::new());
    ///
    /// let future = async {
    ///     // ...
    /// };
    ///
    /// // If a multi-threaded async runtime is in use, this spawned task may
    /// // run on a different thread, in a different default collector's context.
    /// tokio::spawn(future);
    ///
    /// // However, calling `with_current_collector` on the future before
    /// // spawning it, ensures that the current thread's default collector is
    /// // propagated to the spawned task, regardless of where it executes:
    /// # let future = async { };
    /// tokio::spawn(future.with_current_collector());
    /// # }
    /// ```
    /// [collector]: super::Collect
    /// [default]: crate::dispatch#setting-the-default-collector
    /// [`Future`]: std::future::Future
    #[inline]
    fn with_current_collector(self) -> WithDispatch<Self> {
        WithDispatch {
            inner: self,
            dispatch: dispatch::get_default(|default| default.clone()),
        }
    }
}

#[cfg(feature = "std")]
pin_project! {
    /// A [`Future`] that has been instrumented with a `tracing` [collector].
    ///
    /// This type is returned by the [`WithCollector`] extension trait. See that
    /// trait's documentation for details.
    ///
    /// [`Future`]: std::future::Future
    /// [collector]: crate::Collector
    #[derive(Clone, Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub struct WithDispatch<T> {
        #[pin]
        inner: T,
        dispatch: Dispatch,
    }
}

pin_project! {
    /// A [`Future`] that has been instrumented with a `tracing` [`Span`].
    ///
    /// This type is returned by the [`Instrument`] extension trait. See that
    /// trait's documentation for details.
    ///
    /// [`Future`]: std::future::Future
    /// [`Span`]: crate::Span
    #[derive(Debug, Clone)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Instrumented<T> {
        #[pin]
        inner: T,
        span: Span,
    }
}

// === impl Instrumented ===

impl<T: Future> Future for Instrumented<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _enter = this.span.enter();
        this.inner.poll(cx)
    }
}

impl<T: Sized> Instrument for T {}

impl<T> Instrumented<T> {
    /// Borrows the `Span` that this type is instrumented by.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Mutably borrows the `Span` that this type is instrumented by.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Borrows the wrapped type.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the wrapped type.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Get a pinned reference to the wrapped type.
    pub fn inner_pin_ref(self: Pin<&Self>) -> Pin<&T> {
        self.project_ref().inner
    }

    /// Get a pinned mutable reference to the wrapped type.
    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().inner
    }

    /// Consumes the `Instrumented`, returning the wrapped type.
    ///
    /// Note that this drops the span.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

// === impl WithDispatch ===

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T: Future> Future for WithDispatch<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let dispatch = this.dispatch;
        let future = this.inner;
        let _default = dispatch::set_default(dispatch);
        future.poll(cx)
    }
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T: Sized> WithCollector for T {}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl<T> WithDispatch<T> {
    /// Borrows the [`Dispatch`] that is entered when this type is polled.
    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }

    /// Borrows the wrapped type.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the wrapped type.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Get a pinned reference to the wrapped type.
    pub fn inner_pin_ref(self: Pin<&Self>) -> Pin<&T> {
        self.project_ref().inner
    }

    /// Get a pinned mutable reference to the wrapped type.
    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().inner
    }

    /// Consumes the `Instrumented`, returning the wrapped type.
    ///
    /// Note that this drops the span.
    pub fn into_inner(self) -> T {
        self.inner
    }
}
