#[tracing::instrument(parent = None, parent = None)]
fn duplicate_parent() {}

#[tracing::instrument(follows_from = [tracing::Span::none()], follows_from = [tracing::Span::none()])]
fn duplicate_follows_from() {}

#[tracing::instrument(err, err)]
fn duplicate_err() -> Result<(), std::io::Error> {
    Ok(())
}

#[tracing::instrument(ret, ret)]
fn duplicate_ret() -> i32 {
    0
}

fn main() {}
