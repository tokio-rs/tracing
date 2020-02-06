use crate::SpanTrace;
use std::error::Error;
use std::fmt::{self, Debug, Display};

struct Erased;

///
pub struct TracedError<E> {
    inner: ErrorImpl<E>,
}

impl<E> From<E> for TracedError<E>
where
    E: Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        let vtable = &ErrorVTable {
            object_ref: object_ref::<E>,
        };

        Self {
            inner: ErrorImpl {
                vtable,
                spantrace: SpanTrace::capture(),
                _object: error,
            },
        }
    }
}

#[repr(C)]
struct ErrorImpl<E> {
    vtable: &'static ErrorVTable,
    spantrace: SpanTrace,
    // NOTE: Don't use directly. Use only through vtable. Erased type may have
    // different alignment.
    _object: E,
}

impl ErrorImpl<Erased> {
    pub(crate) fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        // Use vtable to attach E's native StdError vtable for the right
        // original type E.
        unsafe { &*(self.vtable.object_ref)(self) }
    }
}

struct ErrorVTable {
    object_ref: unsafe fn(&ErrorImpl<Erased>) -> &(dyn Error + Send + Sync + 'static),
}

unsafe fn object_ref<E>(e: &ErrorImpl<Erased>) -> &(dyn Error + Send + Sync + 'static)
where
    E: Error + Send + Sync + 'static,
{
    // Attach E's native Error vtable onto a pointer to self._object.
    &(*(e as *const ErrorImpl<Erased> as *const ErrorImpl<E>))._object
}

impl<E> Error for TracedError<E> {
    fn source<'a>(&'a self) -> Option<&'a (dyn Error + 'static)> {
        let erased = unsafe { &*(&self.inner as *const ErrorImpl<E> as *const ErrorImpl<Erased>) };
        Some(erased)
    }
}

impl<E> Debug for TracedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TRACED ERROR PLACEHOLDER")
    }
}

impl<E> Display for TracedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TRACED ERROR PLACEHOLDER")
    }
}

impl Error for ErrorImpl<Erased> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.error().source()
    }
}

impl Debug for ErrorImpl<Erased> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.error(), f)
    }
}

impl Display for ErrorImpl<Erased> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.error(), f)
    }
}

impl<E> Error for ErrorImpl<E>
where
    E: Error + Send + Sync + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self._object)
    }
}

impl<E> Debug for ErrorImpl<E>
where
    E: Error + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self._object, f)
    }
}

impl<E> Display for ErrorImpl<E>
where
    E: Error + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self._object, f)
    }
}

///
pub trait Instrument<T, E> {
    ///
    fn in_current_span(self) -> Result<T, TracedError<E>>;
}

impl<T, E> Instrument<T, E> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    fn in_current_span(self) -> Result<T, TracedError<E>> {
        self.map_err(TracedError::from)
    }
}

///
pub trait SpanTraceExt {
    ///
    fn spantrace(&self) -> Option<&SpanTrace>;
}

impl SpanTraceExt for &(dyn Error + 'static) {
    fn spantrace(&self) -> Option<&SpanTrace> {
        self.downcast_ref::<ErrorImpl<Erased>>()
            .map(|inner| &inner.spantrace)
    }
}
