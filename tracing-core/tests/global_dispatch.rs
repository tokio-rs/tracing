mod common;

use common::*;
use tracing_core::dispatch::*;
#[test]
fn global_dispatch() {
    static TEST: TestCollectorA = TestCollectorA;
    set_global_default(Dispatch::from_static(&TEST)).expect("global dispatch set failed");
    get_default(|current| assert!(current.is::<TestCollectorA>(), "global dispatch get failed"));

    #[cfg(feature = "std")]
    with_default(Dispatch::new(TestCollectorB), || {
        get_default(|current| {
            assert!(
                current.is::<TestCollectorB>(),
                "thread-local override of global dispatch failed"
            )
        });
    });

    get_default(|current| {
        assert!(
            current.is::<TestCollectorA>(),
            "reset to global override failed"
        )
    });

    set_global_default(Dispatch::from_static(&TEST))
        .expect_err("double global dispatch set succeeded");
}
