extern crate proc_macro;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro2;
extern crate syn_mid;
extern crate tokio_trace;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn_mid::{ArgCaptured, FnArg, ItemFn, Pat, PatIdent};

#[proc_macro_attribute]
pub fn trace(_args: TokenStream, function: TokenStream) -> TokenStream {
    let mut function: ItemFn = parse_macro_input!(function);

    let body = function.block.stmts;
    let call_site = Span::call_site();
    let ident_str = function.ident.to_string();

    let param_names: Vec<_> = function
        .inputs
        .iter()
        .filter_map(|param| match param {
            FnArg::Captured(ArgCaptured {
                pat: Pat::Ident(PatIdent { ident, .. }),
                ..
            }) => Some(ident),
            _ => None,
        })
        .collect();
    let param_names_clone = param_names.clone();

    let span = quote_spanned!(call_site =>
        span!(
            tokio_trace::Level::TRACE,
            #ident_str,
            traced_function = &#ident_str
            #(, #param_names = tokio_trace::field::debug(&#param_names_clone)),*
        )
    );

    function.block.stmts = quote_spanned!(call_site =>
        #span.enter(move || {
            #body
        })
    );

    quote_spanned!(call_site => #function).into()
}
