use crate::layer::WithContext;
use std::fmt;
use tracing::{Metadata, Span};

/// A captured trace of [`tracing`] spans.
///
/// This type can be thought of as a relative of
/// [`std::backtrace::Backtrace`][`Backtrace`].
/// However, rather than capturing the current call stack when it is
/// constructed, a `SpanTrace` instead captures the current [span] and its
/// [parents].
///
/// In many cases, span traces may be as useful as stack backtraces useful in
/// pinpointing where an error occurred and why, if not moreso:
///
/// * A span trace captures only the user-defined, human-readable `tracing`
///   spans, rather than _every_ frame in the call stack, often cutting out a
///   lot of noise.
/// * Span traces include the [fields] recorded by each span in the trace, as
///   well as their names and source code location, so different invocations of
///   a function can be distinguished,
/// * In asynchronous code, backtraces for errors that occur in [futures] often
///   consist not of the stack frames that _spawned_ a future, but the stack
///   frames of the executor that is responsible for running that future. This
///   means that if an `async fn` calls another `async fn` which generates an
///   error, the calling async function will not appear in the stack trace (and
///   often, the callee won't either!). On the other hand, when the
///   [`tracing-futures`] crate is used to instrument async code, the span trace
///   will represent the logical application context a future was running in,
///   rather than the stack trace of the executor that was polling a future when
///   an error occurred.
///
/// Finally, unlike stack [`Backtrace`]s, capturing a `SpanTrace` is fairly
/// lightweight, and the resulting struct is not large. The `SpanTrace` struct
/// is formatted lazily; instead, it simply stores a copy of the current span,
/// and allows visiting the spans in that span's trace tree by calling the
/// [`with_spans` method][`with_spans`].
///
/// # Formatting
///
/// The `SpanTrace` type implements `fmt::Display`, formatting the span trace
/// similarly to how Rust formats panics. For example:
///
/// ```text
/// span backtrace:
///    0: custom_error::do_another_thing
///         with answer=42 will_succeed=false
///           at examples/examples/custom_error.rs:42
///    1: custom_error::do_something
///         with foo="hello world"
///           at examples/examples/custom_error.rs:37
/// ```
///
/// Additionally, if custom formatting is desired, the [`with_spans`] method can
/// be used to visit each span in the trace, formatting them in order.
///
/// [`tracing`]: https://docs.rs/tracing
/// [`Backtrace`]: https://doc.rust-lang.org/std/backtrace/struct.Backtrace.html
/// [span]: https://docs.rs/tracing/latest/tracing/span/index.html
/// [parents]: https://docs.rs/tracing/latest/tracing/span/index.html#span-relationships
/// [fields]: https://docs.rs/tracing/latest/tracing/field/index.html
/// [futures]: https://doc.rust-lang.org/std/future/trait.Future.html
/// [`tracing-futures`]: https://docs.rs/tracing-futures/
/// [`with_spans`]: #method.with_spans
#[derive(Clone)]
pub struct SpanTrace {
    span: Span,
}

// === impl SpanTrace ===

impl SpanTrace {
    /// Capture the current span trace.
    ///
    /// # Examples
    /// ```rust
    /// use tracing_error::SpanTrace;
    ///
    /// pub struct MyError {
    ///     span_trace: SpanTrace,
    ///     // ...
    /// }
    ///
    /// # fn some_error_condition() -> bool { true }
    ///
    /// #[tracing::instrument]
    /// pub fn my_function(arg: &str) -> Result<(), MyError> {
    ///     if some_error_condition() {
    ///         return Err(MyError {
    ///             span_trace: SpanTrace::capture(),
    ///             // ...
    ///         });
    ///     }
    ///
    ///     // ...
    /// #   Ok(())
    /// }
    /// ```
    pub fn capture() -> Self {
        SpanTrace {
            span: Span::current(),
        }
    }

    /// Apply a function to all captured spans in the trace until it returns
    /// `false`.
    ///
    /// This will call the provided function with a reference to the
    /// [`Metadata`] and a formatted representation of the [fields] of each span
    /// captured in the trace, starting with the span that was current when the
    /// trace was captured. The function may return `true` or `false` to
    /// indicate whether to continue iterating over spans; if it returns
    /// `false`, no additional spans will be visited.
    ///
    /// [fields]: https://docs.rs/tracing/latest/tracing/field/index.html
    /// [`Metadata`]: https://docs.rs/tracing/latest/tracing/struct.Metadata.html
    pub fn with_spans(&self, f: impl FnMut(&'static Metadata<'static>, &str) -> bool) {
        self.span.with_subscriber(|(id, s)| {
            if let Some(getcx) = s.downcast_ref::<WithContext>() {
                getcx.with_context(s, id, f);
            }
        });
    }
}

macro_rules! try_bool {
    ($e:expr, $dest:ident) => {{
        let ret = $e.unwrap_or_else(|e| $dest = Err(e));

        if $dest.is_err() {
            return false;
        }

        ret
    }};
}

impl fmt::Display for SpanTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut err = Ok(());
        let mut span = 0;

        self.with_spans(|metadata, fields| {
            if span > 0 {
                try_bool!(write!(f, "\n",), err);
            }

            try_bool!(
                write!(f, "{:>4}: {}::{}", span, metadata.target(), metadata.name()),
                err
            );

            if !fields.is_empty() {
                try_bool!(write!(f, "\n           with {}", fields), err);
            }

            if let Some((file, line)) = metadata
                .file()
                .and_then(|file| metadata.line().map(|line| (file, line)))
            {
                try_bool!(write!(f, "\n             at {}:{}", file, line), err);
            }

            span += 1;
            true
        });

        err
    }
}

impl fmt::Debug for SpanTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugSpan<'a> {
            metadata: &'a Metadata<'a>,
            fields: &'a str,
        }

        impl<'a> fmt::Debug for DebugSpan<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "{{ target: {:?}, name: {:?}",
                    self.metadata.target(),
                    self.metadata.name()
                )?;

                if !self.fields.is_empty() {
                    write!(f, ", fields: {:?}", self.fields)?;
                }

                if let Some((file, line)) = self
                    .metadata
                    .file()
                    .and_then(|file| self.metadata.line().map(|line| (file, line)))
                {
                    write!(f, ", file: {:?}, line: {:?}", file, line)?;
                }

                write!(f, " }}")?;

                Ok(())
            }
        }

        write!(f, "SpanTrace ")?;
        let mut dbg = f.debug_list();
        self.with_spans(|metadata, fields| {
            dbg.entry(&DebugSpan { metadata, fields });
            true
        });
        dbg.finish()
    }
}
