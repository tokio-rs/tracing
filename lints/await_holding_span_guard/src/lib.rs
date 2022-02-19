#![feature(rustc_private)]
#![warn(unused_extern_crates)]

dylint_linting::dylint_library!();

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod await_holding_span_guard;

#[doc(hidden)]
#[no_mangle]
pub fn register_lints(_sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    lint_store.register_lints(&[await_holding_span_guard::AWAIT_HOLDING_SPAN_GUARD]);
    lint_store.register_late_pass(|| Box::new(await_holding_span_guard::AwaitHoldingSpanGuard));
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
