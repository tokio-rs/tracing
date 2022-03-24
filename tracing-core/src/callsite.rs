//! Callsites represent the source locations from which spans or events
//! originate.
use crate::stdlib::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering},
        Mutex,
    },
    vec::Vec,
};
use crate::{
    dispatcher::Dispatch,
    metadata::{LevelFilter, Metadata},
    subscriber::Interest,
    Once,
};

use self::dispatchers::Dispatchers;

/// Trait implemented by callsites.
///
/// These functions are only intended to be called by the callsite registry, which
/// correctly handles determining the common interest between all subscribers.
pub trait Callsite: Sync {
    /// Sets the [`Interest`] for this callsite.
    ///
    /// [`Interest`]: ../subscriber/struct.Interest.html
    fn set_interest(&self, interest: Interest);

    /// Returns the [metadata] associated with the callsite.
    ///
    /// [metadata]: ../metadata/struct.Metadata.html
    fn metadata(&self) -> &Metadata<'_>;
}

/// Uniquely identifies a [`Callsite`]
///
/// Two `Identifier`s are equal if they both refer to the same callsite.
///
/// [`Callsite`]: ../callsite/trait.Callsite.html
#[derive(Clone)]
pub struct Identifier(
    /// **Warning**: The fields on this type are currently `pub` because it must
    /// be able to be constructed statically by macros. However, when `const
    /// fn`s are available on stable Rust, this will no longer be necessary.
    /// Thus, these fields are *not* considered stable public API, and they may
    /// change warning. Do not rely on any fields on `Identifier`. When
    /// constructing new `Identifier`s, use the `identify_callsite!` macro or
    /// the `Callsite::id` function instead.
    // TODO: When `Callsite::id` is a const fn, this need no longer be `pub`.
    #[doc(hidden)]
    pub &'static dyn Callsite,
);

/// A default [`Callsite`] implementation.
#[derive(Debug)]
pub struct DefaultCallsite {
    interest: AtomicUsize,
    meta: &'static Metadata<'static>,
    next: AtomicPtr<Self>,
    registration: Once,
}

/// Clear and reregister interest on every [`Callsite`]
///
/// This function is intended for runtime reconfiguration of filters on traces
/// when the filter recalculation is much less frequent than trace events are.
/// The alternative is to have the [`Subscriber`] that supports runtime
/// reconfiguration of filters always return [`Interest::sometimes()`] so that
/// [`enabled`] is evaluated for every event.
///
/// This function will also re-compute the global maximum level as determined by
/// the [`max_level_hint`] method. If a [`Subscriber`]
/// implementation changes the value returned by its `max_level_hint`
/// implementation at runtime, then it **must** call this function after that
/// value changes, in order for the change to be reflected.
///
/// [`max_level_hint`]: ../subscriber/trait.Subscriber.html#method.max_level_hint
/// [`Callsite`]: ../callsite/trait.Callsite.html
/// [`enabled`]: ../subscriber/trait.Subscriber.html#tymethod.enabled
/// [`Interest::sometimes()`]: ../subscriber/struct.Interest.html#method.sometimes
/// [`Subscriber`]: ../subscriber/trait.Subscriber.html
pub fn rebuild_interest_cache() {
    CALLSITES.rebuild_interest(DISPATCHERS.rebuilder());
}

/// Register a new `Callsite` with the global registry.
///
/// This should be called once per callsite after the callsite has been
/// constructed.
pub fn register(callsite: &'static dyn Callsite) {
    rebuild_callsite_interest(callsite, &DISPATCHERS.rebuilder());

    // TODO(eliza): if we wanted to be *really* cute we could also add some kind
    // of downcasting to `Callsite` and use `push_default` here, too, if we are
    // registering a default callsite...
    CALLSITES.push_dyn(callsite);
}

crate::lazy_static! {
    static ref CALLSITES: Callsites = Callsites {
        list_head: AtomicPtr::new(ptr::null_mut()),
        has_locked_callsites: AtomicBool::new(false),
        locked_callsites: Mutex::new(Vec::new()),
    };

    static ref DISPATCHERS: Dispatchers = Dispatchers::new();
}

struct Callsites {
    list_head: AtomicPtr<DefaultCallsite>,
    has_locked_callsites: AtomicBool,
    locked_callsites: Mutex<Vec<&'static dyn Callsite>>,
}

// === impl DefaultCallsite ===

impl DefaultCallsite {
    /// Returns a new `DefaultCallsite` with the specified `Metadata`.
    pub const fn new(meta: &'static Metadata<'static>) -> Self {
        Self {
            interest: AtomicUsize::new(0xDEADFACED),
            meta,
            next: AtomicPtr::new(ptr::null_mut()),
            registration: Once::new(),
        }
    }

    /// Registers this callsite with the global callsite registry.
    ///
    /// If the callsite is already registered, this does nothing. When using
    /// [`DefaultCallsite`], this method should be preferred over
    /// [`tracing_core::callsite::register`].
    #[inline(never)]
    // This only happens once (or if the cached interest value was corrupted).
    #[cold]
    pub fn register(&'static self) -> Interest {
        self.registration.call_once(|| {
            rebuild_callsite_interest(self, &DISPATCHERS.rebuilder());
            CALLSITES.push_default(self);
        });
        match self.interest.load(Ordering::Relaxed) {
            0 => Interest::never(),
            2 => Interest::always(),
            _ => Interest::sometimes(),
        }
    }

    /// Returns the callsite's cached `Interest`, or registers it for the
    /// first time if it has not yet been registered.
    #[inline]
    pub fn interest(&'static self) -> Interest {
        match self.interest.load(Ordering::Relaxed) {
            0 => Interest::never(),
            1 => Interest::sometimes(),
            2 => Interest::always(),
            _ => self.register(),
        }
    }
}

impl Callsite for DefaultCallsite {
    fn set_interest(&self, interest: Interest) {
        let interest = match () {
            _ if interest.is_never() => 0,
            _ if interest.is_always() => 2,
            _ => 1,
        };
        self.interest.store(interest, Ordering::SeqCst);
    }

    #[inline(always)]
    fn metadata(&self) -> &Metadata<'static> {
        self.meta
    }
}

// ===== impl Identifier =====

impl PartialEq for Identifier {
    fn eq(&self, other: &Identifier) -> bool {
        core::ptr::eq(
            self.0 as *const _ as *const (),
            other.0 as *const _ as *const (),
        )
    }
}

impl Eq for Identifier {}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identifier({:p})", self.0)
    }
}

impl Hash for Identifier {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        (self.0 as *const dyn Callsite).hash(state)
    }
}

// === impl Callsites ===

impl Callsites {
    /// Rebuild `Interest`s for all callsites in the registry.
    ///
    /// This also re-computes the max level hint.
    fn rebuild_interest(&self, dispatchers: dispatchers::Rebuilder<'_>) {
        let mut max_level = LevelFilter::OFF;
        dispatchers.for_each(|dispatch| {
            // If the subscriber did not provide a max level hint, assume
            // that it may enable every level.
            let level_hint = dispatch.max_level_hint().unwrap_or(LevelFilter::TRACE);
            if level_hint > max_level {
                max_level = level_hint;
            }
        });

        self.for_each(|callsite| {
            rebuild_callsite_interest(callsite, &dispatchers);
        });
        LevelFilter::set_max(max_level);
    }

    /// Push a `dyn Callsite` trait object to the callsite registry.
    ///
    /// This will attempt to lock the callsites vector.
    fn push_dyn(&self, callsite: &'static dyn Callsite) {
        let mut lock = self.locked_callsites.lock().unwrap();
        self.has_locked_callsites.store(true, Ordering::Release);
        lock.push(callsite);
    }

    /// Push a `DefaultCallsite` to the callsite registry.
    ///
    /// If we know the callsite being pushed is a `DefaultCallsite`, we can push
    /// it to the linked list without having to acquire a lock.
    fn push_default(&self, callsite: &'static DefaultCallsite) {
        let mut head = self.list_head.load(Ordering::Acquire);

        loop {
            callsite.next.store(head, Ordering::Release);

            assert_ne!(
                callsite as *const _, head,
                "Attempted to register a `DefaultCallsite` that already exists! \
                This will cause an infinite loop when attempting to read from the \
                callsite cache. This is likely a bug! You should only need to call \
                `DefaultCallsite::register` once per `DefaultCallsite`."
            );

            match self.list_head.compare_exchange(
                head,
                callsite as *const _ as *mut _,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    break;
                }
                Err(current) => head = current,
            }
        }
    }

    /// Invokes the provided closure `f` with each callsite in the registry.
    fn for_each(&self, mut f: impl FnMut(&'static dyn Callsite)) {
        let mut head = self.list_head.load(Ordering::Acquire);

        while let Some(cs) = unsafe { head.as_ref() } {
            f(cs);

            head = cs.next.load(Ordering::Acquire);
        }

        if self.has_locked_callsites.load(Ordering::Acquire) {
            let locked = self.locked_callsites.lock().unwrap();
            for &cs in locked.iter() {
                f(cs);
            }
        }
    }
}

pub(crate) fn register_dispatch(dispatch: &Dispatch) {
    let dispatchers = DISPATCHERS.register_dispatch(dispatch);
    CALLSITES.rebuild_interest(dispatchers);
}

fn rebuild_callsite_interest(
    callsite: &'static dyn Callsite,
    dispatchers: &dispatchers::Rebuilder<'_>,
) {
    let meta = callsite.metadata();

    let mut interest = None;
    dispatchers.for_each(|dispatch| {
        let this_interest = dispatch.register_callsite(meta);
        interest = match interest.take() {
            None => Some(this_interest),
            Some(that_interest) => Some(that_interest.and(this_interest)),
        }
    });

    let interest = interest.unwrap_or_else(Interest::never);
    callsite.set_interest(interest)
}

#[cfg(feature = "std")]
mod dispatchers {
    use crate::dispatcher;
    use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

    pub(super) struct Dispatchers(RwLock<Vec<dispatcher::Registrar>>);
    pub(super) enum Rebuilder<'a> {
        Read(RwLockReadGuard<'a, Vec<dispatcher::Registrar>>),
        Write(RwLockWriteGuard<'a, Vec<dispatcher::Registrar>>),
    }

    impl Dispatchers {
        pub(super) fn new() -> Self {
            Self(RwLock::new(Vec::new()))
        }

        pub(super) fn rebuilder(&self) -> Rebuilder<'_> {
            Rebuilder::Read(self.0.read().unwrap())
        }

        pub(super) fn register_dispatch(&self, dispatch: &dispatcher::Dispatch) -> Rebuilder<'_> {
            let mut dispatchers = self.0.write().unwrap();
            dispatchers.retain(|d| d.upgrade().is_some());
            dispatchers.push(dispatch.registrar());
            Rebuilder::Write(dispatchers)
        }
    }

    impl Rebuilder<'_> {
        pub(super) fn for_each(&self, mut f: impl FnMut(&dispatcher::Dispatch)) {
            let iter = match self {
                Rebuilder::Read(vec) => vec.iter(),
                Rebuilder::Write(vec) => vec.iter(),
            };
            iter.filter_map(dispatcher::Registrar::upgrade)
                .for_each(|dispatch| f(&dispatch))
        }
    }
}

#[cfg(not(feature = "std"))]
mod dispatchers {
    use crate::dispatcher;
    use core::marker::PhantomData;
    use std::marker::PhantomData;

    pub(super) struct Dispatchers(());
    pub(super) struct Rebuilder<'a>(PhantomData<'a>);

    impl Dispatchers {
        pub(super) fn new() -> Self {
            Self(())
        }

        pub(super) fn rebuilder(&self) -> Rebuilder<'_> {
            Rebuilder(PhantomData)
        }

        pub(super) fn register_dispatch(&self, _: &dispatcher::Dispatch) -> Rebuilder<'_> {
            // nop; on no_std, there can only ever be one dispatcher
            Rebuilder(PhantomData)
        }
    }

    impl Rebuilder<'_> {
        pub(super) fn for_each(&self, mut f: impl FnMut(&dispatcher::Dispatch)) {
            // on no_std, there can only ever be one dispatcher
            dispatcher::get_default(f)
        }
    }
}
