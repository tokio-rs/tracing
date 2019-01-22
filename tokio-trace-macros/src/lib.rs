#[macro_use]
extern crate tokio_trace;

/// Similar to the `std::dbg!` macro, but generates `tokio-trace` events rather
/// than printing to stdout.
#[macro_export]
macro_rules! trace_dbg {
    (level: $level:expr, $ex:expr) => {
        {
            #[allow(unused_imports)]
            use tokio_trace::{callsite, Id, Subscriber, Event, field::{debug, Value}};
            use tokio_trace::callsite::Callsite;
            let callsite = callsite! {@
                name: concat!("event:trace_dbg(", stringify!($ex), ")"),
                target: module_path!(),
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
    ($ex:expr) => {
        trace_dbg!(level: tokio_trace::Level::DEBUG, $ex)
    };
}
