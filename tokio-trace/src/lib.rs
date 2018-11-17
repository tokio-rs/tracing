extern crate tokio_trace_core;

/// Constructs a new static callsite for a span or event.
#[macro_export]
macro_rules! callsite {
    (span: $name:expr, $( $field_name:ident ),*) => ({
        callsite!(@ $crate::Meta {
            name: Some($name),
            target: module_path!(),
            level: $crate::Level::Trace,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::MetaKind::SPAN,
        })
    });
    (event: $lvl:expr, $( $field_name:ident ),*) =>
        (callsite!(event: $lvl, target: module_path!(), $( $field_name ),* ));
    (event: $lvl:expr, target: $target:expr, $( $field_name:ident ),*) => ({
        callsite!(@ $crate::Meta {
            name: None,
            target: $target,
            level: $lvl,
            module_path: Some(module_path!()),
            file: Some(file!()),
            line: Some(line!()),
            field_names: &[ "message", $(stringify!($field_name)),* ],
            kind: $crate::MetaKind::EVENT,
        })
    });
    (@ $meta:expr ) => ({
        use std::sync::{Once, atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering}};
        use $crate::{callsite, Meta, subscriber::{Interest}};
        static META: Meta<'static> = $meta;
        static INTEREST: AtomicUsize = ATOMIC_USIZE_INIT;
        static REGISTRATION: Once = Once::new();
        struct MyCallsite;
        impl callsite::Callsite for MyCallsite {
            #[inline]
            fn interest(&self) -> Interest {
                Interest::from_usize(INTEREST.load(Ordering::Relaxed))
                    .unwrap_or(Interest::SOMETIMES)
            }
            fn add_interest(&self, interest: Interest) {
                let current_interest = INTEREST.load(Ordering::Relaxed);
                match Interest::from_usize(current_interest) {
                    Some(current) if interest > current =>
                        INTEREST.store(interest.as_usize(), Ordering::Relaxed),
                    None =>
                        INTEREST.store(interest.as_usize(), Ordering::Relaxed),
                    _ => {}
                }
            }
            fn remove_interest(&self) {
                INTEREST.store(0, Ordering::Relaxed);
            }
            fn metadata(&self) -> &Meta {
                &META
            }
        }
        REGISTRATION.call_once(|| {
            callsite::register(&MyCallsite);
        });
        &MyCallsite
    })
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
/// let mut span = span!("my span");
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
/// span!("my span", foo = 2u64, bar = "a string").enter(|| {
///     // do work inside the span...
/// });
/// # }
/// ```
#[macro_export]
macro_rules! span {
    ($name:expr) => { span!($name,) };
    ($name:expr, $($k:ident $( = $val:expr )* ) ,*) => {
        {
            use $crate::{callsite, callsite::Callsite, Span,  span::SpanExt, field::{Value, AsKey}};
            let callsite = callsite! { span: $name, $( $k ),* };
            // Depending on how many fields are generated, this may or may
            // not actually be used, but it doesn't make sense to repeat it.
            #[allow(unused_variables, unused_mut)]
            Span::new(callsite, |span| {
                let mut keys = callsite.metadata().fields();
                $(
                    let key = keys.next()
                        .expect(concat!("metadata should define a key for '", stringify!($k), "'"));
                    span!(@ record: span, $k, &key, $($val)*);
                )*
            })
        }
    };
    (@ record: $span:expr, $k:expr, $i:expr, $val:expr) => (
        $span.record($i, &$val)
            .expect(concat!("adding value for field '", stringify!($k), "' failed"));
    );
    (@ record: $span:expr, $k:expr, $i:expr,) => (
        // skip
    );
}

#[macro_export]
macro_rules! event {
    (target: $target:expr, $lvl:expr, { $( $k:ident $( = $val:expr )* ),* }, $($arg:tt)+ ) => ({
        {
            use $crate::{callsite, Id, Subscriber, Event, span::SpanExt, field::{Value, AsKey}};
            use $crate::callsite::Callsite;
            let callsite = callsite! { event:
                $lvl,
                target:
                $target, $( $k ),*
            };
            // Depending on how many fields are generated, this may or may
            // not actually be used, but it doesn't make sense to repeat it.
            #[allow(unused_variables, unused_mut)]
            Event::new(callsite, |event| {
                let mut keys = callsite.metadata().fields();
                event.message(
                    &keys.next().expect("event metadata should define a key for the message"),
                    format_args!( $($arg)+ )
                )
                .expect("adding value for event message failed");
                $(
                    let key = keys.next()
                        .expect(concat!("metadata should define a key for '", stringify!($k), "'"));
                    event!(@ record: event, $k, &key, $($val)*);
                )*
            })
        }
    });
    ( $lvl:expr, { $( $k:ident $( = $val:expr )* ),* }, $($arg:tt)+ ) => (
        event!(target: module_path!(), $lvl, { $($k $( = $val)* ),* }, $($arg)+)
    );
    (@ record: $ev:expr, $k:expr, $i:expr, $val:expr) => (
        $ev.record($i, &$val)
            .expect(concat!("adding value for field '", stringify!($k), "' failed"));
    );
    (@ record: $ev:expr, $k:expr, $i:expr,) => (
        // skip
    );
}

mod dispatcher;
pub mod field;
pub mod span;
pub mod subscriber;

pub use self::{
    dispatcher::Dispatch,
    field::Value,
    span::{Attributes, Event, Id, Span, SpanAttributes},
    subscriber::Subscriber,
    tokio_trace_core::{
        callsite::{self, Callsite},
        Level, Meta, MetaKind,
    },
};

mod sealed {
    pub trait Sealed {}
}
