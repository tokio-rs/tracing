use tracing::Level;

#[no_mangle]
pub fn foo() {
    tracing::error!("SOMETHING IS SERIOUSLY WRONG!!!");
}

fn main() {
    tracing_subscriber::fmt()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // sets this to be the default, global subscriber for this application.
        .init();

    foo()
}
