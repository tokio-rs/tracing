macro_rules! try_lock {
    ($lock:expr) => {
        try_lock!($lock, else return)
    };
    ($lock:expr, else $els:expr) => {
        if let Ok(l) = $lock {
            l
        } else if std::thread::panicking() {
            $els
        } else {
            panic!("lock poisoned")
        }
    };
}

/// Declares fmt items.
#[doc(hidden)]
macro_rules! cfg_fmt {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "fmt")]
            #[cfg_attr(docsrs, doc(cfg(feature = "fmt")))]
            $item
        )*
    }
}

/// Declares registry items.
#[doc(hidden)]
macro_rules! cfg_registry {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "registry")]
            #[cfg_attr(docsrs, doc(cfg(feature = "registry")))]
            $item
        )*
    }
}
