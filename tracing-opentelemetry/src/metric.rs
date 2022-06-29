use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};
use tracing::field::Visit;
use tracing_core::Field;

use opentelemetry::metrics::{Counter, Meter, UpDownCounter, ValueRecorder};

const METRIC_PREFIX_MONOTONIC_COUNTER: &str = "MONOTONIC_COUNTER_";
const METRIC_PREFIX_COUNTER: &str = "COUNTER_";
const METRIC_PREFIX_VALUE: &str = "VALUE_";

#[derive(Default)]
pub(crate) struct Instruments {
    pub(crate) u64_counter: HashMap<String, Counter<u64>>,
    pub(crate) f64_counter: HashMap<String, Counter<f64>>,
    pub(crate) i64_up_down_counter: HashMap<String, UpDownCounter<i64>>,
    pub(crate) f64_up_down_counter: HashMap<String, UpDownCounter<f64>>,
    pub(crate) u64_value_recorder: HashMap<String, ValueRecorder<u64>>,
    pub(crate) i64_value_recorder: HashMap<String, ValueRecorder<i64>>,
    pub(crate) f64_value_recorder: HashMap<String, ValueRecorder<f64>>,
}

#[derive(Debug)]
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
    pub(crate) fn init_metric_for(
        &mut self,
        meter: &Meter,
        instrument_type: InstrumentType,
        metric_name: String,
    ) {
        match instrument_type {
            InstrumentType::CounterU64(value) => {
                let ctr = self
                    .u64_counter
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.u64_counter(metric_name).init());
                ctr.add(value, &[]);
            }
            InstrumentType::CounterF64(value) => {
                let ctr = self
                    .f64_counter
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.f64_counter(metric_name).init());
                ctr.add(value, &[]);
            }
            InstrumentType::UpDownCounterI64(value) => {
                let ctr = self
                    .i64_up_down_counter
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.i64_up_down_counter(metric_name).init());
                ctr.add(value, &[]);
            }
            InstrumentType::UpDownCounterF64(value) => {
                let ctr = self
                    .f64_up_down_counter
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.f64_up_down_counter(metric_name).init());
                ctr.add(value, &[]);
            }
            InstrumentType::ValueRecorderU64(value) => {
                let rec = self
                    .u64_value_recorder
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.u64_value_recorder(metric_name).init());
                rec.record(value, &[]);
            }
            InstrumentType::ValueRecorderI64(value) => {
                let rec = self
                    .i64_value_recorder
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.i64_value_recorder(metric_name).init());
                rec.record(value, &[]);
            }
            InstrumentType::ValueRecorderF64(value) => {
                let rec = self
                    .f64_value_recorder
                    .entry(metric_name.clone())
                    .or_insert_with(|| meter.f64_value_recorder(metric_name).init());
                rec.record(value, &[]);
            }
        };
    }
}

pub(crate) struct MetricVisitor<'a> {
    pub(crate) instruments: &'a Arc<RwLock<Instruments>>,
    pub(crate) meter: &'a Meter,
}

// impl<'a> Visit for MetricVisitor<'a> {
impl<'a> Visit for MetricVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {
        // Do nothing
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name().starts_with(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::CounterU64(value),
                field.name().to_string(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_VALUE) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::ValueRecorderU64(value),
                field.name().to_string(),
            );
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name().starts_with(METRIC_PREFIX_MONOTONIC_COUNTER) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::CounterF64(value),
                field.name().to_string(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_COUNTER) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::UpDownCounterF64(value),
                field.name().to_string(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_VALUE) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::ValueRecorderF64(value),
                field.name().to_string(),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name().starts_with(METRIC_PREFIX_COUNTER) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::UpDownCounterI64(value),
                field.name().to_string(),
            );
        } else if field.name().starts_with(METRIC_PREFIX_VALUE) {
            self.instruments.write().unwrap().init_metric_for(
                self.meter,
                InstrumentType::ValueRecorderI64(value),
                field.name().to_string(),
            );
        }
    }
}
