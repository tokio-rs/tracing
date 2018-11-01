extern crate tokio_trace_core;

/// Constructs a new span.
///
/// # Examples
///
/// Creating a new span with no fields:
/// ```
/// # #[macro_use]
/// # extern crate tokio_trace;
/// # fn main() {
/// let span = span!("my span");
/// span.enter(|| {
///     // do work inside the span...
/// });
/// # }
/// ```
///
/// Creating a span with fields:
/// ```
/// # #[macro_use]
/// # extern crate tokio_trace;
/// # fn main() {
/// span!("my span", foo = &2, bar = &"a string").enter(|| {
///     // do work inside the span...
/// });
/// # }
/// ```
#[macro_export]
macro_rules! span {
    ($name:expr) => { span!($name,) };
    ($name:expr, $($k:ident $( = $val:expr )* ) ,*) => {
        {
            use $crate::{callsite, Dispatch};
            let callsite = callsite! { span: $name, $( $k ),* };
            let span = callsite.new_span(Dispatch::current());
            $(
                span.add_value(stringify!($k), $( $val )* )
                    .expect(concat!("adding value for field ", stringify!($k), " failed"));
            )*
            span
        }
    }
}

#[macro_export]
macro_rules! event {
    (target: $target:expr, $lvl:expr, { $($k:ident = $val:expr),* }, $($arg:tt)+ ) => ({
        {
            use $crate::{callsite, Dispatch, SpanData, SpanId, Subscriber, Event, value::AsValue};
            let callsite = callsite! { event:
                $lvl,
                target:
                $target, $( $k ),*
            };
            let dispatcher = Dispatch::current();
            if callsite.is_enabled(&dispatcher) {
                let field_values: &[ &dyn AsValue ] = &[ $( &$val ),* ];
                dispatcher.observe_event(&Event {
                    parent: SpanId::current(),
                    follows_from: &[],
                    meta: callsite.metadata(),
                    field_values: &field_values[..],
                    message: format_args!( $($arg)+ ),
                });
            }
        }
    });
    ($lvl:expr, { $($k:ident = $val:expr),* }, $($arg:tt)+ ) => (
        event!(target: module_path!(), $lvl, { $($k = $val),* }, $($arg)+)
    )
}

mod dispatcher;
pub mod span;
pub mod subscriber;

pub use self::{
    dispatcher::Dispatch,
    span::{Data as SpanData, Id as SpanId, Span},
    subscriber::Subscriber,
    tokio_trace_core::{
        callsite,
        value::{self, AsValue, IntoValue, Value},
        Event, Level, Meta,
    },
};

#[doc(hidden)]
pub use tokio_trace_core::Kind;
