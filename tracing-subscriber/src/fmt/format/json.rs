use super::{Format, FormatEvent, FormatFields, FormatTime};
use crate::{
    field::MakeVisitor,
    fmt::fmt_layer::FmtContext,
    fmt::fmt_layer::FormattedFields,
    registry::{LookupMetadata, LookupSpan},
};
use serde::{
    ser::{SerializeMap, Serializer as _},
    Serialize,
};
use serde_json::{Serializer, Value};
use std::{
    collections::BTreeMap,
    fmt::{self, Write},
    io,
};
use tracing_core::{
    field::{self, Field},
    Event, Subscriber,
};
use tracing_serde::fields::SerializeFieldMap;

#[cfg(feature = "tracing-log")]
use tracing_log::NormalizeEvent;

/// Marker for `Format` that indicates that the verbose json log format should be used.
///
/// The full format includes fields from all entered spans.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct Json;

#[derive(Serialize, Default)]
struct SerializedEvent<'a> {
    timestamp: Option<&'a str>,
    level: Option<String>,
    target: Option<&'a str>,
    fields: Option<&'a SerializeFieldMap<'a, Event<'a>>>,
    parent_spans: Vec<Value>,
}

impl<S, N, T> FormatEvent<S, N> for Format<Json, T>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup> + LookupMetadata,
    N: for<'writer> FormatFields<'writer> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: &mut dyn fmt::Write,
        event: &Event<'_>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a> + LookupMetadata,
    {
        use serde_json::json;
        use tracing_serde::fields::AsMap;
        let mut timestamp = String::new();
        self.timer.format_time(&mut timestamp)?;

        #[cfg(feature = "tracing-log")]
        let normalized_meta = event.normalized_metadata();
        #[cfg(feature = "tracing-log")]
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());
        #[cfg(not(feature = "tracing-log"))]
        let meta = event.metadata();

        let mut visit = || {
            let ctx = &ctx.ctx;

            let mut serialized_event = SerializedEvent::default();

            for span in ctx.scope() {
                let id = span.id();
                let span = ctx
                    .span(&id)
                    .expect(&format!("Missing span for id: {:?}", id));

                let ext = span.extensions();
                let data = ext
                    .get::<FormattedFields<N>>()
                    .expect("Unable to find FormattedFields in extensions; this is a bug");
                // TODO: let's _not_ do this, but this resolves
                // https://github.com/tokio-rs/tracing/issues/391.
                // We should probably rework this to use a `serde_json::Value` or something
                // similar in a JSON-specific layer, but I'd (david)
                // rather have a uglier fix now rather than shipping broken JSON.
                let mut fields: Value = match serde_json::from_str(&data) {
                    Ok(fields) => fields,
                    Err(_) => return,
                };
                fields["name"] = json!(span.metadata().name());
                serialized_event.parent_spans.push(fields);
            }
            serialized_event.timestamp = Some(&timestamp);
            serialized_event.level = Some(meta.level().to_string());

            if self.display_target {
                serialized_event.target = Some(meta.target());
            }

            let map = &event.field_map();
            serialized_event.fields = Some(map);
            serde_json::to_writer(WriteAdaptor::new(writer), &serialized_event).unwrap();
        };

        visit();
        writeln!(writer)
    }
}

/// The JSON [`FormatFields`] implementation.
///
/// [`FormatFields`]: trait.FormatFields.html
#[derive(Debug)]
pub struct JsonFields {
    // reserve the ability to add fields to this without causing a breaking
    // change in the future.
    _private: (),
}

impl JsonFields {
    /// Returns a new JSON [`FormatFields`] implementation.
    ///
    /// [`FormatFields`]: trait.FormatFields.html
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for JsonFields {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MakeVisitor<&'a mut dyn Write> for JsonFields {
    type Visitor = JsonVisitor<'a>;

    #[inline]
    fn make_visitor(&self, target: &'a mut dyn Write) -> Self::Visitor {
        JsonVisitor::new(target)
    }
}

/// The [visitor] produced by [`JsonFields`]'s [`MakeVisitor`] implementation.
///
/// [visitor]: ../../field/trait.Visit.html
/// [`JsonFields`]: struct.JsonFields.html
/// [`MakeVisitor`]: ../../field/trait.MakeVisitor.html
pub struct JsonVisitor<'a> {
    values: BTreeMap<&'a str, serde_json::Value>,
    writer: &'a mut dyn Write,
}

impl<'a> fmt::Debug for JsonVisitor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("JsonVisitor {{ values: {:?} }}", self.values))
    }
}

impl<'a> JsonVisitor<'a> {
    /// Returns a new default visitor that formats to the provided `writer`.
    ///
    /// # Arguments
    /// - `writer`: the writer to format to.
    /// - `is_empty`: whether or not any fields have been previously written to
    ///   that writer.
    pub fn new(writer: &'a mut dyn Write) -> Self {
        Self {
            values: BTreeMap::new(),
            writer,
        }
    }
}

impl<'a> crate::field::VisitFmt for JsonVisitor<'a> {
    fn writer(&mut self) -> &mut dyn fmt::Write {
        self.writer
    }
}

impl<'a> crate::field::VisitOutput<fmt::Result> for JsonVisitor<'a> {
    fn finish(self) -> fmt::Result {
        let inner = || {
            let mut serializer = Serializer::new(WriteAdaptor::new(self.writer));
            let mut ser_map = serializer.serialize_map(None)?;

            for (k, v) in self.values {
                ser_map.serialize_entry(k, &v)?;
            }

            SerializeMap::end(ser_map)
        };

        if inner().is_err() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

impl<'a> field::Visit for JsonVisitor<'a> {
    /// Visit a signed 64-bit integer value.
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    /// Visit an unsigned 64-bit integer value.
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    /// Visit a boolean value.
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    /// Visit a string value.
    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(&field.name(), serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        match field.name() {
            // Skip fields that are actually log metadata that have already been handled
            #[cfg(feature = "tracing-log")]
            name if name.starts_with("log.") => (),
            name if name.starts_with("r#") => {
                self.values
                    .insert(&name[2..], serde_json::Value::from(format!("{:?}", value)));
            }
            name => {
                self.values
                    .insert(name, serde_json::Value::from(format!("{:?}", value)));
            }
        };
    }
}

/// A bridge between `fmt::Write` and `io::Write`.
///
/// This is needed because tracing-subscriber's FormatEvent expects a fmt::Write
/// while serde_json's Serializer expects an io::Write.
struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn fmt::Write,
}

impl<'a> WriteAdaptor<'a> {
    fn new(fmt_write: &'a mut dyn fmt::Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(&s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> fmt::Debug for WriteAdaptor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("WriteAdaptor { .. }")
    }
}

#[cfg(test)]
mod test {
    use crate::fmt::{test::MockWriter, time::FormatTime};
    use lazy_static::lazy_static;
    use tracing::{self, subscriber::with_default};

    use std::{fmt, sync::Mutex};

    struct MockTime;
    impl FormatTime for MockTime {
        fn format_time(&self, w: &mut dyn fmt::Write) -> fmt::Result {
            write!(w, "fake time")
        }
    }

    #[test]
    fn json() {
        lazy_static! {
            static ref BUF: Mutex<Vec<u8>> = Mutex::new(vec![]);
        }

        let make_writer = || MockWriter::new(&BUF);

        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3},\"target\":\"tracing_subscriber::fmt::format::json::test\",\"fields\":{\"message\":\"some json test\"}}\n";

        test_json(make_writer, expected, &BUF);
    }

    #[cfg(feature = "json")]
    fn test_json<T>(make_writer: T, expected: &str, buf: &Mutex<Vec<u8>>)
    where
        T: crate::fmt::MakeWriter + Send + Sync + 'static,
    {
        let subscriber = crate::fmt::Subscriber::builder()
            .json()
            .with_writer(make_writer)
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            tracing::info!("some json test");
        });

        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        assert_eq!(expected, actual.as_str());
    }
}
