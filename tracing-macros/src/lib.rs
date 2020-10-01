#![cfg_attr(docsrs, deny(broken_intra_doc_links))]
#[doc(hidden)]
pub use tracing;

/// Alias of `dbg!` for avoiding conflicts with the `std::dbg!` macro.
#[macro_export]
macro_rules! trace_dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {
        $crate::dbg!(target: $target, level: $level, $ex)
    };
    (level: $level:expr, $ex:expr) => {
        $crate::dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        $crate::dbg!(target: $target, level: $crate::tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        $crate::dbg!(level: $crate::tracing::Level::DEBUG, $ex)
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
        match $ex {
            value => {
                $crate::tracing::event!(target: $target, $level, ?value, stringify!($ex));
                value
            }
        }
    }};
    (level: $level:expr, $ex:expr) => {
        $crate::dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        $crate::dbg!(target: $target, level: $crate::tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        $crate::dbg!(level: $crate::tracing::Level::DEBUG, $ex)
    };
}
