use test_log_support::Test;
use tracing::{enabled, error, warn, Level};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[test]
fn test_log_enabled() {
    static OTHER_MODULE: &str = "other_module";

    let test = Test::with_filters(&[
        (module_path!(), log::LevelFilter::Error),
        (OTHER_MODULE, log::LevelFilter::Warn),
    ]);

    if enabled!(Level::ERROR) {
        error!(msg = 1);
    }
    test.assert_logged("msg=1");

    if enabled!(Level::WARN) {
        panic!("warn level unexpectedly enabled");
    }

    if enabled!(target: module_path!(), Level::ERROR) {
        error!(msg = 5);
    }
    test.assert_logged("msg=5");

    if enabled!(target: module_path!(), Level::WARN) {
        panic!(
            "warn level unexpectedly enabled for target {}",
            module_path!()
        );
    }

    if enabled!(target: OTHER_MODULE, Level::WARN) {
        warn!(target: OTHER_MODULE, msg = 7);
    }
    test.assert_logged("msg=7");

    if enabled!(target: OTHER_MODULE, Level::INFO) {
        panic!(
            "info level unexpectedly enabled for target {}",
            OTHER_MODULE
        );
    }
}
