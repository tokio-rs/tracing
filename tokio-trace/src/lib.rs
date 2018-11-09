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
            field_names: &[ $(stringify!($field_name)),* ],
            kind: $crate::MetaKind::EVENT,
        })
    });
    (@ $meta:expr ) => ({
        use std::sync::{Once, atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering}};
        use $crate::{callsite, Meta, subscriber::{Subscriber, Interest}};
        static META: Meta<'static> = $meta;
        static INTEREST: AtomicUsize = ATOMIC_USIZE_INIT;
        static REGISTRATION: Once = Once::new();
        struct MyCallsite;
        impl callsite::Callsite for MyCallsite {
            fn is_enabled(&self, dispatch: &Dispatch) -> bool {
                let current_interest = INTEREST.load(Ordering::Relaxed);
                match Interest::from_usize(current_interest) {
                    Some(Interest::ALWAYS) => true,
                    Some(Interest::NEVER) => false,
                    _ => dispatch.enabled(&META),
                }
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
            use $crate::{callsite, Dispatch, Span};
            use $crate::callsite::Callsite;
            let callsite = callsite! { span: $name, $( $k ),* };
            // Depending on how many fields are generated, this may or may
            // not actually be used, but it doesn't make sense to repeat it.
            #[allow(unused_variables, unused_mut)]
            Span::new(callsite, |span| {
                let mut keys = callsite.metadata().fields();
                $(
                    let key = keys.next()
                        .expect(concat!("metadata should define a key for '", stringify!($k), "'"));
                    span!(@ add_value: span, $k, &key, $($val)*);
                )*
            })
        }
    };
    (@ add_value: $span:expr, $k:expr, $i:expr, $val:expr) => (
        $span.add_value($i, $val)
            .expect(concat!("adding value for field '", stringify!($k), "' failed"));
    );
    (@ add_value: $span:expr, $k:expr, $i:expr,) => (
        // skip
    );
}

#[macro_export]
macro_rules! event {
    (target: $target:expr, $lvl:expr, { $($k:ident = $val:expr),* }, $($arg:tt)+ ) => ({
        {
            use $crate::{callsite, Dispatch, SpanData, SpanId, Subscriber, Event, field::AsValue};
            use $crate::callsite::Callsite;
            let callsite = callsite! { event:
                $lvl,
                target:
                $target, $( $k ),*
            };
            let field_values: &[ &dyn AsValue ] = &[ $( &$val ),* ];
            Event::observe(
                callsite,
                &field_values[..],
                &[],
                format_args!( $($arg)+ ),
            );
        }
    });
    ($lvl:expr, { $($k:ident = $val:expr),* }, $($arg:tt)+ ) => (
        event!(target: module_path!(), $lvl, { $($k = $val),* }, $($arg)+)
    )
}

mod dispatcher;
pub mod field;
pub mod span;
pub mod subscriber;

pub use self::{
    callsite::Callsite,
    dispatcher::Dispatch,
    field::{AsValue, IntoValue, Value},
    span::{Data as SpanData, Id as SpanId, Span},
    subscriber::Subscriber,
    tokio_trace_core::{
        callsite::{self, Callsite},
        Event, Level, Meta, MetaKind,
    },
};

mod sealed {
    pub trait Sealed {}
}
