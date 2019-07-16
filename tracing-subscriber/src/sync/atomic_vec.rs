use super::atomic::{self, AtomicUsize, Ordering, AtomicPtr};

use std::{ptr, cell::UnsafeCell};

pub struct AtomicVec<T> {
    head: Box<Block<T>>,
    tail: AtomicPtr<Block<T>>,
    len: AtomicUsize,
}

struct Block<T> {
    next_block: AtomicPtr<Block<T>>,
    push_idx: AtomicUsize,
    last_idx: AtomicUsize,
    block: Box<[Slot<T>]>
}

type Slot<T> = UnsafeCell<Option<T>>;

impl<T> AtomicVec<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let block = Block::with_capacity(capacity);
        let tail = AtomicPtr::new(block);
        let head = unsafe {
            // this is safe; we just constructed this box...
            Box::from_raw(block)
        };
        Self {
            head,
            tail,
            len: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, elem: T) -> usize {
        let mut elem = Some(elem);
        loop {
            let tail = self.tail.load(Ordering::Acquire);
            debug_assert!(!tail.is_null(), "invariant violated: tail should never be null");

            let block = unsafe { &*tail };
            if block.try_push(&mut elem) {
                return self.len.fetch_add(1, Ordering::AcqRel);
            }

            if let Some(new_block) = block.try_cons(tail, &self.tail) {
                if new_block.try_push(&mut elem) {
                    return self.len.fetch_add(1, Ordering::AcqRel);
                }
            }

            atomic::spin_loop_hint()
        }
    }

    pub fn insert_at(&self, elem: )

    #[inline]
    pub fn get(&self, i: usize) -> Option<&T> {
        unsafe {
            self.get_raw(i)?.as_ref()
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    pub unsafe fn get_raw(&self, mut i: usize) -> Option<*mut T> {
        if i < self.len() {
            return None
        }

        let mut curr = self.head.as_ref();
        loop {
            let len = curr.len();
            if i < len {
                // The slot's index is higher than this block's length, try the next
                // block if one exists.
                curr = curr.next()?;
                i -= len;
            } else {
                // Found the block with that slot --- see if it's filled?
                return curr.get_raw(i);
            }

        }
    }

}

impl<T> Block<T> {
    #[inline]
    fn next(&self) -> Option<&Self> {
        unsafe {
            self.next_block.load(Ordering::Acquire).as_ref()
        }
    }

    #[inline]
    unsafe fn get_raw(&self, i: usize) -> Option<*mut T> {
        let slot = &*(self.block[i].get());
        slot.as_ref().map(|slot| slot as *const T as *mut T)
    }

    fn with_capacity(capacity: usize) -> *mut Self {
        let mut block = Vec::with_capacity(capacity);
        for i in 0..capacity {
            block[i] = Slot::default();
        }
        let block = block.into_boxed_slice();
        let block = Block {
            next_block: AtomicPtr::new(ptr::null_mut()),
            push_idx: AtomicUsize::new(0),
            last_idx: AtomicUsize::new(0),
            block,
        };
        Box::into_raw(Box::new(block))
    }

    fn try_push(&self, elem: &mut Option<T>) -> bool {
        let i = self.push_idx.fetch_add(1, Ordering::AcqRel);

        if i >= self.block.len() {
            // We've reached the end of the block; time to push a new block.
            return false;
        }

        let slot = unsafe { &mut *(self.block[i].get()) };
        debug_assert!(slot.is_none(),
            "invariant violated: tried to overwrite existing slot ({:?})",
            i
        );

        let elem = elem.take();
        debug_assert!(elem.is_some(), "invariant violated: tried to push nothing");
        *slot = elem;

        self.last_idx.fetch_add(1, Ordering::Release);
        true
    }

    #[cold]
    fn try_cons(&self, prev: *mut Self, tail: &AtomicPtr<Self>) -> Option<&Self> {
        let next = self.next_block.load(Ordering::Acquire);

        let mut block = if !next.is_null() {
            // Someone else has already pushed a new block, we're done.
            next
        } else {
            Block::with_capacity(self.capacity() * 2)
        };

        if tail.compare_and_swap(prev, block, Ordering::AcqRel) == block {
            self.next_block.store(block, Ordering::Release);
            return unsafe { block.as_ref() };
        }

        // Someone beat us to it, and a new block has already been pushed.
        // We need to clean up the block we allocated.
        if !block.is_null() {
            unsafe {
                // This is safe, since we just created that block; it is our
                // *responsibility* to destroy it.
                drop(Box::from_raw(block));
            };
        }
        None
    }

    #[inline]
    fn len(&self) -> usize {
        self.last_idx.load(Ordering::Acquire)
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.block.len()
    }
}
