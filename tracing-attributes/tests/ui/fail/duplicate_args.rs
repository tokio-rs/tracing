#[tracing::instrument(parent = None, parent = None)]
fn duplicate_parent() {}

#[tracing::instrument(follows_from = [tracing::Span::none()], follows_from = [tracing::Span::none()])]
fn duplicate_follows_from() {}

fn main() {}
