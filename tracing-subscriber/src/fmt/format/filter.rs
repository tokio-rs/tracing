use core::fmt;

use tracing_core::{Collect, Event};

use crate::{fmt::FmtContext, registry::LookupSpan};

use super::{FormatEvent, FormatFields, Writer};

/// `FilteringFormatter` is useful if you want to not filter the entire event but only want to not display it
/// ```
/// use tracing_core::Event;
/// use tracing_subscriber::fmt::format::{FilteringFormatter, Format};
/// tracing_subscriber::fmt::fmt()
/// .event_format(FilteringFormatter::new(
///     Format::default().pretty(),
///     // Do not display the event if an attribute name starts with "counter"
///     |event: &Event| !event.metadata().fields().iter().any(|f| f.name().starts_with("counter")),
/// ))
/// .finish();
/// ```
#[derive(Debug)]
pub struct FilteringFormatter<T, F> {
    inner: T,
    filter_fn: F,
}

impl<T, F> FilteringFormatter<T, F>
where
    F: Fn(&Event<'_>) -> bool,
{
    /// Creates a new FilteringFormatter
    pub fn new(inner: T, filter_fn: F) -> Self {
        Self { inner, filter_fn }
    }
}

impl<T, F, S, N> FormatEvent<S, N> for FilteringFormatter<T, F>
where
    T: FormatEvent<S, N>,
    F: Fn(&Event<'_>) -> bool,
    S: Collect + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        if (self.filter_fn)(event) {
            self.inner.format_event(ctx, writer, event)
        } else {
            Ok(())
        }
    }
}
