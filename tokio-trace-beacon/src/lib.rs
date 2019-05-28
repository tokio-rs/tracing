//! This library emits instrumentation points akin to those seen in real applications.
//! Its main intended use case is debugging subscribers.

#[macro_use]
extern crate tokio_trace;

mod server;
mod yak_shave;

pub enum ApplicationKind {
    Server,
    YakShave,
}

pub fn emit(kind: &ApplicationKind) {
    match kind {
        ApplicationKind::Server => server::incoming_connection(),
        ApplicationKind::YakShave => yak_shave::yak_shave(),
    }
}
