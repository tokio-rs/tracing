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

macro_rules! cfg_feature {
    ($name:literal, { $($item:item)* }) => {
        $(
            #[cfg(feature = $name)]
            #[cfg_attr(docsrs, doc(cfg(feature = $name)))]
            $item
        )*
    }
}
