#[cfg(tracing_unstable)]
mod app {
    use std::collections::HashMap;
    use tracing::field::valuable;
    use tracing::{info, instrument};
    use valuable::Valuable;

    #[derive(Valuable)]
    struct Headers<'a> {
        headers: HashMap<&'a str, &'a str>,
    }

    // Current there's no way to automatically apply valuable to a type, so we need to make use of
    // the fields argument for instrument
    #[instrument(fields(headers=valuable(&headers)))]
    fn process(headers: Headers) {
        info!("Handle request")
    }

    pub fn run() {
        let headers = [
            ("content-type", "application/json"),
            ("content-length", "568"),
            ("server", "github.com"),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<_, _>>();

        let http_headers = Headers { headers };

        process(http_headers);
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    #[cfg(tracing_unstable)]
    app::run();
    #[cfg(not(tracing_unstable))]
    println!("Nothing to do, this example needs --cfg=tracing_unstable to run");
}
