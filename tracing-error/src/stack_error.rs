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
        // SAFETY
        //
        // This function + the repr(C) on the ErrorImpl make the type erasure throughout the rest
        // of the class safe. This saves a function pointer that is parameterized on the Error type
        // being stored inside the ErrorImpl, this lets the object_ref function safely cast a type
        // erased `ErrorImpl` back to its original type, which is needed in order to forward our
        // error/display/debug impls to the internal error type from the type erased error type.
        //
        // The repr(C) is necessary to ensure that the struct is layed out in the order we
        // specified it so that we can safely access the vtable and spantrace fields thru a type
        // erased pointer to the original object.
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
        // # Safety
        //
        // The pointer used in this fn is guaranteed to be parameterized on the original error type
        // that was erased from the ErrorImpl object pointer before calling this fn. This means it
        // can safely be used to cast our pointer back to its original type. The original pointer
        // type is then implicitly converted to a trait object which then attaches the correct
        // Error vtable to the pointer when we return it as a dyn Error.
        unsafe { &*(self.vtable.object_ref)(self) }
    }
}

struct ErrorVTable {
    object_ref: unsafe fn(&ErrorImpl<Erased>) -> &(dyn Error + Send + Sync + 'static),
}

// # SAFETY
//
// This function must be parameterized on the type E of the original error that is being stored
// inside of the `ErrorImpl`. When it is correctly parameterized it defines a function that safely
// casts the Erased ErrorImpl pointer type back to the original pointer type.
unsafe fn object_ref<E>(e: &ErrorImpl<Erased>) -> &(dyn Error + Send + Sync + 'static)
where
    E: Error + Send + Sync + 'static,
{
    // Attach E's native Error vtable onto a pointer to self._object.
    &(*(e as *const ErrorImpl<Erased> as *const ErrorImpl<E>))._object
}

impl<E> Error for TracedError<E> {
    // # SAFETY
    //
    // This function is safe so long as all functions on `ErrorImpl<Erased>` only ever access the
    // wrapped error type via the `error` method defined on `ErrorImpl<Erased>`, which uses the
    // function in the vtable to safely convert the pointer type back to the original type then
    // returns the reference to the internal error.
    //
    // This function is necessary for the `downcast_ref` in `SpanTraceExt` to work, because it
    // needs a concrete type to downcast to and we cannot downcast to ErrorImpls parameterized on
    // errors defined in other crates. By erasing the type here we can always cast back to the
    // Erased version of the ErrorImpl pointer and still access the internal error type safely
    // through the vtable.
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
