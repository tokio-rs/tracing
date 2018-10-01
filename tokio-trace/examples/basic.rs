#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_log;
extern crate env_logger;

use tokio_trace::Level;

fn main() {
    tokio_trace::Dispatcher::builder()
        .add_subscriber(tokio_trace_log::LogSubscriber::new())
        .init();
    env_logger::Builder::new().parse("info").init();

    let foo = 3;
    event!(Level::Info, { foo = foo, bar = "bar" }, "hello world");

    let span = span!("my_great_span", foo = 4, baz = 5);
    span.enter(|| {
        event!(Level::Info, { yak_shaved = true }, "hi from inside my span");
    });
}
