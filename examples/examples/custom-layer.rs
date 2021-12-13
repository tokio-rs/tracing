use tracing_subscriber::prelude::*;
use tracing_subscriber::{layer::Layer, registry::LookupSpan};

#[derive(Default)]
struct DiscordLayer;

impl<S> Layer<S> for DiscordLayer
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(
        &self,
        attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let _ = (attrs, id, ctx);
    }

    fn on_record(
        &self,
        _span: &tracing_core::span::Id,
        _values: &tracing_core::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_event(
        &self,
        _event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_enter(
        &self,
        _id: &tracing_core::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_exit(
        &self,
        _id: &tracing_core::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }

    fn on_close(
        &self,
        _id: tracing_core::span::Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
    }
}

fn main() {
    let discord_layer = DiscordLayer::default();

    tracing_subscriber::registry().with(discord_layer).init();
}
