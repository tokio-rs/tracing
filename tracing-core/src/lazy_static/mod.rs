

// Copyright 2016 lazy-static.rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

/*!
A macro for declaring lazily evaluated statics.
Using this macro, it is possible to have `static`s that require code to be
executed at runtime in order to be initialized.
This includes anything requiring heap allocations, like vectors or hash maps,
as well as anything that requires function calls to be computed.
*/

#[cfg(feature = "std")]
#[path="inline_lazy.rs"]
#[doc(hidden)]
pub mod lazy;

#[cfg(not(feature = "std"))]
#[path="core_lazy.rs"]
#[doc(hidden)]
pub mod lazy;

#[doc(hidden)]
pub use core::ops::Deref as __Deref;

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! __lazy_static_internal {
    // optional visibility restrictions are wrapped in `()` to allow for
    // explicitly passing otherwise implicit information about private items
    ($(#[$attr:meta])* ($($vis:tt)*) static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!(@MAKE TY, $(#[$attr])*, ($($vis)*), $N);
        __lazy_static_internal!(@TAIL, $N : $T = $e);
        lazy_static!($($t)*);
    };
    (@TAIL, $N:ident : $T:ty = $e:expr) => {
        impl $crate::lazy_static::__Deref for $N {
            type Target = $T;
            fn deref(&self) -> &$T {
                #[inline(always)]
                fn __static_ref_initialize() -> $T { $e }

                #[inline(always)]
                fn __stability() -> &'static $T {
                    __lazy_static_create!(LAZY, $T);
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl $crate::lazy_static::LazyStatic for $N {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
    };
    // `vis` is wrapped in `()` to prevent parsing ambiguity
    (@MAKE TY, $(#[$attr:meta])*, ($($vis:tt)*), $N:ident) => {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        $(#[$attr])*
        $($vis)* struct $N {__private_field: ()}
        #[doc(hidden)]
        $($vis)* static $N: $N = $N {__private_field: ()};
    };
    () => ()
}

#[macro_export(local_inner_macros)]
/// lazy_static (suppress docs_missing warning)
macro_rules! lazy_static {
    ($(#[$attr:meta])* static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        // use `()` to explicitly forward the information about private items
        __lazy_static_internal!($(#[$attr])* () static ref $N : $T = $e; $($t)*);
    };
    ($(#[$attr:meta])* pub static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!($(#[$attr])* (pub) static ref $N : $T = $e; $($t)*);
    };
    ($(#[$attr:meta])* pub ($($vis:tt)+) static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        __lazy_static_internal!($(#[$attr])* (pub ($($vis)+)) static ref $N : $T = $e; $($t)*);
    };
    () => ()
}

/// Support trait for enabling a few common operation on lazy static values.
///
/// This is implemented by each defined lazy static, and
/// used by the free functions in this crate.
pub trait LazyStatic {
    #[doc(hidden)]
    fn initialize(lazy: &Self);
}

/// Takes a shared reference to a lazy static and initializes
/// it if it has not been already.
///
/// This can be used to control the initialization point of a lazy static.
pub fn initialize<T: LazyStatic>(lazy: &T) {
    LazyStatic::initialize(lazy);
}
