#[macro_use]
extern crate tokio_trace;
extern crate env_logger;
extern crate tokio_trace_log;

use tokio_trace::{Level, Span};

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tokio_trace_log::TraceLogger::new();

    tokio_trace::Dispatch::to(subscriber).as_default(|| {
        let foo: u64 = 3;
        event!(Level::Info, { foo = foo, bar = "bar" }, "hello world");

        span!("my_great_span", foo = 4u64, baz = 5u64).enter(|| {
            Span::current().close();
            event!(Level::Info, { yak_shaved = true }, "hi from inside my span");
        });
    });
}
