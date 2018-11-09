//! `Span` and `Event` key-value data.
//!
//! Spans and events may be annotated with key-value data, referred to as known
//! as _fields_. These fields consist of a mapping from a `&'static str` to a
//! piece of data, known as a `Value`.
//!
//! # Value Type Erasure
//!
//! Rather than restricting `Value`s to a set of Rust primitives, `tokio-trace`
//! allows values to be of any type. This means that arbitrary user-defined
//! types may be attached to spans or events, provided they meet certain
//! requirements.
//!
//! Typically, we might accept arbitrarily-typed values by making the
//! `Subscriber` APIs that accept them generic. However, as the `Dispatch` type
//! holds a subscriber as a boxed trait object, the `Subscriber` trait must be
//! object-safe --- it cannot have trait methods that accept a generic
//! parameter. Thus, we erase the value's original type.
//!
//! However, a `Value` which is valid for the `'static` lifetime may also be
//! _downcast_ back to a concrete type, similarly to `std::error::Error` and
//! `std::any::Any`. If the erased type of the value is known, downcasting
//! allows it to be used as an instance of that type.
//!
//! # `Value`s and `Subscriber`s
//!
//! `Subscriber`s consume `Value`s as fields attached to `Event`s or `Span`s.
//! These cases are handled somewhat differently.
//!
//! When a field is attached to an `Event`, the `Subscriber::observe_event`
//! method is passed an `Event` struct which provides an iterator
//! (`Event::fields`) to iterate over the event's fields, providing references
//! to the values as `BorrowedValue`s. Since an `Event` represents a _moment_ in
//! time, it   does not expect to outlive the scope that created it. Thus, the
//! values attached to an `Event` are _borrowed_ from the scope where the
//! `Event` originated.
//!
//! `Span`s, on the other hand, are somewhat more complex. A `Span` may outlive
//! scope in which it was created, and as `Span`s are not instantaneous, the
//! values of their fields may be discovered and added to the span _during_ the
//! `Span`'s execution. Thus, rather than receiving all the field values when
//! the span is initially created, subscribers are instead notified of each
//! field as it is added to the span, via the `Subscriber::add_value` method.
//! That method is called with the span's ID, the name of the field whose value
//! is being added, and the value to add.
//!
//! Since spans may have arbitrarily long lifetimes, passing the subscriber a
//! `&dyn AsValue` isn't sufficient. Instead, if a subscriber wishes to persist a
//! span value for the entire lifetime of the span, it needs the ability to
//! convert the value into a form in which it is owned by the _subscriber_,
//! rather than the scope in which it was added to the span. For this reason,
//! span values are passed as `&dyn IntoValue`. The `IntoValue` trait is an
//! extension of the `Value` trait that allows conversion into an `OwnedValue`,
//! a type which represents an owned value allocated on the heap. Since some
//! subscriber implementations may _not_ need to persist span field values
//! indefinitely, they are not heap-allocated by default, to avoid unnecessary
//! allocations, but the `IntoValue` trait presents `Subscriber`s with the
//! _option_ to box values should they need to do so.
use super::Meta;
use std::{any::TypeId, borrow::Borrow, fmt};

/// A formattable field value of an erased type.
pub trait Value: fmt::Debug + Send + Sync {
    /// Returns true if the boxed type is the same as `T`
    fn is<T: AsValue + 'static>(&self) -> bool
    where
        Self: 'static;

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    fn downcast_ref<T: AsValue + 'static>(&self) -> Option<&T>
    where
        Self: 'static;
}

/// Trait implemented by types which may be converted into a `Value`.
///
/// Implementors of `AsValue` must provide an implementation of the `fmt_value`
/// function, which describes how to format a value of this type.
pub trait AsValue: Send + Sync {
    /// Formats the value with the given formatter.
    fn fmt_value(&self, f: &mut fmt::Formatter) -> fmt::Result;

    #[doc(hidden)]
    fn type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}

/// Trait representing a type which may be converted into an `OwnedValue`.
///
/// References to types implementing `IntoValue` may be formatted (as `Value`s),
/// _or_ may be converted into owned `OwnedValue`s. In addition to being owned,
/// instances of `OwnedValue` may also be downcast to their original erased type.
pub trait IntoValue: AsValue {
    /// Converts this type into an `OwnedValue`.
    fn into_value(&self) -> OwnedValue;
}

/// An opaque key allowing _O_(1) access to a field in a `Span` or `Event`'s
/// key-value data.
///
/// As keys are defined by the _metadata_ of a span or event, rather than by an
/// individual instance of a span or event, a key may be used to access the same
/// field across all instances of a given span or event with the same metadata.
/// Thus, when a subscriber observes a new span or event, it need only access a
/// field by name _once_, and use the key for that name for all other accesses.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Key<'a> {
    i: usize,
    metadata: &'a Meta<'a>,
}

/// A borrowed value of an erased type.
///
/// Like `Any`,`BorrowedValue`s may attempt to downcast the value to
/// a concrete type. However, unlike `Any`, `BorrowedValue`s are constructed
/// from types known to implement `AsValue`, providing the `fmt_value` method.
/// This means that arbitrary `BorrowedValue`s may be formatted using the erased
/// type's `fmt_value` implementation, _even when the erased type is no longer
/// known_.
pub struct BorrowedValue<'a>(&'a dyn AsValue);

/// A `Value` which is formatted using `fmt::Display` rather than `fmt::Debug`.
#[derive(Clone)]
pub struct DisplayValue<T: fmt::Display>(T);

/// An owned value of an erased type.
///
/// Like `Any`, references to `OwnedValue` may attempt to downcast the value to
/// a concrete type. However, unlike `Any`, `OwnedValue`s are constructed from
/// types known to implement `AsValue`, providing the `fmt_value` method.
/// This means that arbitrary `OwnedValue`s may be formatted using the erased
/// type's `fmt_value` implementation, _even when the erased type is no longer
/// known_.
pub struct OwnedValue(Box<dyn AsValue>);

/// Converts a reference to a `T` into a `BorrowedValue`.
pub fn borrowed<'a>(t: &'a dyn AsValue) -> BorrowedValue<'a> {
    BorrowedValue(t)
}

/// Wraps a type implementing `fmt::Display` so that its `Display`
/// implementation will be used when formatting it as a `Value`.
///
/// # Examples
/// ```
/// # extern crate tokio_trace_core as tokio_trace;
/// use tokio_trace::field;
/// # use std::fmt;
/// # fn main() {
///
/// #[derive(Clone, Debug)]
/// struct Foo;
///
/// impl fmt::Display for Foo {
///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
///         f.pad("Hello, I'm Foo")
///     }
/// }
///
/// let foo = Foo;
/// assert_eq!("Foo".to_owned(), format!("{:?}", foo));
///
/// let display_foo = field::display(foo.clone());
/// assert_eq!(
///     format!("{}", foo),
///     format!("{:?}", field::borrowed(&display_foo)),
/// );
/// # }
/// ```
///
/// ```
/// # extern crate tokio_trace_core as tokio_trace;
/// # use std::fmt;
/// # fn main() {
/// #
/// # #[derive(Clone, Debug)]
/// # struct Foo;
/// #
/// # impl fmt::Display for Foo {
/// #   fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
/// #       f.pad("Hello, I'm Foo")
/// #   }
/// # }
/// use tokio_trace::field::{self, Value, IntoValue};
/// let foo = field::display(Foo);
///
/// let owned_value = foo.into_value();
/// assert_eq!("Hello, I'm Foo".to_owned(), format!("{:?}", owned_value));
///
/// assert!(owned_value.downcast_ref::<Foo>().is_some());
/// # }
/// ```
pub fn display<T>(t: T) -> DisplayValue<T>
where
    T: AsValue + fmt::Display,
{
    DisplayValue(t)
}

// ===== impl AsValue =====

// Copied from `std::any::Any`.
impl AsValue + 'static {
    /// Returns true if the boxed type is the same as `T`
    #[inline]
    fn is<T: AsValue + 'static>(&self) -> bool
    where
        Self: 'static,
    {
        // Get TypeId of the type this function is instantiated with
        let t = TypeId::of::<T>();

        // Get TypeId of the type in the trait object
        let boxed = self.type_id();

        // Compare both TypeIds on equality
        t == boxed
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    fn downcast_ref<T: AsValue + 'static>(&self) -> Option<&T>
    where
        Self: 'static,
    {
        if self.is::<T>() {
            unsafe { Some(&*(self as *const AsValue as *const T)) }
        } else {
            None
        }
    }
}

impl<T> AsValue for T
where
    T: fmt::Debug + Send + Sync,
{
    fn fmt_value(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// ===== impl IntoValue =====

impl<T, V> IntoValue for T
where
    T: ToOwned<Owned = V> + AsValue,
    V: Borrow<T> + AsValue + 'static,
{
    fn into_value(&self) -> OwnedValue {
        OwnedValue(Box::new(self.to_owned()))
    }
}

// ===== impl BorrowedValue =====

impl<'a> fmt::Debug for BorrowedValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt_value(f)
    }
}

impl<'a> Value for BorrowedValue<'a> {
    #[inline]
    fn downcast_ref<T: AsValue + 'static>(&self) -> Option<&T>
    where
        Self: 'static,
    {
        self.0.downcast_ref::<T>()
    }

    fn is<T: AsValue + 'static>(&self) -> bool
    where
        Self: 'static,
    {
        self.0.is::<T>()
    }
}

// ===== impl DisplayValue =====

impl<T: AsValue + fmt::Display> AsValue for DisplayValue<T> {
    fn fmt_value(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }

    #[doc(hidden)]
    fn type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        self.0.type_id()
    }
}

// ===== impl OwnedValue =====

impl Value for OwnedValue {
    #[inline]
    fn downcast_ref<T: AsValue + 'static>(&self) -> Option<&T>
    where
        Self: 'static,
    {
        self.0.as_ref().downcast_ref::<T>()
    }

    #[inline]
    fn is<T: AsValue + 'static>(&self) -> bool
    where
        Self: 'static,
    {
        self.0.as_ref().is::<T>()
    }
}

impl<'a> Value for &'a OwnedValue {
    #[inline]
    fn downcast_ref<T: AsValue + 'static>(&self) -> Option<&T>
    where
        Self: 'static,
    {
        (*self).downcast_ref()
    }

    #[inline]
    fn is<T: AsValue + 'static>(&self) -> bool
    where
        Self: 'static,
    {
        (*self).is::<T>()
    }
}

impl fmt::Debug for OwnedValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt_value(f)
    }
}

// ===== impl Field =====

impl<'a> Key<'a> {
    pub(crate) fn new(i: usize, metadata: &'a Meta<'a>) -> Self {
        Self { i, metadata }
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.i
    }

    pub(crate) fn metadata(&self) -> &Meta<'a> {
        self.metadata
    }

    /// Returns a string representing the name of the field, or `None` if the
    /// field does not exist.
    pub fn name(&self) -> Option<&'a str> {
        self.metadata.field_names.get(self.i).map(|&n| n)
    }

    /// If `self` indexes the given `metadata`, returns a new key into that
    /// metadata. Otherwise, returns `None`.
    ///
    /// This is essentially just a trick to tell the compiler that the lifetine
    /// parameters of two references to a metadata are equal if they are the
    /// same metadata (which can't be inferred when dealing with metadata with
    /// generic lifetimes).
    #[inline]
    pub fn with_metadata<'b>(&self, metadata: &'b Meta<'b>) -> Option<Key<'b>> {
        if self.metadata == metadata {
            Some(Key {
                i: self.i,
                metadata,
            })
        } else {
            None
        }
    }
}

impl<'a> fmt::Display for Key<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.name().unwrap_or("???"))
    }
}

impl<'a> AsRef<str> for Key<'a> {
    fn as_ref(&self) -> &str {
        self.name().unwrap_or("???")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct Foo {
        bar: &'static str,
    }

    impl fmt::Display for Foo {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "hello, I'm {}", self.bar)
        }
    }

    #[test]
    fn display_value_formats_with_display() {
        let foo = Foo { bar: "foo" };
        let display_foo = display(foo.clone());

        assert_eq!(
            format!("{:?}", BorrowedValue(&foo)),
            "Foo { bar: \"foo\" }".to_owned()
        );
        assert_eq!(
            format!("{:?}", BorrowedValue(&display_foo)),
            format!("{}", foo)
        );
    }

    #[test]
    fn display_value_is_into_value() {
        let foo = Foo { bar: "foo" };
        let display_foo = display(foo.clone());

        let owned_value: OwnedValue = display_foo.into_value();
        assert_eq!(format!("{:?}", owned_value), format!("{}", foo));
    }

    #[test]
    fn display_value_downcasts_to_original_type() {
        let foo = Foo { bar: "foo" };
        let display_foo = display(foo);

        let owned_value: OwnedValue = display_foo.into_value();
        assert!(owned_value.downcast_ref::<Foo>().is_some());
    }

    #[test]
    fn owned_value_downcasts_to_original_type() {
        let foo = Foo { bar: "foo" };

        let owned_value: OwnedValue = foo.into_value();
        assert!(owned_value.downcast_ref::<Foo>().is_some());
    }
}
