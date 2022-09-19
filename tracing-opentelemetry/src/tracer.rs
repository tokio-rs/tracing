use opentelemetry::sdk::trace::{Tracer, TracerProvider};
use opentelemetry::trace::OrderMap;
use opentelemetry::{
    trace as otel,
    trace::{
        noop, SamplingDecision, SamplingResult, SpanBuilder, SpanContext, SpanId, SpanKind,
        TraceContextExt, TraceFlags, TraceId, TraceState,
    },
    Context as OtelContext,
};

/// An interface for authors of OpenTelemetry SDKs to build pre-sampled tracers.
///
/// The OpenTelemetry spec does not allow trace ids to be updated after a span
/// has been created. In order to associate extracted parent trace ids with
/// existing `tracing` spans, `tracing-opentelemetry` builds up otel span data
/// using a [`SpanBuilder`] instead, and creates / exports full otel spans only
/// when the associated `tracing` span is closed. However, in order to properly
/// inject otel [`Context`] information to downstream requests, the sampling
/// state must now be known _before_ the otel span has been created.
///
/// The logic for coming to a sampling decision and creating an injectable span
/// context from a [`SpanBuilder`] is encapsulated in the
/// [`PreSampledTracer::sampled_context`] method and has been implemented
/// for the standard OpenTelemetry SDK, but this trait may be implemented by
/// authors of alternate OpenTelemetry SDK implementations if they wish to have
/// `tracing` compatibility.
///
/// See the [`OpenTelemetrySpanExt::set_parent`] and
/// [`OpenTelemetrySpanExt::context`] methods for example usage.
///
/// [`Tracer`]: opentelemetry::trace::Tracer
/// [`SpanBuilder`]: opentelemetry::trace::SpanBuilder
/// [`PreSampledTracer::sampled_span_context`]: crate::PreSampledTracer::sampled_span_context
/// [`OpenTelemetrySpanExt::set_parent`]: crate::OpenTelemetrySpanExt::set_parent
/// [`OpenTelemetrySpanExt::context`]: crate::OpenTelemetrySpanExt::context
/// [`Context`]: opentelemetry::Context
pub trait PreSampledTracer {
    /// Produce an otel context containing an active and pre-sampled span for
    /// the given span builder data.
    ///
    /// The sampling decision, span context information, and parent context
    /// values must match the values recorded when the tracing span is closed.
    fn sampled_context(&self, data: &mut crate::OtelData) -> OtelContext;

    /// Generate a new trace id.
    fn new_trace_id(&self) -> otel::TraceId;

    /// Generate a new span id.
    fn new_span_id(&self) -> otel::SpanId;
}

impl PreSampledTracer for noop::NoopTracer {
    fn sampled_context(&self, data: &mut crate::OtelData) -> OtelContext {
        data.parent_cx.clone()
    }

    fn new_trace_id(&self) -> otel::TraceId {
        otel::TraceId::INVALID
    }

    fn new_span_id(&self) -> otel::SpanId {
        otel::SpanId::INVALID
    }
}

impl PreSampledTracer for Tracer {
    fn sampled_context(&self, data: &mut crate::OtelData) -> OtelContext {
        // Ensure tracing pipeline is still installed.
        if self.provider().is_none() {
            return OtelContext::new();
        }
        let provider = self.provider().unwrap();
        let parent_cx = &data.parent_cx;
        let builder = &mut data.builder;

        // Gather trace state
        let (trace_id, parent_trace_flags) = current_trace_state(builder, parent_cx, &provider);

        // Sample or defer to existing sampling decisions
        let (flags, trace_state) = if let Some(result) = &builder.sampling_result {
            process_sampling_result(result, parent_trace_flags)
        } else {
            builder.sampling_result = Some(provider.config().sampler.should_sample(
                Some(parent_cx),
                trace_id,
                &builder.name,
                builder.span_kind.as_ref().unwrap_or(&SpanKind::Internal),
                builder.attributes.as_ref().unwrap_or(&OrderMap::default()),
                builder.links.as_deref().unwrap_or(&[]),
                self.instrumentation_library(),
            ));

            process_sampling_result(
                builder.sampling_result.as_ref().unwrap(),
                parent_trace_flags,
            )
        }
        .unwrap_or_default();

        let span_id = builder.span_id.unwrap_or(SpanId::INVALID);
        let span_context = SpanContext::new(trace_id, span_id, flags, false, trace_state);
        parent_cx.with_remote_span_context(span_context)
    }

    fn new_trace_id(&self) -> otel::TraceId {
        self.provider()
            .map(|provider| provider.config().id_generator.new_trace_id())
            .unwrap_or(otel::TraceId::INVALID)
    }

    fn new_span_id(&self) -> otel::SpanId {
        self.provider()
            .map(|provider| provider.config().id_generator.new_span_id())
            .unwrap_or(otel::SpanId::INVALID)
    }
}

fn current_trace_state(
    builder: &SpanBuilder,
    parent_cx: &OtelContext,
    provider: &TracerProvider,
) -> (TraceId, TraceFlags) {
    if parent_cx.has_active_span() {
        let span = parent_cx.span();
        let sc = span.span_context();
        (sc.trace_id(), sc.trace_flags())
    } else {
        (
            builder
                .trace_id
                .unwrap_or_else(|| provider.config().id_generator.new_trace_id()),
            Default::default(),
        )
    }
}

fn process_sampling_result(
    sampling_result: &SamplingResult,
    trace_flags: TraceFlags,
) -> Option<(TraceFlags, TraceState)> {
    match sampling_result {
        SamplingResult {
            decision: SamplingDecision::Drop,
            ..
        } => None,
        SamplingResult {
            decision: SamplingDecision::RecordOnly,
            trace_state,
            ..
        } => Some((trace_flags & !TraceFlags::SAMPLED, trace_state.clone())),
        SamplingResult {
            decision: SamplingDecision::RecordAndSample,
            trace_state,
            ..
        } => Some((trace_flags | TraceFlags::SAMPLED, trace_state.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OtelData;
    use opentelemetry::sdk::trace::{config, Sampler, TracerProvider};
    use opentelemetry::trace::{SpanBuilder, SpanId, TracerProvider as _};

    #[test]
    fn assigns_default_trace_id_if_missing() {
        let provider = TracerProvider::default();
        let tracer = provider.tracer("test");
        let mut builder = SpanBuilder::from_name("empty".to_string());
        builder.span_id = Some(SpanId::from(1u64.to_be_bytes()));
        builder.trace_id = None;
        let parent_cx = OtelContext::new();
        let cx = tracer.sampled_context(&mut OtelData { builder, parent_cx });
        let span = cx.span();
        let span_context = span.span_context();

        assert!(span_context.is_valid());
    }

    #[rustfmt::skip]
    fn sampler_data() -> Vec<(&'static str, Sampler, OtelContext, Option<SamplingResult>, bool)> {
        vec![
            // No parent samples
            ("empty_parent_cx_always_on", Sampler::AlwaysOn, OtelContext::new(), None, true),
            ("empty_parent_cx_always_off", Sampler::AlwaysOff, OtelContext::new(), None, false),

            // Remote parent samples
            ("remote_parent_cx_always_on", Sampler::AlwaysOn, OtelContext::new().with_remote_span_context(span_context(TraceFlags::SAMPLED, true)), None, true),
            ("remote_parent_cx_always_off", Sampler::AlwaysOff, OtelContext::new().with_remote_span_context(span_context(TraceFlags::SAMPLED, true)), None, false),
            ("sampled_remote_parent_cx_parent_based", Sampler::ParentBased(Box::new(Sampler::AlwaysOff)), OtelContext::new().with_remote_span_context(span_context(TraceFlags::SAMPLED, true)), None, true),
            ("unsampled_remote_parent_cx_parent_based", Sampler::ParentBased(Box::new(Sampler::AlwaysOn)), OtelContext::new().with_remote_span_context(span_context(TraceFlags::default(), true)), None, false),

            // Existing sampling result defers
            ("previous_drop_result_always_on", Sampler::AlwaysOn, OtelContext::new(), Some(SamplingResult { decision: SamplingDecision::Drop, attributes: vec![], trace_state: Default::default() }), false),
            ("previous_record_and_sample_result_always_off", Sampler::AlwaysOff, OtelContext::new(), Some(SamplingResult { decision: SamplingDecision::RecordAndSample, attributes: vec![], trace_state: Default::default() }), true),
 
            // Existing local parent, defers
            ("previous_drop_result_always_on", Sampler::AlwaysOn, OtelContext::new(), Some(SamplingResult { decision: SamplingDecision::Drop, attributes: vec![], trace_state: Default::default() }), false),
            ("previous_record_and_sample_result_always_off", Sampler::AlwaysOff, OtelContext::new(), Some(SamplingResult { decision: SamplingDecision::RecordAndSample, attributes: vec![], trace_state: Default::default() }), true),
        ]
    }

    #[test]
    fn sampled_context() {
        for (name, sampler, parent_cx, previous_sampling_result, is_sampled) in sampler_data() {
            let provider = TracerProvider::builder()
                .with_config(config().with_sampler(sampler))
                .build();
            let tracer = provider.tracer("test");
            let mut builder = SpanBuilder::from_name("parent".to_string());
            builder.sampling_result = previous_sampling_result;
            let sampled = tracer.sampled_context(&mut OtelData { builder, parent_cx });

            assert_eq!(
                sampled.span().span_context().is_sampled(),
                is_sampled,
                "{}",
                name
            )
        }
    }

    fn span_context(trace_flags: TraceFlags, is_remote: bool) -> SpanContext {
        SpanContext::new(
            TraceId::from(1u128.to_be_bytes()),
            SpanId::from(1u64.to_be_bytes()),
            trace_flags,
            is_remote,
            Default::default(),
        )
    }
}
