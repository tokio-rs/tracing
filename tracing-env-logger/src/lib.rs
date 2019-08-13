use env_logger;
use log;
use tracing_log;

pub fn try_init_from_builder(mut builder: env_logger::Builder) -> Result<(), log::SetLoggerError> {
    // TODO: make this an extension trait method
    builder
        .format(|_, record| tracing_log::format_trace(record))
        .try_init()
}

pub fn try_init() -> Result<(), log::SetLoggerError> {
    try_init_from_builder(env_logger::Builder::from_default_env())
}

pub fn init() {
    try_init().unwrap()
}
