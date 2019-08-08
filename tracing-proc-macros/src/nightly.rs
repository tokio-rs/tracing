use syn::spanned::Spanned;

/// Generates the instrumented body for an `async fn`.
///
/// # Arguments
/// - `body`: the body of the `async fn`.
/// - `err_span`: the callsite of the `trace` attribute macro, to emit global
///   errors (if there are any).
pub(crate) fn gen_async_body(
    body: Box<syn::Block>,
    _err_span: &proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let span = body.span();
    quote_spanned! {span=>
        tracing_futures::Instrument::instrument(
            async { #body },
            __tracing_attr_span
        )
            .await
    }
}
