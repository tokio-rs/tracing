use std::{collections::HashMap, fmt, sync::RwLock};
use tracing::{field::Visit, Collect};
use tracing_core::Field;

use opentelemetry::{
    metrics::{Counter, Meter, MeterProvider, UpDownCounter, ValueRecorder},
    sdk::metrics::PushController,
};
use tracing_subscriber::{registry::LookupSpan, subscribe::Context, Subscribe};

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTRUMENTATION_LIBRARY_NAME: &str = "tracing/tracing-opentelemetry";

const METRIC_PREFIX_MONOTONIC_COUNTER: &str = "MONOTONIC_COUNTER_";
const METRIC_PREFIX_COUNTER: &str = "COUNTER_";
const METRIC_PREFIX_VALUE: &str = "VALUE_";
const I64_MAX: u64 = i64::MAX as u64;

#[derive(Default)]
pub(crate) struct Instruments {
    u64_counter: MetricsMap<Counter<u64>>,
    f64_counter: MetricsMap<Counter<f64>>,
    i64_up_down_counter: MetricsMap<UpDownCounter<i64>>,
    f64_up_down_counter: MetricsMap<UpDownCounter<f64>>,
    u64_value_recorder: MetricsMap<ValueRecorder<u64>>,
    i64_value_recorder: MetricsMap<ValueRecorder<i64>>,
    f64_value_recorder: MetricsMap<ValueRecorder<f64>>,
}

type MetricsMap<T> = RwLock<HashMap<String, T>>;

#[derive(Copy, Clone, Debug)]
pub(crate) enum InstrumentType {
    CounterU64(u64),
    CounterF64(f64),
    UpDownCounterI64(i64),
    UpDownCounterF64(f64),
    ValueRecorderU64(u64),
    ValueRecorderI64(i64),
    ValueRecorderF64(f64),
}

impl Instruments {
    pub(crate) fn update_metric(
        &self,
        meter: &Meter,
        instrument_type: InstrumentType,
        metric_name: &str,
    ) {
        fn update_or_insert<T>(
            map: &MetricsMap<T>,
            name: &str,
            insert: impl FnOnce() -> T,
            update: impl FnOnce(&T),
        ) {
            let lock = map.read().unwrap();
            if let Some(metric) = lock.get(name) {
                update(metric);
                return;
            }

            // that metric did not already exist, so we have to acquire a write lock to
            // create it.
            let mut lock = map.write().unwrap();
            // handle the case where the entry was created while we were waiting to
            // acquire the write lock
            let metric = lock.entry(name.to_owned()).or_insert_with(insert);
            update(metric)
        }

        match instrument_type {
            InstrumentType::CounterU64(value) => {
                update_or_insert(
                    &self.u64_counter,
                    metric_name,
                    || meter.u64_counter(metric_name).init(),
                    |ctr| ctr.add(value, &[]),
                );
            }
            InstrumentType::CounterF64(value) => {
                update_or_insert(
                    &self.f64_counter,
                    metric_name,
                    || meter.f64_counter(metric_name).init(),
                    |ctr| ctr.add(value, &[]),
                );
            }
            InstrumentType::UpDownCounterI64(value) => {
                update_or_insert(
                    &self.i64_up_down_counter,
                    metric_name,
                    || meter.i64_up_down_counter(metric_name).init(),
                    |ctr| ctr.add(value, &[]),
                );
            }
            InstrumentType::UpDownCounterF64(value) => {
                update_or_insert(
                    &self.f64_up_down_counter,
                    metric_name,
                    || meter.f64_up_down_counter(metric_name).init(),
                    |ctr| ctr.add(value, &[]),
                );
            }
            InstrumentType::ValueRecorderU64(value) => {
                update_or_insert(
                    &self.u64_value_recorder,
                    metric_name,
                    || meter.u64_value_recorder(metric_name).init(),
                    |rec| rec.record(value, &[]),
                );
            }
            InstrumentType::ValueRecorderI64(value) => {
                update_or_insert(
                    &self.i64_value_recorder,
                    metric_name,
                    || meter.i64_value_recorder(metric_name).init(),
                    |rec| rec.record(value, &[]),
                );
            }
            InstrumentType::ValueRecorderF64(value) => {
                update_or_insert(
                    &self.f64_value_recorder,
                    metric_name,
                    || meter.f64_value_recorder(metric_name).init(),
                    |rec| rec.record(value, &[]),
                );
            }
        };
    }
}

pub(crate) struct MetricVisitor<'a> {
    pub(crate) instruments: &'a Instruments,
    pub(crate) meter: &'a Meter,
}

impl<'a> Visit for MetricVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {
        // Do nothing
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name().starts_with(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::CounterU64(value),
                field.name(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_COUNTER) {
            if value <= I64_MAX {
                self.instruments.update_metric(
                    self.meter,
                    InstrumentType::UpDownCounterI64(value as i64),
                    field.name(),
                );
            } else {
                eprintln!(
                    "[tracing-opentelemetry]: Received Counter metric, but \
                    provided u64: {} is greater than i64::MAX. Ignoring \
                    this metric.",
                    value
                );
            }
        } else if field.name().starts_with(METRIC_PREFIX_VALUE) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::ValueRecorderU64(value),
                field.name(),
            );
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name().starts_with(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::CounterF64(value),
                field.name(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_COUNTER) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::UpDownCounterF64(value),
                field.name(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_VALUE) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::ValueRecorderF64(value),
                field.name(),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name().starts_with(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::CounterU64(value as u64),
                field.name(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_COUNTER) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::UpDownCounterI64(value),
                field.name(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_VALUE) {
            self.instruments.update_metric(
                self.meter,
                InstrumentType::ValueRecorderI64(value),
                field.name(),
            );
        }
    }
}

/// A subscriber that publishes metrics via the OpenTelemetry SDK.
///
/// # Usage
///
/// No configuration is needed for this Subscriber, as it's only responsible for
/// pushing data out to the `opentelemetry` family of crates. For example, when
/// using `opentelemetry-otlp`, that crate will provide its own set of
/// configuration options for setting up the duration metrics will be collected
/// before exporting to the OpenTelemetry Collector, aggregation of data points,
/// etc.
///
/// ```no_run
/// use tracing_opentelemetry::OpenTelemetryMetricsSubscriber;
/// use tracing_subscriber::subscribe::CollectExt;
/// use tracing_subscriber::Registry;
/// # use opentelemetry::sdk::metrics::PushController;
///
/// // Constructing a PushController is out-of-scope for the docs here, but there
/// // are examples in the opentelemetry repository. See:
/// // https://github.com/open-telemetry/opentelemetry-rust/blob/c13a11e62a68eacd8c41a0742a0d097808e28fbd/examples/basic-otlp/src/main.rs#L39-L53
/// # let push_controller: PushController = unimplemented!();
///
/// let opentelemetry_metrics =  OpenTelemetryMetricsSubscriber::new(push_controller);
/// let collector = Registry::default().with(opentelemetry_metrics);
/// tracing::collect::set_global_default(collector).unwrap();
/// ```
///
/// To publish a new metric from your instrumentation point, all that is needed
/// is to add a key-value pair to your `tracing::Event` that contains a specific
/// prefix. One of the following:
/// - `MONOTONIC_COUNTER_` (non-negative numbers): Used when the counter should
///   only ever increase
/// - `COUNTER_`: Used when the counter can go up or down
/// - `VALUE_`: Used for discrete data points (i.e., summing them does not make
///   semantic sense)
///
/// Examples:
/// ```
/// # use tracing::info;
/// info!(MONOTONIC_COUNTER_FOO = 1);
/// info!(MONOTONIC_COUNTER_BAR = 1.1);
///
/// info!(COUNTER_BAZ = 1);
/// info!(COUNTER_BAZ = -1);
/// info!(COUNTER_XYZ = 1.1);
///
/// info!(VALUE_QUX = 1);
/// info!(VALUE_ABC = -1);
/// info!(VALUE_DEF = 1.1);
/// ```
///
/// # Mixing data types
///
/// ## Floating-point numbers
///
/// Do not mix floating-point and non-floating-point numbers for the same
/// metric. Instead, if you are ever going to use floating-point numbers for a
/// metric, cast any other usages of that metric to `f64`.
///
/// Don't do this:
/// ```
/// # use tracing::info;
/// info!(MONOTONIC_COUNTER_FOO = 1);
/// info!(MONOTONIC_COUNTER_FOO = 1.0);
/// ```
///
/// Do this:
/// ```
/// # use tracing::info;
/// info!(MONOTONIC_COUNTER_FOO = 1 as f64);
/// info!(MONOTONIC_COUNTER_FOO = 1.1);
/// ```
///
/// This is because all data published for a given metric name must be the same
/// numeric type.
///
/// ## Integers
///
/// Positive and negative integers can be mixed freely. The instrumentation
/// provided by `tracing` assumes that all integers are `i64` unless explicitly
/// cast to something else. In the case that an integer *is* cast to `u64`, this
/// subscriber will handle the conversion internally.
///
/// To illustrate this:
/// ```
/// # use tracing::info;
/// // The subscriber receives an i64
/// info!(COUNTER_BAZ = 1);
///
/// // The subscriber receives an i64
/// info!(COUNTER_BAZ = -1);
///
/// // The subscriber receives a u64, but casts it to i64 internally
/// info!(COUNTER_BAZ = 1 as u64);
///
/// // The subscriber receives a u64, but cannot cast it to i64 because of
/// // overflow. An error is printed to stderr, and the metric is dropped.
/// info!(COUNTER_BAZ = (i64::MAX as u64) + 1)
/// ```
///
/// # Under-the-hood
///
/// This subscriber holds a set of maps, each map corresponding to a type of
/// metric supported by OpenTelemetry. These maps are populated lazily. The
/// first time that a metric is emitted by the instrumentation, a `Metric`
/// instance will be created and added to the corresponding map. This means that
/// any time a metric is emitted by the instrumentation, one map lookup has to
/// be performed.
///
/// In the future, this can be improved by associating each Metric instance to
/// its callsite. However, per-callsite storage is not yet supported by tracing.
pub struct OpenTelemetryMetricsSubscriber {
    meter: Meter,
    instruments: Instruments,
}

impl OpenTelemetryMetricsSubscriber {
    /// Create a new instance of OpenTelemetryMetricsSubscriber.
    pub fn new(push_controller: PushController) -> Self {
        let meter = push_controller
            .provider()
            .meter(INSTRUMENTATION_LIBRARY_NAME, Some(CARGO_PKG_VERSION));
        OpenTelemetryMetricsSubscriber {
            meter,
            instruments: Default::default(),
        }
    }
}

impl<C> Subscribe<C> for OpenTelemetryMetricsSubscriber
where
    C: Collect + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, C>) {
        let mut metric_visitor = MetricVisitor {
            instruments: &self.instruments,
            meter: &self.meter,
        };
        event.record(&mut metric_visitor);
    }
}
