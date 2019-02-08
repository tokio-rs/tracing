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
    (target: $target:expr, level: $level:expr, $ex:expr) => {{
        use tokio_trace::callsite::Callsite;
        use tokio_trace::{
            callsite,
            field::{debug, Value},
            Event, Id, Subscriber,
        };
        let callsite = callsite! {
            name: concat!("event:trace_dbg(", stringify!($ex), ")"),
            target: $target,
            level: $level,
            fields: $ex
        };
        let val = $ex;
        if is_enabled!(callsite) {
            let meta = callsite.metadata();
            let fields = meta.fields();
            let key = meta
                .fields()
                .into_iter()
                .next()
                .expect("trace_dbg event must have one field");
            Event::observe(
                meta,
                &fields.value_set(&[(&key, Some(&debug(&val) as &Value))]),
            );
        }
        val
    }};
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
