use opentelemetry::{api, sdk};

/// An interface for authors of OpenTelemetry SDKs to build pre-sampled tracers.
///
/// The OpenTelemetry spec does not allow trace ids to be updated after a span
/// has been created. In order to associate extracted parent trace ids with
/// existing `tracing` spans, `tracing-opentelemetry` builds up otel span data
/// using a [`SpanBuilder`] instead, and creates / exports full otel spans only
/// when the associated `tracing` span is closed. However, in order to properly
/// inject otel [`SpanContext`] information to downstream requests, the sampling
/// state must now be known _before_ the otel span has been created.
///
/// The logic for coming to a sampling decision and creating an injectable span
/// context from a [`SpanBuilder`] is encapsulated in the
/// [`PreSampledTracer::sampled_span_context`] method and has been implemented
/// for the standard OpenTelemetry SDK, but this trait may be implemented by
/// authors of alternate OpenTelemetry SDK implementations if they wish to have
/// `tracing` compatibility.
///
/// See the [`OpenTelemetrySpanExt::set_parent`] and
/// [`OpenTelemetrySpanExt::context`] methods for example usage.
///
/// [`Tracer`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/trace/tracer/trait.Tracer.html
/// [`SpanBuilder`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/trace/tracer/struct.SpanBuilder.html
/// [`SpanContext`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/trace/span_context/struct.SpanContext.html
/// [`PreSampledTracer::sampled_span_context`]: trait.PreSampledTracer.html#tymethod.sampled_span_context
/// [`OpenTelemetrySpanExt::set_parent`]: trait.OpenTelemetrySpanExt.html#tymethod.set_parent
/// [`OpenTelemetrySpanExt::context`]: trait.OpenTelemetrySpanExt.html#tymethod.context
pub trait PreSampledTracer {
    /// Produce a pre-sampled span context for the given span builder.
    fn sampled_span_context(&self, builder: &mut api::SpanBuilder) -> api::SpanContext;

    /// Generate a new trace id.
    fn new_trace_id(&self) -> api::TraceId;

    /// Generate a new span id.
    fn new_span_id(&self) -> api::SpanId;
}

impl PreSampledTracer for api::NoopTracer {
    fn sampled_span_context(&self, builder: &mut api::SpanBuilder) -> api::SpanContext {
        builder
            .parent_context
            .clone()
            .unwrap_or_else(api::SpanContext::empty_context)
    }

    fn new_trace_id(&self) -> api::TraceId {
        api::TraceId::invalid()
    }

    fn new_span_id(&self) -> api::SpanId {
        api::SpanId::invalid()
    }
}

impl PreSampledTracer for sdk::Tracer {
    fn sampled_span_context(&self, builder: &mut api::SpanBuilder) -> api::SpanContext {
        let span_id = builder.span_id.expect("Builders must have id");
        let (trace_id, trace_flags) = builder
            .parent_context
            .as_ref()
            .filter(|parent_context| parent_context.is_valid())
            .map(|parent_context| (parent_context.trace_id(), parent_context.trace_flags()))
            .unwrap_or_else(|| {
                let trace_id = builder.trace_id.expect("trace_id should exist");

                // ensure sampling decision is recorded so all span contexts have consistent flags
                let sampling_decision = if let Some(result) = builder.sampling_result.as_ref() {
                    result.decision.clone()
                } else {
                    let mut result = self.provider().config().default_sampler.should_sample(
                        builder.parent_context.as_ref(),
                        trace_id,
                        &builder.name,
                        builder
                            .span_kind
                            .as_ref()
                            .unwrap_or(&api::SpanKind::Internal),
                        builder.attributes.as_ref().unwrap_or(&Vec::new()),
                        builder.links.as_ref().unwrap_or(&Vec::new()),
                    );

                    // Record additional attributes resulting from sampling
                    if let Some(attributes) = &mut builder.attributes {
                        attributes.append(&mut result.attributes)
                    } else {
                        builder.attributes = Some(result.attributes);
                    }

                    result.decision
                };

                let trace_flags = if sampling_decision == sdk::SamplingDecision::RecordAndSampled {
                    api::TRACE_FLAG_SAMPLED
                } else {
                    0
                };

                (trace_id, trace_flags)
            });

        api::SpanContext::new(trace_id, span_id, trace_flags, false)
    }

    fn new_trace_id(&self) -> api::TraceId {
        self.provider().config().id_generator.new_trace_id()
    }

    fn new_span_id(&self) -> api::SpanId {
        self.provider().config().id_generator.new_span_id()
    }
}
