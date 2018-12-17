use futures::{
    future::{ExecuteError, Executor},
    Future,
};
use tokio_trace::Span;
use {Instrument, Instrumented, WithDispatch};

pub trait InstrumentExecutor<'a, F>
where
    Self: Executor<Instrumented<'a, F>>,
    F: Future<Item = (), Error = ()>,
{
    fn instrument<G>(self, mk_span: G) -> InstrumentedExecutor<Self, G>
    where
        G: Fn() -> Span<'a>,
        Self: Sized,
    {
        InstrumentedExecutor {
            inner: self,
            mk_span,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstrumentedExecutor<T, G> {
    inner: T,
    mk_span: G,
}

impl<'a, T, F> InstrumentExecutor<'a, F> for T
where
    T: Executor<Instrumented<'a, F>>,
    F: Future<Item = (), Error = ()>,
{
}

macro_rules! deinstrument_err {
    ($e:expr) => {
        $e.map_err(|e| {
            let kind = e.kind();
            let future = e.into_future().inner;
            ExecuteError::new(kind, future)
        })
    };
}

impl<'a, T, F, N> Executor<F> for InstrumentedExecutor<T, N>
where
    T: Executor<Instrumented<'a, F>>,
    F: Future<Item = (), Error = ()>,
    N: Fn() -> Span<'a>,
{
    fn execute(&self, future: F) -> Result<(), ExecuteError<F>> {
        let future = future.instrument((self.mk_span)());
        deinstrument_err!(self.inner.execute(future))
    }
}

impl<T, F> Executor<F> for WithDispatch<T>
where
    T: Executor<WithDispatch<F>>,
    F: Future<Item = (), Error = ()>,
{
    fn execute(&self, future: F) -> Result<(), ExecuteError<F>> {
        let future = self.with_dispatch(future);
        deinstrument_err!(self.inner.execute(future))
    }
}
