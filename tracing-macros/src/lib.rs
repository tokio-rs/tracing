#![cfg_attr(assert_matches, feature(assert_matches))]

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
/// This macro works like [`assert!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `assert` because `core::macros` is imported as default and redirecting from `assert` to `$crate::assert` does not work.
///
/// Based on [`assert!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_assert;
/// // the panic message for these assertions is the stringified value of the
/// // expression given.
/// assert!(true);
///
/// fn some_computation() -> bool { true } // a very simple function
///
/// trace_assert!(some_computation());
///
/// // assert with a custom message
/// let x = true;
/// trace_assert!(x, "x wasn't true!");
///
/// let a = 3; let b = 27;
/// trace_assert!(a + b == 30, a, b);
/// ```
#[macro_export]
macro_rules! trace_assert {
    ($cond:expr $(,)?) => {{
        if !$cond {
            $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed");
        }
        std::assert!($cond)
    }};
    ($cond:expr, $($arg:tt)+) => {{
        if !$cond {
            $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed", $($arg)+);
        }
        std::assert!($cond)
    }};
}

/// This macro works like [`assert_eq!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `assert_eq` because `core::macros` is imported as default and redirecting from `assert_eq` to `$crate::assert_eq` does not work.
///
/// Based on [`assert_eq!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_assert_eq;
/// let a = 3;
/// let b = 1 + 2;
/// trace_assert_eq!(a, b);
///
/// trace_assert_eq!(a, b, "we are testing addition with {} and {}", a, b);
/// ```
#[macro_export]
macro_rules! trace_assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed", left = ?left_val, right = ?right_val);
                }
                std::assert_eq!($left, $right);
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed", left = ?left_val, right = ?right_val, $($arg)+);
                }
                std::assert_eq!($left, $right);
            }
        }
    };
}

/// This macro works like [`assert_ne!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `assert_ne` because `core::macros` is imported as default and redirecting from `assert_ne` to `$crate::assert_ne` does not work.
///
/// Based on [`assert_ne!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_assert_ne;
/// let a = 3;
/// let b = 2;
/// trace_assert_ne!(a, b);
///
/// trace_assert_ne!(a, b, "we are testing that the values are not equal");
/// ```
#[macro_export]
macro_rules! trace_assert_ne {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed", left = ?left_val, right = ?right_val);
                }
                std::assert_ne!($left, $right);
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if *left_val == *right_val {
                    $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed", left = ?left_val, right = ?right_val, $($arg)+);
                }
                std::assert_ne!($left, $right);
            }
        }
    };
}

/// This macro works like [`std::assert_matches::assert_matches!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `std::assert_matches::assert_matches!` because `core::macros` is imported as default and redirecting from `std::assert_matches::assert_matches!` to `$crate::std::assert_matches::assert_matches!` does not work.
///
/// Based on [`std::assert_matches::assert_matches!`].
///
/// ```
/// # use tracing_macros::trace_assert_matches;
/// #![feature(assert_matches)]
///
/// use std::assert_matches::assert_matches;
///
/// let a = 1u32.checked_add(2);
/// let b = 1u32.checked_sub(2);
/// trace_assert_matches!(a, Some(_), msg = "Was none", yaks="shaved");
/// trace_assert_matches!(b, None, msg = "Was some");
///
/// let c = Ok("abc".to_string());
/// trace_assert_matches!(c, Ok(x) | Err(x) if x.len() < 100, ?a, ?b, ?c);
/// ```
#[cfg(feature = "assert_matches")]
#[macro_export]
macro_rules! trace_assert_matches {
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed");
                std::assert_matches::assert_matches!(left_val, $( $pattern )|+ $( if $guard )?);
            }
        }
    };
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )?, $($arg:tt)+) => {
        match $left {
            $( $pattern )|+ $( if $guard )? => {}
            ref left_val => {
                $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, assert="failed", $($arg)+);
                std::assert_matches::assert_matches!(left_val, $( $pattern )|+ $( if $guard )?);
            }
        }
    }
}

/// This macro works like [`std::assert_matches::debug_assert_matches!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `std::assert_matches::debug_assert_matches!` because `core::macros` is imported as default and redirecting from `std::assert_matches::debug_assert_matches!` to `$crate::std::assert_matches::debug_assert_matches!` does not work.
///
/// Based on [`std::assert_matches::debug_assert_matches!`].
///
/// ```
/// #![feature(assert_matches)]
///
/// # use tracing_macros::trace_debug_assert_matches;
///
/// let a = 1u32.checked_add(2);
/// let b = 1u32.checked_sub(2);
/// trace_debug_assert_matches!(a, Some(_), msg = "Was none", yaks="shaved");
/// trace_debug_assert_matches!(b, None, msg = "Was some");
///
/// let c = Ok("abc".to_string());
/// trace_debug_assert_matches!(c, Ok(x) | Err(x) if x.len() < 100, ?a, ?b, ?c);
/// ```
///
/// ```should_panic
/// # use tracing_macros::trace_debug_assert_matches;
/// # use tracing_macros::trace_assert_matches;
/// let x: Option<u32> = Some(4);
/// if cfg!(debug_assertions) {
///     trace_debug_assert_matches!(x, None);
/// } else {
///     trace_assert_matches!(x, None);
/// }
/// ```
#[cfg(feature = "assert_matches")]
#[macro_export]
macro_rules! trace_debug_assert_matches {
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        if std::cfg!(debug_assertions) {
            $crate::trace_assert_matches!(left_val, $( $pattern )|+ $( if $guard )?);
        }
    };
    ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )?, $($arg:tt)+) => {
        if std::cfg!(debug_assertions) {
            $crate::trace_assert_matches!(left_val, $( $pattern )|+ $( if $guard )?, $($arg)+);
        }
    }
}

/// This macro works like [`debug_assert!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `debug_assert` because `core::macros` is imported as default and redirecting from `debug_assert` to `$crate::debug_assert` does not work.
///
/// Based on [`std::debug_assert!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_debug_assert;
/// // the panic message for these assertions is the stringified value of the
/// // expression given.
/// trace_debug_assert!(true);
///
/// fn some_expensive_computation() -> bool { true } // a very simple function
/// trace_debug_assert!(some_expensive_computation());
///
/// // assert with a custom message
/// let x = true;
/// trace_debug_assert!(x, msg="x wasn't true!");
///
/// let a = 3; let b = 27;
/// trace_debug_assert!(a + b == 30, a, b);
/// ```
///
/// ```should_panic
/// # use tracing_macros::trace_debug_assert;
/// # use tracing_macros::trace_assert;
/// if cfg!(debug_assertions) {
///     trace_debug_assert!(1 == 2);
/// } else {
///     trace_assert!(1 == 2);
/// }
/// ```
#[macro_export]
macro_rules! trace_debug_assert {
    ($($arg:tt)*) => {
        if std::cfg!(debug_assertions) {
            $crate::trace_assert!($($arg)*);
        }
    };
}

/// This macro works like [`debug_assert_eq!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `debug_assert_eq  because `core::macros` is imported as default and redirecting from `debug_assert_eq` to `$crate::debug_assert_eq` does not work.
///
/// Based on [`std::debug_assert_eq!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_debug_assert_eq;
/// let a = 3;
/// let b = 1 + 2;
/// trace_debug_assert_eq!(a, b);
/// ```
///
/// ```should_panic
/// # use tracing_macros::trace_debug_assert_eq;
/// # use tracing_macros::trace_assert_eq;
/// if cfg!(debug_assertions) {
///     trace_debug_assert_eq!(1, 2);
/// } else {
///     trace_assert_eq!(1, 2);
/// }
/// ```
#[macro_export]
macro_rules! trace_debug_assert_eq {
    ($($arg:tt)*) => {
        if std::cfg!(debug_assertions) {
            $crate::trace_assert_eq!($($arg)*);
        }
    };
}

/// This macro works like [`debug_assert_ne!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `debug_assert_ne` because `core::macros` is imported as default and redirecting from `debug_assert_ne` to `$crate::debug_assert_ne` does not work.
///
/// Based on [`std::debug_assert_ne!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_debug_assert_ne;
/// let a = 3;
/// let b = 2;
/// trace_debug_assert_ne!(a, b);
/// ```
///
/// ```should_panic
/// # use tracing_macros::trace_debug_assert_ne;
/// # use tracing_macros::trace_assert_ne;
/// if cfg!(debug_assertions) {
///     trace_debug_assert_ne!(1, 1);
/// } else {
///     trace_assert_ne!(1, 1);
/// }
/// ```
#[macro_export]
macro_rules! trace_debug_assert_ne {
    ($($arg:tt)*) => {
        if std::cfg!(debug_assertions) {
            $crate::trace_assert_ne!($($arg)*);
        }
    };
}

/// This macro works like [`unreachable!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `unreachable` because `core::macros` is imported as default and redirecting from `unreachable` to `$crate::unreachable` does not work.
///
/// Based on [`std::unreachable!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_unreachable;
/// # use tracing_attributes::instrument;
/// # #[allow(dead_code)]
/// #[instrument]
/// fn divide_by_three(x: u32) -> u32 { // one of the poorest implementations of x/3
///     for i in 0.. {
///         if 3*i < i { panic!("u32 overflow"); }
///         if x < 3*i { return i-1; }
///     }
///     trace_unreachable!("The loop should always return");
/// }
/// ```
///
/// ```should_panic
/// # use tracing_macros::trace_unreachable;
/// trace_unreachable!(msg="This always panics");
/// ```
#[macro_export]
macro_rules! trace_unreachable {
    ($($arg:tt)*) => {{
        $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, unreachable=true, $($arg)*);
        unreachable!()
    }};
}
/// This macro works like [`todo!`], but generates a [`tracing`] [`tracing::event!`] before [`std::panic!`]ing.
/// This is useful when you want to log the context of the panic without having to put all the context
/// into the panic message.
/// We did not call it `todo` because `core::macros` is imported as default and redirecting from `todo` to `$crate::todo` does not work.
///
/// Based on [`std::todo!`].
///
/// # Examples
///
/// ```
/// # use tracing_macros::trace_todo;
/// # use tracing_attributes::instrument;
/// # #[allow(dead_code)]
/// #[instrument]
/// fn divide_by_three(x: u32) -> u32 {
///     trace_todo!()
/// }
/// ```
///
/// ```should_panic
/// # use tracing_macros::trace_todo;
/// # use tracing_attributes::instrument;
/// # #[allow(dead_code)]
/// #[instrument]
/// fn divide_by_three(x: u32) -> u32 {
///     let b = 4;
///     trace_todo!(?b)
/// }
/// divide_by_three(4);
/// ```
#[macro_export]
macro_rules! trace_todo {
    () => {{
        $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, todo=true);
        todo!()
    }};
    ($($arg:tt)+) => {{
        $crate::tracing::event!(target: module_path!(), $crate::tracing::Level::ERROR, todo=true, $($arg)*);
        todo!()
    }};
}
