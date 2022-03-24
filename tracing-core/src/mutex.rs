#[cfg(feature = "std")]
pub(crate) use self::imp::*;
#[cfg(feature = "std")]
mod imp {
    use std::sync::Mutex as StdMutex;
    pub(crate) use std::sync::MutexGuard;

    #[derive(Debug)]
    pub(crate) struct Mutex<T>(StdMutex<T>);

    impl<T> Mutex<T> {
        pub(crate) fn new(data: T) -> Self {
            Self(StdMutex::new(data))
        }

        pub(crate) fn lock(&self) -> MutexGuard<'_, T> {
            match self.0.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            }
        }
    }
}

#[cfg(not(feature = "std"))]
pub(crate) use crate::spin::{Mutex, MutexGuard};
