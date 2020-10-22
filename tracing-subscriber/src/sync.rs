//! Abstracts over sync primitive implementations.
//!
//! Optionally, we allow the Rust standard library's `RwLock` to be replaced
//! with the `parking_lot` crate's implementation. This may provide improved
//! performance in some cases. However, the `parking_lot` dependency is an
//! opt-in feature flag. Because `parking_lot::RwLock` has a slightly different
//! API than `std::sync::RwLock` (it does not support poisoning on panics), we
//! wrap it with a type that provides the same method signatures. This allows us
//! to transparently swap `parking_lot` in without changing code at the cal

#[cfg(not(feature = "parking_lot"))]
pub(crate) use self::std_impl::*;

#[cfg(feature = "parking_lot")]
pub(crate) use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

#[cfg(not(feature = "parking_lot"))]
mod std_impl {
    #![allow(dead_code)] // some of this may be used later.

    use std::sync::{self, PoisonError, TryLockError};
    pub(crate) use std::sync::{RwLockReadGuard, RwLockWriteGuard};

    #[derive(Debug)]
    pub(crate) struct RwLock<T> {
        inner: sync::RwLock<T>,
    }

    impl<T> RwLock<T> {
        pub(crate) fn new(val: T) -> Self {
            Self {
                inner: sync::RwLock::new(val),
            }
        }

        #[inline]
        pub(crate) fn get_mut(&mut self) -> &mut T {
            self.inner.get_mut().unwrap_or_else(PoisonError::into_inner)
        }

        #[inline]
        pub(crate) fn read<'a>(&'a self) -> RwLockReadGuard<'a, T> {
            self.inner.read().unwrap_or_else(PoisonError::into_inner)
        }

        #[inline]
        pub(crate) fn try_read<'a>(&'a self) -> Option<RwLockReadGuard<'a, T>> {
            match self.inner.try_read() {
                Ok(guard) => Some(guard),
                Err(TryLockError::Poisoned(guard)) => Some(guard.into_inner()),
                Err(TryLockError::WouldBlock) => None,
            }
        }

        #[inline]
        pub(crate) fn write<'a>(&'a self) -> RwLockWriteGuard<'a, T> {
            self.inner.write().unwrap_or_else(PoisonError::into_inner)
        }
    }
}
