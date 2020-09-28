//! Callsites represent the source locations from which spans or events
//! originate.
use crate::stdlib::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Mutex,
    },
    vec::Vec,
};
use crate::{
    dispatcher::{self, Dispatch},
    metadata::{LevelFilter, Metadata},
    subscriber::Interest,
};

lazy_static! {
    static ref REGISTRY: Registry = Registry {
        callsites: LinkedList::new(),
        dispatchers: Mutex::new(Vec::new()),
    };
}

struct Registry {
    callsites: LinkedList,
    dispatchers: Mutex<Vec<dispatcher::Registrar>>,
}

impl Registry {
    fn rebuild_callsite_interest(&self, callsite: &'static dyn Callsite) {
        let meta = callsite.metadata();

        // Iterate over the subscribers in the registry, and — if they are
        // active — register the callsite with them.
        let lock = self.dispatchers.lock().unwrap();
        let mut interests = lock
            .iter()
            .filter_map(|registrar| registrar.try_register(meta));

        // Use the first subscriber's `Interest` as the base value.
        let interest = if let Some(interest) = interests.next() {
            // Combine all remaining `Interest`s.
            interests.fold(interest, Interest::and)
        } else {
            // If nobody was interested in this thing, just return `never`.
            Interest::never()
        };

        callsite.set_interest(interest)
    }

    fn rebuild_interest(&self) {
        let mut max_level = LevelFilter::OFF;
        self.dispatchers.lock().unwrap().retain(|registrar| {
            if let Some(dispatch) = registrar.upgrade() {
                // If the subscriber did not provide a max level hint, assume
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

        self.callsites
            .for_each(|cs| self.rebuild_callsite_interest(cs));

        LevelFilter::set_max(max_level);
    }
}

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

    /// Retrive the callsites intrusive registration.
    fn registration(&'static self) -> &'static Registration;
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

/// A registration within the callsite cache.
///
/// Every callsite implementation must store this type internally to the
/// callsite and provide a `&'static Registration` reference via the
/// `Callsite` trait.
pub struct Registration<T = &'static dyn Callsite> {
    callsite: T,
    next: AtomicPtr<Registration<T>>,
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
    REGISTRY.rebuild_interest();
}

/// Register a new `Callsite` with the global registry.
///
/// This should be called once per callsite after the callsite has been
/// constructed.
pub fn register(callsite: &'static dyn Callsite) {
    REGISTRY.rebuild_callsite_interest(callsite);
    REGISTRY.callsites.push(callsite);
}

pub(crate) fn register_dispatch(dispatch: &Dispatch) {
    REGISTRY
        .dispatchers
        .lock()
        .unwrap()
        .push(dispatch.registrar());
    REGISTRY.rebuild_interest();
}

// ===== impl Identifier =====

impl PartialEq for Identifier {
    fn eq(&self, other: &Identifier) -> bool {
        self.0 as *const _ as *const () == other.0 as *const _ as *const ()
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
        f.debug_struct("Registration").finish()
    }
}

// ===== impl LinkedList =====

/// An intrusive atomic push only linked-list.
struct LinkedList {
    head: AtomicPtr<Registration>,
}

impl LinkedList {
    fn new() -> Self {
        LinkedList {
            head: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn for_each(&self, mut f: impl FnMut(&'static dyn Callsite)) {
        let mut head = self.head.load(Ordering::Acquire);

        while !head.is_null() {
            let reg = unsafe { &*head };
            f(reg.callsite);

            head = reg.next.load(Ordering::Acquire);
        }
    }

    fn push(&self, cs: &'static dyn Callsite) {
        let mut head = self.head.load(Ordering::Acquire);

        loop {
            let registration = cs.registration() as *const _ as *mut _;

            cs.registration().next.store(head, Ordering::Release);

            match self.head.compare_exchange(
                head,
                registration,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(old) => {
                    assert_ne!(
                        cs.registration() as *const _,
                        old,
                        "Attempting to push a `Callsite` that already exists. \
                        This will cause an infinite loop when attempting to read from the \
                        callsite cache. This is likely a bug! You should only need to push a \
                        `Callsite` once."
                    );
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

    #[derive(Eq, PartialEq)]
    struct Cs1;
    static CS1: Cs1 = Cs1;
    static REG1: Registration = Registration::new(&CS1);

    impl Callsite for Cs1 {
        fn set_interest(&self, _interest: Interest) {}
        fn metadata(&self) -> &Metadata<'_> {
            todo!()
        }

        fn registration(&'static self) -> &'static Registration {
            &REG1
        }
    }

    struct Cs2;
    static CS2: Cs2 = Cs2;
    static REG2: Registration = Registration::new(&CS2);

    impl Callsite for Cs2 {
        fn set_interest(&self, _interest: Interest) {}
        fn metadata(&self) -> &Metadata<'_> {
            todo!()
        }

        fn registration(&'static self) -> &'static Registration {
            &REG2
        }
    }

    #[test]
    fn linked_list_push() {
        let linked_list = LinkedList::new();

        linked_list.push(&CS1);
        linked_list.push(&CS2);

        let mut i = 0;

        linked_list.for_each(|cs| {
            if i == 0 {
                assert!(
                    ptr::eq(cs.registration(), &REG2),
                    "Registration pointers need to match REG2"
                );
            } else {
                assert!(
                    ptr::eq(cs.registration(), &REG1),
                    "Registration pointers need to match REG1"
                );
            }

            i += 1;
        });
    }

    #[test]
    #[should_panic]
    fn linked_list_repeated() {
        let linked_list = LinkedList::new();

        linked_list.push(&CS1);
        linked_list.push(&CS1);

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
