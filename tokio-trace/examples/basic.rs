#[macro_use]
extern crate tokio_trace;
extern crate env_logger;
extern crate tokio_trace_log;

use tokio_trace::Level;

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tokio_trace_log::TraceLogger::new();

    tokio_trace::Dispatch::to(subscriber).with(|| {
        let foo = 3;
        event!(Level::Info, { foo = foo, bar = "bar" }, "hello world");

        let span = span!("my_great_span", foo = &4, baz = &5);
        span.enter(|| {
            event!(
                Level::Info,
                { yak_shaved = &true },
                "hi from inside my span"
            );
        });
    });
}
