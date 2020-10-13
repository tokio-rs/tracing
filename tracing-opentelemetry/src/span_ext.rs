use crate::layer::WithContext;
use opentelemetry::api::{trace as otel, trace::TraceContextExt, Context, KeyValue};
use std::time::SystemTime;

/// Utility functions to allow tracing [`Span`]s to accept and return
/// [OpenTelemetry] [`Context`]s.
///
/// [`Span`]: https://docs.rs/tracing/latest/tracing/struct.Span.html
/// [OpenTelemetry]: https://opentelemetry.io
/// [`Context`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/context/struct.Context.html
pub trait OpenTelemetrySpanExt {
    /// Associates `self` with a given OpenTelemetry trace, using the provided
    /// parent [`Context`].
    ///
    /// [`Context`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/context/struct.Context.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opentelemetry::api::{propagation::TextMapPropagator, trace::TraceContextExt};
    /// use opentelemetry::sdk::propagation::B3Propagator;
    /// use tracing_opentelemetry::OpenTelemetrySpanExt;
    /// use std::collections::HashMap;
    /// use tracing::Span;
    ///
    /// // Example carrier, could be a framework header map that impls otel's `Extract`.
    /// let mut carrier = HashMap::new();
    ///
    /// // Propagator can be swapped with trace context propagator, binary propagator, etc.
    /// let propagator = B3Propagator::new();
    ///
    /// // Extract otel parent context via the chosen propagator
    /// let parent_context = propagator.extract(&carrier);
    ///
    /// // Generate a tracing span as usual
    /// let app_root = tracing::span!(tracing::Level::INFO, "app_start");
    ///
    /// // Assign parent trace from external context
    /// app_root.set_parent(&parent_context);
    ///
    /// // Or if the current span has been created elsewhere:
    /// Span::current().set_parent(&parent_context);
    /// ```
    fn set_parent(&self, cx: &Context);

    /// Extracts an OpenTelemetry [`Context`] from `self`.
    ///
    /// [`Context`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/context/struct.Context.html
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opentelemetry::api;
    /// use tracing_opentelemetry::OpenTelemetrySpanExt;
    /// use tracing::Span;
    ///
    /// fn make_request(cx: api::Context) {
    ///     // perform external request after injecting context
    ///     // e.g. if the request's headers impl `opentelemetry::api::propagation::Inject`
    ///     // then `propagator.inject_context(cx, request.headers_mut())`
    /// }
    ///
    /// // Generate a tracing span as usual
    /// let app_root = tracing::span!(tracing::Level::INFO, "app_start");
    ///
    /// // To include tracing context in client requests from _this_ app,
    /// // extract the current OpenTelemetry context.
    /// make_request(app_root.context());
    ///
    /// // Or if the current span has been created elsewhere:
    /// make_request(Span::current().context())
    /// ```
    fn context(&self) -> Context;
}

impl OpenTelemetrySpanExt for tracing::Span {
    fn set_parent(&self, cx: &Context) {
        self.with_subscriber(move |(id, subscriber)| {
            if let Some(get_context) = subscriber.downcast_ref::<WithContext>() {
                get_context.with_context(subscriber, id, move |builder, _tracer| {
                    builder.parent_reference = cx.remote_span_reference().cloned()
                });
            }
        });
    }

    fn context(&self) -> Context {
        let mut span_reference = None;
        self.with_subscriber(|(id, subscriber)| {
            if let Some(get_context) = subscriber.downcast_ref::<WithContext>() {
                get_context.with_context(subscriber, id, |builder, tracer| {
                    span_reference = Some(tracer.sampled_span_reference(builder));
                })
            }
        });

        let span_reference = span_reference.unwrap_or_else(otel::SpanReference::empty_context);
        let compat_span = CompatSpan(span_reference);
        Context::current_with_span(compat_span)
    }
}

/// A compatibility wrapper for an injectable OpenTelemetry span reference.
#[derive(Debug)]
struct CompatSpan(otel::SpanReference);
impl otel::Span for CompatSpan {
    fn add_event_with_timestamp(
        &self,
        _name: String,
        _timestamp: std::time::SystemTime,
        _attributes: Vec<KeyValue>,
    ) {
        #[cfg(debug_assertions)]
        panic!(
            "OpenTelemetry and tracing APIs cannot be mixed, use `tracing::event!` macro instead."
        );
    }

    /// This method is used by OpenTelemetry propagators to inject span reference
    /// information into [`Carrier`]s.
    ///
    /// [`Carrier`]: https://docs.rs/opentelemetry/latest/opentelemetry/api/context/propagation/trait.Carrier.html
    fn span_reference(&self) -> otel::SpanReference {
        self.0.clone()
    }

    fn is_recording(&self) -> bool {
        #[cfg(debug_assertions)]
        panic!("cannot record via OpenTelemetry API when using extracted span in tracing");

        #[cfg(not(debug_assertions))]
        false
    }

    fn set_attribute(&self, _attribute: KeyValue) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, use `tracing::span!` macro or `span.record()` instead.");
    }

    fn set_status(&self, _code: otel::StatusCode, _message: String) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, use `tracing::span!` macro or `span.record()` instead.");
    }

    fn update_name(&self, _new_name: String) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, span names are not mutable.");
    }

    fn end_with_timestamp(&self, _timestamp: SystemTime) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, span end times are set when the underlying tracing span closes.");
    }
}
