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
