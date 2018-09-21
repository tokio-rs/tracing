use super::{Event, Span, StaticMeta};
use log;
use std::time::Instant;

pub trait Subscriber {
    fn observe_event<'event>(&self, event: &'event Event<'event>);
    fn enter(&self, span: &Span, at: Instant);
    fn exit(&self, span: &Span, at: Instant);
}

pub struct LogSubscriber;

impl LogSubscriber {
    pub fn new() -> Self {
        LogSubscriber
    }
}

impl Subscriber for LogSubscriber {
    fn observe_event<'event>(&self, event: &'event Event<'event>) {
        let fields = event.debug_fields();
        let meta = event.static_meta.into();
        let logger = log::logger();
        if logger.enabled(&meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(meta)
                    .module_path(Some(event.static_meta.module_path))
                    .file(Some(event.static_meta.file))
                    .line(Some(event.static_meta.line))
                    .args(format_args!(
                        "[{}] {:?} {}",
                        event.parent.name().unwrap_or("???"),
                        fields,
                        event.message
                    )).build(),
            );
        }
    }

    fn enter(&self, _span: &Span, _at: Instant) {}
    fn exit(&self, _span: &Span, _at: Instant) {}
}

impl<'a, 'b> Into<log::Metadata<'a>> for &'b StaticMeta {
    fn into(self) -> log::Metadata<'a> {
        log::Metadata::builder()
            .level(self.level)
            .target(self.target.unwrap_or(""))
            .build()
    }
}
