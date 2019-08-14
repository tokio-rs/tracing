use crate::{Instrument, Instrumented, WithDispatch};
use std::future::Future;
use tokio_01::executor::{Executor, SpawnError, TypedExecutor};

impl<T> Executor for Instrumented<T>
where
    T: Executor,
{
    /// Spawns a future object to run on this executor.
    ///
    /// `future` is passed to the executor, which will begin running it. The
    /// future may run on the current thread or another thread at the discretion
    /// of the `Executor` implementation.
    ///
    /// # Panics
    ///
    /// Implementations are encouraged to avoid panics. However, panics are
    /// permitted and the caller should check the implementation specific
    /// documentation for more details on possible panics.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    ///
    /// use tokio_executor::Executor;
    ///
    /// # fn docs(my_executor: &mut dyn Executor) {
    /// my_executor.spawn(Box::pin(async {
    ///     println!("running on the executor");
    /// })).unwrap();
    /// # }
    /// ```
    fn spawn(
        &mut self,
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> Result<(), SpawnError> {
        let future = future.instrument(self.span.clone());
        self.inner.spawn(Box::pin(future))
    }

    /// Provides a best effort **hint** to whether or not `spawn` will succeed.
    ///
    /// This function may return both false positives **and** false negatives.
    /// If `status` returns `Ok`, then a call to `spawn` will *probably*
    /// succeed, but may fail. If `status` returns `Err`, a call to `spawn` will
    /// *probably* fail, but may succeed.
    ///
    /// This allows a caller to avoid creating the task if the call to `spawn`
    /// has a high likelihood of failing.
    ///
    /// # Panics
    ///
    /// This function must not panic. Implementers must ensure that panics do
    /// not happen.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    ///
    /// use tokio_executor::Executor;
    ///
    /// # fn docs(my_executor: &mut dyn Executor) {
    /// if my_executor.status().is_ok() {
    ///     my_executor.spawn(Box::pin(async {
    ///         println!("running on the executor");
    ///     })).unwrap();
    /// } else {
    ///     println!("the executor is not in a good state");
    /// }
    /// # }
    /// ```
    fn status(&self) -> Result<(), SpawnError> {
        self.inner.status()
    }
}

impl<T> Executor for WithDispatch<T>
where
    T: Executor,
{
    /// Spawns a future object to run on this executor.
    ///
    /// `future` is passed to the executor, which will begin running it. The
    /// future may run on the current thread or another thread at the discretion
    /// of the `Executor` implementation.
    ///
    /// # Panics
    ///
    /// Implementations are encouraged to avoid panics. However, panics are
    /// permitted and the caller should check the implementation specific
    /// documentation for more details on possible panics.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    ///
    /// use tokio_executor::Executor;
    ///
    /// # fn docs(my_executor: &mut dyn Executor) {
    /// my_executor.spawn(Box::pin(async {
    ///     println!("running on the executor");
    /// })).unwrap();
    /// # }
    /// ```
    fn spawn(
        &mut self,
        future: Pin<Box<dyn Future<Output = ()> + Send>>,
    ) -> Result<(), SpawnError> {
        let future = self.with_dispatch(future);
        self.inner.spawn(Box::pin(future))
    }

    /// Provides a best effort **hint** to whether or not `spawn` will succeed.
    ///
    /// This function may return both false positives **and** false negatives.
    /// If `status` returns `Ok`, then a call to `spawn` will *probably*
    /// succeed, but may fail. If `status` returns `Err`, a call to `spawn` will
    /// *probably* fail, but may succeed.
    ///
    /// This allows a caller to avoid creating the task if the call to `spawn`
    /// has a high likelihood of failing.
    ///
    /// # Panics
    ///
    /// This function must not panic. Implementers must ensure that panics do
    /// not happen.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(async_await)]
    ///
    /// use tokio_executor::Executor;
    ///
    /// # fn docs(my_executor: &mut dyn Executor) {
    /// if my_executor.status().is_ok() {
    ///     my_executor.spawn(Box::pin(async {
    ///         println!("running on the executor");
    ///     })).unwrap();
    /// } else {
    ///     println!("the executor is not in a good state");
    /// }
    /// # }
    /// ```
    fn status(&self) -> Result<(), SpawnError> {
        self.inner.status()
    }
}

impl<T, F> TypedExecutor<F> for Instrumented<T>
where
    T: TypedExecutor<Instrumented<F>>,
{
    /// Spawns a future to run on this executor.
    ///
    /// `future` is passed to the executor, which will begin running it. The
    /// executor takes ownership of the future and becomes responsible for
    /// driving the future to completion.
    ///
    /// # Panics
    ///
    /// Implementations are encouraged to avoid panics. However, panics are
    /// permitted and the caller should check the implementation specific
    /// documentation for more details on possible panics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio_executor::TypedExecutor;
    ///
    /// use std::future::Future;
    /// use std::pin::Pin;
    /// use std::task::{Context, Poll};
    ///
    /// fn example<T>(my_executor: &mut T)
    /// where
    ///     T: TypedExecutor<MyFuture>,
    /// {
    ///     my_executor.spawn(MyFuture).unwrap();
    /// }
    ///
    /// struct MyFuture;
    ///
    /// impl Future for MyFuture {
    ///     type Output = ();
    ///
    ///     fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
    ///         println!("running on the executor");
    ///         Poll::Ready(())
    ///     }
    /// }
    /// ```
    fn spawn(&mut self, future: F) -> Result<(), SpawnError> {
        let future = future.instrument(self.span.clone());
        self.inner.spawn(future)
    }

    /// Provides a best effort **hint** to whether or not `spawn` will succeed.
    ///
    /// This function may return both false positives **and** false negatives.
    /// If `status` returns `Ok`, then a call to `spawn` will *probably*
    /// succeed, but may fail. If `status` returns `Err`, a call to `spawn` will
    /// *probably* fail, but may succeed.
    ///
    /// This allows a caller to avoid creating the task if the call to `spawn`
    /// has a high likelihood of failing.
    ///
    /// # Panics
    ///
    /// This function must not panic. Implementers must ensure that panics do
    /// not happen.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio_executor::TypedExecutor;
    ///
    /// use std::future::Future;
    /// use std::pin::Pin;
    /// use std::task::{Context, Poll};
    ///
    /// fn example<T>(my_executor: &mut T)
    /// where
    ///     T: TypedExecutor<MyFuture>,
    /// {
    ///     if my_executor.status().is_ok() {
    ///         my_executor.spawn(MyFuture).unwrap();
    ///     } else {
    ///         println!("the executor is not in a good state");
    ///     }
    /// }
    ///
    /// struct MyFuture;
    ///
    /// impl Future for MyFuture {
    ///     type Output = ();
    ///
    ///     fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
    ///         println!("running on the executor");
    ///         Poll::Ready(())
    ///     }
    /// }
    /// ```
    fn status(&self) -> Result<(), SpawnError> {
        self.inner.status()
    }
}

impl<T, F> TypedExecutor<F> for WithDispatch<T>
where
    T: TypedExecutor<WithDispatch<F>>,
{
    /// Spawns a future to run on this executor.
    ///
    /// `future` is passed to the executor, which will begin running it. The
    /// executor takes ownership of the future and becomes responsible for
    /// driving the future to completion.
    ///
    /// # Panics
    ///
    /// Implementations are encouraged to avoid panics. However, panics are
    /// permitted and the caller should check the implementation specific
    /// documentation for more details on possible panics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio_executor::TypedExecutor;
    ///
    /// use std::future::Future;
    /// use std::pin::Pin;
    /// use std::task::{Context, Poll};
    ///
    /// fn example<T>(my_executor: &mut T)
    /// where
    ///     T: TypedExecutor<MyFuture>,
    /// {
    ///     my_executor.spawn(MyFuture).unwrap();
    /// }
    ///
    /// struct MyFuture;
    ///
    /// impl Future for MyFuture {
    ///     type Output = ();
    ///
    ///     fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
    ///         println!("running on the executor");
    ///         Poll::Ready(())
    ///     }
    /// }
    /// ```
    fn spawn(&mut self, future: F) -> Result<(), SpawnError> {
        let future = self.with_dispatch(future);
        self.inner.spawn(future)
    }

    /// Provides a best effort **hint** to whether or not `spawn` will succeed.
    ///
    /// This function may return both false positives **and** false negatives.
    /// If `status` returns `Ok`, then a call to `spawn` will *probably*
    /// succeed, but may fail. If `status` returns `Err`, a call to `spawn` will
    /// *probably* fail, but may succeed.
    ///
    /// This allows a caller to avoid creating the task if the call to `spawn`
    /// has a high likelihood of failing.
    ///
    /// # Panics
    ///
    /// This function must not panic. Implementers must ensure that panics do
    /// not happen.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio_executor::TypedExecutor;
    ///
    /// use std::future::Future;
    /// use std::pin::Pin;
    /// use std::task::{Context, Poll};
    ///
    /// fn example<T>(my_executor: &mut T)
    /// where
    ///     T: TypedExecutor<MyFuture>,
    /// {
    ///     if my_executor.status().is_ok() {
    ///         my_executor.spawn(MyFuture).unwrap();
    ///     } else {
    ///         println!("the executor is not in a good state");
    ///     }
    /// }
    ///
    /// struct MyFuture;
    ///
    /// impl Future for MyFuture {
    ///     type Output = ();
    ///
    ///     fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
    ///         println!("running on the executor");
    ///         Poll::Ready(())
    ///     }
    /// }
    /// ```
    fn status(&self) -> Result<(), SpawnError> {
        self.inner.status()
    }
}
