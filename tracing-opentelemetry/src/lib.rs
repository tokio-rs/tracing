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
//! Fields with an `otel.` prefix are reserved for this crate and have specific
//! meaning. They are treated as ordinary fields by other layers. The current
//! special fields are:
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
//!     // Spans will be sent to the configured OpenTelemetry exporter
//!     let root = span!(tracing::Level::TRACE, "app_start", work_units = 2);
//!     let _enter = root.enter();
//!
//!     error!("This event will be logged in the root span.");
//! });
//! ```
#![deny(unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
#![doc(html_root_url = "https://docs.rs/tracing-opentelemetry/0.5.0")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo.svg",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]

/// Implementation of the trace::Layer as a source of OpenTelemetry data.
mod layer;
/// Span extension which enables OpenTelemetry context management.
mod span_ext;

pub use layer::{layer, OpenTelemetryLayer};
pub use span_ext::OpenTelemetrySpanExt;
