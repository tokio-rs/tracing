use std::error::Error;
mod layer;
use tracing_core::{Metadata, dispatcher};
#[derive(Debug)]
pub struct ContextError {
    inner: Box<dyn Error + Send + Sync>,
    context: Option<Context>,
}

#[derive(Clone, Debug)]
pub struct Context {
    context: Vec<Span>,
}

#[derive(Clone, Debug)]
struct Span {
    metadata: &'static Metadata<'static>,
    fields: String,
}

impl ContextError {
    pub fn from_error<F>(error: Box<dyn Error + Send + Sync>) -> Self {
        ContextError {
            inner: error,
            context: Context::<F>::current(),
        }
    }
}

pub trait TraceError: Error + Send + Sync {
    fn in_context<F>(self) -> ContextError
    where
        Self: Sized,
    {
        ContextError::from_error::<F>(Box::new(self))
    }
}

impl<T> TraceError for T where T: Error + Send + Sync {}

impl Context {
    fn current<F>() -> Option<Self> {
        dispatcher::get_default(|curr| {
            curr.downcast_ref::<layer::LayerMarker<F>>()?
                .current_context(&curr)
        })
    }

    fn new() -> Self {
        unimplemented!()
    }
}