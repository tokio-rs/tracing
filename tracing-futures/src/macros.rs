#[cfg(feature = "tokio")]
#[doc(hidden)]
pub use tokio::spawn as __spawn;
#[cfg(all(feature = "tokio-executor", not(feature = "tokio")))]
#[doc(hidden)]
pub use tokio_executor::spawn as __spawn;

#[cfg(any(feature = "tokio", feature = "tokio-executor"))]
#[doc(hidden)]
pub use tracing::{span as __tracing_futures_span, Level as __Level};

/// Spawns a future on the default executor, instrumented with its own span.
///
/// # Spawning
///
/// This macro behaves similarly to the [`tokio::spawn`] function, but the
/// spawned future is instrumented with a new `tokio-trace` span prior to
/// spawning. This macro may be used as a drop-in replacement for `tokio::spawn`
/// in projects using `tokio-trace`.
///
/// In order for a future to do work, it must be spawned on an executor. The
/// `spawn` function is the easiest way to do this. It spawns a future on the
/// [default executor] for the current execution context (tracked using a
/// thread-local variable).
///
/// The default executor is **usually** a thread pool.
///
/// # Span Customization
///
/// By default, the span created by this macro is named by stringifying the
/// expression providing the future to spawn, is at the [`TRACE`] verbosity
/// level, and has the current module path as its target. However, if desired,
/// all of these may be overridden.
///
/// In addition, the span will always have a field `tokio.task.is_spawned` set
/// to `true`. This is intended for use by subscribers which wish to identify
/// what spans correspond to spawned tasks, regardless of other span metadata
/// (such as the name) which may be user-customizable. Additional information
/// about the task may be added in the future, in fields under the
/// `tokio.task.` namespace.
///
/// Overriding the name of the span:
///
/// ```rust
/// # extern crate futures;
/// #[macro_use]
/// extern crate tracing_futures;
/// # use futures::future;
/// # fn main() {
/// let fut = future::lazy(|| {
///     // ...
/// #    Ok(())
/// });
/// spawn!(name: "my_future", fut);
/// # }
/// ```
///
/// Overriding the target:
///
///```rust
/// # extern crate futures;
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # use futures::future;
/// # fn main() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(target: "spawned_futures", fut);
/// # }
/// ```
/// Overriding the level:
///
///```rust
/// # extern crate futures;
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # extern crate tracing;
/// # use futures::future;
/// use tracing::Level;
///
/// # fn main() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(level: Level::INFO, fut);
/// # }
/// ```
///
/// Any number of metadata items may be overridden:
///
///```rust
/// # extern crate futures;
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # extern crate tracing;
/// # use futures::future;
/// # use tracing::Level;
/// # fn main() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(level: Level::WARN, target: "spawned_futures", name: "a_bad_future", fut);
/// # }
/// ```
///
/// Adding fields to the span:
///
///```rust
/// # extern crate futures;
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # extern crate tracing;
/// # use futures::future;
/// # fn main() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(fut, foo = "bar", baz = 42);
/// # }
/// ```
///
///```rust
/// # extern crate futures;
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # extern crate tracing;
/// # use futures::future;
/// # fn main() {
/// for i in 0..10 {
///     let fut = future::lazy(|| {
///         // ...
///         # Ok(())
///     });
///     spawn!(fut, number = 1);
/// }
// # }
/// # }
/// ```
/// # Examples
///
/// In this example, based on the example in the documentation for
/// [`tokio::spawn`], a server is started and `spawn!` is used to start
/// a new task that processes each received connection.
///
/// ```rust
/// # extern crate tokio;
/// # extern crate futures;
/// #[macro_use]
/// extern crate tracing_futures;
/// # use futures::{Future, Stream};
/// use tokio::net::TcpListener;
///
/// # fn process<T>(_: T) -> Box<Future<Item = (), Error = ()> + Send> {
/// # unimplemented!();
/// # }
/// # fn dox() {
/// # let addr = "127.0.0.1:8080".parse().unwrap();
/// let listener = TcpListener::bind(&addr).unwrap();
///
/// let server = listener.incoming()
///     .map_err(|e| println!("error = {:?}", e))
///     .for_each(|socket| {
///         spawn!(process(socket))
///     });
///
/// tokio::run(server);
/// # }
/// # pub fn main() {}
/// ```
///
/// # Panics
///
/// This function will panic if the default executor is not set or if spawning
/// onto the default executor returns an error. To avoid the panic, use
/// [`DefaultExecutor`].
///
/// [default executor]: https://docs.rs/tokio/latest/tokio/executor/struct.DefaultExecutor.html
/// [`DefaultExecutor`]: https://docs.rs/tokio/latest/tokio/executor/struct.DefaultExecutor.html
/// [`tokio::spawn`]: https://docs.rs/tokio/latest/tokio/executor/fn.spawn.html
/// [`TRACE` verbosity level]: https://docs.rs/tokio-trace/latest/tracing/struct.Level.html#associatedconstant.TRACE
#[cfg(any(feature = "tokio", feature = "tokio-executor"))]
#[macro_export(inner_local_macros)]
macro_rules! spawn {
    (level: $lvl:expr, target: $tgt:expr, name: $name:expr, $fut:expr, $($field:tt)*) => {{
        use $crate::macros::__spawn;
        use $crate::Instrument;
        let span = $crate::macros::__tokio_trace_futures_span!(
            $lvl,
            target: $tgt,
            $name,
            tokio.task.is_spawned = true,
            $($field)*
        );
        let fut = Box::new($fut.instrument(span));
        __spawn(fut)
    }};
    (level: $lvl:expr, name: $name:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: __tracing_futures_module_path!(),
            name: $name,
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, target: $tgt:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: $tgt,
            name: __tracing_futures_stringify!($fut),
            $fut,
            $($field)*
        )
    };
    (target: $tgt:expr, name: $name:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $crate::macros::__Level::TRACE,
            target: $tgt,
            name: $name,
            $fut,
            $($field)*
        )
    };
    (target: $tgt:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $crate::macros::__Level::TRACE,
            target: $tgt,
            $fut,
            $($field)*
        )
    };
    (name: $name:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            target: __tracing_futures_module_path!(),
            name: $name,
            $fut,
            $($field)*
        )
    };
    ($fut:expr, $($field:tt)*) => {
        spawn!(name: __tracing_futures_stringify!($fut), $fut, $($field)*)
    };
    (level: $lvl:expr, target: $tgt:expr, name: $name:expr, $fut:expr) => {
        spawn!(level: $lvl, target: $tgt, name: $name, $fut,)
    };
    (level: $lvl:expr, name: $name:expr, $fut:expr) => {
        spawn!(level: $lvl, name: $name, $fut,)
    };
    (level: $lvl:expr, target: $tgt:expr, $fut:expr) => {
        spawn!(level: $lvl, target: $tgt, $fut,)
    };
    (target: $tgt:expr, name: $name:expr, $fut:expr) => {
        spawn!(target: $tgt, name: $name, $fut,)
    };
    (target: $tgt:expr, $fut:expr) => {
        spawn!(target: $tgt, $fut,)
    };
    (name: $name:expr, $fut:expr) => {
        spawn!(name: $name, $fut,)
    };
    ($fut:expr) => {
        spawn!($fut,)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tracing_futures_module_path {
    () => {
        module_path!()
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __tracing_futures_stringify {
    ($ex:expr) => {
        stringify!($ex)
    };
}
