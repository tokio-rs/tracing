#![allow(missing_docs)]
use super::{Field, FieldSet};
use std::fmt;

/// A map of values indexed by `Field`s.
pub struct FieldMap<T> {
    fields: FieldSet,
    values: [Option<T>; 32],
}

#[derive(Debug)]
pub struct Iter<'a, T> {
    keys: super::Iter,
    map: &'a [Option<T>],
}

#[derive(Debug)]
pub struct Values<'a, T> {
    keys: super::Iter,
    map: &'a [Option<T>],
}

// ===== impl FieldMap =====

impl<T> FieldMap<T> {
    pub fn new(set: &FieldSet) -> Self {
        Self {
            fields: set.duplicate(),
            values: Default::default(),
        }
    }

    #[inline]
    pub fn can_contain(&self, key: &Field) -> bool {
        self.fields.contains(key)
    }

    #[inline]
    pub fn contains(&self, key: &Field) -> bool {
        self.can_contain(key) && self.values[key.i].is_some()
    }

    pub fn get(&self, key: &Field) -> Option<&T> {
        if !self.can_contain(key) {
            return None;
        }
        self.values[key.i].as_ref()
    }

    pub fn get_mut(&mut self, key: &Field) -> Option<&mut T> {
        if !self.can_contain(key) {
            return None;
        }
        self.values[key.i].as_mut()
    }

    pub fn insert(&mut self, key: &Field, value: T) -> Option<T> {
        if !self.can_contain(key) {
            return None;
        }
        std::mem::replace(&mut self.values[key.i], Some(value))
    }

    pub fn remove(&mut self, key: &Field) -> Option<T> {
        if !self.can_contain(key) {
            return None;
        }
        self.values[key.i].take()
    }

    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter {
            keys: self.keys(),
            map: &self.values[..],
        }
    }

    #[inline]
    pub fn keys(&self) -> super::Iter {
        self.fields.iter()
    }

    #[inline]
    pub fn values(&self) -> Values<T> {
        Values {
            keys: self.keys(),
            map: &self.values[..],
        }
    }
}

impl<T: Clone> Clone for FieldMap<T> {
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.duplicate(),
            values: self.values.clone(),
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for FieldMap<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map()
            .entries(self.iter().map(|(k, v)| (super::display(k), v)))
            .finish()
    }
}

impl<'a, T> IntoIterator for &'a FieldMap<T> {
    type Item = (Field, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ===== iterators =====

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Field, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let k = self.keys.next()?;
            if let Some(ref v) = self.map[k.i] {
                return Some((k, v));
            }
        }
    }

    // TODO: size hint?
}

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let k = self.keys.next()?;
            if let Some(ref v) = self.map[k.i] {
                return Some(v);
            }
        }
    }

    // TODO: size hint?
}
