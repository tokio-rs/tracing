extern crate log;
use log::{LevelFilter, Log, Metadata, Record};
use std::sync::{Arc, Mutex};

pub struct Test {
    state: Arc<State>,
}

struct State {
    last_log: Mutex<Option<String>>,
}

struct Logger(Arc<State>);

impl Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let line = format!("{}", record.args());
        println!("{:<5} {} {}", record.level(), record.target(), line);
        if let Ok(mut last) = self.0.last_log.lock() {
            *last = Some(line);
        }
    }

    fn flush(&self) {}
}

impl Test {
    pub fn start() -> Self {
        let me = Arc::new(State {
            last_log: Mutex::new(None),
        });
        let state = me.clone();
        log::set_boxed_logger(Box::new(Logger(me))).unwrap();
        log::set_max_level(LevelFilter::Trace);
        Test {
            state,
        }
    }

    pub fn assert_logged(&self, expected: &str) {
        let last = match self.state.last_log.lock().unwrap().take() {
            Some(last) => last,
            _ => panic!("test failed: expected \"{}\", but nothing was logged", expected),
        };

        assert_eq!(last.as_str().trim(), expected);
    }

}

