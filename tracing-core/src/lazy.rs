#[cfg(feature = "std")]
pub(crate) use once_cell::sync::Lazy;

#[cfg(not(feature = "std"))]
pub(crate) use self::spin::Lazy;

#[cfg(not(feature = "std"))]
mod spin {
    use crate::spin::Once;
    use core::{cell::Cell, fmt, ops::Deref};

    /// Re-implementation of `once_cell::sync::Lazy` on top of `spin::Once`
    /// rather than `OnceCell`.
    ///
    /// This is used when the standard library is disabled.
    pub(crate) struct Lazy<T, F = fn() -> T> {
        cell: Once<T>,
        init: Cell<Option<F>>,
    }

    impl<T: fmt::Debug, F> fmt::Debug for Lazy<T, F> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.debug_struct("Lazy")
                .field("cell", &self.cell)
                .field("init", &"..")
                .finish()
        }
    }

    impl<T, F> Lazy<T, F> {
        /// Creates a new lazy value with the given initializing function.
        pub(crate) const fn new(init: F) -> Lazy<T, F> {
            Lazy {
                cell: Once::new(),
                init: Cell::new(Some(init)),
            }
        }

        /// Consumes this `Lazy` returning the stored value.
        ///
        /// Returns `Ok(value)` if `Lazy` is initialized and `Err(f)` otherwise.
        pub(crate) fn into_value(this: Lazy<T, F>) -> Result<T, F> {
            let cell = this.cell;
            let init = this.init;
            cell.into_inner().ok_or_else(|| {
                init.take()
                    .unwrap_or_else(|| panic!("Lazy instance has previously been poisoned"))
            })
        }
    }

    impl<T, F: FnOnce() -> T> Lazy<T, F> {
        /// Forces the evaluation of this lazy value and returns a reference to
        /// the result.
        ///
        /// This is equivalent to the `Deref` impl, but is explicit.
        pub(crate) fn force(this: &Lazy<T, F>) -> &T {
            this.cell.get_or_init(|| match this.init.take() {
                Some(f) => f(),
                None => panic!("Lazy instance has previously been poisoned"),
            })
        }
    }

    impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
        type Target = T;
        fn deref(&self) -> &T {
            Lazy::force(self)
        }
    }

    impl<T, F: FnOnce() -> T> DerefMut for Lazy<T, F> {
        fn deref_mut(&mut self) -> &mut T {
            Lazy::force(self);
            self.cell.get_mut().unwrap_or_else(|| unreachable!())
        }
    }

    impl<T: Default> Default for Lazy<T> {
        /// Creates a new lazy value using `Default` as the initializing function.
        fn default() -> Lazy<T> {
            Lazy::new(T::default)
        }
    }
}
