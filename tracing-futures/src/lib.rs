//! Futures compatibility for [`tracing`].
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate provides utilities
//! for using `tracing` to instrument asynchronous code written using futures and
//! async/await.
//!
//! The crate provides the following traits:
//!
//! * [`Instrument`] allows a `tracing` [span] to be attached to a future, sink,
//!   stream, or executor.
//!
//! * [`WithCollector`] allows a `tracing` [collector] to be attached to a
//!   future, sink, stream, or executor.
//!
//! *Compiler support: [requires `rustc` 1.63+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! # Feature flags
//!
//! This crate provides a number of feature flags that enable compatibility
//! features with other crates in the asynchronous ecosystem:
//!
//! - `tokio`: Enables compatibility with the `tokio` crate, including
//!    [`Instrument`] and [`WithCollector`] implementations for
//!    `tokio::executor::Executor`, `tokio::runtime::Runtime`, and
//!    `tokio::runtime::current_thread`. Enabled by default.
//! - `tokio-executor`: Enables compatibility with the `tokio-executor`
//!    crate, including [`Instrument`] and [`WithCollector`]
//!    implementations for types implementing `tokio_executor::Executor`.
//!    This is intended primarily for use in crates which depend on
//!    `tokio-executor` rather than `tokio`; in general the `tokio` feature
//!    should be used instead.
//! - `std-future`: Enables compatibility with `std::future::Future`.
//! - `futures-01`: Enables compatibility with version 0.1.x of the [`futures`]
//!   crate.
//! - `futures-03`: Enables compatibility with version 0.3.x of the `futures`
//!   crate's `Spawn` and `LocalSpawn` traits.
//! - `tokio-alpha`: Enables compatibility with `tokio` 0.2's alpha releases,
//!   including the `tokio` 0.2 `Executor` and `TypedExecutor` traits.
//! - `std`: Depend on the Rust standard library.
//!
//!   `no_std` users may disable this feature with `default-features = false`:
//!
//!   ```toml
//!   [dependencies]
//!   tracing-futures = { version = "0.2.3", default-features = false }
//!   ```
//!
//! The `tokio`, `std-future` and `std` features are enabled by default.
//!
//! [`tracing`]: https://crates.io/crates/tracing
//! [span]: mod@tracing::span
//! [collector]: tracing::collect
//! [`futures`]: https://crates.io/crates/futures
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.63. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.69, the minimum supported version will not be
//! increased past 1.66, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_interfaces,
    private_bounds,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#[cfg(feature = "std-future")]
use pin_project_lite::pin_project;

#[cfg(feature = "std-future")]
use core::{
    mem::{self, ManuallyDrop},
    pin::Pin,
    task::Context,
};

#[cfg(feature = "std")]
use tracing::{dispatch, Dispatch};

use tracing::Span;

/// Implementations for `Instrument`ed future executors.
pub mod executor;

/// Extension trait allowing futures, streams, sinks, and executors to be
/// instrumented with a `tracing` [span].
///
/// [span]: mod@tracing::span
pub trait Instrument: Sized {
    /// Instruments this type with the provided [`Span`], returning an
    /// [`Instrumented`] wrapper.
    ///
    /// If the instrumented type is a future, stream, or sink, the attached
    /// [`Span`] will be [entered] every time it is polled or [`Drop`]ped. If
    /// the instrumented type is a future executor, every future spawned on that
    /// executor will be instrumented by the attached [`Span`].
    ///
    /// # Examples
    ///
    /// Instrumenting a future:
    ///
    // TODO: ignored until async-await is stable...
    /// ```rust,ignore
    /// use tracing_futures::Instrument;
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
    /// [entered]: Span::enter()
    fn instrument(self, span: Span) -> Instrumented<Self> {
        #[cfg(feature = "std-future")]
        let inner = ManuallyDrop::new(self);
        #[cfg(not(feature = "std-future"))]
        let inner = self;
        Instrumented { inner, span }
    }

    /// Instruments this type with the [current] [`Span`], returning an
    /// [`Instrumented`] wrapper.
    ///
    /// If the instrumented type is a future, stream, or sink, the attached
    /// [`Span`] will be [entered] every time it is polled or [`Drop`]ped. If
    /// the instrumented type is a future executor, every future spawned on that
    /// executor will be instrumented by the attached [`Span`].
    ///
    /// This can be used to propagate the current span when spawning a new future.
    ///
    /// # Examples
    ///
    // TODO: ignored until async-await is stable...
    /// ```rust,ignore
    /// use tracing_futures::Instrument;
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
    /// [current]: Span::current()
    /// [entered]: Span::enter()
    #[inline]
    fn in_current_span(self) -> Instrumented<Self> {
        self.instrument(Span::current())
    }
}

/// Extension trait allowing futures, streams, and sinks to be instrumented with
/// a `tracing` [collector].
///
/// [collector]: tracing::collect::Collect
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub trait WithCollector: Sized {
    /// Attaches the provided [collector] to this type, returning a
    /// `WithDispatch` wrapper.
    ///
    /// When the wrapped type is a future, stream, or sink, the attached
    /// subscriber will be set as the [default] while it is being polled.
    /// When the wrapped type is an executor, the subscriber will be set as the
    /// default for any futures spawned on that executor.
    ///
    /// [collector]: tracing::collect::Collect
    /// [default]: tracing::dispatch#setting-the-default-collector
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
    /// [collector]: tracing::collect::Collect
    /// [default]: tracing::dispatch#setting-the-default-collector
    #[inline]
    fn with_current_collector(self) -> WithDispatch<Self> {
        WithDispatch {
            inner: self,
            dispatch: dispatch::get_default(|default| default.clone()),
        }
    }
}

#[cfg(feature = "std-future")]
pin_project! {
    /// A future, stream, sink, or executor that has been instrumented with a `tracing` span.
    #[project = InstrumentedProj]
    #[project_ref = InstrumentedProjRef]
    #[derive(Debug, Clone)]
    pub struct Instrumented<T> {
        // `ManuallyDrop` is used here to to enter instrument `Drop` by entering
        // `Span` and executing `ManuallyDrop::drop`.
        #[pin]
        inner: ManuallyDrop<T>,
        span: Span,
    }

    impl<T> PinnedDrop for Instrumented<T> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            let _enter = this.span.enter();
            // SAFETY: 1. `Pin::get_unchecked_mut()` is safe, because this isn't
            //             different from wrapping `T` in `Option` and calling
            //             `Pin::set(&mut this.inner, None)`, except avoiding
            //             additional memory overhead.
            //         2. `ManuallyDrop::drop()` is safe, because
            //            `PinnedDrop::drop()` is guaranteed to be called only
            //            once.
            unsafe { ManuallyDrop::drop(this.inner.get_unchecked_mut()) }
        }
    }
}

#[cfg(feature = "std-future")]
impl<'a, T> InstrumentedProj<'a, T> {
    /// Get a mutable reference to the [`Span`] a pinned mutable reference to
    /// the wrapped type.
    fn span_and_inner_pin_mut(self) -> (&'a mut Span, Pin<&'a mut T>) {
        // SAFETY: As long as `ManuallyDrop<T>` does not move, `T` won't move
        //         and `inner` is valid, because `ManuallyDrop::drop` is called
        //         only inside `Drop` of the `Instrumented`.
        let inner = unsafe { self.inner.map_unchecked_mut(|v| &mut **v) };
        (self.span, inner)
    }
}

#[cfg(feature = "std-future")]
impl<'a, T> InstrumentedProjRef<'a, T> {
    /// Get a reference to the [`Span`] a pinned reference to the wrapped type.
    fn span_and_inner_pin_ref(self) -> (&'a Span, Pin<&'a T>) {
        // SAFETY: As long as `ManuallyDrop<T>` does not move, `T` won't move
        //         and `inner` is valid, because `ManuallyDrop::drop` is called
        //         only inside `Drop` of the `Instrumented`.
        let inner = unsafe { self.inner.map_unchecked(|v| &**v) };
        (self.span, inner)
    }
}

/// A future, stream, sink, or executor that has been instrumented with a `tracing` span.
#[cfg(not(feature = "std-future"))]
#[derive(Debug, Clone)]
pub struct Instrumented<T> {
    inner: T,
    span: Span,
}

#[cfg(all(feature = "std", feature = "std-future"))]
pin_project! {
    /// A future, stream, sink, or executor that has been instrumented with a
    /// `tracing` subscriber.
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    #[derive(Clone, Debug)]
    pub struct WithDispatch<T> {
        #[pin]
        inner: T,
        dispatch: Dispatch,
    }
}

/// A future, stream, sink, or executor that has been instrumented with a
/// `tracing` subscriber.
#[cfg(all(feature = "std", not(feature = "std-future")))]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Clone, Debug)]
pub struct WithDispatch<T> {
    inner: T,
    dispatch: Dispatch,
}

impl<T: Sized> Instrument for T {}

#[cfg(feature = "std-future")]
#[cfg_attr(docsrs, doc(cfg(feature = "std-future")))]
impl<T: core::future::Future> core::future::Future for Instrumented<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
        let (span, inner) = self.project().span_and_inner_pin_mut();
        let _enter = span.enter();
        inner.poll(cx)
    }
}

#[cfg(feature = "futures-01")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures-01")))]
impl<T: futures_01::Future> futures_01::Future for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> futures_01::Poll<Self::Item, Self::Error> {
        let _enter = self.span.enter();
        self.inner.poll()
    }
}

#[cfg(feature = "futures-01")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures-01")))]
impl<T: futures_01::Stream> futures_01::Stream for Instrumented<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> futures_01::Poll<Option<Self::Item>, Self::Error> {
        let _enter = self.span.enter();
        self.inner.poll()
    }
}

#[cfg(feature = "futures-01")]
#[cfg_attr(docsrs, doc(cfg(feature = "futures-01")))]
impl<T: futures_01::Sink> futures_01::Sink for Instrumented<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(
        &mut self,
        item: Self::SinkItem,
    ) -> futures_01::StartSend<Self::SinkItem, Self::SinkError> {
        let _enter = self.span.enter();
        self.inner.start_send(item)
    }

    fn poll_complete(&mut self) -> futures_01::Poll<(), Self::SinkError> {
        let _enter = self.span.enter();
        self.inner.poll_complete()
    }
}

#[cfg(all(feature = "futures-03", feature = "std-future"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "futures-03", feature = "std-future"))))]
impl<T: futures::Stream> futures::Stream for Instrumented<T> {
    type Item = T::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> futures::task::Poll<Option<Self::Item>> {
        let (span, inner) = self.project().span_and_inner_pin_mut();
        let _enter = span.enter();
        T::poll_next(inner, cx)
    }
}

#[cfg(all(feature = "futures-03", feature = "std-future"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "futures-03", feature = "std-future"))))]
impl<I, T: futures::Sink<I>> futures::Sink<I> for Instrumented<T>
where
    T: futures::Sink<I>,
{
    type Error = T::Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> futures::task::Poll<Result<(), Self::Error>> {
        let (span, inner) = self.project().span_and_inner_pin_mut();
        let _enter = span.enter();
        T::poll_ready(inner, cx)
    }

    fn start_send(self: Pin<&mut Self>, item: I) -> Result<(), Self::Error> {
        let (span, inner) = self.project().span_and_inner_pin_mut();
        let _enter = span.enter();
        T::start_send(inner, item)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> futures::task::Poll<Result<(), Self::Error>> {
        let (span, inner) = self.project().span_and_inner_pin_mut();
        let _enter = span.enter();
        T::poll_flush(inner, cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> futures::task::Poll<Result<(), Self::Error>> {
        let (span, inner) = self.project().span_and_inner_pin_mut();
        let _enter = span.enter();
        T::poll_close(inner, cx)
    }
}

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
    #[cfg(feature = "std-future")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std-future")))]
    pub fn inner_pin_ref(self: Pin<&Self>) -> Pin<&T> {
        self.project_ref().span_and_inner_pin_ref().1
    }

    /// Get a pinned mutable reference to the wrapped type.
    #[cfg(feature = "std-future")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std-future")))]
    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().span_and_inner_pin_mut().1
    }

    /// Consumes the `Instrumented`, returning the wrapped type.
    ///
    /// Note that this drops the span.
    pub fn into_inner(self) -> T {
        #[cfg(feature = "std-future")]
        {
            // To manually destructure `Instrumented` without `Drop`, we save
            // pointers to the fields and use `mem::forget` to leave those pointers
            // valid.
            let span: *const Span = &self.span;
            let inner: *const ManuallyDrop<T> = &self.inner;
            mem::forget(self);
            // SAFETY: Those pointers are valid for reads, because `Drop` didn't
            //         run, and properly aligned, because `Instrumented` isn't
            //         `#[repr(packed)]`.
            let _span = unsafe { span.read() };
            let inner = unsafe { inner.read() };
            ManuallyDrop::into_inner(inner)
        }
        #[cfg(not(feature = "std-future"))]
        self.inner
    }
}

#[cfg(feature = "std")]
impl<T: Sized> WithCollector for T {}

#[cfg(all(feature = "futures-01", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "futures-01", feature = "std"))))]
impl<T: futures_01::Future> futures_01::Future for WithDispatch<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> futures_01::Poll<Self::Item, Self::Error> {
        let inner = &mut self.inner;
        dispatch::with_default(&self.dispatch, || inner.poll())
    }
}

#[cfg(all(feature = "std-future", feature = "std"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "std-future", feature = "std"))))]
impl<T: core::future::Future> core::future::Future for WithDispatch<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
        let this = self.project();
        let dispatch = this.dispatch;
        let future = this.inner;
        dispatch::with_default(dispatch, || future.poll(cx))
    }
}

#[cfg(feature = "std")]
impl<T> WithDispatch<T> {
    /// Wrap a future, stream, sink or executor with the same subscriber as this WithDispatch.
    pub fn with_dispatch<U>(&self, inner: U) -> WithDispatch<U> {
        WithDispatch {
            dispatch: self.dispatch.clone(),
            inner,
        }
    }

    /// Borrows the `Dispatch` that this type is instrumented by.
    pub fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }

    /// Get a pinned reference to the wrapped type.
    #[cfg(feature = "std-future")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std-future")))]
    pub fn inner_pin_ref(self: Pin<&Self>) -> Pin<&T> {
        self.project_ref().inner
    }

    /// Get a pinned mutable reference to the wrapped type.
    #[cfg(feature = "std-future")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std-future")))]
    pub fn inner_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().inner
    }

    /// Borrows the wrapped type.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the wrapped type.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the `WithDispatch`, returning the wrapped type.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_mock::*;

    #[cfg(feature = "futures-01")]
    mod futures_01_tests {
        use futures_01::{future, stream, task, Async, Future, Stream};
        use tracing::collect::with_default;

        use super::*;

        struct PollN<T, E> {
            and_return: Option<Result<T, E>>,
            finish_at: usize,
            polls: usize,
        }

        impl PollN<(), ()> {
            fn new_ok(finish_at: usize) -> Self {
                Self {
                    and_return: Some(Ok(())),
                    finish_at,
                    polls: 0,
                }
            }

            fn new_err(finish_at: usize) -> Self {
                Self {
                    and_return: Some(Err(())),
                    finish_at,
                    polls: 0,
                }
            }
        }

        impl<T, E> futures_01::Future for PollN<T, E> {
            type Item = T;
            type Error = E;
            fn poll(&mut self) -> futures_01::Poll<Self::Item, Self::Error> {
                self.polls += 1;
                if self.polls == self.finish_at {
                    self.and_return
                        .take()
                        .expect("polled after ready")
                        .map(Async::Ready)
                } else {
                    task::current().notify();
                    Ok(Async::NotReady)
                }
            }
        }

        #[test]
        fn future_enter_exit_is_reasonable() {
            let (collector, handle) = collector::mock()
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .drop_span(expect::span().named("foo"))
                .only()
                .run_with_handle();
            with_default(collector, || {
                PollN::new_ok(2)
                    .instrument(tracing::trace_span!("foo"))
                    .wait()
                    .unwrap();
            });
            handle.assert_finished();
        }

        #[test]
        fn future_error_ends_span() {
            let (collector, handle) = collector::mock()
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .drop_span(expect::span().named("foo"))
                .only()
                .run_with_handle();
            with_default(collector, || {
                PollN::new_err(2)
                    .instrument(tracing::trace_span!("foo"))
                    .wait()
                    .unwrap_err();
            });

            handle.assert_finished();
        }

        #[test]
        fn stream_enter_exit_is_reasonable() {
            let (collector, handle) = collector::mock()
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .drop_span(expect::span().named("foo"))
                .run_with_handle();
            with_default(collector, || {
                stream::iter_ok::<_, ()>(&[1, 2, 3])
                    .instrument(tracing::trace_span!("foo"))
                    .for_each(|_| future::ok(()))
                    .wait()
                    .unwrap();
            });
            handle.assert_finished();
        }

        // #[test]
        // fn span_follows_future_onto_threadpool() {
        //     let (collector, handle) = collector::mock()
        //         .enter(span::mock().named("a"))
        //         .enter(span::mock().named("b"))
        //         .exit(span::mock().named("b"))
        //         .enter(span::mock().named("b"))
        //         .exit(span::mock().named("b"))
        //         .drop_span(span::mock().named("b"))
        //         .exit(span::mock().named("a"))
        //         .drop_span(span::mock().named("a"))
        //         .only()
        //         .run_with_handle();
        //     let mut runtime = tokio::runtime::Runtime::new().unwrap();
        //     with_default(collector, || {
        //         tracing::trace_span!("a").in_scope(|| {
        //             let future = PollN::new_ok(2)
        //                 .instrument(tracing::trace_span!("b"))
        //                 .map(|_| {
        //                     tracing::trace_span!("c").in_scope(|| {
        //                         // "c" happens _outside_ of the instrumented future's
        //                         // span, so we don't expect it.
        //                     })
        //                 });
        //             runtime.block_on(Box::new(future)).unwrap();
        //         })
        //     });
        //     handle.assert_finished();
        // }
    }

    #[cfg(all(feature = "futures-03", feature = "std-future"))]
    mod futures_03_tests {
        use futures::{future, sink, stream, FutureExt, SinkExt, StreamExt};
        use tracing::collect::with_default;

        use super::*;

        #[test]
        fn stream_enter_exit_is_reasonable() {
            let (collector, handle) = collector::mock()
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .drop_span(expect::span().named("foo"))
                .run_with_handle();
            with_default(collector, || {
                Instrument::instrument(stream::iter(&[1, 2, 3]), tracing::trace_span!("foo"))
                    .for_each(|_| future::ready(()))
                    .now_or_never()
                    .unwrap();
            });
            handle.assert_finished();
        }

        #[test]
        fn sink_enter_exit_is_reasonable() {
            let (collector, handle) = collector::mock()
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .enter(expect::span().named("foo"))
                .exit(expect::span().named("foo"))
                .drop_span(expect::span().named("foo"))
                .run_with_handle();
            with_default(collector, || {
                Instrument::instrument(sink::drain(), tracing::trace_span!("foo"))
                    .send(1u8)
                    .now_or_never()
                    .unwrap()
                    .unwrap()
            });
            handle.assert_finished();
        }
    }
}
