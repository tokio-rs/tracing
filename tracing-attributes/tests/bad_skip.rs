use tracing_attributes::instrument;

#[instrument(skip(baz))]
fn bad_skip(foo: usize) {}

#[instrument(fields(bar = whangle))]
fn bad_field(foo: usize) {}
