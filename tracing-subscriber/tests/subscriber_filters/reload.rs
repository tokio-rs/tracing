use super::*;
use tracing_subscriber::reload;

mod trees {
    use super::*;
    use crate::trees::{basic, span_scopes};

    #[test]
    fn basic_trees() {
        let (with_target, with_target_handle) = basic::with_target_subscriber();
        let (info, info_handle) = basic::info_subscriber();
        let (all, all_handle) = basic::all_subscriber();

        let (target_filter, _) =
            reload::Subscriber::new(filter::filter_fn(basic::target_filter_fn));
        let (info_filter, _) = reload::Subscriber::new(LevelFilter::INFO);
        let info_tree = info
            .and_then(with_target.with_filter(target_filter))
            .with_filter(info_filter);

        let subscriber = tracing_subscriber::registry().with(info_tree).with(all);
        let _guard = dbg!(subscriber).set_default();

        basic::run();

        all_handle.assert_finished();
        info_handle.assert_finished();
        with_target_handle.assert_finished();
    }

    #[test]
    fn filter_span_scopes() {
        let (a_subscriber, a_handle) = span_scopes::a_subscriber();
        let (b_subscriber, b_handle) = span_scopes::b_subscriber();
        let (info_subscriber, info_handle) = span_scopes::info_subscriber();
        let (all_subscriber, all_handle) = span_scopes::all_subscriber();

        let (a_filter, _) = reload::Subscriber::new(filter::filter_fn(span_scopes::a_filter_fn));
        let (b_filter, _) = reload::Subscriber::new(filter::filter_fn(span_scopes::b_filter_fn));
        let (info_filter, _) = reload::Subscriber::new(LevelFilter::INFO);

        let info_tree = info_subscriber
            .and_then(a_subscriber.with_filter(a_filter))
            .and_then(b_subscriber.with_filter(b_filter))
            .with_filter(info_filter);

        let subscriber = tracing_subscriber::registry()
            .with(info_tree)
            .with(all_subscriber);
        let _guard = dbg!(subscriber).set_default();

        span_scopes::run();

        all_handle.assert_finished();
        info_handle.assert_finished();
        a_handle.assert_finished();
        b_handle.assert_finished();
    }
}
