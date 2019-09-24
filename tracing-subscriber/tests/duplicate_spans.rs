mod support;
use self::support::*;
use tracing::{self, subscriber::with_default, Level, Span};
use tracing_subscriber::{filter::EnvFilter, prelude::*, FmtSubscriber};

#[test]
fn duplicate_spans() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new("[root]=debug"))
        .finish();

    with_default(subscriber, || {
        let root = tracing::debug_span!("root");
        let _enter = root.enter();
        // root:
        assert_eq!(root, Span::current());
        {
            let leaf = tracing::debug_span!("leaf");
            let _enter_leaf = leaf.enter();
            // root:leaf:
            assert_eq!(leaf, Span::current());

            let _reenter_root = root.enter();
            // root:leaf:
            assert_eq!(leaf, Span::current());
        }
        // root:
        assert_eq!(root, Span::current());
        {
            let _reenter_root = root.enter();
            // root:
            assert_eq!(root, Span::current());
        }
        // root:
        assert_eq!(root, Span::current());
    });
}
