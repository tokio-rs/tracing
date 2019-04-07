use parking_lot::RwLock;
use std::{
    error, fmt,
    sync::{Arc, Weak},
};
use tokio_trace_core::{subscriber::Interest, Metadata};
use {filter::Filter, span::Context};

#[derive(Debug)]
pub struct ReloadFilter<F> {
    inner: Arc<RwLock<F>>,
}

#[derive(Debug, Clone)]
pub struct Handle<F> {
    inner: Weak<RwLock<F>>,
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

impl<F, N> Filter<N> for ReloadFilter<F>
where
    F: Filter<N>,
{
    fn callsite_enabled(&self, _: &Metadata, _: &Context<N>) -> Interest {
        // TODO(eliza): When tokio-rs/tokio#1039 lands, we can allow our
        // interest to be cached. For now, we must always return `sometimes`.
        Interest::sometimes()
    }

    fn enabled(&self, metadata: &Metadata, ctx: &Context<N>) -> bool {
        self.inner.read().enabled(metadata, ctx)
    }
}

impl<F: 'static> ReloadFilter<F> {
    pub fn new<N>(f: F) -> Self
    where
        F: Filter<N>,
    {
        Self {
            inner: Arc::new(RwLock::new(f)),
        }
    }

    pub fn handle(&self) -> Handle<F> {
        Handle {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

// ===== impl Handle =====

impl<F: 'static> Handle<F> {
    pub fn reload<N>(&self, new_filter: impl Into<F>) -> Result<(), Error>
    where
        F: Filter<N>,
    {
        // TODO(eliza): When tokio-rs/tokio#1039 lands, this is where we would
        // invalidate the callsite cache.
        let inner = self.inner.upgrade().ok_or(Error {
            kind: ErrorKind::SubscriberGone,
        })?;
        *inner.write() = new_filter.into();
        Ok(())
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio_trace_core::{
        dispatcher::{self, Dispatch},
        Metadata,
    };
    use *;

    #[test]
    fn reload_handle() {
        static FILTER1_CALLS: AtomicUsize = AtomicUsize::new(0);
        static FILTER2_CALLS: AtomicUsize = AtomicUsize::new(0);

        fn filter1<N>(_: &Metadata, _: &span::Context<N>) -> bool {
            FILTER1_CALLS.fetch_add(1, Ordering::Relaxed);
            true
        }

        fn filter2<N>(_: &Metadata, _: &span::Context<N>) -> bool {
            FILTER2_CALLS.fetch_add(1, Ordering::Relaxed);
            true
        }

        fn event() {
            trace!("my event");
        }

        let subscriber = FmtSubscriber::builder()
            .with_filter(filter1 as fn(&Metadata, &span::Context<_>) -> bool)
            .with_filter_reloading();
        let handle = subscriber.reload_handle();
        let subscriber = Dispatch::new(subscriber.finish());

        dispatcher::with_default(&subscriber, || {
            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 0);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 0);

            event();

            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 0);

            handle
                .reload(filter2 as fn(&Metadata, &span::Context<_>) -> bool)
                .expect("should reload");

            event();

            assert_eq!(FILTER1_CALLS.load(Ordering::Relaxed), 1);
            assert_eq!(FILTER2_CALLS.load(Ordering::Relaxed), 1);
        })
    }
}
