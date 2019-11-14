pub mod fmt;
mod layer;
use std::error::Error;
use std::marker::PhantomData;
use tracing_core::{dispatcher, Metadata};

pub use self::layer::ErrorLayer;

pub struct ContextError<F = fmt::DefaultFields> {
    inner: Box<dyn Error + Send + Sync>,
    context: Option<Context<F>>,
}

#[derive(Clone)]
pub struct Context<F = fmt::DefaultFields> {
    context: Vec<Span>,
    _fmt: PhantomData<fn(F)>,
}

#[derive(Clone, Debug)]
struct Span {
    metadata: &'static Metadata<'static>,
    fields: String,
}

impl<F> ContextError<F>
where
    F: for<'writer> fmt::FormatFields<'writer> + 'static,
{
    pub fn from_error(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        ContextError {
            inner: error,
            context: Context::<F>::current(),
        }
    }

    pub fn context(&self) -> Option<&Context<F>> {
        self.context.as_ref()
    }
}

pub trait TraceError: Error + Sized + Send + Sync + 'static {
    fn in_context(self) -> ContextError {
        ContextError::from_error(Box::new(self))
    }
}

impl<T> TraceError for T where T: Error + Sized + Send + Sync + 'static {}

impl<F> Context<F>
where
    F: for<'writer> fmt::FormatFields<'writer> + 'static,
{
    fn current() -> Option<Self> {
        dispatcher::get_default(|curr| curr.downcast_ref::<ErrorLayer<F>>()?.current_context(&curr))
    }

    fn new() -> Self {
        Self {
            context: Vec::new(),
            _fmt: PhantomData,
        }
    }
}

impl<F> std::fmt::Debug for Context<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for Span {
            ref metadata,
            ref fields,
        } in self.context.iter()
        {
            map.entry(&metadata.name(), &fmt_args!("{}", fields));
        }
        map.finish()
    }
}

impl<F> std::fmt::Display for ContextError<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)?;
        if let Some(ctx) = self.context.as_ref() {
            writeln!(f, "");
            for Span {
                ref metadata,
                ref fields,
            } in ctx.context.iter()
            {
                write!(f, "   in {}::{}", metadata.target(), metadata.name())?;
                if fields.len() > 0 {
                    write!(f, ", {}", fields)?
                }
                writeln!(f, "");
                if let Some((file, line)) = metadata
                    .file()
                    .and_then(|f| metadata.line().map(|l| (f, l)))
                {
                    writeln!(f, "\tat {}:{}", file, line)?;
                }
            }
        }
        Ok(())
    }
}

impl<F> std::fmt::Debug for ContextError<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextError")
            .field("inner", &self.inner)
            .field("context", &self.context)
            .finish()
    }
}

impl<F> Error for ContextError<F> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.inner.as_ref())
    }
}
