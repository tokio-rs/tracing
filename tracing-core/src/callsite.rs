//! Callsites represent the source locations from which spans or events
//! originate.
//!
//! # What Are Callsites?
//!
//! Every span or event in `tracing` is associated with a [`Callsite`]. A
//! callsite is a small `static` value that is responsible for the following:
//!
//! * Storing the span or event's [`Metadata`],
//! * Uniquely [identifying](Identifier) the span or event definition,
//! * Caching the collector's [`Interest`][^1] in that span or event, to avoid
//!   re-evaluating filters,
//! * Storing a [`Registration`] that allows the callsite to be part of a global
//!   list of all callsites in the program.
//!
//! # Registering Callsites
//!
//! When a span or event is recorded for the first time, its callsite
//! [`register`]s itself with the global callsite registry. Registering a
//! callsite calls the [`Collect::register_callsite`][`register_callsite`]
//! method with that callsite's [`Metadata`] on every currently active
//! collector. This serves two primary purposes: informing collectors of the
//! callsite's existence, and performing static filtering.
//!
//! ## Callsite Existence
//!
//! If a [`Collect`] implementation wishes to allocate storage for each
//! unique span/event location in the program, or pre-compute some value
//! that will be used to record that span or event in the future, it can
//! do so in its [`register_callsite`] method.
//!
//! ## Performing Static Filtering
//!
//! The [`register_callsite`] method returns an [`Interest`] value,
//! which indicates that the collector either [always] wishes to record
//! that span or event, [sometimes] wishes to record it based on a
//! dynamic filter evaluation, or [never] wishes to record it.
//!
//! When registering a new callsite, the [`Interest`]s returned by every
//! currently active collector are combined, and the result is stored at
//! each callsite. This way, when the span or event occurs in the
//! future, the cached [`Interest`] value can be checked efficiently
//! to determine if the span or event should be recorded, without
//! needing to perform expensive filtering (i.e. calling the
//! [`Collect::enabled`] method every time a span or event occurs).
//!
//! ### Rebuilding Cached Interest
//!
//! When a new [`Dispatch`] is created (i.e. a new collector becomes
//! active), any previously cached [`Interest`] values are re-evaluated
//! for all callsites in the program. This way, if the new collector
//! will enable a callsite that was not previously enabled, the
//! [`Interest`] in that callsite is updated. Similarly, when a
//! collector is dropped, the interest cache is also re-evaluated, so
//! that any callsites enabled only by that collector are disabled.
//!
//! In addition, the [`rebuild_interest_cache`] function in this module can be
//! used to manually invalidate all cached interest and re-register those
//! callsites. This function is useful in situations where a collector's
//! interest can change, but it does so relatively infrequently. The collector
//! may wish for its interest to be cached most of the time, and return
//! [`Interest::always`][always] or [`Interest::never`][never] in its
//! [`register_callsite`] method, so that its [`Collect::enabled`] method
//! doesn't need to be evaluated every time a span or event is recorded.
//! However, when the configuration changes, the collector can call
//! [`rebuild_interest_cache`] to re-evaluate the entire interest cache with its
//! new configuration. This is a relatively costly operation, but if the
//! configuration changes infrequently, it may be more efficient than calling
//! [`Collect::enabled`] frequently.
//!
//! [^1]: Returned by the [`Collect::register_callsite`][`register_callsite`]
//!     method.
//!
//! [`Metadata`]: crate::metadata::Metadata
//! [`Interest`]: crate::collect::Interest
//! [`Collect`]: crate::collect::Collect
//! [`register_callsite`]: crate::collect::Collect::register_callsite
//! [`Collect::enabled`]: crate::collect::Collect::enabled
//! [always]: crate::collect::Interest::always
//! [sometimes]: crate::collect::Interest::sometimes
//! [never]: crate::collect::Interest::never
//! [`Dispatch`]: crate::dispatch::Dispatch
use crate::{
    collect::Interest,
    dispatch::{self, Dispatch},
    metadata::{LevelFilter, Metadata},
};
use core::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

type Callsites = LinkedList;

/// Trait implemented by callsites.
///
/// These functions are only intended to be called by the callsite registry, which
/// correctly handles determining the common interest between all collectors.
///
/// See the [module-level documentation](crate::callsite) for details on
/// callsites.
pub trait Callsite: Sync {
    /// Sets the [`Interest`] for this callsite.
    ///
    /// See the [documentation on callsite interest caching][cache-docs] for
    /// details.
    ///
    /// [`Interest`]: super::collect::Interest
    /// [cache-docs]: crate::callsite#performing-static-filtering
    fn set_interest(&self, interest: Interest);

    /// Returns the [metadata] associated with the callsite.
    ///
    /// [metadata]: super::metadata::Metadata
    fn metadata(&self) -> &Metadata<'_>;
}

/// Uniquely identifies a [`Callsite`]
///
/// Two `Identifier`s are equal if they both refer to the same callsite.
///
/// [`Callsite`]: super::callsite::Callsite
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

/// A registration with the callsite registry.
///
/// Every [`Callsite`] implementation must provide a `&'static Registration`
/// when calling [`register`] to add itself to the global callsite registry.
///
/// See [the documentation on callsite registration][registry-docs] for details
/// on how callsites are registered.
///
/// [`Callsite`]: crate::callsite::Callsite
/// [`register`]: crate::callsite::register
/// [registry-docs]: crate::callsite#registering-callsites
pub struct Registration<T = &'static dyn Callsite> {
    callsite: T,
    next: AtomicPtr<Registration<T>>,
}

pub(crate) use self::inner::register_dispatch;
pub use self::inner::{rebuild_interest_cache, register};

#[cfg(feature = "std")]
mod inner {
    use super::*;
    use lazy_static::lazy_static;
    use std::sync::RwLock;
    use std::vec::Vec;

    type Dispatchers = Vec<dispatch::Registrar>;

    struct Registry {
        callsites: Callsites,
        dispatchers: RwLock<Dispatchers>,
    }

    lazy_static! {
        static ref REGISTRY: Registry = Registry {
            callsites: LinkedList::new(),
            dispatchers: RwLock::new(Vec::new()),
        };
    }

    /// Clear and reregister interest on every [`Callsite`]
    ///
    /// This function is intended for runtime reconfiguration of filters on traces
    /// when the filter recalculation is much less frequent than trace events are.
    /// The alternative is to have the [`Collect`] that supports runtime
    /// reconfiguration of filters always return [`Interest::sometimes()`] so that
    /// [`enabled`] is evaluated for every event.
    ///
    /// This function will also re-compute the global maximum level as determined by
    /// the [`max_level_hint`] method. If a [`Collect`]
    /// implementation changes the value returned by its `max_level_hint`
    /// implementation at runtime, then it **must** call this function after that
    /// value changes, in order for the change to be reflected.
    ///
    /// See the [documentation on callsite interest caching][cache-docs] for
    /// additional information on this function's usage.
    ///
    /// [`max_level_hint`]: crate::collect::Collect::max_level_hint
    /// [`Callsite`]: crate::callsite::Callsite
    /// [`enabled`]: crate::collect::Collect::enabled
    /// [`Interest::sometimes()`]: crate::collect::Interest::sometimes
    /// [`Collect`]: crate::collect::Collect
    /// [cache-docs]: crate::callsite#rebuilding-cached-interest
    pub fn rebuild_interest_cache() {
        let mut dispatchers = REGISTRY.dispatchers.write().unwrap();
        let callsites = &REGISTRY.callsites;
        rebuild_interest(callsites, &mut dispatchers);
    }

    /// Register a new [`Callsite`] with the global registry.
    ///
    /// This should be called once per callsite after the callsite has been
    /// constructed.
    ///
    /// See the [documentation on callsite registration][reg-docs] for details
    /// on the global callsite registry.
    ///
    /// [`Callsite`]: crate::callsite::Callsite
    /// [reg-docs]: crate::callsite#registering-callsites
    pub fn register(registration: &'static Registration) {
        let dispatchers = REGISTRY.dispatchers.read().unwrap();
        rebuild_callsite_interest(&dispatchers, registration.callsite);
        REGISTRY.callsites.push(registration);
    }

    pub(crate) fn register_dispatch(dispatch: &Dispatch) {
        let mut dispatchers = REGISTRY.dispatchers.write().unwrap();
        let callsites = &REGISTRY.callsites;

        dispatchers.push(dispatch.registrar());

        rebuild_interest(callsites, &mut dispatchers);
    }

    fn rebuild_callsite_interest(
        dispatchers: &[dispatch::Registrar],
        callsite: &'static dyn Callsite,
    ) {
        let meta = callsite.metadata();

        // Iterate over the collectors in the registry, and — if they are
        // active — register the callsite with them.
        let mut interests = dispatchers.iter().filter_map(|registrar| {
            registrar
                .upgrade()
                .map(|dispatch| dispatch.register_callsite(meta))
        });

        // Use the first collector's `Interest` as the base value.
        let interest = if let Some(interest) = interests.next() {
            // Combine all remaining `Interest`s.
            interests.fold(interest, Interest::and)
        } else {
            // If nobody was interested in this thing, just return `never`.
            Interest::never()
        };

        callsite.set_interest(interest)
    }

    fn rebuild_interest(callsites: &Callsites, dispatchers: &mut Vec<dispatch::Registrar>) {
        let mut max_level = LevelFilter::OFF;
        dispatchers.retain(|registrar| {
            if let Some(dispatch) = registrar.upgrade() {
                // If the collector did not provide a max level hint, assume
                // that it may enable every level.
                let level_hint = dispatch.max_level_hint().unwrap_or(LevelFilter::TRACE);
                if level_hint > max_level {
                    max_level = level_hint;
                }
                true
            } else {
                false
            }
        });

        callsites.for_each(|reg| rebuild_callsite_interest(dispatchers, reg.callsite));

        LevelFilter::set_max(max_level);
    }
}

#[cfg(not(feature = "std"))]
mod inner {
    use super::*;
    static REGISTRY: Callsites = LinkedList::new();

    /// Clear and reregister interest on every [`Callsite`]
    ///
    /// This function is intended for runtime reconfiguration of filters on traces
    /// when the filter recalculation is much less frequent than trace events are.
    /// The alternative is to have the [collector] that supports runtime
    /// reconfiguration of filters always return [`Interest::sometimes()`] so that
    /// [`enabled`] is evaluated for every event.
    ///
    /// This function will also re-compute the global maximum level as determined by
    /// the [`max_level_hint`] method. If a [`Collect`]
    /// implementation changes the value returned by its `max_level_hint`
    /// implementation at runtime, then it **must** call this function after that
    /// value changes, in order for the change to be reflected.
    ///
    /// See the [documentation on callsite interest caching][cache-docs] for
    /// additional information on this function's usage.
    ///
    /// [`max_level_hint`]: crate::collector::Collector::max_level_hint
    /// [`Callsite`]: crate::callsite::Callsite
    /// [`enabled`]: crate::collector::Collector::enabled
    /// [`Interest::sometimes()`]: crate::collect::Interest::sometimes
    /// [collector]: crate::collect::Collect
    /// [`Collect`]: crate::collect::Collect
    /// [cache-docs]: crate::callsite#rebuilding-cached-interest
    pub fn rebuild_interest_cache() {
        register_dispatch(dispatch::get_global());
    }

    /// Register a new [`Callsite`] with the global registry.
    ///
    /// This should be called once per callsite after the callsite has been
    /// constructed.
    ///
    /// See the [documentation on callsite registration][reg-docs] for details
    /// on the global callsite registry.
    ///
    /// [`Callsite`]: crate::callsite::Callsite
    /// [reg-docs]: crate::callsite#registering-callsites
    pub fn register(registration: &'static Registration) {
        rebuild_callsite_interest(dispatch::get_global(), registration.callsite);
        REGISTRY.push(registration);
    }

    pub(crate) fn register_dispatch(dispatcher: &Dispatch) {
        // If the collector did not provide a max level hint, assume
        // that it may enable every level.
        let level_hint = dispatcher.max_level_hint().unwrap_or(LevelFilter::TRACE);

        REGISTRY.for_each(|reg| rebuild_callsite_interest(dispatcher, reg.callsite));

        LevelFilter::set_max(level_hint);
    }

    fn rebuild_callsite_interest(dispatcher: &Dispatch, callsite: &'static dyn Callsite) {
        let meta = callsite.metadata();

        callsite.set_interest(dispatcher.register_callsite(meta))
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

// ===== impl Registration =====

impl<T> Registration<T> {
    /// Construct a new `Registration` from some `&'static dyn Callsite`
    pub const fn new(callsite: T) -> Self {
        Self {
            callsite,
            next: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl fmt::Debug for Registration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registration")
            .field("callsite", &format_args!("{:p}", self.callsite))
            .field(
                "next",
                &format_args!("{:p}", self.next.load(Ordering::Acquire)),
            )
            .finish()
    }
}

// ===== impl LinkedList =====

/// An intrusive atomic push-only linked list.
struct LinkedList<T = &'static dyn Callsite> {
    head: AtomicPtr<Registration<T>>,
}

impl<T> LinkedList<T> {
    const fn new() -> Self {
        LinkedList {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl LinkedList {
    fn for_each(&self, mut f: impl FnMut(&'static Registration)) {
        let mut head = self.head.load(Ordering::Acquire);

        while let Some(reg) = unsafe { head.as_ref() } {
            f(reg);

            head = reg.next.load(Ordering::Acquire);
        }
    }

    fn push(&self, registration: &'static Registration) {
        let mut head = self.head.load(Ordering::Acquire);

        loop {
            registration.next.store(head, Ordering::Release);

            assert_ne!(
                registration as *const _, head,
                "Attempted to register a `Callsite` that already exists! \
                This will cause an infinite loop when attempting to read from the \
                callsite cache. This is likely a bug! You should only need to call \
                `tracing-core::callsite::register` once per `Callsite`."
            );

            match self.head.compare_exchange(
                head,
                registration as *const _ as *mut _,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCallsite;
    static CS1: TestCallsite = TestCallsite;
    static CS2: TestCallsite = TestCallsite;

    impl Callsite for TestCallsite {
        fn set_interest(&self, _interest: Interest) {}
        fn metadata(&self) -> &Metadata<'_> {
            unimplemented!("not needed for this test")
        }
    }

    #[test]
    fn linked_list_push() {
        static REG1: Registration = Registration::new(&CS1);
        static REG2: Registration = Registration::new(&CS2);

        let linked_list = LinkedList::new();

        linked_list.push(&REG1);
        linked_list.push(&REG2);

        let mut i = 0;

        linked_list.for_each(|reg| {
            if i == 0 {
                assert!(
                    ptr::eq(reg, &REG2),
                    "Registration pointers need to match REG2"
                );
            } else {
                assert!(
                    ptr::eq(reg, &REG1),
                    "Registration pointers need to match REG1"
                );
            }

            i += 1;
        });
    }

    #[test]
    fn linked_list_push_several() {
        static REG1: Registration = Registration::new(&CS1);
        static REG2: Registration = Registration::new(&CS2);
        static REG3: Registration = Registration::new(&CS1);
        static REG4: Registration = Registration::new(&CS2);

        let linked_list = LinkedList::new();

        fn expect<'a>(
            callsites: &'a mut impl Iterator<Item = &'static Registration>,
        ) -> impl FnMut(&'static Registration) + 'a {
            move |reg: &'static Registration| {
                let ptr = callsites
                    .next()
                    .expect("list contained more than the expected number of registrations!");

                assert!(
                    ptr::eq(reg, ptr),
                    "Registration pointers need to match ({:?} != {:?})",
                    reg,
                    ptr
                );
            }
        }

        linked_list.push(&REG1);
        linked_list.push(&REG2);
        let regs = [&REG2, &REG1];
        let mut callsites = regs.iter().copied();
        linked_list.for_each(expect(&mut callsites));
        assert!(
            callsites.next().is_none(),
            "some registrations were expected but not present: {:?}",
            callsites
        );

        linked_list.push(&REG3);
        let regs = [&REG3, &REG2, &REG1];
        let mut callsites = regs.iter().copied();
        linked_list.for_each(expect(&mut callsites));
        assert!(
            callsites.next().is_none(),
            "some registrations were expected but not present: {:?}",
            callsites
        );

        linked_list.push(&REG4);
        let regs = [&REG4, &REG3, &REG2, &REG1];
        let mut callsites = regs.iter().copied();
        linked_list.for_each(expect(&mut callsites));
        assert!(
            callsites.next().is_none(),
            "some registrations were expected but not present: {:?}",
            callsites
        );
    }

    #[test]
    #[should_panic]
    fn linked_list_repeated() {
        static REG1: Registration = Registration::new(&CS1);

        let linked_list = LinkedList::new();

        linked_list.push(&REG1);
        // Pass in same reg and we should panic...
        linked_list.push(&REG1);

        linked_list.for_each(|_| {});
    }

    #[test]
    fn linked_list_empty() {
        let linked_list = LinkedList::new();

        linked_list.for_each(|_| {
            panic!("List should be empty");
        });
    }
}
