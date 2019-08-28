use tracing::Level;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_fmt::FmtSubscriber::builder()
        // Disable `tracing-fmt`'s filter implementation...
        .with_filter(tracing_fmt::filter::none())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing_macros::event!(Level::INFO, target: "my target", foo = 3, bar.baz = "a string");

    tracing_macros::trace!("hello", foo = 3, bar.baz = "a string");

    tracing_macros::event!(
        Level::TRACE,
        "hello {}, i have {:?}!",
        hello_to = "world",
        bar.baz = "a string"
    );

    Ok(())
}
