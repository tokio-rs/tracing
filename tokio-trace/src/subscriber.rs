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
        let parents = event.parents().filter_map(Span::name).collect::<Vec<_>>();
        if logger.enabled(&meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(meta)
                    .module_path(Some(event.static_meta.module_path))
                    .file(Some(event.static_meta.file))
                    .line(Some(event.static_meta.line))
                    .args(format_args!(
                        "[{}] {:?} {}",
                        parents.join(":"),
                        fields,
                        event.message
                    )).build(),
            );
        }
    }

    fn enter(&self, span: &Span, _at: Instant) {
        let logger = log::logger();
        logger.log(&log::Record::builder()
            .args(format_args!("-> {:?}", span.name()))
            .build()
        )
    }
    fn exit(&self, span: &Span, _at: Instant) {
        let logger = log::logger();
        logger.log(&log::Record::builder().args(format_args!("<- {:?}", span.name())).build())
    }
}

impl<'a, 'b> Into<log::Metadata<'a>> for &'b StaticMeta {
    fn into(self) -> log::Metadata<'a> {
        log::Metadata::builder()
            .level(self.level)
            .target(self.target.unwrap_or(""))
            .build()
    }
}
