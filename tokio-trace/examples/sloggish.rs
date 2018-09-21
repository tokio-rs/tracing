#[macro_use]
extern crate tokio_trace_prototype as tokio_trace;
extern crate ansi_term;
use ansi_term::Style;

use std::{
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
};

struct SloggishSubscriber {
    indent: AtomicUsize,
    indent_amount: usize,
}

impl SloggishSubscriber {
    fn print_kvs<I>(&self, writer: &mut dyn Write, kvs: I) -> fmt::Result<()>
    where
        I: IntoIterator<Item=(&'static str, &dyn tokio_trace::Value)>,
    {
        for (k, v) in kvs {
            write!(writer, "{}: {:?}", Style::new().bold().paint(k), v)?;
        }
        Ok(())
    }

    fn print_meta(&self, writer: &mut dyn Write, meta: &tokio_trace::StaticMeta) {
        write!(writer, "")
    }
}

impl tokio_trace::Subscriber for SloggishSubscriber {
    #[inline]
    fn observe_event<'event>(&self, event: &'event Event<'event>) {
    }

    #[inline]
    fn enter(&self, span: &Span, at: Instant) {
        self.
    }

    #[inline]
    fn exit(&self, span: &Span, at: Instant) {
    }
}

fn main() {

}
