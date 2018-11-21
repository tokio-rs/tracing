#[macro_use]
extern crate tokio_trace;
extern crate env_logger;
extern crate tokio_trace_log;

fn main() {
    env_logger::Builder::new().parse("trace").init();
    let subscriber = tokio_trace_log::TraceLogger::builder()
        .with_parent_fields(true)
        .with_span_entry(true)
        .with_span_exits(true)
        .finish();

    tokio_trace::Dispatch::new(subscriber).as_default(|| {
        let foo = 3;
        trace!({ foo = foo, bar = "bar" }, "hello! I'm gonna shave a yak.");

        let mut span = span!("my_great_span", foo = &4, baz = &5);
        span.enter(|| {
            error!({ yak_shaved = false }, "Huh, no yaks to be found...");
        });
    });
}
