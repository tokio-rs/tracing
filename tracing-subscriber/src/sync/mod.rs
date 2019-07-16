
mod atomic_vec;
pub use self::inner::*;

#[cfg(test)]
mod inner {
    pub use loom::sync::Arc;
    pub mod atomic {
        pub use self::atomic_vec::AtomicVec;
        pub use loom::sync::atomic::*;
        pub use std::sync::atomic::{spin_loop_hint, Ordering};
    }
}

#[cfg(not(test))]
mod inner {
    pub mod atomic {
        pub use std::sync::atomic::*;
        pub use self::atomic_vec::AtomicVec;
    }
    pub use std::sync::Arc;
}
