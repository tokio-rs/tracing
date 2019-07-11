use hashbrown::HashMap;
use tracing_core::span::Id;
use std::{
    mem,
    thread,
    sync::{
        atomic::{AtomicUsize, Ordering},
        PoisonError,
    },
    cell::{RefCell, Cell, Ref, RefMut, UnsafeCell},
    fmt,
};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard, MappedReentrantMutexGuard};
use crossbeam_utils::sync::{ShardedLock, ShardedLockReadGuard};
use owning_ref::OwningHandle;

pub struct Registry<T> {
    shards: ShardedLock<Shards<T>>,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
struct Thread {
    id: usize,
}

struct Shards<T>(HashMap<Thread, Shard<T>>);

struct Shard<T> {
    spans: ReentrantMutex<UnsafeCell<Spans<T>>>,
}

pub struct Span<'a, T> {
    inner: MappedReentrantMutexGuard<'a, Ref<'a, T>>,
}

pub struct SpanMut<'a, T> {
    inner: MappedReentrantMutexGuard<'a, RefMut<'a, T>>,
}

#[derive(Debug)]
enum Error {
    Poisoned,
    BadInsert,
}

type Spans<T> = HashMap<Id, RefCell<T>>;

fn handle_err<T>(result: Result<T, Error>) -> Option<T> {
    match result {
        Ok(e) => Some(e),
        Err(e) if thread::panicking() => {
            println!("{}", e);
            None
        },
        Err(e) =>  panic!("{}", e),
    }
}

impl<T> Registry<T> {
    fn with_local<'a, I>(&'a self, mut f: impl FnOnce(&'a Shard<T>) -> I) -> Result<I, Error> {
        // fast path --- the shard already exists
        let thread = Thread::current();
        let mut f = Some(f);

        if let Some(r) = self.shards.read()?
            .with_shard(&thread, &mut f)
        {
            Ok(r)
        } else {
            // slow path --- need to insert a shard.
            self.with_new_local(&thread, &mut f)
        }
    }

    #[cold]
    fn with_new_local<'a, I>(&'a self, thread: &Thread, f: &mut Option<impl FnOnce(&'a Shard<T>)-> I>,) -> Result<I, Error> {
        self.shards.write()?
            .new_shard_for(thread.clone())
            .with_shard(&thread, &mut f).ok_or(Error::BadInsert)
    }

    pub fn span(&self, id: &Id) -> Option<Span<T>> {
        let local = self.with_local(|shard| { shard.span(id) });
        let local = handle_err(local)?;
        match local {
            Some(slot) => {
                let inner = MappedReentrantMutexGuard::try_map(slot, |slot| slot.as_ref()).ok()?;
                return Some(Span { inner });
            },
            None => {
                // TODO: steal
            }
        }

        None
    }

    pub fn span_mut<'a>(&'a self, id: &Id) -> Option<SpanMut<'a, T>> {
        // let res = self.with_local(|shard| {
        //     shard.span(id).map(|inner| Ref { inner })
        // });
        // handle_err(res)?
        unimplemented!()
    }

    pub fn with_span<I>(&self, id: &Id, f: impl FnOnce(&mut T) -> I) -> Option<I> {
        // let mut f = Some(f);
        // let res = self.with_shard(|shard| {
        //     shard.get_mut(id).and_then(Slot::get_mut).map(|span| {
        //         let mut f = f.take().expect("called twice!");
        //         f(span)
        //     })
        // });
        // handle_poison(res)?

        // TODO: steal
        unimplemented!()
    }

    pub fn insert(&self, id: Id, span: T) -> &Self {
        let ok = self.with_local(move |shard| { shard.insert(id, span) });
        if !thread::panicking() {
            ok.expect("poisoned");
        }

        self
    }

    pub fn new() -> Self {
        Self {
            shards: ShardedLock::new(Shards(HashMap::new()))
        }
    }
}

impl<T> Shards<T> {
    fn with_shard<'a, I>(
        &'a self,
        thread: &Thread,
        f: &mut Option<impl FnOnce(&'a Shard<T>)-> I>,
    ) -> Option<I> {
        let shard = self.0.get(thread)?;
        let mut f = match f.take() {
            None if thread::panicking() => {
                println!("with_shard: closure called twice; this is a bug");
                return None;
            },
            None => panic!("with_shard: closure called twice; this is a bug"),
            Some(f) => f,
        };
        Some(f(&*shard))
    }

    fn new_shard_for(&mut self, thread: Thread) -> &mut Self {
        self.0.insert(thread, Shard::new());
        self
    }
}

impl<T> Shard<T> {
    fn new() -> Self {
        Self {
            spans: ReentrantMutex::new(UnsafeCell::new(HashMap::new()))
        }
    }

    fn insert(&self, id: Id, span: T) -> Option<T> {
        let guard = self.spans.lock();
        let spans = unsafe {
            // this is safe as the mutex ensures that the map is not mutated
            // concurrently, and the mutable ref will not leave this function.
            &mut *(guard.get())
        };
        spans.insert(id, RefCell::new(span))
    }

    // fn spans_mut<'a>(&'a self) -> MappedReentrantMutexGuard<'a, RefMut<'a, Spans<T>>> {
    //     let guard = self.spans.lock();
    //     ReentrantMutexGuard::map(guard, RefCell::borrow_mut)
    // }
    fn span<'a>(&'a self, id: &Id) -> Option<MappedReentrantMutexGuard<'a, RefMut<T>>> {
        let guard = self.spans.lock();
        ReentrantMutexGuard::try_map(
            guard,
            move |spans| {
                let spans = unsafe { &mut *(guard.get()) };
                spans.get_mut(id)
            }
        ).ok()
    }

    fn try_steal(&self, id: &Id) -> Option<Slot<T>> {
        let guard = self.spans.lock();
        let spans = unsafe {
            // this is safe as the mutex ensures that the map is not mutated
            // concurrently, and the mutable ref will not leave this function.
            &mut *(guard.get())
        };
        spans.insert(id.clone(), Slot::Stolen(Thread::current()))
    }
}

impl Thread {
    fn current() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        thread_local! {
            static MY_ID: Cell<Option<usize>> = Cell::new(None);
        }
        MY_ID.with(|my_id| if let Some(id) = my_id.get() {
            Thread {
                id
            }
        } else {
            let id = NEXT.fetch_add(1, Ordering::SeqCst);
            my_id.set(Some(id));
            Thread {
                id
            }
        })
    }
}

impl<T> Slot<T> {

    fn as_ref<'a>(&'a self) -> Option<Ref<'a, T>> {
        match self {
            Slot::Present(ref span) => Some(span.borrow()),
            _ => None,
        }
    }

    fn as_mut<'a>(&'a self) -> Option<RefMut<'a, T>> {
        match self {
            Slot::Present(ref span) => Some(span.borrow_mut()),
            _ => None,
        }
    }

    fn into_option(self) -> Option<T> {
        match self {
            Slot::Present(span) => Some(span.into_inner()),
            _ => None,
        }
    }
}


impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Error::Poisoned
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Poisoned =>
                "registry poisoned: another thread panicked inside".fmt(f),
            Error::BadInsert =>
                "new thread was inserted but did not exist; this is a bug".fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basically_works() {
        let registry: Registry<usize> = Registry::new();
        registry
            .insert(Id::from_u64(1), 1)
            .insert(Id::from_u64(2), 2);

        assert_eq!(registry.span(&Id::from_u64(1)), Some(1));
        assert_eq!(registry.span(&Id::from_u64(2)), Some(2));

        registry.insert(Id::from_u64(3), 3);

        assert_eq!(registry.span(&Id::from_u64(1)), Some(1));
        assert_eq!(registry.span(&Id::from_u64(2)), Some(2));
        assert_eq!(registry.span(&Id::from_u64(3)), Some(3));
    }
}
