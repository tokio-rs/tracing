//! Abstracts over sync primitive implementations.
//!
//! Optionally, we allow the Rust standard library's `RwLock` to be replaced
//! with the `parking_lot` crate's implementation. This may provide improved
//! performance in some cases. However, the `parking_lot` dependency is an
//! opt-in feature flag. Because `parking_lot::RwLock` has a slightly different
//! API than `std::sync::RwLock` (it does not support poisoning on panics), we
//! wrap it with a type that provides the same method signatures. This allows us
//! to transparently swap `parking_lot` in without changing code at the callsite.
#[allow(unused_imports)] // may be used later;
pub(crate) use std::sync::{LockResult, PoisonError, TryLockResult};

#[cfg(not(feature = "parking_lot"))]
pub(crate) use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(feature = "parking_lot")]
pub(crate) use self::parking_lot_impl::*;

#[cfg(feature = "parking_lot")]
mod parking_lot_impl {
    pub(crate) use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
    use std::sync::{LockResult, TryLockError, TryLockResult};

    #[derive(Debug)]
    pub struct RwLock<T> {
        inner: parking_lot::RwLock<T>,
    }

    impl<T> RwLock<T> {
        pub(crate) fn new(val: T) -> Self {
            Self {
                inner: parking_lot::RwLock::new(val),
            }
        }

        #[inline]
        pub(crate) fn read<'a>(&'a self) -> LockResult<RwLockReadGuard<'a, T>> {
            Ok(self.inner.read())
        }

        #[inline]
        #[allow(dead_code)] // may be used later;
        pub(crate) fn try_read<'a>(&'a self) -> TryLockResult<RwLockReadGuard<'a, T>> {
            self.inner.try_read().ok_or(TryLockError::WouldBlock)
        }

        #[inline]
        pub(crate) fn write<'a>(&'a self) -> LockResult<RwLockWriteGuard<'a, T>> {
            Ok(self.inner.write())
        }

        #[inline]
        pub(crate) fn try_write<'a>(&'a self) -> TryLockResult<RwLockWriteGuard<'a, T>> {
            self.inner.try_write().ok_or(TryLockError::WouldBlock)
        }
    }
}
