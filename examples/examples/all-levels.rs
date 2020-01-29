use tracing::Level;
use tracing_subscriber;

fn main() {
    tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // sets this to be the default, global subscriber for this application.
        .init();

    tracing::error!("SOMETHING IS SERIOUSLY WRONG!!!");
    tracing::warn!("important informational messages; might indicate an error");
    tracing::info!("general informational messages relevant to users");
    tracing::debug!("diagnostics used for internal debugging of a library or application");
    tracing::trace!("very verbose diagnostic events");
}
