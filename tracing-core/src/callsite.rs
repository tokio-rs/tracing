//! Callsites represent the source locations from which spans or events
//! originate.
// Types in this module are never formatted.
#![allow(missing_debug_implementations)]
use crate::{
    dispatcher::{self, Dispatch, Registrar},
    subscriber::Interest,
    Metadata,
};

use std::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
    sync::{
        RwLock,
        atomic::{AtomicPtr, Ordering},
    },
};

lazy_static! {
    static ref REGISTRY: RwLock<Registry> = RwLock::new(Registry {
        dispatchers: Vec::new(),
    });
}

static CALLSITES: Callsites = Callsites {
    head: AtomicPtr::new(ptr::null_mut()),
};

struct Registry {
    dispatchers: Vec<dispatcher::Registrar>,
}

struct Callsites {
    head: AtomicPtr<CsEntry>,
}

struct CsEntry {
    next: Option<ptr::NonNull<CsEntry>>,
    callsite: &'static dyn Callsite,
}

struct CsIter<'a> {
    current: Option<&'a CsEntry>,
}

impl Registry {
    fn rebuild_callsite_interest(&self, callsite: &'static dyn Callsite) {
        let meta = callsite.metadata();

        let mut interest = Interest::never();

        for registrar in &self.dispatchers {
            if let Some(sub_interest) = registrar.try_register(meta) {
                interest = interest.and(sub_interest);
            }
        }

        callsite.set_interest(interest)
    }

    fn rebuild_interest(&mut self) {
        self.dispatchers.retain(Registrar::is_alive);

        for callsite in &CALLSITES {
            self.rebuild_callsite_interest(callsite);
        }
    }
}

/// Trait implemented by callsites.
///
/// These functions are only intended to be called by the [`Registry`] which
/// correctly handles determining the common interest between all subscribers.
pub trait Callsite: Sync {
    /// Sets the [`Interest`] for this callsite.
    ///
    /// [`Interest`]: ../subscriber/struct.Interest.html
    fn set_interest(&self, interest: Interest);

    /// Returns the [metadata] associated with the callsite.
    ///
    /// [metadata]: ../metadata/struct.Metadata.html
    fn metadata(&self) -> &Metadata;
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

/// Clear and reregister interest on every [`Callsite`]
///
/// This function is intended for runtime reconfiguration of filters on traces
/// when the filter recalculation is much less frequent than trace events are.
/// The alternative is to have the [`Subscriber`] that supports runtime
/// reconfiguration of filters always return [`Interest::sometimes()`] so that
/// [`enabled`] is evaluated for every event.
///
/// [`Callsite`]: ../callsite/trait.Callsite.html
/// [`enabled`]: ../subscriber/trait.Subscriber.html#tymethod.enabled
/// [`Interest::sometimes()`]: ../subscriber/struct.Interest.html#method.sometimes
/// [`Subscriber`]: ../subscriber/trait.Subscriber.html
pub fn rebuild_interest_cache() {
    REGISTRY.write().unwrap().rebuild_interest();
}

/// Register a new `Callsite` with the global registry.
///
/// This should be called once per callsite after the callsite has been
/// constructed.
pub fn register(callsite: &'static dyn Callsite) {
    let registry = REGISTRY.read().unwrap();
    registry.rebuild_callsite_interest(callsite);
    CALLSITES.push(callsite);
}

pub(crate) fn register_dispatch(dispatch: &Dispatch) {
    let mut registry = REGISTRY.write().unwrap();
    registry.dispatchers.push(dispatch.registrar());
    registry.rebuild_interest();
}

// ===== impl Identifier =====

impl PartialEq for Identifier {
    fn eq(&self, other: &Identifier) -> bool {
        ptr::eq(self.0, other.0)
    }
}

impl Eq for Identifier {}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

// ===== impl Callsites =====

impl Callsites {
    fn push(&self, callsite: &'static dyn Callsite) {
        let entry = CsEntry::new(callsite);
        loop {
            let head = self.head.load(Ordering::Relaxed);

            unsafe {
                (*entry).next = ptr::NonNull::new(head);
            }

            if self.head.compare_and_swap(head, entry, Ordering::Release) == head {
                break;
            }
        }
    }
}

impl<'a> IntoIterator for &'a Callsites {
    type Item = &'static dyn Callsite;
    type IntoIter = CsIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        let head = self.head.load(Ordering::Acquire);
        CsIter {
            current: unsafe { head.as_ref() }
        }
    }
}

impl CsEntry {
    fn new(callsite: &'static dyn Callsite) -> *mut Self {
        Box::into_raw(Box::new(Self {
            callsite,
            next: None,
        }))
    }

    #[inline]
    fn next(&self) -> Option<&Self> {
        self.next.as_ref().map(|ptr| unsafe { ptr.as_ref() })
    }
}

impl<'a> Iterator for CsIter<'a> {
    type Item = &'static dyn Callsite;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.take()?;
        self.current = current.next();
        Some(current.callsite)
    }
}
