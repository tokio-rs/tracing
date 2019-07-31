#[cfg(feature = "tokio")]
#[doc(hidden)]
pub use tokio::executor::{DefaultExecutor, Executor as __Executor};
#[cfg(all(feature = "tokio-executor", not(feature = "tokio")))]
#[doc(hidden)]
pub use tokio_executor::executor::{DefaultExecutor, Executor as __Executor};

#[cfg(any(feature = "tokio", feature = "tokio-executor"))]
#[doc(hidden)]
pub use tracing::{field::debug, span as __tracing_futures_span, Level};

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
/// `spawn!` macro spawns a future on the provided executor, or using the
/// [default executor] for the current execution context if none is provided.
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
/// # fn doc() {
/// let fut = future::lazy(|| {
///     // ...
/// #    Ok(())
/// });
/// spawn!(name: "my_future", fut);
/// # }
///  # fn main() {}
/// ```
///
/// Overriding the target:
///
///```rust
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # extern crate futures;
/// # use futures::future;
/// # fn doc() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(target: "spawned_futures", fut);
/// # }
///  # fn main() {}
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
/// # fn doc() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(level: Level::INFO, fut);
/// # }
///  # fn main() {}
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
/// # fn doc() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(level: Level::WARN, target: "spawned_futures", name: "a_bad_future", fut);
/// # }
/// # fn main() {}
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
/// # fn doc() {
/// # let fut = future::lazy(|| { Ok(()) });
/// spawn!(fut, foo = "bar", baz = 42);
/// # }
///  # fn main() {}
/// ```
///
///```rust
/// # extern crate futures;
/// # #[macro_use]
/// # extern crate tracing_futures;
/// # extern crate tracing;
/// # use futures::future;
/// # fn docs() {
/// for i in 0..10 {
///     let fut = future::lazy(|| {
///         // ...
///         # Ok(())
///     });
///     spawn!(fut, number = 1);
/// }
/// # }
/// # fn main() {}
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
/// # Providing an Executor
///
/// By default, `spawn!` uses the [`DefaultExecutor`]. However, an alternative
/// executor can be provided using the `on:` macro field. For example,
///
/// ```rust
/// # extern crate futures;
/// # extern crate tokio
/// #[macro_use]
/// extern crate tracing_futures;
/// # use futures::future;
/// # fn doc() {
/// let fut = future::lazy(|| {
///     // ...
/// #    Ok(())
/// });
/// # fn get_custom_executor() -> tokio::executor::DefaultExecutor {
/// #    tokio::executor::DefaultExecutor::current()
/// #}
/// let my_executor = get_custom_executor();
/// spawn!(name: "my_future", on: my_executor, fut);
/// # }
///  # fn main() {}
/// ```
///
/// The executor that spawned the future is recorded in the
/// `tokio.task.executor` field on the generated span.
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
macro_rules! spawn {
    (level: $lvl:expr, target: $tgt:expr, name: $name:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {{
        use $crate::{Instrument, macros::__Executor};
        let span = $crate::macros::__tracing_futures_span!(
            $lvl,
            target: $tgt,
            $name,
            tokio.task.is_spawned = true,
            tokio.task.executor = ?$ex,
            $($field)*
        );
        $ex.spawn(Box::new($fut.instrument(span))).unwrap();
    }};
    (level: $lvl:expr, name: $name:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: module_path!(),
            name: $name,
            on:  $ex,
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, target: $tgt:expr, name: $name:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(level: $lvl, target: $tgt, name: $name, on:  $ex, $fut,)
    };
    (level: $lvl:expr, target: $tgt:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: $tgt,
            name: stringify!($fut),
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, name: $name:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(level: $lvl, name: $name, on:  $ex, $fut,)
    };
    (target: $tgt:expr, name: $name:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $crate::macros::Level::TRACE,
            target: $tgt,
            name: $name,
            on:  $ex,
            $fut,
            $($field)*
        )
    };
    (target: $tgt:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $crate::macros::Level::TRACE,
            target: $tgt,
            on:  $ex,
            $fut,
            $($field)*
        )
    };
    (name: $name:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            target: module_path!(),
            name: $name,
            on:  $ex,
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, target: $tgt:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(level: $lvl, target: $tgt, on:  $ex, $fut,)
    };
    (target: $tgt:expr, name: $name:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(
            target: $tgt,
            name: $name,
            on:  $ex,
            $fut,
        )
    };
    (level: $lvl:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(
            level: $lvl,
            name: stringify!($fut),
            on:  $ex,
            $fut,
        )
    };
    (target: $tgt:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(target: $tgt, on:  $ex, $fut,)
    };
    (name: $name:expr, on:  $ex:expr, $fut:expr) => {
        spawn!(name: $name, on:  $ex, $fut,)
    };
    (level: $lvl:expr, on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: module_path!(),
            name: stringify!($fut),
            on:  $ex,
            $fut,
            $($field)*
        )
    };
    (on:  $ex:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            name: stringify!($fut),
            on:  $ex,
            $fut,
            $($field)*,
        )
    };
    (on:  $ex:expr, $fut:expr) => {
        spawn!(on:  $ex, $fut,)
    };

    // === default executor ===

    (level: $lvl:expr, target: $tgt:expr, name: $name:expr, $fut:expr, $($field:tt)*) => {{
        spawn!(
            level: $lvl,
            target: $tgt,
            name: $name,
            on:  $crate::macros::DefaultExecutor::current(),
            $fut,
            $($field)*
        )
    }};
    (level: $lvl:expr, name: $name:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: module_path!(),
            name: $name,
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, target: $tgt:expr, name: $name:expr, $fut:expr) => {
        spawn!(level: $lvl, target: $tgt, name: $name, $fut,)
    };
    (level: $lvl:expr, target: $tgt:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: $tgt,
            name: stringify!($fut),
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, name: $name:expr, $fut:expr) => {
        spawn!(level: $lvl, name: $name, $fut,)
    };
    (target: $tgt:expr, name: $name:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $crate::macros::Level::TRACE,
            target: $tgt,
            name: $name,
            $fut,
            $($field)*
        )
    };
    (target: $tgt:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $crate::macros::Level::TRACE,
            target: $tgt,
            $fut,
            $($field)*
        )
    };
    (name: $name:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            target: module_path!(),
            name: $name,
            $fut,
            $($field)*
        )
    };
    (level: $lvl:expr, target: $tgt:expr, $fut:expr) => {
        spawn!(level: $lvl, target: $tgt, $fut,)
    };
    (target: $tgt:expr, name: $name:expr, $fut:expr) => {
        spawn!(target: $tgt, name: $name, $fut,)
    };
    (level: $lvl:expr, $fut:expr) => {
        spawn!(level: $lvl, name: stringify!($fut), $fut,)
    };
    (target: $tgt:expr, $fut:expr) => {
        spawn!(target: $tgt, $fut,)
    };
    (name: $name:expr, $fut:expr) => {
        spawn!(name: $name, $fut,)
    };
    (level: $lvl:expr, $fut:expr, $($field:tt)*) => {
        spawn!(
            level: $lvl,
            target: module_path!(),
            name: stringify!($fut),
            $fut,
            $($field)*
        )
    };
    ($fut:expr, $($field:tt)*) => {
        spawn!(name: stringify!($fut), $fut, $($field)*)
    };

    ($fut:expr) => {
        spawn!($fut,)
    };

}

#[doc(hidden)]
#[macro_export]
macro_rules! module_path {
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
