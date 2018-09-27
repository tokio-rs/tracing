use super::{Event, SpanData, Meta};
use log;
use std::time::Instant;

pub trait Subscriber {
    /// Note that this function is generic over a pair of lifetimes because the
    /// `Event` type is. See the documentation for [`Event`] for details.
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>);
    fn enter(&self, span: &SpanData, at: Instant);
    fn exit(&self, span: &SpanData, at: Instant);
}

pub struct LogSubscriber;

impl LogSubscriber {
    pub fn new() -> Self {
        LogSubscriber
    }
}

impl Subscriber for LogSubscriber {
    fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
        let fields = event.debug_fields();
        let meta = event.meta.into();
        let logger = log::logger();
        let parents = event.parents().filter_map(SpanData::name).collect::<Vec<_>>();
        if logger.enabled(&meta) {
            logger.log(
                &log::Record::builder()
                    .metadata(meta)
                    .module_path(Some(event.meta.module_path))
                    .file(Some(event.meta.file))
                    .line(Some(event.meta.line))
                    .args(format_args!(
                        "[{}] {:?} {}",
                        parents.join(":"),
                        fields,
                        event.message
                    )).build(),
            );
        }
    }

    fn enter(&self, span: &SpanData, _at: Instant) {
        let logger = log::logger();
        logger.log(&log::Record::builder()
            .args(format_args!("-> {:?}", span.name()))
            .build()
        )
    }
    fn exit(&self, span: &SpanData, _at: Instant) {
        let logger = log::logger();
        logger.log(&log::Record::builder().args(format_args!("<- {:?}", span.name())).build())
    }
}

impl<'a, 'b> Into<log::Metadata<'a>> for &'b Meta<'a> {
    fn into(self) -> log::Metadata<'a> {
        log::Metadata::builder()
            .level(self.level)
            .target(self.target.unwrap_or(""))
            .build()
    }
}

#[cfg(test)]
pub use self::test_support::*;
#[cfg(test)]
mod test_support {
    use super::Subscriber;
    use ::{Event, SpanData};
    use ::span::MockSpan;

    use std::{
        cell::RefCell,
        collections::VecDeque,
        time::Instant,
        thread,
    };

    struct ExpectEvent {
        // TODO: implement
    }

    enum Expect {
        Event(ExpectEvent),
        Enter(MockSpan),
        Exit(MockSpan),
    }

    struct Running {
        expected: RefCell<VecDeque<Expect>>,
    }

    pub struct MockSubscriber {
        expected: VecDeque<Expect>,
    }

    pub fn mock() -> MockSubscriber {
        MockSubscriber {
            expected: VecDeque::new(),
        }
    }

    // hack so each test thread can run its own mock subscriber, even though the
    // global dispatcher is static for the lifetime of the whole test binary.
    struct MockDispatch {}

    thread_local! {
        static MOCK_SUBSCRIBER: RefCell<Option<Running>> = RefCell::new(None);
    }

    impl MockSubscriber {
        pub fn enter(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Enter(span
                .with_state(::span::State::Running)));
            self
        }

        pub fn exit(mut self, span: MockSpan) -> Self {
            self.expected.push_back(Expect::Exit(span));
            self
        }

        pub fn run(self) {
            // don't care if this succeeds --- another test may have already
            // installed the test dispatcher.
            let _ = ::Dispatcher::builder()
                .add_subscriber(MockDispatch {})
                .try_init();
            let subscriber = Running {
                expected: RefCell::new(self.expected),
            };
            MOCK_SUBSCRIBER.with(move |mock| {
                *mock.borrow_mut() = Some(subscriber);
            })
        }
    }

    impl Subscriber for Running {
        fn observe_event<'event, 'meta: 'event>(&self, _event: &'event Event<'event, 'meta>) {
            match self.expected.borrow_mut().pop_front() {
                None => {}
                Some(Expect::Event(_)) => unimplemented!(),
                Some(Expect::Enter(expected_span)) => panic!("expected to enter span {:?}, but got an event", expected_span.name),
                Some(Expect::Exit(expected_span)) => panic!("expected to exit span {:?} but got an event", expected_span.name),
            }
        }

        fn enter(&self, span: &SpanData, _at: Instant) {
            println!("+ {}: {:?}", thread::current().name().unwrap_or("unknown thread"), span);
            match self.expected.borrow_mut().pop_front() {
                None => {},
                Some(Expect::Event(_)) => panic!("expected an event, but entered span {:?} instead", span.name()),
                Some(Expect::Enter(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    if let Some(state) = expected_span.state {
                        assert_eq!(state, span.state());
                    }
                    // TODO: expect fields
                }
                Some(Expect::Exit(expected_span)) => panic!(
                    "expected to exit span {:?}, but entered span {:?} instead",
                    expected_span.name,
                    span.name()),
            }
        }

        fn exit(&self, span: &SpanData, _at: Instant) {
            println!("- {}: {:?}", thread::current().name().unwrap_or("unknown_thread"), span);
            match self.expected.borrow_mut().pop_front() {
                None => {},
                Some(Expect::Event(_)) => panic!("expected an event, but exited span {:?} instead", span.name()),
                Some(Expect::Enter(expected_span)) => panic!(
                    "expected to enter span {:?}, but exited span {:?} instead",
                    expected_span.name,
                    span.name()),
                Some(Expect::Exit(expected_span)) => {
                    if let Some(name) = expected_span.name {
                        assert_eq!(name, span.name());
                    }
                    if let Some(state) = expected_span.state {
                        assert_eq!(state, span.state());
                    }
                    // TODO: expect fields
                }
            }
        }
    }

    impl Subscriber for MockDispatch {
        fn observe_event<'event, 'meta: 'event>(&self, event: &'event Event<'event, 'meta>) {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.observe_event(event)
                }
            })
        }

        #[inline]
        fn enter(&self, span: &SpanData, at: Instant) {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.enter(span, at)
                }
            })
        }

        #[inline]
        fn exit(&self, span: &SpanData, at: Instant) {
            MOCK_SUBSCRIBER.with(|mock| {
                if let Some(ref subscriber) = *mock.borrow() {
                    subscriber.exit(span, at)
                }
            })
        }
    }
}
