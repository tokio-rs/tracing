#![cfg(feature = "std")]
//! Tests that the `VisitDelimited` wrapper produced by
//! [`MakeExt::delimited`] forwards every typed field value to the inner
//! visitor, rather than downgrading it to `record_debug`.
//!
//! `DefaultVisitor::record_error` renders an error's `source` chain as
//! `<field>.sources=[...]`. If `VisitDelimited` does not forward
//! `record_error`, that chain is silently dropped from the formatted output.
use std::error::Error;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};

use tracing_subscriber::field::MakeExt;
use tracing_subscriber::fmt::format::DefaultFields;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;

/// An in-memory `MakeWriter` that collects everything written to it.
#[derive(Clone, Default)]
struct MockMakeWriter(Arc<Mutex<Vec<u8>>>);

impl MockMakeWriter {
    fn contents(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl std::io::Write for MockMakeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for MockMakeWriter {
    type Writer = MockMakeWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// An error with a `source`, so the default formatter has a source chain to
/// render.
#[derive(Debug)]
struct Outer {
    source: IoError,
}

impl std::fmt::Display for Outer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "outer")
    }
}

impl Error for Outer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// A `VisitDelimited`-wrapped `DefaultVisitor` must still reach
/// `DefaultVisitor::record_error`, so the error's `source` chain is rendered.
#[test]
fn delimited_fields_render_error_sources() {
    let writer = MockMakeWriter::default();
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(writer.clone())
        .map_fmt_fields(|fields: DefaultFields| fields.delimited(" | "));

    let err: Box<dyn Error + Send + Sync + 'static> = Box::new(Outer {
        source: IoError::new(ErrorKind::Other, "inner"),
    });
    tracing::subscriber::with_default(
        tracing_subscriber::registry().with(layer),
        || tracing::info!(error = &err),
    );

    let output = writer.contents();
    assert!(
        output.contains("error.sources="),
        "expected the error source chain in delimited output, got:\n{}",
        output
    );
}

/// The delimiter must still be applied exactly once between fields.
#[test]
fn delimited_fields_still_separate_fields() {
    let writer = MockMakeWriter::default();
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(writer.clone())
        .map_fmt_fields(|fields: DefaultFields| fields.delimited(" | "));

    tracing::subscriber::with_default(tracing_subscriber::registry().with(layer), || {
        tracing::info!(a = 1u32, b = 2u32)
    });

    let output = writer.contents();
    // `DefaultVisitor` already separates fields with a single space, so the
    // delimiter is appended after that space: `a=1` + ` ` + `| ` + ` b=2`.
    assert!(
        output.contains("a=1 |  b=2"),
        "expected delimited fields in output, got:\n{}",
        output
    );
}
