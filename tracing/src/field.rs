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
/// <strong>Note</strong>: Although this is implemented for both the
/// <a href="./struct.Field.html"><code>Field</code></a> type <em>and</em> any
/// type that can be borrowed as an <code>&str</code>, only <code>Field</code>
/// allows <em>O</em>(1) access.
/// Indexing a field with a string results in an iterative search that performs
/// string comparisons. Thus, if possible, once the key for a field is known, it
/// should be used whenever possible.
/// </pre>
pub trait AsField: crate::sealed::Sealed {
    /// Attempts to convert `&self` into a `Field` with the specified `metadata`.
    ///
    /// If `metadata` defines this field, then the field is returned. Otherwise,
    /// this returns `None`.
    fn as_field(&self, metadata: &Metadata<'_>) -> Option<Field>;
}

pub trait AsValue<'a> {
    fn as_value(self) -> Value<'a>;
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

pub(crate) mod convert {
    use super::Value;
    use std::{
        any::Any,
        fmt::{Debug, Display},
    };

    pub struct Specialize<'a, T: ?Sized>(pub &'a T);

    #[doc(hidden)]
    pub trait AsDebugValue<'a> {
        fn as_debug_value(&'a self) -> Value<'a>;
    }

    // impl<'a, T: Debug> AsDebugValue<'a> for &'_ &'_ Specialize<'a, T> {
    //     fn as_debug_value(&self) -> Value<'a> {
    //         dbg!("as debug value");
    //         Value::debug(self.0)
    //     }
    // // }
    // // #[doc(hidden)]
    // pub trait AsDebugUnsizedValue<'a> {
    //     fn as_debug_value(&'a self) -> Value<'a>;
    // }
    impl<'a, T: ?Sized + Debug> AsDebugValue<'a> for &T {
        fn as_debug_value(&'a self) -> Value<'a> {
            dbg!(("as debug unsized value", std::any::type_name::<T>()));
            Value::debug(self)
        }
    }

    // impl<T: ?Sized + Debug> AsDebugValue for &T {
    //     fn as_debug_value(&self) -> Value<'_> {
    //         dbg!(("as debug unsized value", std::any::type_name::<T>()));
    //         Value::debug(self)
    //     }
    // }

    #[doc(hidden)]
    pub trait AsDebugAnyValue<'a> {
        fn as_debug_value(&'a self) -> Value<'a>;
    }

    impl<'a, T: Any + Debug> AsDebugAnyValue<'a> for T {
        fn as_debug_value(&'a self) -> Value<'a> {
            dbg!("as debug any value");
            Value::any(self)
        }
    }

    #[doc(hidden)]
    pub trait AsDisplayValue<'a> {
        fn as_display_value(&self) -> Value<'a>;
    }

    impl<'a, T: Display> AsDisplayValue<'a> for &'_ &'_ Specialize<'a, T> {
        fn as_display_value(&self) -> Value<'a> {
            Value::display(self.0)
        }
    }

    #[doc(hidden)]
    pub trait AsDisplayUnsizedValue<'a> {
        fn as_display_value(&self) -> Value<'a>;
    }

    impl<'a, T: ?Sized + Display> AsDisplayUnsizedValue<'a> for &'a Specialize<'a, T> {
        fn as_display_value(&self) -> Value<'a> {
            Value::display(&self.0)
        }
    }

    #[doc(hidden)]
    pub trait AsDisplayAnyValue<'a> {
        fn as_display_value(&'a self) -> Value<'a>;
    }

    impl<'a, T: Any + Display> AsDisplayAnyValue<'a> for Specialize<'a, T> {
        fn as_display_value(&'a self) -> Value<'a> {
            Value::any_display(self.0)
        }
    }

    #[doc(hidden)]
    pub trait AsValue<'a> {
        fn as_value(&self) -> Value<'a>;
    }

    impl<'a, T: ?Sized> AsValue<'a> for &'_ &'_ Specialize<'a, T>
    where
        Value<'a>: From<&'a T>,
    {
        fn as_value(&self) -> Value<'a> {
            Value::from(self.0)
        }
    }

    #[doc(hidden)]
    pub trait AsOptionValue<'a> {
        fn as_value(&self) -> Value<'a>;
    }

    impl<'a, T> AsOptionValue<'a> for &'_ Specialize<'a, Option<T>>
    where
        Value<'a>: From<&'a T>,
    {
        fn as_value(&self) -> Value<'a> {
            self.0
                .as_ref()
                .map(Value::from)
                .unwrap_or_else(Value::empty)
        }
    }

    #[doc(hidden)]
    #[cfg(feature = "std")]
    pub trait AsErrorValue<'a> {
        fn as_value(&self) -> Value<'a>;
    }

    #[cfg(feature = "std")]
    impl<'a, T> AsErrorValue<'a> for Specialize<'a, T>
    where
        T: Any + std::error::Error + 'static,
    {
        fn as_value(&self) -> Value<'a> {
            Value::any_error(self.0)
        }
    }
}
