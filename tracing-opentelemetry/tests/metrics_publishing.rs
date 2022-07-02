use async_trait::async_trait;
use futures_util::{Stream, StreamExt as _};
use opentelemetry::{
    metrics::{Descriptor, InstrumentKind},
    metrics::{Number, NumberKind},
    sdk::{
        export::{
            metrics::{
                CheckpointSet, ExportKind, ExportKindFor, ExportKindSelector,
                Exporter as MetricsExporter, Points, Sum,
            },
            trace::{SpanData, SpanExporter},
        },
        metrics::{
            aggregators::{ArrayAggregator, SumAggregator},
            selectors::simple::Selector,
        },
    },
    Key, Value,
};
use std::cmp::Ordering;
use std::time::Duration;
use tracing::Collect;
use tracing_opentelemetry::OpenTelemetryMetricsSubscriber;
use tracing_subscriber::prelude::*;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTRUMENTATION_LIBRARY_NAME: &str = "tracing/tracing-opentelemetry";

#[tokio::test]
async fn u64_counter_is_exported() {
    let subscriber = init_subscriber(
        "MONOTONIC_COUNTER_HELLO_WORLD".to_string(),
        InstrumentKind::Counter,
        NumberKind::U64,
        Number::from(1_u64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(MONOTONIC_COUNTER_HELLO_WORLD = 1_u64);
    });
}

#[tokio::test]
async fn u64_counter_is_exported_i64_at_instrumentation_point() {
    let subscriber = init_subscriber(
        "MONOTONIC_COUNTER_HELLO_WORLD2".to_string(),
        InstrumentKind::Counter,
        NumberKind::U64,
        Number::from(1_u64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(MONOTONIC_COUNTER_HELLO_WORLD2 = 1_i64);
    });
}

#[tokio::test]
async fn f64_counter_is_exported() {
    let subscriber = init_subscriber(
        "MONOTONIC_COUNTER_FLOAT_HELLO_WORLD".to_string(),
        InstrumentKind::Counter,
        NumberKind::F64,
        Number::from(1.000000123_f64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(MONOTONIC_COUNTER_FLOAT_HELLO_WORLD = 1.000000123_f64);
    });
}

#[tokio::test]
async fn i64_up_down_counter_is_exported() {
    let subscriber = init_subscriber(
        "COUNTER_PEBCAK".to_string(),
        InstrumentKind::UpDownCounter,
        NumberKind::I64,
        Number::from(-5_i64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(COUNTER_PEBCAK = -5_i64);
    });
}

#[tokio::test]
async fn i64_up_down_counter_is_exported_u64_at_instrumentation_point() {
    let subscriber = init_subscriber(
        "COUNTER_PEBCAK2".to_string(),
        InstrumentKind::UpDownCounter,
        NumberKind::I64,
        Number::from(5_i64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(COUNTER_PEBCAK2 = 5_u64);
    });
}

#[tokio::test]
async fn f64_up_down_counter_is_exported() {
    let subscriber = init_subscriber(
        "COUNTER_PEBCAK_BLAH".to_string(),
        InstrumentKind::UpDownCounter,
        NumberKind::F64,
        Number::from(99.123_f64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(COUNTER_PEBCAK_BLAH = 99.123_f64);
    });
}

#[tokio::test]
async fn u64_value_is_exported() {
    let subscriber = init_subscriber(
        "VALUE_ABCDEFG".to_string(),
        InstrumentKind::ValueRecorder,
        NumberKind::U64,
        Number::from(9_u64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(VALUE_ABCDEFG = 9_u64);
    });
}

#[tokio::test]
async fn i64_value_is_exported() {
    let subscriber = init_subscriber(
        "VALUE_ABCDEFG_AUENATSOU".to_string(),
        InstrumentKind::ValueRecorder,
        NumberKind::I64,
        Number::from(-19_i64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(VALUE_ABCDEFG_AUENATSOU = -19_i64);
    });
}

#[tokio::test]
async fn f64_value_is_exported() {
    let subscriber = init_subscriber(
        "VALUE_ABCDEFG_RACECAR".to_string(),
        InstrumentKind::ValueRecorder,
        NumberKind::F64,
        Number::from(777.0012_f64),
    );

    tracing::collect::with_default(subscriber, || {
        tracing::info!(VALUE_ABCDEFG_RACECAR = 777.0012_f64);
    });
}

fn init_subscriber(
    expected_metric_name: String,
    expected_instrument_kind: InstrumentKind,
    expected_number_kind: NumberKind,
    expected_value: Number,
) -> impl Collect + 'static {
    let exporter = TestExporter {
        expected_metric_name,
        expected_instrument_kind,
        expected_number_kind,
        expected_value,
    };

    let push_controller = opentelemetry::sdk::metrics::controllers::push(
        Selector::Exact,
        ExportKindSelector::Stateless,
        exporter,
        tokio::spawn,
        delayed_interval,
    )
    .build();

    tracing_subscriber::registry().with(OpenTelemetryMetricsSubscriber::new(push_controller))
}

#[derive(Clone, Debug)]
struct TestExporter {
    expected_metric_name: String,
    expected_instrument_kind: InstrumentKind,
    expected_number_kind: NumberKind,
    expected_value: Number,
}

#[async_trait]
impl SpanExporter for TestExporter {
    async fn export(
        &mut self,
        mut _batch: Vec<SpanData>,
    ) -> opentelemetry::sdk::export::trace::ExportResult {
        Ok(())
    }
}

impl MetricsExporter for TestExporter {
    fn export(&self, checkpoint_set: &mut dyn CheckpointSet) -> opentelemetry::metrics::Result<()> {
        checkpoint_set.try_for_each(self, &mut |record| {
            assert_eq!(self.expected_metric_name, record.descriptor().name());
            assert_eq!(
                self.expected_instrument_kind,
                *record.descriptor().instrument_kind()
            );
            assert_eq!(
                self.expected_number_kind,
                *record.descriptor().number_kind()
            );
            let number = match self.expected_instrument_kind {
                InstrumentKind::Counter | InstrumentKind::UpDownCounter => record
                    .aggregator()
                    .unwrap()
                    .as_any()
                    .downcast_ref::<SumAggregator>()
                    .unwrap()
                    .sum()
                    .unwrap(),
                InstrumentKind::ValueRecorder => record
                    .aggregator()
                    .unwrap()
                    .as_any()
                    .downcast_ref::<ArrayAggregator>()
                    .unwrap()
                    .points()
                    .unwrap()[0]
                    .clone(),
                _ => panic!(
                    "InstrumentKind {:?} not currently supported!",
                    self.expected_instrument_kind
                ),
            };
            assert_eq!(
                Ordering::Equal,
                number
                    .partial_cmp(&NumberKind::U64, &self.expected_value)
                    .unwrap()
            );

            // The following are the same regardless of the individual metric.
            assert_eq!(
                INSTRUMENTATION_LIBRARY_NAME,
                record.descriptor().instrumentation_library().name
            );
            assert_eq!(
                CARGO_PKG_VERSION,
                record.descriptor().instrumentation_version().unwrap()
            );
            assert_eq!(
                Value::String("unknown_service".into()),
                record
                    .resource()
                    .get(Key::new("service.name".to_string()))
                    .unwrap()
            );

            opentelemetry::metrics::Result::Ok(())
        })
    }
}

impl ExportKindFor for TestExporter {
    fn export_kind_for(&self, _descriptor: &Descriptor) -> ExportKind {
        // I don't think the value here makes a difference since
        // we are just testing a single metric.
        ExportKind::Cumulative
    }
}

// From opentelemetry::sdk::util::
// For some reason I can't pull it in from the other crate, it gives
//   could not find `util` in `sdk`
/// Helper which wraps `tokio::time::interval` and makes it return a stream
fn tokio_interval_stream(period: std::time::Duration) -> tokio_stream::wrappers::IntervalStream {
    tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(period))
}

// https://github.com/open-telemetry/opentelemetry-rust/blob/2585d109bf90d53d57c91e19c758dca8c36f5512/examples/basic-otlp/src/main.rs#L34-L37
// Skip first immediate tick from tokio, not needed for async_std.
fn delayed_interval(duration: Duration) -> impl Stream<Item = tokio::time::Instant> {
    tokio_interval_stream(duration).skip(0)
}
