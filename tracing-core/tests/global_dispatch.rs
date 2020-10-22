mod common;

use common::*;
use tracing_core::dispatcher::*;
#[test]
fn global_dispatch() {
    static TEST: TestSubscriberA = TestSubscriberA;
    set_global_default(Dispatch::from_static(&TEST)).expect("global dispatch set failed");
    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "global dispatch get failed"
        )
    });

    #[cfg(feature = "std")]
    with_default(Dispatch::new(TestSubscriberB), || {
        get_default(|current| {
            assert!(
                current.is::<TestSubscriberB>(),
                "thread-local override of global dispatch failed"
            )
        });
    });

    get_default(|current| {
        assert!(
            current.is::<TestSubscriberA>(),
            "reset to global override failed"
        )
    });

    set_global_default(Dispatch::from_static(&TEST))
        .expect_err("double global dispatch set succeeded");
}
