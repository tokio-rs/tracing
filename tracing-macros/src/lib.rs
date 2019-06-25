extern crate tracing;

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
        dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        dbg!(level: tracing::Level::DEBUG, $ex)
    };
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {{
        use tracing::callsite::Callsite;
        use tracing::{
            callsite,
            field::{debug, Value},
            Event, Id, Subscriber,
        };
        let callsite = callsite! {
            name: stringify!($ex),
            kind: tracing::metadata::Kind::EVENT,
            target: $target,
            level: $level,
            fields: value,
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
            Event::dispatch(
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
        dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        dbg!(level: tracing::Level::DEBUG, $ex)
    };
}
