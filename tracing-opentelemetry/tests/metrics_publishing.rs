use opentelemetry::{
    metrics::MetricsError,
    sdk::{
        export::metrics::{
            aggregation::{self, Histogram, Sum, TemporalitySelector},
            InstrumentationLibraryReader,
        },
        metrics::{
            aggregators::{HistogramAggregator, SumAggregator},
            controllers::BasicController,
            processors,
            sdk_api::{Descriptor, InstrumentKind, Number, NumberKind},
            selectors,
        },
    },
    Context,
};
use std::cmp::Ordering;
use tracing::Subscriber;
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::prelude::*;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTRUMENTATION_LIBRARY_NAME: &str = "tracing/tracing-opentelemetry";

#[tokio::test]
async fn u64_counter_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "hello_world".to_string(),
        InstrumentKind::Counter,
        NumberKind::U64,
        Number::from(1_u64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(monotonic_counter.hello_world = 1_u64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn u64_counter_is_exported_i64_at_instrumentation_point() {
    let (subscriber, exporter) = init_subscriber(
        "hello_world2".to_string(),
        InstrumentKind::Counter,
        NumberKind::U64,
        Number::from(1_u64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(monotonic_counter.hello_world2 = 1_i64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn f64_counter_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "float_hello_world".to_string(),
        InstrumentKind::Counter,
        NumberKind::F64,
        Number::from(1.000000123_f64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(monotonic_counter.float_hello_world = 1.000000123_f64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn i64_up_down_counter_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "pebcak".to_string(),
        InstrumentKind::UpDownCounter,
        NumberKind::I64,
        Number::from(-5_i64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(counter.pebcak = -5_i64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn i64_up_down_counter_is_exported_u64_at_instrumentation_point() {
    let (subscriber, exporter) = init_subscriber(
        "pebcak2".to_string(),
        InstrumentKind::UpDownCounter,
        NumberKind::I64,
        Number::from(5_i64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(counter.pebcak2 = 5_u64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn f64_up_down_counter_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "pebcak_blah".to_string(),
        InstrumentKind::UpDownCounter,
        NumberKind::F64,
        Number::from(99.123_f64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(counter.pebcak_blah = 99.123_f64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn u64_histogram_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "abcdefg".to_string(),
        InstrumentKind::Histogram,
        NumberKind::U64,
        Number::from(9_u64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(value.abcdefg = 9_u64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn i64_histogram_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "abcdefg_auenatsou".to_string(),
        InstrumentKind::Histogram,
        NumberKind::I64,
        Number::from(-19_i64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(value.abcdefg_auenatsou = -19_i64);
    });

    exporter.export().unwrap();
}

#[tokio::test]
async fn f64_histogram_is_exported() {
    let (subscriber, exporter) = init_subscriber(
        "abcdefg_racecar".to_string(),
        InstrumentKind::Histogram,
        NumberKind::F64,
        Number::from(777.0012_f64),
    );

    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(value.abcdefg_racecar = 777.0012_f64);
    });

    exporter.export().unwrap();
}

fn init_subscriber(
    expected_metric_name: String,
    expected_instrument_kind: InstrumentKind,
    expected_number_kind: NumberKind,
    expected_value: Number,
) -> (impl Subscriber + 'static, TestExporter) {
    let controller = opentelemetry::sdk::metrics::controllers::basic(processors::factory(
        selectors::simple::histogram(vec![-10.0, 100.0]),
        aggregation::cumulative_temporality_selector(),
    ))
    .build();

    let exporter = TestExporter {
        expected_metric_name,
        expected_instrument_kind,
        expected_number_kind,
        expected_value,
        controller: controller.clone(),
    };

    (
        tracing_subscriber::registry().with(MetricsLayer::new(controller)),
        exporter,
    )
}

#[derive(Clone, Debug)]
struct TestExporter {
    expected_metric_name: String,
    expected_instrument_kind: InstrumentKind,
    expected_number_kind: NumberKind,
    expected_value: Number,
    controller: BasicController,
}

impl TestExporter {
    fn export(&self) -> Result<(), MetricsError> {
        self.controller.collect(&Context::current())?;
        self.controller.try_for_each(&mut |library, reader| {
            reader.try_for_each(self, &mut |record| {
                assert_eq!(self.expected_metric_name, record.descriptor().name());
                assert_eq!(
                    self.expected_instrument_kind,
                    *record.descriptor().instrument_kind()
                );
                assert_eq!(
                    self.expected_number_kind,
                    *record.descriptor().number_kind()
                );
                match self.expected_instrument_kind {
                    InstrumentKind::Counter | InstrumentKind::UpDownCounter => {
                        let number = record
                            .aggregator()
                            .unwrap()
                            .as_any()
                            .downcast_ref::<SumAggregator>()
                            .unwrap()
                            .sum()
                            .unwrap();

                        assert_eq!(
                            Ordering::Equal,
                            number
                                .partial_cmp(&NumberKind::U64, &self.expected_value)
                                .unwrap()
                        );
                    }
                    InstrumentKind::Histogram => {
                        let histogram = record
                            .aggregator()
                            .unwrap()
                            .as_any()
                            .downcast_ref::<HistogramAggregator>()
                            .unwrap()
                            .histogram()
                            .unwrap();

                        let counts = histogram.counts();
                        if dbg!(self.expected_value.to_i64(&self.expected_number_kind)) > 100 {
                            assert_eq!(counts, &[0.0, 0.0, 1.0]);
                        } else if self.expected_value.to_i64(&self.expected_number_kind) > 0 {
                            assert_eq!(counts, &[0.0, 1.0, 0.0]);
                        } else {
                            assert_eq!(counts, &[1.0, 0.0, 0.0]);
                        }
                    }
                    _ => panic!(
                        "InstrumentKind {:?} not currently supported!",
                        self.expected_instrument_kind
                    ),
                };

                // The following are the same regardless of the individual metric.
                assert_eq!(INSTRUMENTATION_LIBRARY_NAME, library.name);
                assert_eq!(CARGO_PKG_VERSION, library.version.as_ref().unwrap());

                Ok(())
            })
        })
    }
}

impl TemporalitySelector for TestExporter {
    fn temporality_for(
        &self,
        _descriptor: &Descriptor,
        _kind: &aggregation::AggregationKind,
    ) -> aggregation::Temporality {
        // I don't think the value here makes a difference since
        // we are just testing a single metric.
        aggregation::Temporality::Cumulative
    }
}
