#![allow(missing_docs)]
use super::{Field, FieldSet};
use std::fmt;

/// A map of `Field`s to `T`s, with fast O(1) indexing.
///
/// In many cases, subscriber implementations may wish to store data for each
/// field recorded by a span or event. `FieldMap` provides specialized storage
/// for that use-case, with `Field`s from a `FieldSet` as the key type.
///
/// Unlike other storage, such as `HashMap`, a `FieldMap` does not allocate, and
/// can store all data on the stack or inline in another struct. Like `HashMap`,
/// it has O(1) indexing, but with significantly faster constant-faster
/// performance. However, it may _only_ be indexed by `Field`s.
pub struct FieldMap<T> {
    fields: FieldSet,
    values: [Option<T>; 32],
}

/// Iterator over the key-value pairs in a `FieldMap`.
#[derive(Debug)]
pub struct Iter<'a, T> {
    keys: super::Iter,
    map: &'a [Option<T>],
}

/// Iterator over the values stored in a `FieldMap`.
#[derive(Debug)]
pub struct Values<'a, T> {
    keys: super::Iter,
    map: &'a [Option<T>],
}

// ===== impl FieldMap =====

impl<T> FieldMap<T> {
    /// Returns a new `FieldMap` keyed by the fields in the given `FieldSet`.
    pub fn new(set: &FieldSet) -> Self {
        Self {
            fields: set.duplicate(),
            values: Default::default(),
        }
    }

    /// Returns `true` if the given `Field` is a valid key for this `FieldMap`
    /// (i.e. it came from the same span or event as the `FieldSet` that created
    /// this map).
    #[inline]
    pub fn can_contain(&self, key: &Field) -> bool {
        self.fields.contains(key)
    }

    /// Returns `true` if this `FieldMap` _currently_ contains a value for the
    /// given `Field`.
    #[inline]
    pub fn contains(&self, key: &Field) -> bool {
        self.can_contain(key) && self.values[key.i].is_some()
    }

    /// Borrows the underlying `FieldSet` that created this `FieldMap`.
    #[inline]
    pub fn fields(&self) -> &FieldSet {
        &self.fields
    }

    /// Borrows the value for the given `Field`, if that field is a valid key to
    /// this map and a value currently exists for it.
    pub fn get(&self, key: &Field) -> Option<&T> {
        if !self.can_contain(key) {
            return None;
        }
        self.values[key.i].as_ref()
    }

    /// Mutably borrows the value for the given `Field`, if that field is a
    /// valid key to this map and a value currently exists for it.
    pub fn get_mut(&mut self, key: &Field) -> Option<&mut T> {
        if !self.can_contain(key) {
            return None;
        }
        self.values[key.i].as_mut()
    }

    /// Inserts the given value into the `FieldMap` at the index of the given
    /// `Field`. If there was previously a value present for that key, the old
    /// value is returned.
    ///
    /// **Note** that if the given `Field` does _not_ correspond to the `FieldSet`
    /// that created this map (i.e., if `self.can_contain(&key) == false`), the
    /// value will not be inserted.
    pub fn insert(&mut self, key: &Field, value: T) -> Option<T> {
        if !self.can_contain(key) {
            return None;
        }
        std::mem::replace(&mut self.values[key.i], Some(value))
    }

    /// Removes the value indexed by the given `Field`, if one exists.
    pub fn remove(&mut self, key: &Field) -> Option<T> {
        if !self.can_contain(key) {
            return None;
        }
        self.values[key.i].take()
    }

    /// Returns `true` if no values are currently stored in this `FieldMap`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.iter().all(Option::is_none)
    }

    /// Returns a forward iterator over the key-value pairs in this `FieldMap`.
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter {
            keys: self.keys(),
            map: &self.values[..],
        }
    }

    /// Returns a forward iterator over the `Field` keys in this `FieldMap`.
    #[inline]
    pub fn keys(&self) -> super::Iter {
        self.fields.iter()
    }

    /// Returns a forward iterator over the `Values` in this `FieldMap`.
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

impl<'a, T> From<&'a FieldSet> for FieldMap<T> {
    fn from(set: &'a FieldSet) -> Self {
        Self::new(set)
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::metadata::{Kind, Level, Metadata};

    struct TestCallsite1;
    static TEST_CALLSITE_1: TestCallsite1 = TestCallsite1;
    static TEST_META_1: Metadata<'static> = metadata! {
        name: "field_test1",
        target: module_path!(),
        level: Level::INFO,
        fields: &["foo", "bar", "baz"],
        callsite: &TEST_CALLSITE_1,
        kind: Kind::SPAN,
    };

    impl crate::callsite::Callsite for TestCallsite1 {
        fn set_interest(&self, _: crate::subscriber::Interest) {
            unimplemented!()
        }

        fn metadata(&self) -> &Metadata {
            &TEST_META_1
        }
    }

    struct TestCallsite2;
    static TEST_CALLSITE_2: TestCallsite2 = TestCallsite2;
    static TEST_META_2: Metadata<'static> = metadata! {
        name: "field_test2",
        target: module_path!(),
        level: Level::INFO,
        fields: &["foo", "bar", "baz"],
        callsite: &TEST_CALLSITE_2,
        kind: Kind::SPAN,
    };

    impl crate::callsite::Callsite for TestCallsite2 {
        fn set_interest(&self, _: crate::subscriber::Interest) {
            unimplemented!()
        }

        fn metadata(&self) -> &Metadata {
            &TEST_META_2
        }
    }

    #[test]
    fn can_contain() {
        let fields_1 = TEST_META_1.fields();
        let fields_2 = TEST_META_2.fields();
        let field = fields_1.field("foo").unwrap();

        assert!(FieldMap::<usize>::from(fields_1).can_contain(&field));
        assert_eq!(FieldMap::<usize>::from(fields_2).can_contain(&field), false);

        let field = fields_2.field("baz").unwrap();
        assert!(FieldMap::<usize>::from(fields_2).can_contain(&field));
        assert_eq!(FieldMap::<usize>::from(fields_1).can_contain(&field), false);
    }

    #[test]
    fn contains() {
        let fields_1 = TEST_META_1.fields();
        let foo = fields_1.field("foo").unwrap();
        let bar = fields_1.field("bar").unwrap();
        let baz = fields_1.field("baz").unwrap();

        let mut map: FieldMap<&str> = fields_1.into();
        assert!(map.can_contain(&foo));
        assert!(map.can_contain(&bar));
        assert!(map.can_contain(&baz));

        assert!(map.contains(&foo) == false);
        assert!(map.contains(&bar) == false);
        assert!(map.contains(&baz) == false);

        map.insert(&foo, "hello world");
        assert!(map.can_contain(&foo));

        assert!(map.contains(&foo));
        assert!(map.contains(&bar) == false);
        assert!(map.contains(&baz) == false);

        map.insert(&baz, "hello other world");
        assert!(map.contains(&foo));
        assert!(map.contains(&bar) == false);
        assert!(map.contains(&baz));

        let map2: FieldMap<&str> = fields_1.into();
        assert!(map2.can_contain(&foo));
        assert!(map2.can_contain(&bar));
        assert!(map2.can_contain(&baz));

        assert!(map2.contains(&foo) == false);
        assert!(map2.contains(&bar) == false);
        assert!(map2.contains(&baz) == false);

        let map3: FieldMap<&str> = TEST_META_2.fields().into();
        assert!(map3.can_contain(&foo) == false);
        assert!(map3.can_contain(&bar) == false);
        assert!(map3.can_contain(&baz) == false);

        assert!(map3.contains(&foo) == false);
        assert!(map3.contains(&bar) == false);
        assert!(map3.contains(&baz) == false);
    }

    #[test]
    fn get() {
        let fields_1 = TEST_META_1.fields();
        let foo = fields_1.field("foo").unwrap();
        let bar = fields_1.field("bar").unwrap();
        let baz = fields_1.field("baz").unwrap();

        let mut map: FieldMap<&str> = fields_1.into();
        assert_eq!(map.get(&foo), None);
        assert_eq!(map.get(&bar), None);
        assert_eq!(map.get(&baz), None);

        map.insert(&foo, "hello world");
        assert_eq!(map.get(&foo), Some(&"hello world"));
        assert_eq!(map.get(&bar), None);
        assert_eq!(map.get(&baz), None);

        map.insert(&bar, "hello san francisco!");
        assert_eq!(map.get(&foo), Some(&"hello world"));
        assert_eq!(map.get(&bar), Some(&"hello san francisco!"));
        assert_eq!(map.get(&baz), None);

        let map2: FieldMap<&str> = fields_1.into();
        assert_eq!(map2.get(&foo), None);
        assert_eq!(map2.get(&bar), None);
        assert_eq!(map2.get(&baz), None);

        let map3: FieldMap<&str> = TEST_META_2.fields().into();
        assert_eq!(map3.get(&foo), None);
        assert_eq!(map3.get(&bar), None);
        assert_eq!(map3.get(&baz), None);
    }

    #[test]
    fn insert() {
        let fields_1 = TEST_META_1.fields();
        let foo = fields_1.field("foo").unwrap();

        let mut map1: FieldMap<&str> = TEST_META_1.fields().into();
        map1.insert(&foo, "hello world");
        assert_eq!(map1.get(&foo), Some(&"hello world"));

        let mut map2: FieldMap<&str> = TEST_META_2.fields().into();
        map2.insert(&foo, "hello world");
        assert_eq!(map2.get(&foo), None);
    }

}
