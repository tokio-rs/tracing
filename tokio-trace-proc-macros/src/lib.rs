extern crate proc_macro;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro2;
extern crate tokio_trace;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::token::{Async, Const, Unsafe};
use syn::{Abi, ArgCaptured, Attribute, Block, FnArg, Ident, ItemFn, Pat, PatIdent, Visibility};

#[proc_macro_attribute]
pub fn trace(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = parse_macro_input!(item as ItemFn);
    let call_site = Span::call_site();

    // these are needed ahead of time, as ItemFn contains the function body _and_
    // isn't representable inside a quote!/quote_spanned! macro
    // (Syn's ToTokens isn't implemented for ItemFn)
    let attrs: Vec<Attribute> = input.clone().attrs;
    let vis: Visibility = input.clone().vis;
    let constness: Option<Const> = input.clone().constness;
    let unsafety: Option<Unsafe> = input.clone().unsafety;
    let asyncness: Option<Async> = input.clone().asyncness;
    let abi: Option<Abi> = input.clone().abi;

    // function body
    let block: Box<Block> = input.clone().block;
    // function name
    let ident: Ident = input.clone().ident;
    let ident_str = ident.to_string();

    let return_type = input.clone().decl.output;
    let params = input.clone().decl.inputs;
    let param_names: Vec<Ident> = input
        .clone()
        .decl
        .inputs
        .into_iter()
        .filter_map(|param| match param {
            FnArg::Captured(ArgCaptured {
                pat: Pat::Ident(PatIdent { ident, .. }),
                ..
            }) => Some(ident),
            _ => None,
        })
        .collect();
    let param_names_clone = param_names.clone();

    quote_spanned!(call_site=>
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident(#params) #return_type {
            span!(
                tokio_trace::Level::TRACE,
                #ident_str,
                traced_function = &#ident_str
                #(, #param_names = tokio_trace::field::debug(&#param_names_clone)),*
            )
            .enter(move || {
                #block
            })
        }
    )
    .into()
}
