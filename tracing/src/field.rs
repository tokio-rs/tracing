//! Structured data associated with `Span`s and `Event`s.
pub use tracing_core::field::*;

use crate::Metadata;

/// Trait implemented to allow a type to be used as a field key.
///
/// <div class="information">
///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
/// </div>
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
///
/// **Note**: Although this is implemented for both the [`Field`] type
/// *and* any type that can be borrowed as an `&str`, only `Field` allows *O*(1) access.
/// Indexing a field with a string results in an iterative search that performs
/// string comparisons. Thus, if possible, once the key for a field is known, it
/// should be used whenever possible.
///
/// </pre>
pub trait AsField: crate::sealed::Sealed {
    /// Attempts to convert `&self` into a `Field` with the specified `metadata`.
    ///
    /// If `metadata` defines this field, then the field is returned. Otherwise,
    /// this returns `None`.
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field>;
}

// ===== impl AsField =====

impl AsField for Field {
    #[inline]
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field> {
        if self.callsite() == metadata.callsite() {
            Some(self.clone())
        } else {
            None
        }
    }
}

impl<'a> AsField for &'a Field {
    #[inline]
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field> {
        if self.callsite() == metadata.callsite() {
            Some((*self).clone())
        } else {
            None
        }
    }
}

impl AsField for str {
    #[inline]
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field> {
        metadata.fields().field(&self)
    }
}

impl crate::sealed::Sealed for Field {}
impl<'a> crate::sealed::Sealed for &'a Field {}
impl crate::sealed::Sealed for str {}

pub(crate) mod specialize {
    use super::Value;
    use core::fmt::{Debug, Display};

    #[cfg(feature = "std")]
    use std::error::Error;

    #[cfg(feature = "std")]
    pub trait AsValueError<'a> {
        fn as_value(&'a self) -> Value<'a>;
    }

    #[cfg(feature = "std")]
    impl<'a, T> AsValueError<'a> for &&&Specialize<&'a T>
    where
        T: Error + 'static,
    {
        #[inline]
        fn as_value(&'a self) -> Value<'a> {
            println!("dispatching from err: <{}>", std::any::type_name::<T>());
            Value::error(self.0)
        }
    }

    pub struct Specialize<T>(pub T);

    pub trait AsValuePrimitive<'a> {
        fn as_value(&'a self) -> Value<'a>;
    }

    impl<'a, T> AsValuePrimitive<'a> for &&Specialize<&'a T>
    where
        Value<'a>: From<&'a T>,
        T: 'a,
    {
        #[inline]
        fn as_value(&'a self) -> Value<'a> {
            println!(
                "dispatching from primitive: <{}>",
                std::any::type_name::<T>()
            );
            Value::from(&self.0)
        }
    }
    pub trait AsValueDisplay<'a> {
        fn as_value(&self) -> Value<'a>;
    }

    impl<'a, T: Display + Debug> AsValueDisplay<'a> for &Specialize<&'a T> {
        #[inline]
        fn as_value(&self) -> Value<'a> {
            println!("dispatching from display: <{}>", std::any::type_name::<T>());
            Value::display(self.0)
        }
    }

    pub trait AsValueDebug<'a> {
        fn as_value(&self) -> Value<'a>;
    }

    impl<'a, T: Debug> AsValueDebug<'a> for Specialize<&'a T> {
        #[inline]
        fn as_value(&self) -> Value<'a> {
            println!("dispatching from debug: <{}>", std::any::type_name::<T>());
            Value::debug(self.0)
        }
    }
}
