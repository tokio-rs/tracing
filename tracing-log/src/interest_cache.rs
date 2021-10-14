use ahash::AHasher;
use log::{Level, Metadata};
use lru::LruCache;
use std::cell::RefCell;
use std::hash::Hasher;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// The interest cache configuration.
#[derive(Debug)]
pub struct InterestCacheConfig {
    min_verbosity: Level,
    lru_cache_size: usize,
}

impl Default for InterestCacheConfig {
    fn default() -> Self {
        InterestCacheConfig {
            min_verbosity: Level::Debug,
            lru_cache_size: 1024,
        }
    }
}

impl InterestCacheConfig {
    /// Sets the minimum logging verbosity for which the cache will apply.
    ///
    /// The interest for logs with a lower verbosity than specified here
    /// will not be cached.
    ///
    /// By default this is set to `Debug`.
    pub fn with_min_verbosity(mut self, level: Level) -> Self {
        self.min_verbosity = level;
        self
    }

    /// Sets the size of the LRU cache used to cache the interest.
    ///
    /// By default this is set to 1024.
    pub fn with_lru_cache_size(mut self, size: usize) -> Self {
        self.lru_cache_size = size;
        self
    }
}

struct Bucket {
    level: Level,
    hash: u64,
    result: bool,
}

struct State {
    min_verbosity: Option<Level>,
    epoch: usize,
    cache: LruCache<usize, Bucket, ahash::RandomState>,
}

impl State {
    fn new(epoch: usize, config: Option<&InterestCacheConfig>) -> Self {
        if let Some(config) = config {
            State {
                epoch,
                min_verbosity: Some(config.min_verbosity),
                cache: LruCache::new(config.lru_cache_size),
            }
        } else {
            State {
                epoch,
                min_verbosity: None,
                cache: LruCache::new(0),
            }
        }
    }
}

static INTEREST_CACHE_EPOCH: AtomicUsize = AtomicUsize::new(0);

fn interest_cache_epoch() -> usize {
    INTEREST_CACHE_EPOCH.load(Ordering::Relaxed)
}

struct SentinelCallsite;

impl tracing_core::Callsite for SentinelCallsite {
    fn set_interest(&self, _: tracing_core::subscriber::Interest) {
        INTEREST_CACHE_EPOCH.fetch_add(1, Ordering::SeqCst);
    }

    fn metadata(&self) -> &tracing_core::Metadata<'_> {
        &SENTINEL_METADATA
    }
}

static SENTINEL_CALLSITE: SentinelCallsite = SentinelCallsite;
static SENTINEL_METADATA: tracing_core::Metadata<'static> = tracing_core::Metadata::new(
    "log interest cache",
    "log",
    tracing_core::Level::ERROR,
    None,
    None,
    None,
    tracing_core::field::FieldSet::new(&[], tracing_core::identify_callsite!(&SENTINEL_CALLSITE)),
    tracing_core::metadata::Kind::EVENT,
);

lazy_static::lazy_static! {
    static ref CONFIG: Mutex<Option<InterestCacheConfig>> = {
        tracing_core::callsite::register(&SENTINEL_CALLSITE);
        Mutex::new(None)
    };
}

thread_local! {
    static STATE: RefCell<State> = {
        let config = CONFIG.lock().unwrap();
        RefCell::new(State::new(interest_cache_epoch(), config.as_ref()))
    };
}

pub(crate) fn reconfigure(new_config: Option<InterestCacheConfig>) {
    *CONFIG.lock().unwrap() = new_config;
    INTEREST_CACHE_EPOCH.fetch_add(1, Ordering::SeqCst);
}

pub(crate) fn try_cache(metadata: &Metadata<'_>, callback: impl FnOnce() -> bool) -> bool {
    STATE.with(|state| {
        let mut state = state.borrow_mut();

        // If the interest cache in core was rebuilt we need to reset the cache here too.
        let epoch = interest_cache_epoch();
        if epoch != state.epoch {
            *state = State::new(epoch, CONFIG.lock().unwrap().as_ref());
        }

        let level = metadata.level();
        let is_disabled = state
            .min_verbosity
            .map(|min_verbosity| level < min_verbosity)
            .unwrap_or(true);

        if is_disabled {
            return callback();
        }

        let target = metadata.target();

        let mut hasher = AHasher::default();
        hasher.write(target.as_bytes());
        let hash = hasher.finish();

        // Since log targets are usually static strings we just use
        // the address of the pointer as the key for our cache.
        let key = target.as_ptr() as usize ^ level as usize;
        if let Some(bucket) = state.cache.get_mut(&key) {
            // And here we make sure that the target actually matches.
            //
            // This is just a hash, so theoretically we're not guaranteed that it won't
            // collide, however in practice it shouldn't matter as it is quite unlikely
            // for both the target string's pointer *and* the hash to be equal at
            // the same time. And in case our LRU cache is too small we really want to
            // avoid doing any memory allocations (which we'd have to do if we'd store
            // the whole target string in our cache) as that would completely tank our
            // performance.
            if bucket.hash == hash && bucket.level == level {
                return bucket.result;
            }
        }

        let result = callback();
        state.cache.put(
            key,
            Bucket {
                level,
                hash,
                result,
            },
        );

        result
    })
}

#[cfg(test)]
fn lock_for_test() -> impl Drop {
    // We need to make sure only one test runs at a time.

    lazy_static::lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    match LOCK.lock() {
        Ok(guard) => guard,
        Err(poison) => poison.into_inner()
    }
}

#[test]
fn test_when_disabled_the_callback_is_always_called() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = None;
    std::thread::spawn(|| {
        let metadata = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy")
            .build();
        let mut count = 0;
        try_cache(&metadata, || {
            count += 1;
            true
        });
        assert_eq!(count, 1);
        try_cache(&metadata, || {
            count += 1;
            true
        });
        assert_eq!(count, 2);
    })
    .join()
    .unwrap();
}

#[test]
fn test_when_enabled_the_callback_is_called_only_once_for_a_high_enough_verbosity() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(InterestCacheConfig::default().with_min_verbosity(Level::Debug));
    std::thread::spawn(|| {
        let metadata = log::MetadataBuilder::new()
            .level(Level::Debug)
            .target("dummy")
            .build();
        let mut count = 0;
        try_cache(&metadata, || {
            count += 1;
            true
        });
        assert_eq!(count, 1);
        try_cache(&metadata, || {
            count += 1;
            true
        });
        assert_eq!(count, 1);
    })
    .join()
    .unwrap();
}

#[test]
fn test_when_core_interest_cache_is_rebuilt_this_cache_is_also_flushed() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(InterestCacheConfig::default().with_min_verbosity(Level::Debug));
    std::thread::spawn(|| {
        let metadata = log::MetadataBuilder::new()
            .level(Level::Debug)
            .target("dummy")
            .build();
        {
            let mut count = 0;
            try_cache(&metadata, || {
                count += 1;
                true
            });
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
        }
        tracing_core::callsite::rebuild_interest_cache();
        {
            let mut count = 0;
            try_cache(&metadata, || {
                count += 1;
                true
            });
            try_cache(&metadata, || {
                count += 1;
                true
            });
            assert_eq!(count, 1);
        }
    })
    .join()
    .unwrap();
}

#[test]
fn test_when_enabled_the_callback_is_always_called_for_a_low_enough_verbosity() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(InterestCacheConfig::default().with_min_verbosity(Level::Debug));
    std::thread::spawn(|| {
        let metadata = log::MetadataBuilder::new()
            .level(Level::Info)
            .target("dummy")
            .build();
        let mut count = 0;
        try_cache(&metadata, || {
            count += 1;
            true
        });
        assert_eq!(count, 1);
        try_cache(&metadata, || {
            count += 1;
            true
        });
        assert_eq!(count, 2);
    })
    .join()
    .unwrap();
}

#[test]
fn test_different_log_levels_are_cached_separately() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(InterestCacheConfig::default().with_min_verbosity(Level::Debug));
    std::thread::spawn(|| {
        let metadata_debug = log::MetadataBuilder::new()
            .level(Level::Debug)
            .target("dummy")
            .build();
        let metadata_trace = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy")
            .build();
        let mut count_debug = 0;
        let mut count_trace = 0;
        try_cache(&metadata_debug, || {
            count_debug += 1;
            true
        });
        try_cache(&metadata_trace, || {
            count_trace += 1;
            true
        });
        try_cache(&metadata_debug, || {
            count_debug += 1;
            true
        });
        try_cache(&metadata_trace, || {
            count_trace += 1;
            true
        });
        assert_eq!(count_debug, 1);
        assert_eq!(count_trace, 1);
    })
    .join()
    .unwrap();
}

#[test]
fn test_different_log_targets_are_cached_separately() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(InterestCacheConfig::default().with_min_verbosity(Level::Debug));
    std::thread::spawn(|| {
        let metadata_1 = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy_1")
            .build();
        let metadata_2 = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy_2")
            .build();
        let mut count_1 = 0;
        let mut count_2 = 0;
        try_cache(&metadata_1, || {
            count_1 += 1;
            true
        });
        try_cache(&metadata_2, || {
            count_2 += 1;
            true
        });
        try_cache(&metadata_1, || {
            count_1 += 1;
            true
        });
        try_cache(&metadata_2, || {
            count_2 += 1;
            true
        });
        assert_eq!(count_1, 1);
        assert_eq!(count_2, 1);
    })
    .join()
    .unwrap();
}

#[test]
fn test_when_cache_runs_out_of_space_the_callback_is_called_again() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(
        InterestCacheConfig::default()
            .with_min_verbosity(Level::Debug)
            .with_lru_cache_size(1),
    );
    std::thread::spawn(|| {
        let metadata_1 = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy_1")
            .build();
        let metadata_2 = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy_2")
            .build();
        let mut count = 0;
        try_cache(&metadata_1, || {
            count += 1;
            true
        });
        try_cache(&metadata_1, || {
            count += 1;
            true
        });
        assert_eq!(count, 1);
        try_cache(&metadata_2, || true);
        try_cache(&metadata_1, || {
            count += 1;
            true
        });
        assert_eq!(count, 2);
    })
    .join()
    .unwrap();
}

#[test]
fn test_cache_returns_previously_computed_value() {
    let _lock = lock_for_test();

    *CONFIG.lock().unwrap() = Some(InterestCacheConfig::default().with_min_verbosity(Level::Debug));
    std::thread::spawn(|| {
        let metadata_1 = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy_1")
            .build();
        let metadata_2 = log::MetadataBuilder::new()
            .level(Level::Trace)
            .target("dummy_2")
            .build();
        try_cache(&metadata_1, || true);
        assert_eq!(try_cache(&metadata_1, || { unreachable!() }), true);
        try_cache(&metadata_2, || false);
        assert_eq!(try_cache(&metadata_2, || { unreachable!() }), false);
    })
    .join()
    .unwrap();
}
