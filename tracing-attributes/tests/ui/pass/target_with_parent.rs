//! This program is a regression test for [#3621], where a `target` argument
//! made a following `parent` or `follows_from` argument fail as a duplicate.
//!
//! [#3621]: https://github.com/tokio-rs/tracing/issues/3621
#[tracing::instrument(target = "my_target", parent = None)]
fn target_then_parent() {}

#[tracing::instrument(target = "my_target", follows_from = [tracing::Span::none()])]
fn target_then_follows_from() {}

fn main() {}
