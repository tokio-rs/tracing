extern crate tokio_trace_core;

#[doc(hidden)]
#[macro_export]
macro_rules! meta {
    (span: $name:expr, $( $field_name:ident ),*) => ({
        $crate::Meta {
            name: Some($name),
            target: module_path!(),
            level: $crate::Level::Trace,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::Kind::Span,
        }
    });
    (event: $lvl:expr, $( $field_name:ident ),*) =>
        (meta!(event: $lvl, target: module_path!(), $( $field_name ),* ));
    (event: $lvl:expr, target: $target:expr, $( $field_name:ident ),*) => ({
        $crate::Meta {
            name: None,
            target: $target,
            level: $lvl,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::Kind::Event,
        }
    });
}

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
            use $crate::{callsite, Span, Dispatch, Meta};
            static META: Meta<'static> = meta! { span: $name, $( $k ),* };
            thread_local! {
                // TODO: if callsite caches become an API, can we better
                // encapsulate the thread-local-iness of them? Do we want to?
                static CALLSITE: callsite::Cache<'static> = callsite!(&META);
            }
            let dispatcher = Dispatch::current();
            // TODO: should span construction just become a method on callsites?
            let span = if CALLSITE.with(|c| c.is_enabled(&dispatcher)) {
                Span::new(dispatcher, &META)
            } else {
                Span::new_disabled()
            };
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
            use $crate::{callsite, Dispatch, Meta, SpanData, SpanId, Subscriber, Event, value::AsValue};
            static META: Meta<'static> = meta! { event:
                $lvl,
                target:
                $target, $( $k ),*
            };
            thread_local! {
                static CALLSITE: callsite::Cache<'static> = callsite!(&META);
            }
            let dispatcher = Dispatch::current();
            if CALLSITE.with(|c| c.is_enabled(&dispatcher)) {
                let field_values: &[ &dyn AsValue ] = &[ $( &$val ),* ];
                dispatcher.observe_event(&Event {
                    parent: SpanId::current(),
                    follows_from: &[],
                    meta: &META,
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
