//! A subscriber that tracks the time spent in each span.

use core::time::Duration;
use std::time::Instant;

use tracing::Collect;
use tracing_core::span::{self, Attributes};

use crate::{registry::LookupSpan, subscribe::Context, Subscribe};

/// A subscriber that tracks the time spent in each span.
///
/// This subscriber records the time spent in each span, storing the timing data in the span's
/// extensions. The subscriber records the time spent in each span as either "idle" time, when the
/// span is not executing, or "busy" time, when the span is executing. The subscriber records the
/// time spent in each span as a [`Timing`] resource, which can be accessed by other subscribers.
#[derive(Debug, Default)]
pub struct TimingSubscriber;

/// A resource tracking the idle and busy time spent in each span.
///
/// This is used by the [`TimingSubscriber`] to track the time spent in each span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    state: State,
    idle: Duration,
    busy: Duration,
    last: Instant,
}

impl Default for Timing {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    /// The span is closed.
    ///
    /// No further timing information will be recorded.
    Closed,
    #[default]
    /// The span is currently idle.
    ///
    /// Timing information will be recorded when the span becomes busy or closed.
    Idle,
    /// The span is currently busy.
    ///
    /// Timing information will be recorded when the span becomes idle or closed.
    Busy,
}

impl Timing {
    /// Create a new `Timing` resource.
    fn new() -> Self {
        Self {
            state: State::Idle,
            idle: Duration::ZERO,
            busy: Duration::ZERO,
            last: Instant::now(),
        }
    }

    /// Record that the span has become idle.
    fn enter(&mut self) {
        if self.state == State::Busy {
            return;
        }
        let now = Instant::now();
        self.idle += now.duration_since(self.last);
        self.last = now;
        self.state = State::Busy;
    }

    /// Record that the span has become busy.
    fn exit(&mut self) {
        if self.state == State::Idle {
            return;
        }
        let now = Instant::now();
        self.busy += now.duration_since(self.last);
        self.last = now;
        self.state = State::Idle;
    }

    /// Record that the span has been closed.
    fn close(&mut self) {
        match self.state {
            State::Idle => self.idle += Instant::now().duration_since(self.last),
            State::Busy => self.busy += Instant::now().duration_since(self.last),
            State::Closed => {}
        }
        self.state = State::Closed;
    }

    /// Get the idle time spent in this span.
    pub fn idle(&self) -> Duration {
        self.idle
    }

    /// Get the busy time spent in this span.
    pub fn busy(&self) -> Duration {
        self.busy
    }

    /// Get the total time spent in this span.
    pub fn total(&self) -> Duration {
        self.idle + self.busy
    }
}

impl<C> Subscribe<C> for TimingSubscriber
where
    C: Collect + for<'a> LookupSpan<'a>,
{
    /// Records that a new span has been created.
    fn on_new_span(&self, _attrs: &Attributes<'_>, id: &span::Id, ctx: Context<'_, C>) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();
        extensions.insert(Timing::new());
    }

    /// Records that a span has been entered.
    ///
    /// The subscriber records the time spent in the span as "idle" time, as the span is not
    /// executing.
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, C>) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();
        let timings = extensions.get_mut::<Timing>().expect("timings not found");
        timings.enter();
    }

    /// Records that a span has been exited.
    ///
    /// The subscriber records the time spent in the span as "busy" time, as the span is executing.
    fn on_exit(&self, id: &span::Id, ctx: Context<'_, C>) {
        let span = ctx.span(id).expect("span not found");
        let mut extensions = span.extensions_mut();
        let timings = extensions.get_mut::<Timing>().expect("timings not found");
        timings.exit();
    }

    /// Records that a span has been closed.
    ///
    /// The subscriber records the time spent in the span as either "idle" time or "busy" time, as
    /// the span is not executing.
    fn on_close(&self, id: span::Id, ctx: Context<'_, C>) {
        let span = ctx.span(&id).expect("span not found");
        let mut extensions = span.extensions_mut();
        let timings = extensions.get_mut::<Timing>().expect("timings not found");
        timings.close();
    }
}
