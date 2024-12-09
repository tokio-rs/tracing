#![deny(rust_2018_idioms)]

use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::{span, Collect};
use tracing_subscriber::{
    fmt::format::AdditionalFmtSpanFields,
    registry::LookupSpan,
    subscribe::{CollectExt, Context},
    util::SubscriberInitExt,
    Subscribe,
};
#[path = "fmt/yak_shave.rs"]
mod yak_shave;

struct CountingSubscribe(AtomicUsize);

impl<C: Collect + for<'lookup> LookupSpan<'lookup>> Subscribe<C> for CountingSubscribe {
    fn on_new_span(&self, _attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        // Find the span.
        let span = ctx
            .span(id)
            .expect("The span should exist in the registry.");

        // Get its extensions.
        let mut extensions = span.extensions_mut();

        // Find the additional fields in the extensions or create new ones. It's
        // important to always look for a previous value as another layer may
        // have already added some fields.
        let mut additional_fields = extensions
            .remove::<AdditionalFmtSpanFields>()
            .unwrap_or_default();

        // Add something to the fields.
        let ordinal = self.0.fetch_add(1, Ordering::Relaxed);
        additional_fields.insert("ordinal".to_owned(), ordinal.to_string());

        // And don't forget to then put the additional fields into the extensions!
        extensions.insert(additional_fields);
    }
}

fn main() {
    tracing_subscriber::fmt()
        // Use json output...
        .json()
        // Enable additional span fields.
        .with_additional_span_fields(true)
        // Disable things not needed for the example to make the output more readable.
        .with_span_list(false)
        .without_time()
        .with_target(false)
        // Enable all levels.
        .with_max_level(tracing::Level::TRACE)
        // Create the collector...
        .finish()
        // and add our enriching subscriber as another layer.
        // Try removing this and see what changes in the output!
        .with(CountingSubscribe(AtomicUsize::new(0)))
        // Set this to be the default, global collector for this application.
        .init();

    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    tracing::info!(number_of_yaks, "preparing to shave yaks");

    let number_shaved = yak_shave::shave_all(number_of_yaks);
    tracing::info!(
        all_yaks_shaved = number_shaved == number_of_yaks,
        "yak shaving completed."
    );
}
