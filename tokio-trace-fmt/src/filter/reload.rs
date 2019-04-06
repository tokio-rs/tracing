use parking_lot::RwLock;
use std::{error, fmt, marker::PhantomData};
use tokio_trace_core::{
    dispatcher::{self, Dispatch},
    subscriber::Interest,
    Metadata,
};
use {filter::Filter, span::Context};

#[derive(Debug)]
pub struct ReloadFilter<F> {
    inner: RwLock<F>,
}

#[derive(Debug, Clone)]
pub struct Handle<F> {
    dispatch: Dispatch,
    _p: PhantomData<fn(F)>,
}

#[derive(Debug)]
pub struct Error {
    _p: (),
}

pub fn reload_current<F, N>(new_filter: impl Into<F>) -> Result<(), Error>
where
    F: Filter<N> + 'static,
{
    let mut new_filter = Some(new_filter);
    dispatcher::get_default(|current| {
        let current = current
            .downcast_ref::<ReloadFilter<F>>()
            .ok_or(Error { _p: () })?;
        let new_filter = new_filter.take().expect("cannot be taken twice");
        current.reload(new_filter);
        Ok(())
    })
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
    fn reload<N>(&self, new_filter: impl Into<F>)
    where
        F: Filter<N>,
    {
        *self.inner.write() = new_filter.into();
    }
}

// ===== impl Handle =====

impl<F: 'static> Handle<F> {
    pub fn try_from<N>(dispatch: Dispatch) -> Result<Self, Error>
    where
        F: Filter<N>,
    {
        if dispatch.is::<ReloadFilter<F>>() {
            Ok(Self {
                dispatch,
                _p: PhantomData,
            })
        } else {
            Err(Error { _p: () })
        }
    }

    pub fn reload<N>(&self, new_filter: impl Into<F>)
    where
        F: Filter<N>,
    {
        self.dispatch
            .downcast_ref::<ReloadFilter<F>>()
            .expect("dispatch must still downcast to reload filter")
            .reload(new_filter)
    }
}

// ===== impl Error =====

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt("dispatcher could not be downcast to reloadable filter", f)
    }
}

impl error::Error for Error {}
