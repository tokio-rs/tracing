use super::*;

pub(crate) struct NopCollector;

impl Collect for NopCollector {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::never()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}

    fn current_span(&self) -> span::Current {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct NopSubscriber;
impl<S: Collect> Subscribe<S> for NopSubscriber {}

#[allow(dead_code)]
struct NopSubscriber2;
impl<C: Collect> Subscribe<C> for NopSubscriber2 {}

/// A subscriber that holds a string.
///
/// Used to test that pointers returned by downcasting are actually valid.
struct StringSubscriber(&'static str);
impl<C: Collect> Subscribe<C> for StringSubscriber {}
struct StringSubscriber2(&'static str);
impl<C: Collect> Subscribe<C> for StringSubscriber2 {}

struct StringSubscriber3(&'static str);
impl<C: Collect> Subscribe<C> for StringSubscriber3 {}

pub(crate) struct StringCollector(&'static str);

impl Collect for StringCollector {
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::never()
    }

    fn enabled(&self, _: &Metadata<'_>) -> bool {
        false
    }

    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, _: &Event<'_>) {}
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}

    fn current_span(&self) -> span::Current {
        todo!()
    }
}

fn assert_collector(_s: impl Collect) {}

#[test]
fn subscriber_is_collector() {
    let s = NopSubscriber.with_collector(NopCollector);
    assert_collector(s)
}

#[test]
fn two_subscribers_are_collector() {
    let s = NopSubscriber
        .and_then(NopSubscriber)
        .with_collector(NopCollector);
    assert_collector(s)
}

#[test]
fn three_subscribers_are_collector() {
    let s = NopSubscriber
        .and_then(NopSubscriber)
        .and_then(NopSubscriber)
        .with_collector(NopCollector);
    assert_collector(s)
}

#[test]
fn downcasts_to_collector() {
    let s = NopSubscriber
        .and_then(NopSubscriber)
        .and_then(NopSubscriber)
        .with_collector(StringCollector("collector"));
    let collector =
        <dyn Collect>::downcast_ref::<StringCollector>(&s).expect("collector should downcast");
    assert_eq!(collector.0, "collector");
}

#[test]
fn downcasts_to_subscriber() {
    let s = StringSubscriber("subscriber_1")
        .and_then(StringSubscriber2("subscriber_2"))
        .and_then(StringSubscriber3("subscriber_3"))
        .with_collector(NopCollector);
    let subscriber =
        <dyn Collect>::downcast_ref::<StringSubscriber>(&s).expect("subscriber 1 should downcast");
    assert_eq!(subscriber.0, "subscriber_1");
    let subscriber =
        <dyn Collect>::downcast_ref::<StringSubscriber2>(&s).expect("subscriber 2 should downcast");
    assert_eq!(subscriber.0, "subscriber_2");
    let subscriber =
        <dyn Collect>::downcast_ref::<StringSubscriber3>(&s).expect("subscriber 3 should downcast");
    assert_eq!(subscriber.0, "subscriber_3");
}

#[cfg(all(feature = "registry", feature = "std"))]
mod registry_tests {
    use super::*;
    use crate::registry::LookupSpan;

    #[test]
    fn context_event_span() {
        use std::sync::{Arc, Mutex};
        let last_event_span = Arc::new(Mutex::new(None));

        struct RecordingSubscriber {
            last_event_span: Arc<Mutex<Option<&'static str>>>,
        }

        impl<C> Subscribe<C> for RecordingSubscriber
        where
            C: Collect + for<'lookup> LookupSpan<'lookup>,
        {
            fn on_event(&self, event: &Event<'_>, ctx: Context<'_, C>) {
                let span = ctx.event_span(event);
                *self.last_event_span.lock().unwrap() = span.map(|s| s.name());
            }
        }

        tracing::collect::with_default(
            crate::registry().with(RecordingSubscriber {
                last_event_span: last_event_span.clone(),
            }),
            || {
                tracing::info!("no span");
                assert_eq!(*last_event_span.lock().unwrap(), None);

                let parent = tracing::info_span!("explicit");
                tracing::info!(parent: &parent, "explicit span");
                assert_eq!(*last_event_span.lock().unwrap(), Some("explicit"));

                let _guard = tracing::info_span!("contextual").entered();
                tracing::info!("contextual span");
                assert_eq!(*last_event_span.lock().unwrap(), Some("contextual"));
            },
        );
    }

    /// Tests for how max-level hints are calculated when combining subscribers
    /// with and without per-subscriber filtering.
    mod max_level_hints {

        use super::*;
        use crate::filter::*;

        #[test]
        fn mixed_with_unfiltered() {
            let collector = crate::registry()
                .with(NopSubscriber)
                .with(NopSubscriber.with_filter(LevelFilter::INFO));
            assert_eq!(collector.max_level_hint(), None);
        }

        #[test]
        fn mixed_with_unfiltered_layered() {
            let collector = crate::registry().with(NopSubscriber).with(
                NopSubscriber
                    .with_filter(LevelFilter::INFO)
                    .and_then(NopSubscriber.with_filter(LevelFilter::TRACE)),
            );
            assert_eq!(dbg!(collector).max_level_hint(), None);
        }

        #[test]
        fn mixed_interleaved() {
            let collector = crate::registry()
                .with(NopSubscriber)
                .with(NopSubscriber.with_filter(LevelFilter::INFO))
                .with(NopSubscriber)
                .with(NopSubscriber.with_filter(LevelFilter::INFO));
            assert_eq!(dbg!(collector).max_level_hint(), None);
        }

        #[test]
        fn mixed_layered() {
            let collector = crate::registry()
                .with(
                    NopSubscriber
                        .with_filter(LevelFilter::INFO)
                        .and_then(NopSubscriber),
                )
                .with(NopSubscriber.and_then(NopSubscriber.with_filter(LevelFilter::INFO)));
            assert_eq!(dbg!(collector).max_level_hint(), None);
        }

        #[test]
        fn psf_only_unhinted() {
            let collector = crate::registry()
                .with(NopSubscriber.with_filter(LevelFilter::INFO))
                .with(NopSubscriber.with_filter(filter_fn(|_| true)));
            assert_eq!(dbg!(collector).max_level_hint(), None);
        }

        #[test]
        fn psf_only_unhinted_nested_outer() {
            // if a nested tree of per-subscriber filters has an _outer_ filter with
            // no max level hint, it should return `None`.
            let collector = crate::registry()
                .with(
                    NopSubscriber
                        .with_filter(LevelFilter::INFO)
                        .and_then(NopSubscriber.with_filter(LevelFilter::WARN)),
                )
                .with(
                    NopSubscriber
                        .with_filter(filter_fn(|_| true))
                        .and_then(NopSubscriber.with_filter(LevelFilter::DEBUG)),
                );
            assert_eq!(dbg!(collector).max_level_hint(), None);
        }

        #[test]
        fn psf_only_unhinted_nested_inner() {
            // If a nested tree of per-subscriber filters has an _inner_ filter with
            // no max-level hint, but the _outer_ filter has a max level hint,
            // it should pick the outer hint. This is because the outer filter
            // will disable the spans/events before they make it to the inner
            // filter.
            let collector = dbg!(crate::registry().with(
                NopSubscriber
                    .with_filter(filter_fn(|_| true))
                    .and_then(NopSubscriber.with_filter(filter_fn(|_| true)))
                    .with_filter(LevelFilter::INFO),
            ));
            assert_eq!(dbg!(collector).max_level_hint(), Some(LevelFilter::INFO));
        }

        #[test]
        fn unhinted_nested_inner() {
            let collector = dbg!(crate::registry()
                .with(
                    NopSubscriber
                        .and_then(NopSubscriber)
                        .with_filter(LevelFilter::INFO)
                )
                .with(
                    NopSubscriber
                        .with_filter(filter_fn(|_| true))
                        .and_then(NopSubscriber.with_filter(filter_fn(|_| true)))
                        .with_filter(LevelFilter::WARN),
                ));
            assert_eq!(dbg!(collector).max_level_hint(), Some(LevelFilter::INFO));
        }

        #[test]
        fn unhinted_nested_inner_mixed() {
            let collector = dbg!(crate::registry()
                .with(
                    NopSubscriber
                        .and_then(NopSubscriber.with_filter(filter_fn(|_| true)))
                        .with_filter(LevelFilter::INFO)
                )
                .with(
                    NopSubscriber
                        .with_filter(filter_fn(|_| true))
                        .and_then(NopSubscriber.with_filter(filter_fn(|_| true)))
                        .with_filter(LevelFilter::WARN),
                ));
            assert_eq!(dbg!(collector).max_level_hint(), Some(LevelFilter::INFO));
        }

        #[test]
        fn psf_only_picks_max() {
            let collector = crate::registry()
                .with(NopSubscriber.with_filter(LevelFilter::WARN))
                .with(NopSubscriber.with_filter(LevelFilter::DEBUG));
            assert_eq!(dbg!(collector).max_level_hint(), Some(LevelFilter::DEBUG));
        }

        #[test]
        fn many_psf_only_picks_max() {
            let collector = crate::registry()
                .with(NopSubscriber.with_filter(LevelFilter::WARN))
                .with(NopSubscriber.with_filter(LevelFilter::DEBUG))
                .with(NopSubscriber.with_filter(LevelFilter::INFO))
                .with(NopSubscriber.with_filter(LevelFilter::ERROR));
            assert_eq!(dbg!(collector).max_level_hint(), Some(LevelFilter::DEBUG));
        }

        #[test]
        fn nested_psf_only_picks_max() {
            let collector = crate::registry()
                .with(
                    NopSubscriber.with_filter(LevelFilter::INFO).and_then(
                        NopSubscriber
                            .with_filter(LevelFilter::WARN)
                            .and_then(NopSubscriber.with_filter(LevelFilter::DEBUG)),
                    ),
                )
                .with(
                    NopSubscriber
                        .with_filter(LevelFilter::INFO)
                        .and_then(NopSubscriber.with_filter(LevelFilter::ERROR)),
                );
            assert_eq!(dbg!(collector).max_level_hint(), Some(LevelFilter::DEBUG));
        }
    }
}
