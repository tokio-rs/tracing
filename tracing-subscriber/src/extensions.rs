use crate::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hasher;
use std::marker::PhantomData;

#[derive(Debug, Default)]
pub struct Directory {
    known: HashMap<TypeId, usize, IdHasher>,
}

pub struct Key<T> {
    index: usize,
    ty: PhantomData<fn(T)>,
}

pub struct Extensions {
    values: Vec<ExtensionValue>,
}

type ExtensionValue = RwLock<Option<Box<dyn Any + Send + Sync>>>;

pub struct Extension<'a, T> {
    // XXX(eliza): when `parking_lot` is enabled, this should just use a
    // `MappedRwLockReadGuard`.
    guard: RwLockReadGuard<'a, Option<Box<dyn Any + Send + Sync>>>,
    _ty: PhantomData<T>,
}

pub struct ExtensionMut<'a, T> {
    // XXX(eliza): when `parking_lot` is enabled, this should just use a
    // `MappedRwLockReadGuard`.
    guard: RwLockWriteGuard<'a, Option<Box<dyn Any + Send + Sync>>>,
    _ty: PhantomData<T>,
}

/// With TypeIds as keys, there's no need to hash them. They are already hashes
/// themselves, coming from the compiler. The IdHasher holds the u64 of
/// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Default, Debug)]
struct IdHasher(u64);

impl Directory {
    pub fn register<T: Any + Send + Sync>(&mut self) -> Key<T> {
        let index = self
            .known
            .entry(TypeId::of::<T>())
            .or_insert(self.known.len() + 1);
        Key {
            index,
            _ty: PhantomData,
        }
    }

    pub fn extensions(&self) -> Extensions {
        Extensions {
            values: (0..self.len()).map(|_| RwLock::new(None)).collect(),
        }
    }
}

impl Extensions {
    pub fn get<'a, T>(&'a self, key: &Key<T>) -> Extension<'a, T> {
        let guard = self.values[key.index].read().unwrap();
        Extension { guard, _ty }
    }
}

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
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
