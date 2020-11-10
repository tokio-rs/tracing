use crate::layer::WithContext;
use opentelemetry::api;
use opentelemetry::api::TraceContextExt;

/// Utility functions to allow tracing [`Span`]s to accept and return
/// [OpenTelemetry] [`Context`]s.
///
/// [`Span`]: tracing::Span
/// [OpenTelemetry]: https://opentelemetry.io
/// [`Context`]: opentelemetry::api::context::Context
pub trait OpenTelemetrySpanExt {
    /// Associates `self` with a given OpenTelemetry trace, using the provided
    /// parent [`Context`].
    ///
    /// [`Context`]: opentelemetry::api::context::Context
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opentelemetry::api::{self, HttpTextFormat, TraceContextExt};
    /// use tracing_opentelemetry::OpenTelemetrySpanExt;
    /// use std::collections::HashMap;
    /// use tracing::Span;
    ///
    /// // Example carrier, could be a framework header map that impls `api::Carrier`.
    /// let mut carrier = HashMap::new();
    ///
    /// // Propagator can be swapped with trace context propagator binary propagator, etc.
    /// let propagator = api::B3Propagator::new();
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
    fn set_parent(&self, span_context: &api::Context);

    /// Extracts an OpenTelemetry [`Context`] from `self`.
    ///
    /// [`Context`]: opentelemetry::api::context::Context
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
    ///     // e.g. if there are request headers that impl `opentelemetry::api::Carrier`
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
    fn context(&self) -> api::Context;
}

impl OpenTelemetrySpanExt for tracing::Span {
    fn set_parent(&self, parent_context: &api::Context) {
        self.with_collector(move |(id, collector)| {
            if let Some(get_context) = collector.downcast_ref::<WithContext>() {
                get_context.with_context(collector, id, move |builder, _tracer| {
                    builder.parent_context = parent_context.remote_span_context().cloned()
                });
            }
        });
    }

    fn context(&self) -> api::Context {
        let mut span_context = None;
        self.with_collector(|(id, collector)| {
            if let Some(get_context) = collector.downcast_ref::<WithContext>() {
                get_context.with_context(collector, id, |builder, tracer| {
                    span_context = Some(tracer.sampled_span_context(builder));
                })
            }
        });

        let compat_span = CompatSpan(span_context.unwrap_or_else(api::SpanContext::empty_context));
        api::Context::current_with_span(compat_span)
    }
}

/// A compatibility wrapper for an injectable OpenTelemetry span context.
#[derive(Debug)]
struct CompatSpan(api::SpanContext);
impl api::Span for CompatSpan {
    fn add_event_with_timestamp(
        &self,
        _name: String,
        _timestamp: std::time::SystemTime,
        _attributes: Vec<api::KeyValue>,
    ) {
        #[cfg(debug_assertions)]
        panic!(
            "OpenTelemetry and tracing APIs cannot be mixed, use `tracing::event!` macro instead."
        );
    }

    /// This method is used by OpenTelemetry propagators to inject span context
    /// information into [`Carrier`]s.
    ///
    /// [`Carrier`]: opentelemetry::api::context::propagation::Carrier
    fn span_context(&self) -> api::SpanContext {
        self.0.clone()
    }

    fn is_recording(&self) -> bool {
        #[cfg(debug_assertions)]
        panic!("cannot record via OpenTelemetry API when using extracted span in tracing");

        #[cfg(not(debug_assertions))]
        false
    }

    fn set_attribute(&self, _attribute: api::KeyValue) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, use `tracing::span!` macro or `span.record()` instead.");
    }

    fn set_status(&self, _code: api::StatusCode, _message: String) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, use `tracing::span!` macro or `span.record()` instead.");
    }

    fn update_name(&self, _new_name: String) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, span names are not mutable.");
    }

    fn end(&self) {
        #[cfg(debug_assertions)]
        panic!("OpenTelemetry and tracing APIs cannot be mixed, span end times are set when the underlying tracing span closes.");
    }
}
