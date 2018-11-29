//! `Span` and `Event` key-value data.
//!
//! Spans and events may be annotated with key-value data, referred to as known
//! as _fields_. These fields consist of a mapping from a key (corresponding to
//! a `&str` but represented internally as an array index) to a `Value`.
//!
//! # `Value`s and `Subscriber`s
//!
//! `Subscriber`s consume `Value`s as fields attached to `Event`s or `Span`s.
//! The set of field keys on a given `Span` or `Event` is defined on its
//! `Metadata`. Once the span or event has been created (i.e., the `new_id` or
//! `new_span` methods on the `Subscriber` have been called), field values may
//! be added by calls to the subscriber's `record_` methods.
//!
//! `tokio_trace` represents values as either one of a set of Rust primitives
//! (`i64`, `u64`, `bool`, and `&str`) or using a `fmt::Display` or `fmt::Debug`
//! implementation. The `record_` trait functions on the `Subscriber` trait allow
//! `Subscriber` implementations to provide type-specific behaviour for
//! consuming values of each type.
//!
//! The `Subscriber` trait provides default implementations of `record_u64`,
//! `record_i64`, `record_bool`, and `record_str` which call the `record_fmt`
//! function, so that only the `record_fmt` function must be implemented.
//! However, implementors of `Subscriber` that wish to consume these primitives
//! as their types may override the `record` methods for any types they care
//! about. For example, we might record integers by incrementing counters for
//! their field names, rather than printing them.
//
use callsite::{self, Callsite};
use std::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    ops::Range,
};

/// An opaque key allowing _O_(1) access to a field in a `Span` or `Event`'s
/// key-value data.
///
/// As keys are defined by the _metadata_ of a span or event, rather than by an
/// individual instance of a span or event, a key may be used to access the same
/// field across all instances of a given span or event with the same metadata.
/// Thus, when a subscriber observes a new span or event, it need only access a
/// field by name _once_, and use the key for that name for all other accesses.
#[derive(Debug)]
pub struct Key {
    i: usize,
    fields: Fields,
}

/// Describes the fields present on a span.
// TODO: When `const fn` is stable, make this type's fields private.
pub struct Fields {
    /// The names of each field on the described span.
    ///
    /// **Warning**: The fields on this type are currently `pub` because it must be able
    /// to be constructed statically by macros. However, when `const fn`s are
    /// available on stable Rust, this will no longer be necessary. Thus, these
    /// fields are *not* considered stable public API, and they may change
    /// warning. Do not rely on any fields on `Fields`!
    #[doc(hidden)]
    pub names: &'static [&'static str],
    /// The callsite where the described span originates.
    ///
    /// **Warning**: The fields on this type are currently `pub` because it must be able
    /// to be constructed statically by macros. However, when `const fn`s are
    /// available on stable Rust, this will no longer be necessary. Thus, these
    /// fields are *not* considered stable public API, and they may change
    /// warning. Do not rely on any fields on `Fields`!
    #[doc(hidden)]
    pub callsite: &'static Callsite,
}

/// An iterator over a set of fields.
pub struct Iter {
    idxs: Range<usize>,
    fields: Fields,
}

// ===== impl Key =====

impl Key {
    /// Returns an [`Identifier`](::metadata::Identifier) that uniquely
    /// identifies the callsite that defines the field this key refers to.
    #[inline]
    pub fn id(&self) -> callsite::Identifier {
        self.fields.id()
    }

    /// Return a `usize` representing the index into an array whose indices are
    /// ordered the same as the set of fields that generated this `key`.
    pub fn as_usize(&self) -> usize {
        self.i
    }

    /// Returns a string representing the name of the field, or `None` if the
    /// field does not exist.
    pub fn name(&self) -> Option<&'static str> {
        self.fields.names.get(self.i).map(|&n| n)
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.name().unwrap_or("???"))
    }
}

impl AsRef<str> for Key {
    fn as_ref(&self) -> &str {
        self.name().unwrap_or("???")
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id() && self.i == other.i
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.id().hash(state);
        self.i.hash(state);
    }
}

impl Clone for Key {
    fn clone(&self) -> Self {
        Key {
            i: self.i,
            fields: Fields {
                names: self.fields.names,
                callsite: self.fields.callsite,
            },
        }
    }
}

// ===== impl Fields =====

impl Fields {
    pub(crate) fn id(&self) -> callsite::Identifier {
        self.callsite.id()
    }

    /// Returns a [`Key`](::field::Key) to the field corresponding to `name`, if
    /// one exists, or `None` if no such field exists.
    pub fn key_for<Q>(&self, name: &Q) -> Option<Key>
    where
        Q: Borrow<str>,
    {
        let name = &name.borrow();
        self.names.iter().position(|f| f == name).map(|i| Key {
            i,
            fields: Fields {
                names: self.names,
                callsite: self.callsite,
            },
        })
    }

    /// Returns `true` if `self` contains a field for the given `key`.
    pub fn contains_key(&self, key: &Key) -> bool {
        key.id() == self.id() && key.as_usize() <= self.names.len()
    }

    /// Returns an iterator over the `Key`s to this set of `Fields`.
    pub fn iter(&self) -> Iter {
        let idxs = 0..self.names.len();
        Iter {
            idxs,
            fields: Fields {
                names: self.names,
                callsite: self.callsite,
            },
        }
    }
}

impl<'a> IntoIterator for &'a Fields {
    type IntoIter = Iter;
    type Item = Key;
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl fmt::Debug for Fields {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self).finish()
    }
}

// ===== impl Iter =====

impl Iterator for Iter {
    type Item = Key;
    fn next(&mut self) -> Option<Key> {
        let i = self.idxs.next()?;
        Some(Key {
            i,
            fields: Fields {
                names: self.fields.names,
                callsite: self.fields.callsite,
            },
        })
    }
}
