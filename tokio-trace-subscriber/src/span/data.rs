use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::{BuildHasherDefault, Hasher},
};

#[derive(Default, Debug)]
pub struct Data {
    data: HashMap<TypeId, Box<Any>, BuildHasherDefault<IdHasher>>,
}

#[derive(Default, Debug)]
struct IdHasher(u64);

impl Data {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a new data value.
    ///
    /// If data of this type already existed, it will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_trace_subscriber::span::Data;
    /// let mut data = Data::new();
    /// assert!(data.insert(42).is_none());
    /// assert!(data.insert("hello").is_none());
    /// assert_eq!(data.insert(666), Some(42));
    /// ```
    pub fn insert<T: Any>(&mut self, val: T) -> Option<T> {
        self.data
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(|prev| prev.downcast().ok().map(|v| *v))
    }

    /// Get a reference to a previously-inserted data value.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_trace_subscriber::span::Data;
    /// let mut data = Data::new();
    /// assert!(data.get::<i32>().is_none());
    /// data.insert(5i32);
    ///
    /// assert_eq!(data.get::<i32>(), Some(&5i32));
    /// ```
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            //TODO: we can use unsafe and remove double checking the type id
            .and_then(|v| (&**v as &Any).downcast_ref())
    }

    /// Get a mutable reference to a previously-inserted data value.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_trace_subscriber::span::Data;
    /// let mut data = Data::new();
    /// data.insert(String::from("Hello"));
    /// data.get_mut::<String>().unwrap().push_str(" World");
    ///
    /// assert_eq!(data.get::<String>().unwrap(), "Hello World");
    /// ```
    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&TypeId::of::<T>())
            //TODO: we can use unsafe and remove double checking the type id
            .and_then(|v| (&mut **v as &mut Any).downcast_mut())
    }

    /// Remove a value from this `Data`.
    ///
    /// If a extension of this type existed, it will be returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use tokio_trace_subscriber::span::Data;
    /// let mut data = Data::new();
    /// data.insert(5i32);
    /// assert_eq!(data.remove::<i32>(), Some(5i32));
    /// assert!(data.get::<i32>().is_none());
    /// ```
    pub fn remove<T: Any>(&mut self) -> Option<T> {
        self.data.remove(&TypeId::of::<T>()).and_then(|v| {
            //TODO: we can use unsafe and remove double checking the type id
            (v as Box<Any>).downcast().ok().map(|v| *v)
        })
    }

    #[inline]
    pub(crate) fn clear(&mut self) {
        self.data.clear()
    }
}

// ===== impl IdHasher =====

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("tried to hash something other than a TypeId");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
