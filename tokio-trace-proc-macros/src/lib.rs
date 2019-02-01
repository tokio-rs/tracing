extern crate proc_macro;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro2;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::token::{Async, Const, Unsafe};
use syn::{Abi, Attribute, Block, Ident, ItemFn, Visibility};

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

    quote_spanned!(call_site=>
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident(#params) #return_type {
            span!(#ident_str, traced_function = &#ident_str).enter(move || {
                #block
            })
        }
    )
    .into()
}
