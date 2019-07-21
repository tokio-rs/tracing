//! Reload filters in `FmtSubscriber`

use crate::{filter::Filter, span::Context};
use parking_lot::RwLock;
use std::{
    error, fmt,
    marker::PhantomData,
    sync::{Arc, Weak},
};
use tracing_core::{callsite, subscriber::Interest, Metadata};

#[derive(Debug)]
pub struct ReloadFilter<F, N>
where
    F: Filter<N>,
{
    inner: Arc<RwLock<F>>,
    _f: PhantomData<fn(N)>,
}

#[derive(Debug)]
pub struct Handle<F, N>
where
    F: Filter<N>,
{
    inner: Weak<RwLock<F>>,
    _f: PhantomData<fn(N)>,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    SubscriberGone,
}

// ===== impl ReloadFilter =====

impl<F, N> Filter<N> for ReloadFilter<F, N>
where
    F: Filter<N>,
{
    fn callsite_enabled(&self, metadata: &Metadata, ctx: &Context<N>) -> Interest {
        self.inner.read().callsite_enabled(metadata, ctx)
    }

    fn enabled(&self, metadata: &Metadata, ctx: &Context<N>) -> bool {
        self.inner.read().enabled(metadata, ctx)
    }
}

impl<F, N> ReloadFilter<F, N>
where
    F: Filter<N> + 'static,
{
    pub fn new(f: F) -> Self
    where
        F: Filter<N>,
    {
        Self {
            inner: Arc::new(RwLock::new(f)),
            _f: PhantomData,
        }
    }

    pub fn handle(&self) -> Handle<F, N> {
        Handle {
            inner: Arc::downgrade(&self.inner),
            _f: PhantomData,
        }
    }
}

// ===== impl Handle =====

impl<F, N> Handle<F, N>
where
    F: Filter<N> + 'static,
{
    pub fn reload(&self, new_filter: impl Into<F>) -> Result<(), Error> {
        self.modify(|filter| {
            *filter = new_filter.into();
        })
    }

    /// Invokes a closure with a mutable reference to the current filter,
    /// allowing it to be modified in place.
    pub fn modify(&self, f: impl FnOnce(&mut F)) -> Result<(), Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::SubscriberGone,
        })?;

        let mut lock = inner.write();
        f(&mut *lock);
        // Release the lock before rebuilding the interest cache, as that
        // function will lock the new filter.
        drop(lock);

        callsite::rebuild_interest_cache();
        Ok(())
    }

    /// Returns a clone of the filter's current value if it still exists.
    /// Otherwise, if the filter has been dropped, returns `None`.
    pub fn clone_current(&self) -> Option<F>
    where
        F: Clone,
    {
        self.with_current(F::clone).ok()
    }

    /// Invokes a closure with a borrowed reference to the current filter,
    /// returning the result (or an error if the filter no longer exists).
    pub fn with_current<T>(&self, f: impl FnOnce(&F) -> T) -> Result<T, Error> {
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::SubscriberGone,
        })?;
        let inner = inner.read();
        Ok(f(&*inner))
    }
}

impl<F, N> Clone for Handle<F, N>
where
    F: Filter<N>,
{
    fn clone(&self) -> Self {
        Handle {
            inner: self.inner.clone(),
            _f: PhantomData,
        }
    }
}

// ===== impl Error =====

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        error::Error::description(self).fmt(f)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self.kind {
            ErrorKind::SubscriberGone => "subscriber no longer exists",
        }
    }
}

#[cfg(test)]
mod test {
    use crate::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tracing_core::{
        dispatcher::{self, Dispatch},
        Metadata,
    };

    #[test]
    fn reload_handle() {
        static FILTER1_CALLS: AtomicUsize = AtomicUsize::new(0);
        static FILTER2_CALLS: AtomicUsize = AtomicUsize::new(0);

        enum Filter {
            One,
            Two,
        }
        impl<N> filter::Filter<N> for Filter {
            fn callsite_enabled(&self, _: &Metadata, _: &Context<N>) -> Interest {
                Interest::sometimes()
            }

            fn enabled(&self, _: &Metadata, _: &Context<N>) -> bool {
                match self {
                    Filter::One => FILTER1_CALLS.fetch_add(1, Ordering::Relaxed),
                    Filter::Two => FILTER2_CALLS.fetch_add(1, Ordering::Relaxed),
                };
                true
            }
        }
        fn event() {
            trace!("my event");
        }

        let subscriber = FmtSubscriber::builder()
            .compact()
            .with_filter(Filter::One)
            .with_filter_reloading();
        let handle = subscriber.reload_handle();
        let subscriber = Dispatch::new(subscriber.finish());

        dispatcher::with_default(&subscriber, || {
            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 0);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 0);

            event();

            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 0);

            handle.reload(Filter::Two).expect("should reload");

            event();

            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 1);
        })
    }

    #[test]
    fn reload_from_env() {
        use crate::filter::EnvFilter;
        let subscriber = FmtSubscriber::builder().with_filter_reloading().finish();
        let reload_handle = subscriber.reload_handle();
        reload_handle
            .reload(EnvFilter::from_default_env())
            .expect("should reload");
    }

}
