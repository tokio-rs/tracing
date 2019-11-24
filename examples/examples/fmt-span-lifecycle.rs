use tracing::subscriber::with_default;
use tracing_subscriber::fmt::{self, format};
use tracing_subscriber::layer::Layer;
use tracing_subscriber::Registry;

fn main() {
    let f = format::Format::default().with_spans().with_entry();
    let fmt = fmt::Layer::builder().event_format(f).finish();
    let subscriber = fmt.with_subscriber(Registry::default());

    with_default(subscriber, || {
        let root = tracing::info_span!("root");
        root.in_scope(|| {
            let child = tracing::info_span!("child");
            child.in_scope(|| {
                let leaf = tracing::info_span!("leafA");
                leaf.in_scope(|| {});
                let leaf = tracing::info_span!("leafB");
                leaf.in_scope(|| {});
            });
        });
    });
}
