#![allow(unreachable_code)]

#[tracing::instrument]
async fn unit() {
    ""
}

#[tracing::instrument]
async fn simple_mismatch() -> String {
    ""
}

// FIXME: this span is still pretty poor
#[tracing::instrument]
async fn opaque_unsatisfied() -> impl std::fmt::Display {
    ("",)
}

struct Wrapper<T>(T);

#[tracing::instrument]
async fn mismatch_with_opaque() -> Wrapper<impl std::fmt::Display> {
    ""
}

fn main() {
    let _ = unit();
    let _ = simple_mismatch();
    let _ = opaque_unsatisfied();
    let _ = mismatch_with_opaque();
}
