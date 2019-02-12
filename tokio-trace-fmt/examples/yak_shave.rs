#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_fmt;

fn main() {
    let subscriber = tokio_trace_fmt::FmtSubscriber::builder()
        .full()
        .finish();

    tokio_trace::subscriber::with_default(subscriber, || {
        span!("yak_shave").enter(|| {
            let number_of_yaks = 3;
            info!(yaks_to_shave = number_of_yaks);
            info!({ yak = number_of_yaks, excitement = "yay!" }, "hello! I'm gonna shave a yak.");

            let mut span = span!("my_great_span", foo = &4, baz = &5);
            span.enter(|| {
                error!({ yak_shaved = false }, "Huh, no yaks to be found...");
            });
        })

    });
}
