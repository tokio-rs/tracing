//! # Tracing OpenTelemetry
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate provides a layer
//! that connects spans from multiple systems into a trace and emits them to
//! [OpenTelemetry]-compatible distributed tracing systems for processing and
//! visualization.
//!
//! [OpenTelemetry]: https://opentelemetry.io
//! [`tracing`]: https://github.com/tokio-rs/tracing
//!
//! ### Special Fields
//!
//! These fields have specific meaning for `tracing-opentelemetry` and will be
//! treated as ordinary fields by other layers:
//!
//! * `otel.name`: Override the span name sent to OpenTelemetry exporters.
//! Setting this field is useful if you want to display non-static information
//! in your span name.
//!
//! ## Examples
//!
//! ```
//! use opentelemetry::{api::Provider, sdk};
//! use tracing::{error, span};
//! use tracing_subscriber::layer::SubscriberExt;
//! use tracing_subscriber::Registry;
//!
//! // Create a new tracer
//! let tracer = sdk::Provider::default().get_tracer("service_name");
//!
//! // Create a new OpenTelemetry tracing layer
//! let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
//!
//! let subscriber = Registry::default().with(telemetry);
//!
//! // Trace executed code
//! tracing::subscriber::with_default(subscriber, || {
//!     let root = span!(tracing::Level::TRACE, "app_start", work_units = 2);
//!     let _enter = root.enter();
//!
//!     // Optionally set a dynamic span name for otel to export
//!     let operation = "GET http://example.com".to_string();
//!     span!(tracing::Level::TRACE, "http_request", otel.name=operation.as_str());
//!
//!     error!("This event will be logged in the root span.");
//! });
//! ```
#![deny(unreachable_pub)]
#![cfg_attr(test, deny(warnings))]

/// Implementation of the trace::Layer as a source of OpenTelemetry data.
mod layer;
/// Span extension which enables OpenTelemetry context management.
mod span_ext;

pub use layer::{layer, OpenTelemetryLayer};
pub use span_ext::OpenTelemetrySpanExt;
