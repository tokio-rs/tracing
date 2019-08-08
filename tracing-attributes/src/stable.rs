use quote::quote_spanned;

/// Generates the instrumented body for an `async fn`.
///
/// # Arguments
/// - `body`: the body of the `async fn`.
/// - `err_span`: the callsite of the `trace` attribute macro, to emit global
///   errors (if there are any).
pub(crate) fn gen_async_body(
    _body: Box<syn::Block>,
    err_span: &proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let span = err_span.clone();
    quote_spanned! {span=>
        compile_error!(
            "to use `tracing-macros` with async functions, the \"nightly\" \
             feature flag must be enabled."
        )
    }
}
