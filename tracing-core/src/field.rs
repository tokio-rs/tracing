//! Span and `Event` key-value data.
//!
//! Spans and events may be annotated with key-value data, referred to as known
//! as _fields_. These fields consist of a mapping from a key (corresponding to
//! a `&str` but represented internally as an array index) to a [`Value`].
//!
//! # `Value`s and `Subscriber`s
//!
//! `Subscriber`s consume `Value`s as fields attached to [span]s or [`Event`]s.
//! The set of field keys on a given span or is defined on its [`Metadata`].
//! When a span is created, it provides [`Attributes`] to the `Subscriber`'s
//! [`new_span`] method, containing any fields whose values were provided when
//! the span was created; and may call the `Subscriber`'s [`record`] method
//! with additional [`Record`]s if values are added for more of its fields.
//! Similarly, the [`Event`] type passed to the subscriber's [`event`] method
//! will contain any fields attached to each event.
//!
//! `tracing` represents values as either one of a set of Rust primitives
//! (`i64`, `u64`, `bool`, and `&str`) or using a `fmt::Display` or `fmt::Debug`
//! implementation. `Subscriber`s are provided these primitive value types as
//! `dyn Value` trait objects.
//!
//! These trait objects can be formatted using `fmt::Debug`, but may also be
//! recorded as typed data by calling the [`Value::record`] method on these
//! trait objects with a _visitor_ implementing the [`Visit`] trait. This trait
//! represents the behavior used to record values of various types. For example,
//! we might record integers by incrementing counters for their field names,
//! rather than printing them.
//!
//! [`Value`]: trait.Value.html
//! [span]: ../span/
//! [`Event`]: ../event/struct.Event.html
//! [`Metadata`]: ../metadata/struct.Metadata.html
//! [`Attributes`]:  ../span/struct.Attributes.html
//! [`Record`]: ../span/struct.Record.html
//! [`new_span`]: ../subscriber/trait.Subscriber.html#method.new_span
//! [`record`]: ../subscriber/trait.Subscriber.html#method.record
//! [`event`]:  ../subscriber/trait.Subscriber.html#method.event
//! [`Value::record`]: trait.Value.html#method.record
//! [`Visit`]: trait.Visit.html
use crate::callsite;
use crate::stdlib::{
    any::{Any, TypeId},
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    iter, num,
    ops::Range,
    slice,
};

#[cfg(feature = "std")]
use std::error::Error;

use self::private::ValidLen;

/// An opaque key allowing _O_(1) access to a field in a `Span`'s key-value
/// data.
///
/// As keys are defined by the _metadata_ of a span, rather than by an
/// individual instance of a span, a key may be used to access the same field
/// across all instances of a given span with the same metadata. Thus, when a
/// subscriber observes a new span, it need only access a field by name _once_,
/// and use the key for that name for all other accesses.
#[derive(Debug)]
pub struct Field {
    i: usize,
    fields: FieldSet,
}

/// An empty field.
///
/// This can be used to indicate that the value of a field is not currently
/// present but will be recorded later.
///
/// When a field's value is `Empty`. it will not be recorded.
#[derive(Debug, Eq, PartialEq)]
pub struct Empty;

/// Describes the fields present on a span.
pub struct FieldSet {
    /// The names of each field on the described span.
    names: &'static [&'static str],
    /// The callsite where the described span originates.
    callsite: callsite::Identifier,
}

/// A set of fields and values for a span.
pub struct ValueSet<'a> {
    values: &'a [Value<'a>],
    fields: &'a FieldSet,
}

/// An iterator over a set of fields.
#[derive(Debug)]
pub struct Iter {
    idxs: Range<usize>,
    fields: FieldSet,
}
pub struct Values<'set, 'values> {
    values: iter::Enumerate<slice::Iter<'set, Value<'values>>>,
    fields: &'set FieldSet,
}

#[derive(Clone)]
pub struct Value<'a> {
    inner: ValueKind<'a>,
}

#[derive(Copy, Clone)]
enum ValueKind<'a> {
    Empty,
    Bool(bool),
    U64(u64),
    I64(i64),
    Str(&'a str),
    Display(&'a dyn fmt::Display),
    Debug(&'a dyn fmt::Debug),
    #[cfg(feature = "std")]
    Error(&'a (dyn Error + 'static)),
    #[cfg(feature = "std")]
    ErrorAny(&'a (dyn Error + 'static), TypeId),
    DebugAny(&'a dyn fmt::Debug, TypeId),
    DisplayAny(&'a dyn fmt::Display, TypeId),
    Args(fmt::Arguments<'a>),
}

/// Visits typed values.
///
/// An instance of `Visit` ("a visitor") represents the logic necessary to
/// record field values of various types. When an implementor of [`Value`] is
/// [recorded], it calls the appropriate method on the provided visitor to
/// indicate the type that value should be recorded as.
///
/// When a [`Subscriber`] implementation [records an `Event`] or a
/// [set of `Value`s added to a `Span`], it can pass an `&mut Visit` to the
/// `record` method on the provided [`ValueSet`] or [`Event`]. This visitor
/// will then be used to record all the field-value pairs present on that
/// `Event` or `ValueSet`.
///
/// # Examples
///
/// A simple visitor that writes to a string might be implemented like so:
/// ```
/// # extern crate tracing_core as tracing;
/// use std::fmt::{self, Write};
/// use tracing::field::{Value, Visit, Field};
/// pub struct StringVisitor<'a> {
///     string: &'a mut String,
/// }
///
/// impl<'a> Visit for StringVisitor<'a> {
///     fn record_debug(&mut self, field: &Field, value: &fmt::Debug) {
///         write!(self.string, "{} = {:?}; ", field.name(), value).unwrap();
///     }
/// }
/// ```
/// This visitor will format each recorded value using `fmt::Debug`, and
/// append the field name and formatted value to the provided string,
/// regardless of the type of the recorded value. When all the values have
/// been recorded, the `StringVisitor` may be dropped, allowing the string
/// to be printed or stored in some other data structure.
///
/// The `Visit` trait provides default implementations for `record_i64`,
/// `record_u64`, `record_bool`, `record_str`, and `record_error`, which simply
/// forward the recorded value to `record_debug`. Thus, `record_debug` is the
/// only method which a `Visit` implementation *must* implement. However,
/// visitors may override the default implementations of these functions in
/// order to implement type-specific behavior.
///
/// Additionally, when a visitor receives a value of a type it does not care
/// about, it is free to ignore those values completely. For example, a
/// visitor which only records numeric data might look like this:
///
/// ```
/// # extern crate tracing_core as tracing;
/// # use std::fmt::{self, Write};
/// # use tracing::field::{Value, Visit, Field};
/// pub struct SumVisitor {
///     sum: i64,
/// }
///
/// impl Visit for SumVisitor {
///     fn record_i64(&mut self, _field: &Field, value: i64) {
///        self.sum += value;
///     }
///
///     fn record_u64(&mut self, _field: &Field, value: u64) {
///         self.sum += value as i64;
///     }
///
///     fn record_debug(&mut self, _field: &Field, _value: &fmt::Debug) {
///         // Do nothing
///     }
/// }
/// ```
///
/// This visitor (which is probably not particularly useful) keeps a running
/// sum of all the numeric values it records, and ignores all other values. A
/// more practical example of recording typed values is presented in
/// `examples/counters.rs`, which demonstrates a very simple metrics system
/// implemented using `tracing`.
///
/// <div class="information">
///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
/// </div>
/// <div class="example-wrap" style="display:inline-block">
/// <pre class="ignore" style="white-space:normal;font:inherit;">
/// <strong>Note</strong>: The <code>record_error</code> trait method is only
/// available when the Rust standard library is present, as it requires the
/// <code>std::error::Error</code> trait.
/// </pre></div>
///
/// [`Value`]: trait.Value.html
/// [recorded]: trait.Value.html#method.record
/// [`Subscriber`]: ../subscriber/trait.Subscriber.html
/// [records an `Event`]: ../subscriber/trait.Subscriber.html#method.event
/// [set of `Value`s added to a `Span`]: ../subscriber/trait.Subscriber.html#method.record
/// [`Event`]: ../event/struct.Event.html
/// [`ValueSet`]: struct.ValueSet.html
pub trait Visit {
    /// Visit a signed 64-bit integer value.
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record_debug(field, &value)
    }

    /// Visit an unsigned 64-bit integer value.
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record_debug(field, &value)
    }

    /// Visit a boolean value.
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_debug(field, &value)
    }

    /// Visit a string value.
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_debug(field, &value)
    }

    /// Records a type implementing `Error`.
    ///
    /// <div class="information">
    ///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
    /// </div>
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: This is only enabled when the Rust standard library is
    /// present.
    /// </pre>
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record_debug(field, &format_args!("{}", value))
    }

    /// Visit a value implementing `fmt::Debug`.
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug);
}

// /// A field value of an erased type.
// ///
// /// Implementors of `Value` may call the appropriate typed recording methods on
// /// the [visitor] passed to their `record` method in order to indicate how
// /// their data should be recorded.
// ///
// /// [visitor]: trait.Visit.html
// pub trait Value: crate::sealed::Sealed {
//     /// Visits this value with the given `Visitor`.
//     fn record(&self, key: &Field, visitor: &mut dyn Visit);
// }

// /// A `Value` which serializes using `fmt::Display`.
// ///
// /// Uses `record_debug` in the `Value` implementation to
// /// avoid an unnecessary evaluation.
// #[derive(Clone)]
// pub struct DisplayValue<T: fmt::Display>(T);

// /// A `Value` which serializes as a string using `fmt::Debug`.
// #[derive(Clone)]
// pub struct DebugValue<T: fmt::Debug>(T);

/// Wraps a type implementing `fmt::Display` as a `Value` that can be
/// recorded using its `Display` implementation.
pub fn display<T>(t: &T) -> Value<'_>
where
    T: Any + fmt::Display,
{
    Value::any_display(t)
}

/// Wraps a type implementing `fmt::Debug` as a `Value` that can be
/// recorded using its `Debug` implementation.
pub fn debug<T>(t: &T) -> Value<'_>
where
    T: Any + fmt::Debug,
{
    Value::any(t)
}

// ===== impl Value =====

macro_rules! gen_primitives {
    ($(
        $(#[$m:meta])* ValueKind::$variant:ident($ty:ty) as $as_:ident
    ),+ $(,)?) => {
        impl<'a> Value<'a> {
            $(
                $(#[$m])*
                #[inline]
                pub fn $as_(&self) -> Option<$ty> {
                    match self.inner {
                        ValueKind::$variant(val) => Some(val),
                        _ => None,
                    }
                }
            )+
        }
        $(
            impl<'a> From<$ty> for Value<'a> {
                #[inline]
                fn from(val: $ty) -> Self {
                    Self {
                        inner: ValueKind::$variant(val),
                    }
                }
            }
        )+
    };
}

impl<'a> Value<'a> {
    pub fn empty() -> Self {
        Self {
            inner: ValueKind::Empty,
        }
    }

    pub fn any_display<T: Any + fmt::Display>(val: &'a T) -> Self {
        Self {
            inner: ValueKind::DisplayAny(val as &'a dyn fmt::Display, TypeId::of::<T>()),
        }
    }

    pub fn any<T: Any + fmt::Debug>(val: &'a T) -> Self {
        Self {
            inner: ValueKind::DebugAny(val as &'a dyn fmt::Debug, TypeId::of::<T>()),
        }
    }
    pub fn display<T: fmt::Display>(val: &'a T) -> Self {
        Self {
            inner: ValueKind::Display(val as &'a dyn fmt::Display),
        }
    }

    pub fn debug<T: fmt::Debug>(val: &'a T) -> Self {
        Self {
            inner: ValueKind::Debug(val as &'a dyn fmt::Debug),
        }
    }

    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn any_error<T: Any + Error + 'static>(val: &'a T) -> Self {
        Self {
            inner: ValueKind::ErrorAny(val as &'a (dyn Error + 'static), TypeId::of::<T>()),
        }
    }

    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn error(val: &'a (dyn Error + 'static)) -> Self {
        Self {
            inner: ValueKind::Error(val),
        }
    }

    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn as_error(&self) -> Option<&(dyn Error + 'static)> {
        match self.inner {
            ValueKind::Error(val) => Some(val),
            ValueKind::ErrorAny(val, _) => Some(val),
            _ => None,
        }
    }

    pub fn as_display(&self) -> Option<&dyn fmt::Display> {
        match self.inner {
            ValueKind::Empty => None,
            ValueKind::Display(val) => Some(val),
            ValueKind::DisplayAny(val, _) => Some(val),
            #[cfg(feature = "std")]
            ValueKind::ErrorAny(_, _) | ValueKind::Error(_) => Some(self as &dyn fmt::Display),
            ValueKind::Bool(ref val) => Some(val as &dyn fmt::Display),
            ValueKind::U64(ref val) => Some(val as &dyn fmt::Display),
            ValueKind::I64(ref val) => Some(val as &dyn fmt::Display),
            ValueKind::Str(ref val) => Some(val as &dyn fmt::Display),
            ValueKind::Args(ref val) => Some(val as &dyn fmt::Display),
            _ => None,
        }
    }

    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        let target = TypeId::of::<T>();
        match self.inner {
            ValueKind::Empty => None,
            ValueKind::Bool(ref val) if target == TypeId::of::<bool>() => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            ValueKind::U64(ref val) if target == TypeId::of::<u64>() => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            ValueKind::I64(ref val) if target == TypeId::of::<i64>() => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            ValueKind::Str(val) if target == TypeId::of::<str>() => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            #[cfg(feature = "std")]
            ValueKind::ErrorAny(val, actual) if actual == target => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            ValueKind::DebugAny(val, actual) if actual == target => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            ValueKind::DisplayAny(val, actual) if actual == target => {
                Some(unsafe { &*(val as *const _ as *const T) })
            }
            _ => None,
        }
    }

    pub fn is<T: Any>(&self) -> bool {
        let target = TypeId::of::<T>();
        match self.inner {
            ValueKind::Empty => target == TypeId::of::<Empty>(),
            ValueKind::Bool(_) => target == TypeId::of::<bool>(),
            ValueKind::U64(_) => target == TypeId::of::<u64>(),
            ValueKind::I64(_) => target == TypeId::of::<i64>(),
            ValueKind::Str(_) => target == TypeId::of::<str>(),
            #[cfg(feature = "std")]
            ValueKind::ErrorAny(_, actual) => actual == target,
            ValueKind::DebugAny(_, actual) => actual == target,
            ValueKind::DisplayAny(_, actual) => actual == target,
            _ => false,
        }
    }

    pub fn is_some(&self) -> bool {
        match self.inner {
            ValueKind::Empty => false,
            _ => true,
        }
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {
            ValueKind::Empty => Ok(()),
            ValueKind::DisplayAny(val, _) => fmt::Display::fmt(val, f),
            ValueKind::DebugAny(val, _) => fmt::Debug::fmt(val, f),
            ValueKind::Display(val) => fmt::Display::fmt(val, f),
            ValueKind::Debug(val) => fmt::Debug::fmt(val, f),
            #[cfg(feature = "std")]
            ValueKind::Error(val) => fmt::Debug::fmt(val, f),
            #[cfg(feature = "std")]
            ValueKind::ErrorAny(val, _) => fmt::Debug::fmt(val, f),
            ValueKind::Bool(ref val) => fmt::Debug::fmt(val, f),
            ValueKind::U64(ref val) => fmt::Debug::fmt(val, f),
            ValueKind::I64(ref val) => fmt::Debug::fmt(val, f),
            ValueKind::Str(val) => fmt::Debug::fmt(val, f),
            ValueKind::Args(ref val) => fmt::Debug::fmt(val, f),
        }
    }
}

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {
            ValueKind::Empty => Ok(()),
            ValueKind::DisplayAny(val, _) => fmt::Display::fmt(val, f),
            ValueKind::DebugAny(val, _) => fmt::Debug::fmt(val, f),
            ValueKind::Display(val) => fmt::Display::fmt(val, f),
            ValueKind::Debug(val) => fmt::Debug::fmt(val, f),
            #[cfg(feature = "std")]
            ValueKind::Error(val) => fmt::Display::fmt(val, f),
            #[cfg(feature = "std")]
            ValueKind::ErrorAny(val, _) => fmt::Display::fmt(val, f),
            ValueKind::Bool(ref val) => fmt::Display::fmt(val, f),
            ValueKind::U64(ref val) => fmt::Display::fmt(val, f),
            ValueKind::I64(ref val) => fmt::Display::fmt(val, f),
            ValueKind::Str(val) => fmt::Display::fmt(val, f),
            ValueKind::Args(ref val) => fmt::Display::fmt(val, f),
        }
    }
}

impl Default for Value<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

gen_primitives! {
    /// Returns `Some` if this value is a `bool`, or `None` otherwise.
    ValueKind::Bool(bool) as as_bool,

    /// Returns `Some` if this value is a `u64`, or `None` otherwise.
    ValueKind::U64(u64) as as_u64,

    /// Returns `Some` if this value is an `i64`, or `None` otherwise.
    ValueKind::I64(i64) as as_i64,

    /// Returns `Some` if this value is a `str`, or `None` otherwise.
    ValueKind::Str(&'a str) as as_str,

    // ValueKind::Error(&'a (dyn Error + 'static)) as as_error,
}

// ===== impl Visit =====

impl<'a, 'b> Visit for fmt::DebugStruct<'a, 'b> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.field(field.name(), value);
    }
}

impl<'a, 'b> Visit for fmt::DebugMap<'a, 'b> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.entry(&format_args!("{}", field), value);
    }
}

impl<F> Visit for F
where
    F: FnMut(&Field, &dyn fmt::Debug),
{
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        (self)(field, value)
    }
}

// ===== impl Value =====

macro_rules! impl_values {
    ( $( $record:ident( $( $whatever:tt)+ ) ),+ ) => {
        $(
            impl_value!{ $record( $( $whatever )+ ) }
        )+
    }
}

macro_rules! ty_to_nonzero {
    (u8) => {
        NonZeroU8
    };
    (u16) => {
        NonZeroU16
    };
    (u32) => {
        NonZeroU32
    };
    (u64) => {
        NonZeroU64
    };
    (u128) => {
        NonZeroU128
    };
    (usize) => {
        NonZeroUsize
    };
    (i8) => {
        NonZeroI8
    };
    (i16) => {
        NonZeroI16
    };
    (i32) => {
        NonZeroI32
    };
    (i64) => {
        NonZeroI64
    };
    (i128) => {
        NonZeroI128
    };
    (isize) => {
        NonZeroIsize
    };
}

macro_rules! impl_one_into_value {
    ($value_ty:tt, |$this:ident| $op:expr) => {
        impl<'a> From<$value_ty> for Value<'a> {
            fn from($this: $value_ty) -> Self {
                Self::from($op)
            }
        }
    }; // (nonzero, $value_ty:tt, $op:expr) => {
       //     // This `use num::*;` is reported as unused because it gets emitted
       //     // for every single invocation of this macro, so there are multiple `use`s.
       //     // All but the first are useless indeed.
       //     // We need this import because we can't write a path where one part is
       //     // the `ty_to_nonzero!($value_ty)` invocation.
       //     #[allow(clippy::useless_attribute, unused)]
       //     use num::*;
       //     impl<'a> From<$value_ty> for Value<'a> {
       //         fn from(val: $value_ty) -> Self {
       //             Self::from($op(val.get()))
       //         }
       //     }
       // };
}

macro_rules! impl_value {
    ( $( $value_ty:ty  ),+ |$this:ident| $op:expr ) => {
        $(
            impl_one_into_value! { $value_ty, |$this| $op }
        )+
    };
    ( $( $value_ty:ty ),+ as $as_ty:ty ) => {
        $(
            impl_one_into_value! { $value_ty, |this| this as $as_ty }
        )+
    };
}

impl_value! { usize, u32, u16, u8 as u64 }
impl_value! { isize, i32, i16, i8 as i64 }
// impl_value! { &'a usize, &'a u64, &'a u32, &'a u16, &'a u8 |val| *val as u64 }
// impl_value! { &'a isize, &'a i64, &'a i32, &'a i16, &'a i8 |val| *val as i64 }
// impl_value! { &'a bool |val| *val }
impl_value! {
    num::NonZeroUsize,
    num::NonZeroU64,
    num::NonZeroU32,
    num::NonZeroU16,
    num::NonZeroU8
    |val| val.get() as u64
}
impl_value! {
    num::NonZeroIsize,
    num::NonZeroI64,
    num::NonZeroI32,
    num::NonZeroI16,
    num::NonZeroI8
    |val| val.get() as i64
}

impl<'a, T> From<num::Wrapping<T>> for Value<'a>
where
    Self: From<T>,
{
    fn from(num::Wrapping(val): num::Wrapping<T>) -> Self {
        Self::from(val)
    }
}

impl From<&'_ Empty> for Value<'_> {
    fn from(_: &Empty) -> Self {
        Self::empty()
    }
}

impl<'a, T> From<Option<T>> for Value<'a>
where
    Self: From<T>,
{
    fn from(val: Option<T>) -> Self {
        val.map(Value::from).unwrap_or_else(Value::empty)
    }
}

impl<'a> From<fmt::Arguments<'a>> for Value<'a> {
    fn from(val: fmt::Arguments<'a>) -> Self {
        Value {
            inner: ValueKind::Args(val),
        }
    }
}

impl<'a, T> From<&'_ T> for Value<'a>
where
    Value<'a>: From<T>,
    T: Copy,
{
    fn from(val: &T) -> Self {
        Value::from(*val)
    }
}

impl<'a> From<&'_ Value<'a>> for Value<'a> {
    fn from(val: &Value<'a>) -> Self {
        Value { inner: val.inner }
    }
}

#[cfg(feature = "std")]
impl<'a> From<&'a Box<dyn Error + 'static>> for Value<'a> {
    fn from(err: &'a Box<dyn Error + 'static>) -> Self {
        Self::error(err.as_ref())
    }
}

#[cfg(feature = "std")]
impl<'a> From<&'a Box<dyn Error + Send + 'static>> for Value<'a> {
    fn from(err: &'a Box<dyn Error + Send + 'static>) -> Self {
        Self::error(err.as_ref())
    }
}

#[cfg(feature = "std")]
impl<'a> From<&'a Box<dyn Error + Send + Sync + 'static>> for Value<'a> {
    fn from(err: &'a Box<dyn Error + Send + Sync + 'static>) -> Self {
        Self::error(err.as_ref())
    }
}
// ===== impl Value =====

// impl_values! {
//     record_u64(u64),
//     record_u64(usize, u32, u16, u8 as u64),
//     record_i64(i64),
//     record_i64(isize, i32, i16, i8 as i64),
//     record_bool(bool)
// }

// impl<T: crate::sealed::Sealed> crate::sealed::Sealed for Wrapping<T> {}
// impl<T: crate::field::Value> crate::field::Value for Wrapping<T> {
//     fn record(&self, key: &crate::field::Field, visitor: &mut dyn crate::field::Visit) {
//         self.0.record(key, visitor)
//     }
// }

// impl crate::sealed::Sealed for str {}

// impl Value for str {
//     fn record(&self, key: &Field, visitor: &mut dyn Visit) {
//         visitor.record_str(key, &self)
//     }
// }

// #[cfg(feature = "std")]
// impl crate::sealed::Sealed for dyn std::error::Error + 'static {}

// #[cfg(feature = "std")]
// #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
// impl Value for dyn std::error::Error + 'static {
//     fn record(&self, key: &Field, visitor: &mut dyn Visit) {
//         visitor.record_error(key, self)
//     }
// }

// impl<'a, T: ?Sized> crate::sealed::Sealed for &'a T where T: Value + crate::sealed::Sealed + 'a {}

// impl<'a, T: ?Sized> Value for &'a T
// where
//     T: Value + 'a,
// {
//     fn record(&self, key: &Field, visitor: &mut dyn Visit) {
//         (*self).record(key, visitor)
//     }
// }

// impl<'a> crate::sealed::Sealed for fmt::Arguments<'a> {}

// impl<'a> Value for fmt::Arguments<'a> {
//     fn record(&self, key: &Field, visitor: &mut dyn Visit) {
//         visitor.record_debug(key, self)
//     }
// }

// impl fmt::Debug for dyn Value {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         // We are only going to be recording the field value, so we don't
//         // actually care about the field name here.
//         struct NullCallsite;
//         static NULL_CALLSITE: NullCallsite = NullCallsite;
//         impl crate::callsite::Callsite for NullCallsite {
//             fn set_interest(&self, _: crate::subscriber::Interest) {
//                 unreachable!("you somehow managed to register the null callsite?")
//             }

//             fn metadata(&self) -> &crate::Metadata<'_> {
//                 unreachable!("you somehow managed to access the null callsite?")
//             }
//         }

//         static FIELD: Field = Field {
//             i: 0,
//             fields: FieldSet::new(&[], crate::identify_callsite!(&NULL_CALLSITE)),
//         };

//         let mut res = Ok(());
//         self.record(&FIELD, &mut |_: &Field, val: &dyn fmt::Debug| {
//             res = write!(f, "{:?}", val);
//         });
//         res
//     }
// }

// impl fmt::Display for dyn Value {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         fmt::Debug::fmt(self, f)
//     }
// }

// // ===== impl DisplayValue =====

// impl<T: fmt::Display> crate::sealed::Sealed for DisplayValue<T> {}

// impl<T> Value for DisplayValue<T>
// where
//     T: fmt::Display,
// {
//     fn record(&self, key: &Field, visitor: &mut dyn Visit) {
//         visitor.record_debug(key, &format_args!("{}", self.0))
//     }
// }

// impl<T: fmt::Display> fmt::Debug for DisplayValue<T> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

// impl<T: fmt::Display> fmt::Display for DisplayValue<T> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         fmt::Display::fmt(&self.0, f)
//     }
// }

// // ===== impl DebugValue =====

// impl<T: fmt::Debug> crate::sealed::Sealed for DebugValue<T> {}

// impl<T: fmt::Debug> Value for DebugValue<T>
// where
//     T: fmt::Debug,
// {
//     fn record(&self, key: &Field, visitor: &mut dyn Visit) {
//         visitor.record_debug(key, &self.0)
//     }
// }

// impl<T: fmt::Debug> fmt::Debug for DebugValue<T> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self.0)
//     }
// }

// impl crate::sealed::Sealed for Empty {}
// impl Value for Empty {
//     #[inline]
//     fn record(&self, _: &Field, _: &mut dyn Visit) {}
// }

// ===== impl Field =====

impl Field {
    /// Returns an [`Identifier`] that uniquely identifies the [`Callsite`]
    /// which defines this field.
    ///
    /// [`Identifier`]: ../callsite/struct.Identifier.html
    /// [`Callsite`]: ../callsite/trait.Callsite.html
    #[inline]
    pub fn callsite(&self) -> callsite::Identifier {
        self.fields.callsite()
    }

    /// Returns a string representing the name of the field.
    pub fn name(&self) -> &'static str {
        self.fields.names[self.i]
    }

    #[doc(hidden)] // XXX(eliza): do we want to commit to this? i doubt it.
    pub fn index(&self) -> usize {
        self.i
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.name())
    }
}

impl AsRef<str> for Field {
    fn as_ref(&self) -> &str {
        self.name()
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.callsite() == other.callsite() && self.i == other.i
    }
}

impl Eq for Field {}

impl Hash for Field {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.callsite().hash(state);
        self.i.hash(state);
    }
}

impl Clone for Field {
    fn clone(&self) -> Self {
        Field {
            i: self.i,
            fields: FieldSet {
                names: self.fields.names,
                callsite: self.fields.callsite(),
            },
        }
    }
}

// ===== impl FieldSet =====

impl FieldSet {
    /// Constructs a new `FieldSet` with the given array of field names and callsite.
    pub const fn new(names: &'static [&'static str], callsite: callsite::Identifier) -> Self {
        Self { names, callsite }
    }

    /// Returns an [`Identifier`] that uniquely identifies the [`Callsite`]
    /// which defines this set of fields..
    ///
    /// [`Identifier`]: ../callsite/struct.Identifier.html
    /// [`Callsite`]: ../callsite/trait.Callsite.html
    pub(crate) fn callsite(&self) -> callsite::Identifier {
        callsite::Identifier(self.callsite.0)
    }

    /// Returns the [`Field`] named `name`, or `None` if no such field exists.
    ///
    /// [`Field`]: ../struct.Field.html
    pub fn field<Q: ?Sized>(&self, name: &Q) -> Option<Field>
    where
        Q: Borrow<str>,
    {
        let name = &name.borrow();
        self.names.iter().position(|f| f == name).map(|i| Field {
            i,
            fields: FieldSet {
                names: self.names,
                callsite: self.callsite(),
            },
        })
    }

    /// Returns `true` if `self` contains the given `field`.
    ///
    /// <div class="information">
    ///     <div class="tooltip ignore" style="">ⓘ<span class="tooltiptext">Note</span></div>
    /// </div>
    /// <div class="example-wrap" style="display:inline-block">
    /// <pre class="ignore" style="white-space:normal;font:inherit;">
    /// <strong>Note</strong>: If <code>field</code> shares a name with a field
    /// in this <code>FieldSet</code>, but was created by a <code>FieldSet</code>
    /// with a different callsite, this <code>FieldSet</code> does <em>not</em>
    /// contain it. This is so that if two separate span callsites define a field
    /// named "foo", the <code>Field</code> corresponding to "foo" for each
    /// of those callsites are not equivalent.
    /// </pre></div>
    pub fn contains(&self, field: &Field) -> bool {
        field.callsite() == self.callsite() && field.i <= self.len()
    }

    /// Returns an iterator over the `Field`s in this `FieldSet`.
    pub fn iter(&self) -> Iter {
        let idxs = 0..self.len();
        Iter {
            idxs,
            fields: FieldSet {
                names: self.names,
                callsite: self.callsite(),
            },
        }
    }

    /// Returns a new `ValueSet` with entries for this `FieldSet`'s values.
    ///
    /// Note that a `ValueSet` may not be constructed with arrays of over 32
    /// elements.
    #[doc(hidden)]
    pub fn value_set<'v, V>(&'v self, values: &'v V) -> ValueSet<'v>
    where
        V: ValidLen<'v>,
    {
        ValueSet {
            fields: self,
            values: &values.borrow()[..],
        }
    }

    /// Returns the number of fields in this `FieldSet`.
    #[inline]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns whether or not this `FieldSet` has fields.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

impl<'a> IntoIterator for &'a FieldSet {
    type IntoIter = Iter;
    type Item = Field;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Debug for FieldSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FieldSet")
            .field("names", &self.names)
            .field("callsite", &self.callsite)
            .finish()
    }
}

impl fmt::Display for FieldSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.names.iter().map(display))
            .finish()
    }
}

// ===== impl Iter =====

impl Iterator for Iter {
    type Item = Field;
    fn next(&mut self) -> Option<Field> {
        let i = self.idxs.next()?;
        Some(Field {
            i,
            fields: FieldSet {
                names: self.fields.names,
                callsite: self.fields.callsite(),
            },
        })
    }
}

// ===== impl ValueSet =====

impl<'a> ValueSet<'a> {
    /// Returns an [`Identifier`] that uniquely identifies the [`Callsite`]
    /// defining the fields this `ValueSet` refers to.
    ///
    /// [`Identifier`]: ../callsite/struct.Identifier.html
    /// [`Callsite`]: ../callsite/trait.Callsite.html
    #[inline]
    pub fn callsite(&self) -> callsite::Identifier {
        self.fields.callsite()
    }

    /// Visits all the fields in this `ValueSet` with the provided [visitor].
    ///
    /// [visitor]: ../trait.Visit.html
    pub(crate) fn record(&self, visitor: &mut dyn Visit) {
        let my_callsite = self.callsite();
        for (i, value) in self {
            if let ValueKind::Empty = value.inner {
                continue;
            }

            // value.record(visitor, self.fields.names[i], value)
            todo!()
        }
    }

    /// Returns `true` if this `ValueSet` contains a value for the given `Field`.
    pub(crate) fn contains(&self, field: &Field) -> bool {
        field.callsite() == self.callsite()
            && self
                .values
                .get(field.i)
                .map(Value::is_some)
                .unwrap_or(false)
    }

    /// Returns true if this `ValueSet` contains _no_ values.
    pub(crate) fn is_empty(&self) -> bool {
        !self.values.iter().any(Value::is_some)
    }

    pub(crate) fn field_set(&self) -> &FieldSet {
        self.fields
    }

    pub fn iter<'b>(&'b self) -> Values<'b, 'a> {
        Values {
            values: self.values.iter().enumerate(),
            fields: &self.fields,
        }
    }
}

impl<'a> fmt::Debug for ValueSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_struct("ValueSet");
        for (key, value) in self {
            if let ValueKind::Empty = value.inner {
                continue;
            }
            dbg.field(&key.name(), value);
        }
        dbg.finish()
    }
}

impl<'a> fmt::Display for ValueSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_map();
        for (key, value) in self {
            if let ValueKind::Empty = value.inner {
                continue;
            }
            dbg.entry(&format_args!("{}", key), value);
        }
        dbg.finish()
    }
}

impl<'set, 'values> IntoIterator for &'set ValueSet<'values> {
    type IntoIter = Values<'set, 'values>;
    type Item = (Field, &'set Value<'values>);
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'set, 'values> Iterator for Values<'set, 'values>
where
    'values: 'set,
{
    type Item = (Field, &'set Value<'values>);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, value) = self.values.next()?;
            if !value.is_some() {
                continue;
            }
            let field = Field {
                i,
                fields: FieldSet {
                    callsite: self.fields.callsite.clone(),
                    names: self.fields.names,
                },
            };
            return Some((field, value));
        }
    }
}

// ===== impl ValidLen =====

mod private {
    use super::*;

    /// Marker trait implemented by arrays which are of valid length to
    /// construct a `ValueSet`.
    ///
    /// `ValueSet`s may only be constructed from arrays containing 32 or fewer
    /// elements, to ensure the array is small enough to always be allocated on the
    /// stack. This trait is only implemented by arrays of an appropriate length,
    /// ensuring that the correct size arrays are used at compile-time.
    pub trait ValidLen<'a>: Borrow<[Value<'a>]> {}
}

macro_rules! impl_valid_len {
    ( $( $len:tt ),+ ) => {
        $(
            impl<'a> private::ValidLen<'a> for [Value<'a>; $len] {}
        )+
    }
}

impl_valid_len! {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::metadata::{Kind, Level, Metadata};
//     use crate::stdlib::{borrow::ToOwned, string::String};

//     struct TestCallsite1;
//     static TEST_CALLSITE_1: TestCallsite1 = TestCallsite1;
//     static TEST_META_1: Metadata<'static> = metadata! {
//         name: "field_test1",
//         target: module_path!(),
//         level: Level::INFO,
//         fields: &["foo", "bar", "baz"],
//         callsite: &TEST_CALLSITE_1,
//         kind: Kind::SPAN,
//     };

//     impl crate::callsite::Callsite for TestCallsite1 {
//         fn set_interest(&self, _: crate::subscriber::Interest) {
//             unimplemented!()
//         }

//         fn metadata(&self) -> &Metadata<'_> {
//             &TEST_META_1
//         }
//     }

//     struct TestCallsite2;
//     static TEST_CALLSITE_2: TestCallsite2 = TestCallsite2;
//     static TEST_META_2: Metadata<'static> = metadata! {
//         name: "field_test2",
//         target: module_path!(),
//         level: Level::INFO,
//         fields: &["foo", "bar", "baz"],
//         callsite: &TEST_CALLSITE_2,
//         kind: Kind::SPAN,
//     };

//     impl crate::callsite::Callsite for TestCallsite2 {
//         fn set_interest(&self, _: crate::subscriber::Interest) {
//             unimplemented!()
//         }

//         fn metadata(&self) -> &Metadata<'_> {
//             &TEST_META_2
//         }
//     }

//     #[test]
//     fn value_set_with_no_values_is_empty() {
//         let fields = TEST_META_1.fields();
//         let values = &[
//             (&fields.field("foo").unwrap(), None),
//             (&fields.field("bar").unwrap(), None),
//             (&fields.field("baz").unwrap(), None),
//         ];
//         let valueset = fields.value_set(values);
//         assert!(valueset.is_empty());
//     }

//     #[test]
//     fn empty_value_set_is_empty() {
//         let fields = TEST_META_1.fields();
//         let valueset = fields.value_set(&[]);
//         assert!(valueset.is_empty());
//     }

//     #[test]
//     fn value_sets_with_fields_from_other_callsites_are_empty() {
//         let fields = TEST_META_1.fields();
//         let values = &[
//             (&fields.field("foo").unwrap(), Some(&1 as &dyn Value)),
//             (&fields.field("bar").unwrap(), Some(&2 as &dyn Value)),
//             (&fields.field("baz").unwrap(), Some(&3 as &dyn Value)),
//         ];
//         let valueset = TEST_META_2.fields().value_set(values);
//         assert!(valueset.is_empty())
//     }

//     #[test]
//     fn sparse_value_sets_are_not_empty() {
//         let fields = TEST_META_1.fields();
//         let values = &[
//             (&fields.field("foo").unwrap(), None),
//             (&fields.field("bar").unwrap(), Some(&57 as &dyn Value)),
//             (&fields.field("baz").unwrap(), None),
//         ];
//         let valueset = fields.value_set(values);
//         assert!(!valueset.is_empty());
//     }

//     #[test]
//     fn fields_from_other_callsets_are_skipped() {
//         let fields = TEST_META_1.fields();
//         let values = &[
//             (&fields.field("foo").unwrap(), None),
//             (
//                 &TEST_META_2.fields().field("bar").unwrap(),
//                 Some(&57 as &dyn Value),
//             ),
//             (&fields.field("baz").unwrap(), None),
//         ];

//         struct MyVisitor;
//         impl Visit for MyVisitor {
//             fn record_debug(&mut self, field: &Field, _: &dyn (crate::stdlib::fmt::Debug)) {
//                 assert_eq!(field.callsite(), TEST_META_1.callsite())
//             }
//         }
//         let valueset = fields.value_set(values);
//         valueset.record(&mut MyVisitor);
//     }

//     #[test]
//     fn empty_fields_are_skipped() {
//         let fields = TEST_META_1.fields();
//         let values = &[
//             (&fields.field("foo").unwrap(), Some(&Empty as &dyn Value)),
//             (&fields.field("bar").unwrap(), Some(&57 as &dyn Value)),
//             (&fields.field("baz").unwrap(), Some(&Empty as &dyn Value)),
//         ];

//         struct MyVisitor;
//         impl Visit for MyVisitor {
//             fn record_debug(&mut self, field: &Field, _: &dyn (crate::stdlib::fmt::Debug)) {
//                 assert_eq!(field.name(), "bar")
//             }
//         }
//         let valueset = fields.value_set(values);
//         valueset.record(&mut MyVisitor);
//     }

//     #[test]
//     fn record_debug_fn() {
//         let fields = TEST_META_1.fields();
//         let values = &[
//             (&fields.field("foo").unwrap(), Some(&1 as &dyn Value)),
//             (&fields.field("bar").unwrap(), Some(&2 as &dyn Value)),
//             (&fields.field("baz").unwrap(), Some(&3 as &dyn Value)),
//         ];
//         let valueset = fields.value_set(values);
//         let mut result = String::new();
//         valueset.record(&mut |_: &Field, value: &dyn fmt::Debug| {
//             use crate::stdlib::fmt::Write;
//             write!(&mut result, "{:?}", value).unwrap();
//         });
//         assert_eq!(result, "123".to_owned());
//     }
// }
