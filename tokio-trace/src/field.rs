pub use tokio_trace_core::field::*;
use Meta;

/// Trait implemented to allow a type to be used as a field key.
///
/// **Note**: Although this is implemented for both the [`Key`] type *and* any
/// type that can be borrowed as an `&str`, only `Key` allows _O_(1) access.
/// Indexing a field with a string results in an iterative search that performs
/// string comparisons. Thus, if possible, once the key for a field is known, it
/// should be used whenever possible.
pub trait AsKey {
    /// Attempts to convert `&self` into a `Key` with the specified `metadata`.
    ///
    /// If `metadata` defines a key corresponding to this field, then the key is
    /// returned. Otherwise, this function returns `None`.
    fn as_key<'a>(&self, metadata: &'a Meta<'a>) -> Option<Key<'a>>;
}

// ===== impl AsKey =====

impl<'f> AsKey for Key<'f> {
    #[inline]
    fn as_key<'a>(&self, metadata: &'a Meta<'a>) -> Option<Key<'a>> {
        self.with_metadata(metadata)
    }
}

impl<'f> AsKey for &'f Key<'f> {
    #[inline]
    fn as_key<'a>(&self, metadata: &'a Meta<'a>) -> Option<Key<'a>> {
        self.with_metadata(metadata)
    }
}

impl AsKey for str {
    #[inline]
    fn as_key<'a>(&self, metadata: &'a Meta<'a>) -> Option<Key<'a>> {
        metadata.key_for(&self)
    }
}
