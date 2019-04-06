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
    fn callsite_enabled(&self, metadata: &Metadata, ctx: &Context<N>) -> Interest {
        self.inner.read().callsite_enabled(metadata, ctx)
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
        match self.kind {
            ErrorKind::SubscriberGone => "subscriber no longer exists".fmt(f),
        }
    }
}

impl error::Error for Error {}
