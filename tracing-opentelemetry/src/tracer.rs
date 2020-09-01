use opentelemetry::{api, sdk};

/// A protocol for OpenTelemetry [`Tracer`]s that are capable of producing
/// sampled span contexts _before_ starting their associated spans.
///
/// This enables interoperability between `tracing` and `opentelemetry` by
/// allowing otel trace ids to be associated _after_ a `tracing` span has been
/// created. See the [`OpenTelemetrySpanExt`] in this crate for usage examples.
///
/// [`Tracer`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/trace/tracer/trait.Tracer.html
/// [`OpenTelemetrySpanExt`]: trait.OpenTelemetrySpanExt.html
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
