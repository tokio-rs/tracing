//! # tracing_test_macro
//!
//! This crate provides a proc macro that can be added to test functions in
//! order to ensure that all tracing logs are written to a global buffer.
extern crate proc_macro;

use std::sync::Mutex;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse, ItemFn, Stmt};

lazy_static! {
    /// Registered scopes.
    ///
    /// By default, every traced test registers a span with the function name.
    /// However, since multiple tests can share the same function name, in case
    /// of conflict, a counter is appended.
    ///
    /// This vector is used to store all already registered scopes.
    static ref REGISTERED_SCOPES: Mutex<Vec<String>> = Mutex::new(vec![]);
}

/// Check whether this test function name is already taken as scope. If yes, a
/// counter is appended to make it unique. In the end, a unique scope is returned.
fn get_free_scope(mut test_fn_name: String) -> String {
    let mut vec = REGISTERED_SCOPES.lock().unwrap();
    let mut counter = 1;
    let len = test_fn_name.len();
    while vec.contains(&test_fn_name) {
        counter += 1;
        test_fn_name.replace_range(len.., &counter.to_string());
    }
    vec.push(test_fn_name.clone());
    test_fn_name
}

/// A proc macro that ensures that a global logger is registered for the
/// annotated test.
///
/// Additionally, the macro injects a local function called `logs_contain`,
/// which can be used to assert that a certain string was logged within this
/// test.
///
/// ## Example
///
/// ```rust,no_run
/// use tokio;
/// use tracing::{info, warn};
/// use tracing_test::traced_test;
///
/// #[tokio::test]
/// #[traced_test]
/// async fn test_logs_are_captured() {
///     info!("This is being logged on the info level");
///     tokio::spawn(async {
///         warn!("This is being logged on the warn level from a spawned task");
///     }).await.unwrap();
///     assert!(logs_contain("logged on the info level"));
///     assert!(logs_contain("logged on the warn level"));
///     assert!(!logs_contain("logged on the error level"));
/// }
/// ```
#[proc_macro_attribute]
pub fn traced_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse annotated function
    let mut function: ItemFn = parse(item).expect("Could not parse ItemFn");

    // Determine scope
    let scope = get_free_scope(function.sig.ident.to_string());

    // Prepare code that should be injected at the start of the function
    let init = parse::<Stmt>(
        quote! {
            tracing_test::INITIALIZED.call_once(|| {
                let crate_name = module_path!()
                    .split(":")
                    .next()
                    .expect("Could not find crate name in module path")
                    .to_string();
                let env_filter = format!("{}=trace", crate_name);
                let mock_writer = tracing_test::MockWriter::new(&tracing_test::GLOBAL_BUF);
                let subscriber = tracing_test::get_subscriber(mock_writer, &env_filter);
                tracing::dispatcher::set_global_default(subscriber)
                    .expect("Could not set global tracing subscriber");
            });
        }
        .into(),
    )
    .expect("Could not parse quoted statement init");
    let span = parse::<Stmt>(
        quote! {
            let span = tracing::info_span!(#scope);
        }
        .into(),
    )
    .expect("Could not parse quoted statement span");
    let enter = parse::<Stmt>(
        quote! {
            let _enter = span.enter();
        }
        .into(),
    )
    .expect("Could not parse quoted statement enter");
    let assert_fn = parse::<Stmt>(
        quote! {
            fn logs_contain(val: &str) -> bool {
                tracing_test::logs_with_scope_contain(#scope, val)
            }
        }
        .into(),
    )
    .expect("Could not parse quoted statement assert_fn");

    // Inject code into function
    function.block.stmts.insert(0, init);
    function.block.stmts.insert(1, span);
    function.block.stmts.insert(2, enter);
    function.block.stmts.insert(3, assert_fn);

    // Generate token stream
    TokenStream::from(function.to_token_stream())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_free_scope() {
        let initial = get_free_scope("test_fn_name".to_string());
        assert_eq!(initial, "test_fn_name");

        let second = get_free_scope("test_fn_name".to_string());
        assert_eq!(second, "test_fn_name2");
        let third = get_free_scope("test_fn_name".to_string());
        assert_eq!(third, "test_fn_name3");

        // Insert a conflicting entry
        let fourth = get_free_scope("test_fn_name4".to_string());
        assert_eq!(fourth, "test_fn_name4");

        let fifth = get_free_scope("test_fn_name5".to_string());
        assert_eq!(fifth, "test_fn_name5");
    }
}
