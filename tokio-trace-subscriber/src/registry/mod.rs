use tokio_trace_core::span;

pub trait Registry: 'static {
    type Span: SpanRef;

    fn new_span(&self, attrs: &span::Attributes) -> span::Id;

    fn record(&self, id: &span::Id, fields: &span::Record);

    fn clone_span(&self, id: &span::Id);

    fn drop_span(&self, id: &span::Id);

    fn get(&self, id: &span::Id) -> Option<Self::Span>;
}

pub trait SpanRef {
}
