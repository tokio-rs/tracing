//! This library emits instrumentation points akin to those seen in real applications.
//! Its main intended use case is debugging subscribers.

#[macro_use]
extern crate tracing;

mod server;
mod yak_shave;

pub enum ApplicationKind {
    Server,
    YakShave,
}

impl ApplicationKind {
    pub fn emit(&self) {
        match self {
            ApplicationKind::Server => server::incoming_connection(),
            ApplicationKind::YakShave => yak_shave::yak_shave(),
        }
    }
}
