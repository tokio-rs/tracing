extern crate env_logger;
extern crate log;
extern crate tokio_trace_log;

pub fn try_init() -> Result<(), log::SetLoggerError> {
    env_logger::Builder::from_default_env()
        .format(|_, record| tokio_trace_log::format_trace(record))
        .try_init()
}

pub fn init() {
    try_init().unwrap()
}
