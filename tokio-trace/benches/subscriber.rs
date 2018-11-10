#![feature(test)]
#[macro_use]
extern crate tokio_trace;
extern crate test;
use test::Bencher;

use std::sync::Mutex;
use tokio_trace::{field, span, subscriber, Event, Meta};

/// A subscriber that is enabled but otherwise does nothing.
struct EnabledSubscriber;

impl tokio_trace::Subscriber for EnabledSubscriber {
    fn new_span(&self, span: span::Attributes) -> span::Id {
        let _ = span;
        span::Id::from_u64(0)
    }
    fn add_value(
        &self,
        span: &span::Id,
        field: &field::Key,
        value: &dyn field::IntoValue,
    ) -> Result<(), tokio_trace::subscriber::AddValueError> {
        let _ = (span, field, value);
        Ok(())
    }

    fn add_follows_from(
        &self,
        span: &span::Id,
        follows: span::Id,
    ) -> Result<(), subscriber::FollowsError> {
        let _ = (span, follows);
        Ok(())
    }

    fn enabled(&self, metadata: &Meta) -> bool {
        let _ = metadata;
        true
    }

    fn observe_event<'event>(&self, event: &'event Event<'event>) {
        let _ = event;
    }

    fn enter(&self, span: span::Id) {
        let _ = span;
    }

    fn exit(&self, span: span::Id) {
        let _ = span;
    }

    fn close(&self, span: span::Id) {
        let _ = span;
    }
}

/// Simulates a subscriber that caches span data.
struct AddData(Mutex<Option<span::Data>>);

impl tokio_trace::Subscriber for AddData {
    fn new_span(&self, span: span::Attributes) -> span::Id {
        *self.0.lock().unwrap() = Some(span.into());
        span::Id::from_u64(0)
    }

    fn add_value(
        &self,
        span: &span::Id,
        field: &field::Key,
        value: &dyn field::IntoValue,
    ) -> Result<(), tokio_trace::subscriber::AddValueError> {
        let _ = span;
        if let Some(data) = self.0.lock().unwrap().as_mut() {
            data.add_value(field, value);
        }
        Ok(())
    }

    fn add_follows_from(
        &self,
        span: &span::Id,
        follows: span::Id,
    ) -> Result<(), subscriber::FollowsError> {
        let _ = (span, follows);
        Ok(())
    }

    fn enabled(&self, metadata: &Meta) -> bool {
        let _ = metadata;
        true
    }

    fn observe_event<'event>(&self, event: &'event Event<'event>) {
        let _ = event;
    }

    fn enter(&self, span: span::Id) {
        let _ = span;
    }

    fn exit(&self, span: span::Id) {
        let _ = span;
    }

    fn close(&self, span: span::Id) {
        let _ = span;
    }
}

const N_SPANS: usize = 100;

#[bench]
fn span_no_fields(b: &mut Bencher) {
    tokio_trace::Dispatch::to(EnabledSubscriber).as_default(|| b.iter(|| span!("span")));
}

#[bench]
fn span_repeatedly(b: &mut Bencher) {
    #[inline]
    fn mk_span(i: usize) -> tokio_trace::Span {
        span!("span", i = &i)
    }

    let n = test::black_box(N_SPANS);
    tokio_trace::Dispatch::to(EnabledSubscriber)
        .as_default(|| b.iter(|| (0..n).fold(mk_span(0), |span, i| mk_span(i))));
}

#[bench]
fn span_with_fields(b: &mut Bencher) {
    tokio_trace::Dispatch::to(EnabledSubscriber).as_default(|| {
        b.iter(|| span!("span", foo = &"foo", bar = &"bar", baz = &3, quuux = &0.99))
    });
}

#[bench]
fn span_with_fields_add_data(b: &mut Bencher) {
    tokio_trace::Dispatch::to(AddData(Mutex::new(None))).as_default(|| {
        b.iter(|| span!("span", foo = &"foo", bar = &"bar", baz = &3, quuux = &0.99))
    });
}
