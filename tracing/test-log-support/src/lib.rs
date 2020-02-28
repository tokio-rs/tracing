extern crate log;
use log::{LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

pub struct Test {
    state: Arc<State>,
}

struct State {
    last_log: Mutex<Option<String>>,
}

struct Logger {
    filters: Vec<(&'static str, LevelFilter)>,
    state: Arc<State>,
}

impl Log for Logger {
    fn enabled(&self, meta: &Metadata) -> bool {
        for (target, level) in &self.filters {
            if meta.target().starts_with(dbg!(target)) {
                return meta.level() <= *level;
            }
        }
        false
    }

    fn log(&self, record: &Record) {
        let line = format!("{}", record.args());
        println!("{:<5} {} {}", record.level(), record.target(), line);
        if let Ok(mut last) = self.state.last_log.lock() {
            *last = Some(line);
        }
    }

    fn flush(&self) {}
}

impl Test {
    pub fn start() -> Self {
        Self::with_filters(&[("", LevelFilter::Trace)])
    }

    pub fn with_filters<'a>(
        filters: impl IntoIterator<Item = &'a (&'static str, LevelFilter)>,
    ) -> Self {
        let me = Arc::new(State {
            last_log: Mutex::new(None),
        });
        let state = me.clone();
        let mut max = LevelFilter::Off;
        let filters = filters
            .into_iter()
            .cloned()
            .inspect(|(_, f)| {
                if f > &max {
                    max = *f;
                }
            })
            .collect();
        let logger = Logger { filters, state: me };
        log::set_boxed_logger(Box::new(logger)).unwrap();
        log::set_max_level(max);
        Test { state }
    }

    pub fn assert_logged(&self, expected: &str) {
        let last = match self.state.last_log.lock().unwrap().take() {
            Some(last) => last,
            _ => panic!(
                "test failed: expected \"{}\", but nothing was logged",
                expected
            ),
        };

        assert_eq!(last.as_str().trim(), expected);
    }

    pub fn assert_not_logged(&self) {
        if let Some(last) = self.state.last_log.lock().unwrap().take() {
            panic!(
                "test failed: nothing to be logged, but \"{}\" was logged",
                last
            );
        }
    }
}
