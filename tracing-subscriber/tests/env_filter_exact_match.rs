#![cfg(all(feature = "env-filter", feature = "registry"))]

use std::fmt;
use tracing::{subscriber::with_default, Level};
use tracing_subscriber::{filter::EnvFilter, prelude::*, Registry};

struct DebugChunks<'a>(&'a [&'a str]);

impl fmt::Debug for DebugChunks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.0 {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}

fn assert_matches(chunks: &[&str], expected: bool) {
    let filter = EnvFilter::builder()
        .with_regex(false)
        .parse("[work{state=ReadyForProduction}]=debug")
        .expect("filter should parse");

    with_default(Registry::default().with(filter), || {
        let span = tracing::info_span!("work", state = ?DebugChunks(chunks));
        let _guard = span.enter();
        assert_eq!(tracing::enabled!(Level::DEBUG), expected);
    });
}

#[test]
fn exact_debug_match_accepts_multiple_chunks() {
    assert_matches(&["Ready", "For", "Production"], true);
}

#[test]
fn exact_debug_match_rejects_prefix() {
    assert_matches(&["Ready"], false);
}

#[test]
fn exact_debug_match_rejects_longer_value() {
    assert_matches(&["ReadyForProduction", "Now"], false);
}

#[test]
fn exact_debug_match_rejects_empty_value() {
    assert_matches(&[], false);
}
