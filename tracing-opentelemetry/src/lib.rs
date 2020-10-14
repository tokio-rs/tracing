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
//! *Compiler support: [requires `rustc` 1.42+][msrv]*
//!
//! [msrv]: #supported-rust-versions
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
//! * `otel.kind`: Set the span kind to one of the supported OpenTelemetry
//! [span kinds]. The value should be a string of any of the supported values:
//! `SERVER`, `CLIENT`, `PRODUCER`, `CONSUMER` or `INTERNAL`. Other values are
//! silently ignored.
//!
//! [span kinds]: https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/api.md#spankind
//!
//! ### Semantic Conventions
//!
//! OpenTelemetry defines conventional names for attributes of common
//! operations. These names can be assigned directly as fields, e.g.
//! `trace_span!("request", "otel.kind" = "client", "http.url" = ..)`, and they
//! will be passed through to your configured OpenTelemetry exporter. You can
//! find the full list of the operations and their expected field names in the
//! [semantic conventions] spec.
//!
//! [semantic conventions]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions
//!
//! ### Stability Status
//!
//! The OpenTelemetry specification is currently in beta so some breaking
//! may still occur on the path to 1.0. You can follow the changes via the
//! [spec repository] to track progress toward stabilization.
//!
//! [spec repository]: https://github.com/open-telemetry/opentelemetry-specification
//!
//! ## Examples
//!
//! ```
//! use opentelemetry::{api::Provider, sdk};
//! use tracing::{error, span};
//! use tracing_subscriber::subscriber::CollectorExt;
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
//! tracing::collector::with_default(subscriber, || {
//!     // Spans will be sent to the configured OpenTelemetry exporter
//!     let root = span!(tracing::Level::TRACE, "app_start", work_units = 2);
//!     let _enter = root.enter();
//!
//!     error!("This event will be logged in the root span.");
//! });
//! ```
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.42. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![deny(unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
#![doc(html_root_url = "https://docs.rs/tracing-opentelemetry/0.8.0")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![cfg_attr(docsrs, deny(broken_intra_doc_links))]

/// Implementation of the trace::Subscriber as a source of OpenTelemetry data.
mod layer;
/// Span extension which enables OpenTelemetry context management.
mod span_ext;
/// Protocols for OpenTelemetry Tracers that are compatible with Tracing
mod tracer;

pub use layer::{layer, OpenTelemetryLayer};
pub use span_ext::OpenTelemetrySpanExt;
pub use tracer::PreSampledTracer;
