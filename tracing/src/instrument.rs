use crate::stdlib::pin::Pin;
use crate::stdlib::task::{Context, Poll};
use crate::stdlib::{future::Future, marker::Sized};
use crate::{dispatcher, span::Span, Dispatch};

/// Attaches spans to a `std::future::Future`.
///
/// Extension trait allowing futures to be
/// instrumented with a `tracing` [span].
///
/// [span]: https://docs.rs/tracing/latest/tracing/span/index.html
pub trait Instrument: Sized {
    /// Instruments this type with the provided `Span`, returning an
    /// `Instrumented` wrapper.
    ///
    /// If the instrumented type is a future, stream, or sink, the attached `Span`
    /// will be [entered] every time it is polled. If the instrumented type
    /// is a future executor, every future spawned on that executor will be
    /// instrumented by the attached `Span`.
    ///
    /// # Examples
    ///
    /// Instrumenting a future:
    ///
    // TODO: ignored until async-await is stable...
    /// ```rust,ignore
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
    /// [entered]: https://docs.rs/tracing/latest/tracing/span/struct.Span.html#method.enter
    fn instrument(self, span: Span) -> Instrumented<Self> {
        Instrumented { inner: self, span }
    }

    /// Instruments this type with the [current] `Span`, returning an
    /// `Instrumented` wrapper.
    ///
    /// If the instrumented type is a future, stream, or sink, the attached `Span`
    /// will be [entered] every time it is polled. If the instrumented type
    /// is a future executor, every future spawned on that executor will be
    /// instrumented by the attached `Span`.
    ///
    /// This can be used to propagate the current span when spawning a new future.
    ///
    /// # Examples
    ///
    // TODO: ignored until async-await is stable...
    /// ```rust,ignore
    /// use tracing::Instrument;
    ///
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
    /// [current]: https://docs.rs/tracing/latest/tracing/span/struct.Span.html#method.current
    /// [entered]: https://docs.rs/tracing/latest/tracing/span/struct.Span.html#method.enter
    #[inline]
    fn in_current_span(self) -> Instrumented<Self> {
        self.instrument(Span::current())
    }
}

/// Extension trait allowing futures to be instrumented with
/// a `tracing` [`Subscriber`].
///
/// [`Subscriber`]: https://docs.rs/tracing/latest/tracing/subscriber/trait.Subscriber.html
pub trait WithSubscriber: Sized {
    /// Attaches the provided [`Subscriber`] to this type, returning a
    /// `WithDispatch` wrapper.
    ///
    /// When the wrapped type is a future, stream, or sink, the attached
    /// subscriber will be set as the [default] while it is being polled.
    /// When the wrapped type is an executor, the subscriber will be set as the
    /// default for any futures spawned on that executor.
    ///
    /// [`Subscriber`]: https://docs.rs/tracing/latest/tracing/subscriber/trait.Subscriber.html
    /// [default]: https://docs.rs/tracing/latest/tracing/dispatcher/index.html#setting-the-default-subscriber
    fn with_subscriber<S>(self, subscriber: S) -> WithDispatch<Self>
    where
        S: Into<Dispatch>,
    {
        WithDispatch {
            inner: self,
            dispatch: subscriber.into(),
        }
    }

    /// Attaches the current [default] [`Subscriber`] to this type, returning a
    /// `WithDispatch` wrapper.
    ///
    /// When the wrapped type is a future, stream, or sink, the attached
    /// subscriber will be set as the [default] while it is being polled.
    /// When the wrapped type is an executor, the subscriber will be set as the
    /// default for any futures spawned on that executor.
    ///
    /// This can be used to propagate the current dispatcher context when
    /// spawning a new future.
    ///
    /// [`Subscriber`]: https://docs.rs/tracing/latest/tracing/subscriber/trait.Subscriber.html
    /// [default]: https://docs.rs/tracing/latest/tracing/dispatcher/index.html#setting-the-default-subscriber
    #[inline]
    fn with_current_subscriber(self) -> WithDispatch<Self> {
        WithDispatch {
            inner: self,
            dispatch: dispatcher::get_default(|default| default.clone()),
        }
    }
}

/// A future, stream, sink, or executor that has been instrumented with a
/// `tracing` subscriber.
#[pin_project::pin_project]
#[derive(Clone, Debug)]
pub struct WithDispatch<T> {
    #[pin]
    inner: T,
    dispatch: Dispatch,
}

/// A future that has been instrumented with a `tracing` span.
#[pin_project::pin_project]
#[derive(Debug, Clone)]
pub struct Instrumented<T> {
    #[pin]
    inner: T,
    span: Span,
}

impl<T: Future> Future for Instrumented<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _enter = this.span.enter();
        this.inner.poll(cx)
    }
}

impl<T: Future + Sized> Instrument for T {}

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
