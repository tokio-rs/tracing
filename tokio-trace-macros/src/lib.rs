#[macro_use]
extern crate tokio_trace;

/// Alias of `dbg!` for avoiding conflicts with the `std::dbg!` macro.
#[macro_export(local_inner_macros)]
macro_rules! trace_dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {
        dbg!(target: $target, level: $level, $ex)
    };
    (level: $level:expr, $ex:expr) => {
        dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        dbg!(target: $target, level: tokio_trace::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        dbg!(level: tokio_trace::Level::DEBUG, $ex)
    };

}

/// Similar to the `std::dbg!` macro, but generates `tokio-trace` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {
        {
            #[allow(unused_imports)]
            use tokio_trace::{callsite, Id, Subscriber, Event, field::{debug, Value}};
            use tokio_trace::callsite::Callsite;
            let callsite = callsite! {@
                name: concat!("event:trace_dbg(", stringify!($ex), ")"),
                target: $target,
                level: $level,
                fields: &[stringify!($ex)]
            };
            let interest = callsite.interest();
            let val = $ex;
            if interest.is_never() {
                val
            } else {
                let meta = callsite.metadata();
                let mut event = Event::new(interest, meta);
                if !event.is_disabled() {
                    let key = meta.fields().into_iter().next()
                        .expect("trace_dbg event must have one field");
                    event.record(&key, &debug(val));
                }
                drop(event);
                val
            }
        }
    };
    (level: $level:expr, $ex:expr) => {
        dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        dbg!(target: $target, level: tokio_trace::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        dbg!(level: tokio_trace::Level::DEBUG, $ex)
    };
}
