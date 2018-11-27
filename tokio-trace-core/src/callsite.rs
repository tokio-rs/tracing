//! Callsites represent the source locations from which spans or events
//! originate.
use std::sync::Mutex;
use {
    dispatcher::{self, Dispatch},
    subscriber::{Interest, Subscriber},
    Meta,
};

lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry {
        callsites: Vec::new(),
        dispatchers: Vec::new(),
    });
}

struct Registry {
    callsites: Vec<&'static Callsite>,
    dispatchers: Vec<dispatcher::Registrar>,
}

/// Trait implemented by callsites.
pub trait Callsite: Sync {
    /// Adds the [`Interest`] returned by [registering] the callsite with a
    /// [dispatcher].
    ///
    /// If the interest is greater than or equal to the callsite's current
    /// interest, this should change whether or not the callsite is enabled.
    ///
    /// [`Interest`]: ::subscriber::Interest
    /// [registering]: ::subscriber::Subscriber::register_callsite
    /// [dispatcher]: ::Dispatch
    fn add_interest(&self, interest: Interest);

    /// Remove _all_ [`Interest`] from the callsite, disabling it.
    ///
    /// [`Interest`]: ::subscriber::Interest
    fn remove_interest(&self);

    /// Returns the [metadata] associated with the callsite.
    ///
    /// [metadata]: ::Meta
    fn metadata(&self) -> &Meta;
}

/// Register a new `Callsite` with the global registry.
///
/// This should be called once per callsite after the callsite has been constructed.
pub fn register(callsite: &'static Callsite) {
    let mut registry = REGISTRY.lock().unwrap();
    let meta = callsite.metadata();
    registry.dispatchers.retain(|registrar| {
        match registrar.try_register(meta) {
            Some(interest) => {
                callsite.add_interest(interest);
                true
            }
            // TODO: if the dispatcher has been dropped, should we invalidate
            // any callsites that it previously enabled?
            None => false,
        }
    });
    registry.callsites.push(callsite);
}

pub(crate) fn register_dispatch(dispatch: &Dispatch) {
    let mut registry = REGISTRY.lock().unwrap();
    registry.dispatchers.push(dispatch.registrar());
    for callsite in &registry.callsites {
        let interest = dispatch.register_callsite(callsite.metadata());
        callsite.add_interest(interest);
    }
}

/// Reset the registry. This is typically only useful in tests.
#[cfg(any(test, feature = "test-support"))]
pub fn reset_registry() {
    let mut registry = REGISTRY.lock().unwrap();
    registry.callsites.clear();
    registry.dispatchers.clear();
}
