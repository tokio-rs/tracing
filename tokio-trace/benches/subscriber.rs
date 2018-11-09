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
    fn new_span(&self, span: span::Data) -> span::Id {
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

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
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
    fn new_span(&self, span: span::Data) -> span::Id {
        *self.0.lock().unwrap() = Some(span);
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

    fn should_invalidate_filter(&self, _metadata: &Meta) -> bool {
        false
    }

    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
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

#[bench]
fn span_no_fields(b: &mut Bencher) {
    tokio_trace::Dispatch::to(EnabledSubscriber).as_default(|| b.iter(|| span!("span")));
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
