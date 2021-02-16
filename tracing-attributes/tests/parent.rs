use tracing::Span;
use tracing_attributes::instrument;

#[derive(Debug)]
struct WithSpan {
    span: Span,
}

impl WithSpan {
    fn new(span: Span) -> Self {
        Self { span }
    }

    #[instrument(parent = &self.span)]
    fn foo(&self) {}
}

#[test]
fn test() {
    let span = Span::current();
    let with_span = WithSpan::new(span);
    with_span.foo();
}
