#![allow(missing_docs)]
use super::{
    event::MockEvent,
    field as mock_field,
    span::{MockSpan, NewSpan},
};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tracing::{
    collect::Interest,
    level_filters::LevelFilter,
    span::{self, Attributes, Id},
    Collect, Event, Metadata,
};

#[derive(Debug, Eq, PartialEq)]
pub enum Expect {
    Event(MockEvent),
    FollowsFrom {
        consequence: MockSpan,
        cause: MockSpan,
    },
    Enter(MockSpan),
    Exit(MockSpan),
    CloneSpan(MockSpan),
    DropSpan(MockSpan),
    Visit(MockSpan, mock_field::Expect),
    NewSpan(NewSpan),
    Nothing,
}

struct SpanState {
    name: &'static str,
    refs: usize,
    meta: &'static Metadata<'static>,
}

struct Running<F: Fn(&Metadata<'_>) -> bool> {
    spans: Mutex<HashMap<Id, SpanState>>,
    expected: Arc<Mutex<VecDeque<Expect>>>,
    current: Mutex<Vec<Id>>,
    ids: AtomicUsize,
    max_level: Option<LevelFilter>,
    filter: F,
    name: String,
}

pub struct MockCollector<F: Fn(&Metadata<'_>) -> bool> {
    expected: VecDeque<Expect>,
    max_level: Option<LevelFilter>,
    filter: F,
    name: String,
}

pub struct MockHandle(Arc<Mutex<VecDeque<Expect>>>, String);

pub fn mock() -> MockCollector<fn(&Metadata<'_>) -> bool> {
    MockCollector {
        expected: VecDeque::new(),
        filter: (|_: &Metadata<'_>| true) as for<'r, 's> fn(&'r Metadata<'s>) -> _,
        max_level: None,
        name: thread::current()
            .name()
            .unwrap_or("mock_subscriber")
            .to_string(),
    }
}

impl<F> MockCollector<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    /// Overrides the name printed by the mock subscriber's debugging output.
    ///
    /// The debugging output is displayed if the test panics, or if the test is
    /// run with `--nocapture`.
    ///
    /// By default, the mock subscriber's name is the  name of the test
    /// (*technically*, the name of the thread where it was created, which is
    /// the name of the test unless tests are run with `--test-threads=1`).
    /// When a test has only one mock subscriber, this is sufficient. However,
    /// some tests may include multiple subscribers, in order to test
    /// interactions between multiple subscribers. In that case, it can be
    /// helpful to give each subscriber a separate name to distinguish where the
    /// debugging output comes from.
    pub fn named(self, name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }

    pub fn enter(mut self, span: MockSpan) -> Self {
        self.expected.push_back(Expect::Enter(span));
        self
    }

    pub fn follows_from(mut self, consequence: MockSpan, cause: MockSpan) -> Self {
        self.expected
            .push_back(Expect::FollowsFrom { consequence, cause });
        self
    }

    pub fn event(mut self, event: MockEvent) -> Self {
        self.expected.push_back(Expect::Event(event));
        self
    }

    pub fn exit(mut self, span: MockSpan) -> Self {
        self.expected.push_back(Expect::Exit(span));
        self
    }

    pub fn clone_span(mut self, span: MockSpan) -> Self {
        self.expected.push_back(Expect::CloneSpan(span));
        self
    }

    #[allow(deprecated)]
    pub fn drop_span(mut self, span: MockSpan) -> Self {
        self.expected.push_back(Expect::DropSpan(span));
        self
    }

    pub fn done(mut self) -> Self {
        self.expected.push_back(Expect::Nothing);
        self
    }

    pub fn record<I>(mut self, span: MockSpan, fields: I) -> Self
    where
        I: Into<mock_field::Expect>,
    {
        self.expected.push_back(Expect::Visit(span, fields.into()));
        self
    }

    pub fn new_span<I>(mut self, new_span: I) -> Self
    where
        I: Into<NewSpan>,
    {
        self.expected.push_back(Expect::NewSpan(new_span.into()));
        self
    }

    pub fn with_filter<G>(self, filter: G) -> MockCollector<G>
    where
        G: Fn(&Metadata<'_>) -> bool + 'static,
    {
        MockCollector {
            expected: self.expected,
            filter,
            max_level: self.max_level,
            name: self.name,
        }
    }

    pub fn with_max_level_hint(self, hint: impl Into<LevelFilter>) -> Self {
        Self {
            max_level: Some(hint.into()),
            ..self
        }
    }

    pub fn run(self) -> impl Collect {
        let (collector, _) = self.run_with_handle();
        collector
    }

    pub fn run_with_handle(self) -> (impl Collect, MockHandle) {
        let expected = Arc::new(Mutex::new(self.expected));
        let handle = MockHandle(expected.clone(), self.name.clone());
        let collector = Running {
            spans: Mutex::new(HashMap::new()),
            expected,
            current: Mutex::new(Vec::new()),
            ids: AtomicUsize::new(1),
            filter: self.filter,
            max_level: self.max_level,
            name: self.name,
        };
        (collector, handle)
    }
}

impl<F> Collect for Running<F>
where
    F: Fn(&Metadata<'_>) -> bool + 'static,
{
    fn enabled(&self, meta: &Metadata<'_>) -> bool {
        println!("[{}] enabled: {:#?}", self.name, meta);
        let enabled = (self.filter)(meta);
        println!("[{}] enabled -> {}", self.name, enabled);
        enabled
    }

    fn register_callsite(&self, meta: &'static Metadata<'static>) -> Interest {
        println!("[{}] register_callsite: {:#?}", self.name, meta);
        if self.enabled(meta) {
            Interest::always()
        } else {
            Interest::never()
        }
    }
    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.max_level
    }

    fn record(&self, id: &Id, values: &span::Record<'_>) {
        let spans = self.spans.lock().unwrap();
        let mut expected = self.expected.lock().unwrap();
        let span = spans
            .get(id)
            .unwrap_or_else(|| panic!("[{}] no span for ID {:?}", self.name, id));
        println!(
            "[{}] record: {}; id={:?}; values={:?};",
            self.name, span.name, id, values
        );
        let was_expected = matches!(expected.front(), Some(Expect::Visit(_, _)));
        if was_expected {
            if let Expect::Visit(expected_span, mut expected_values) = expected.pop_front().unwrap()
            {
                if let Some(name) = expected_span.name() {
                    assert_eq!(name, span.name);
                }
                let context = format!("span {}: ", span.name);
                let mut checker = expected_values.checker(&context, &self.name);
                values.record(&mut checker);
                checker.finish();
            }
        }
    }

    fn event(&self, event: &Event<'_>) {
        let name = event.metadata().name();
        println!("[{}] event: {};", self.name, name);
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Event(mut expected)) => {
                let get_parent_name = || {
                    let stack = self.current.lock().unwrap();
                    let spans = self.spans.lock().unwrap();
                    event
                        .parent()
                        .and_then(|id| spans.get(id))
                        .or_else(|| stack.last().and_then(|id| spans.get(id)))
                        .map(|s| s.name.to_string())
                };
                expected.check(event, get_parent_name, &self.name);
            }
            Some(ex) => ex.bad(&self.name, format_args!("observed event {:#?}", event)),
        }
    }

    fn record_follows_from(&self, consequence_id: &Id, cause_id: &Id) {
        let spans = self.spans.lock().unwrap();
        if let Some(consequence_span) = spans.get(consequence_id) {
            if let Some(cause_span) = spans.get(cause_id) {
                println!(
                    "[{}] record_follows_from: {} (id={:?}) follows {} (id={:?})",
                    self.name, consequence_span.name, consequence_id, cause_span.name, cause_id,
                );
                match self.expected.lock().unwrap().pop_front() {
                    None => {}
                    Some(Expect::FollowsFrom {
                        consequence: ref expected_consequence,
                        cause: ref expected_cause,
                    }) => {
                        if let Some(name) = expected_consequence.name() {
                            assert_eq!(name, consequence_span.name);
                        }
                        if let Some(name) = expected_cause.name() {
                            assert_eq!(name, cause_span.name);
                        }
                    }
                    Some(ex) => ex.bad(
                        &self.name,
                        format_args!(
                            "consequence {:?} followed cause {:?}",
                            consequence_span.name, cause_span.name
                        ),
                    ),
                }
            }
        };
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let meta = span.metadata();
        let id = self.ids.fetch_add(1, Ordering::SeqCst);
        let id = Id::from_u64(id as u64);
        println!(
            "[{}] new_span: name={:?}; target={:?}; id={:?};",
            self.name,
            meta.name(),
            meta.target(),
            id
        );
        let mut expected = self.expected.lock().unwrap();
        let was_expected = matches!(expected.front(), Some(Expect::NewSpan(_)));
        let mut spans = self.spans.lock().unwrap();
        if was_expected {
            if let Expect::NewSpan(mut expected) = expected.pop_front().unwrap() {
                let get_parent_name = || {
                    let stack = self.current.lock().unwrap();
                    span.parent()
                        .and_then(|id| spans.get(id))
                        .or_else(|| stack.last().and_then(|id| spans.get(id)))
                        .map(|s| s.name.to_string())
                };
                expected.check(span, get_parent_name, &self.name);
            }
        }
        spans.insert(
            id.clone(),
            SpanState {
                name: meta.name(),
                refs: 1,
                meta,
            },
        );
        id
    }

    fn enter(&self, id: &Id) {
        let spans = self.spans.lock().unwrap();
        if let Some(span) = spans.get(id) {
            println!("[{}] enter: {}; id={:?};", self.name, span.name, id);
            match self.expected.lock().unwrap().pop_front() {
                None => {}
                Some(Expect::Enter(ref expected_span)) => {
                    if let Some(name) = expected_span.name() {
                        assert_eq!(name, span.name);
                    }
                }
                Some(ex) => ex.bad(&self.name, format_args!("entered span {:?}", span.name)),
            }
        };
        self.current.lock().unwrap().push(id.clone());
    }

    fn exit(&self, id: &Id) {
        if std::thread::panicking() {
            // `exit()` can be called in `drop` impls, so we must guard against
            // double panics.
            println!("[{}] exit {:?} while panicking", self.name, id);
            return;
        }
        let spans = self.spans.lock().unwrap();
        let span = spans
            .get(id)
            .unwrap_or_else(|| panic!("[{}] no span for ID {:?}", self.name, id));
        println!("[{}] exit: {}; id={:?};", self.name, span.name, id);
        match self.expected.lock().unwrap().pop_front() {
            None => {}
            Some(Expect::Exit(ref expected_span)) => {
                if let Some(name) = expected_span.name() {
                    assert_eq!(name, span.name);
                }
                let curr = self.current.lock().unwrap().pop();
                assert_eq!(
                    Some(id),
                    curr.as_ref(),
                    "[{}] exited span {:?}, but the current span was {:?}",
                    self.name,
                    span.name,
                    curr.as_ref().and_then(|id| spans.get(id)).map(|s| s.name)
                );
            }
            Some(ex) => ex.bad(&self.name, format_args!("exited span {:?}", span.name)),
        };
    }

    fn clone_span(&self, id: &Id) -> Id {
        let name = self.spans.lock().unwrap().get_mut(id).map(|span| {
            let name = span.name;
            println!(
                "[{}] clone_span: {}; id={:?}; refs={:?};",
                self.name, name, id, span.refs
            );
            span.refs += 1;
            name
        });
        if name.is_none() {
            println!("[{}] clone_span: id={:?};", self.name, id);
        }
        let mut expected = self.expected.lock().unwrap();
        let was_expected = if let Some(Expect::CloneSpan(ref span)) = expected.front() {
            assert_eq!(
                name,
                span.name(),
                "[{}] expected to clone a span named {:?}",
                self.name,
                span.name()
            );
            true
        } else {
            false
        };
        if was_expected {
            expected.pop_front();
        }
        id.clone()
    }

    fn drop_span(&self, id: Id) {
        let mut is_event = false;
        let name = if let Ok(mut spans) = self.spans.try_lock() {
            spans.get_mut(&id).map(|span| {
                let name = span.name;
                if name.contains("event") {
                    is_event = true;
                }
                println!(
                    "[{}] drop_span: {}; id={:?}; refs={:?};",
                    self.name, name, id, span.refs
                );
                span.refs -= 1;
                name
            })
        } else {
            None
        };
        if name.is_none() {
            println!("[{}] drop_span: id={:?}", self.name, id);
        }
        if let Ok(mut expected) = self.expected.try_lock() {
            let was_expected = match expected.front() {
                Some(Expect::DropSpan(ref span)) => {
                    // Don't assert if this function was called while panicking,
                    // as failing the assertion can cause a double panic.
                    if !::std::thread::panicking() {
                        assert_eq!(name, span.name());
                    }
                    true
                }
                Some(Expect::Event(_)) => {
                    if !::std::thread::panicking() {
                        assert!(is_event, "[{}] expected an event", self.name);
                    }
                    true
                }
                _ => false,
            };
            if was_expected {
                expected.pop_front();
            }
        }
    }

    fn current_span(&self) -> tracing_core::span::Current {
        let stack = self.current.lock().unwrap();
        match stack.last() {
            Some(id) => {
                let spans = self.spans.lock().unwrap();
                let state = spans.get(id).expect("state for current span");
                tracing_core::span::Current::new(id.clone(), state.meta)
            }
            None => tracing_core::span::Current::none(),
        }
    }
}

impl MockHandle {
    pub fn new(expected: Arc<Mutex<VecDeque<Expect>>>, name: String) -> Self {
        Self(expected, name)
    }

    pub fn assert_finished(&self) {
        if let Ok(ref expected) = self.0.lock() {
            assert!(
                !expected.iter().any(|thing| thing != &Expect::Nothing),
                "\n[{}] more notifications expected: {:#?}",
                self.1,
                **expected
            );
        }
    }
}

impl Expect {
    pub fn bad(&self, name: impl AsRef<str>, what: fmt::Arguments<'_>) {
        let name = name.as_ref();
        match self {
            Expect::Event(e) => panic!(
                "\n[{}] expected event {}\n[{}] but instead {}",
                name, e, name, what,
            ),
            Expect::FollowsFrom { consequence, cause } => panic!(
                "\n[{}] expected consequence {} to follow cause {} but instead {}",
                name, consequence, cause, what,
            ),
            Expect::Enter(e) => panic!(
                "\n[{}] expected to enter {}\n[{}] but instead {}",
                name, e, name, what,
            ),
            Expect::Exit(e) => panic!(
                "\n[{}] expected to exit {}\n[{}] but instead {}",
                name, e, name, what,
            ),
            Expect::CloneSpan(e) => {
                panic!(
                    "\n[{}] expected to clone {}\n[{}] but instead {}",
                    name, e, name, what,
                )
            }
            Expect::DropSpan(e) => {
                panic!(
                    "\n[{}] expected to drop {}\n[{}] but instead {}",
                    name, e, name, what,
                )
            }
            Expect::Visit(e, fields) => panic!(
                "\n[{}] expected {} to record {}\n[{}] but instead {}",
                name, e, fields, name, what,
            ),
            Expect::NewSpan(e) => panic!(
                "\n[{}] expected {}\n[{}] but instead {}",
                name, e, name, what
            ),
            Expect::Nothing => panic!(
                "\n[{}] expected nothing else to happen\n[{}] but {} instead",
                name, name, what,
            ),
        }
    }
}
