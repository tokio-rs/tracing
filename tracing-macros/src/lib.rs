#![doc(html_root_url = "https://docs.rs/tracing-macros/0.1.0")]
#![deny(missing_debug_implementations, unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_hack::proc_macro_hack;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

#[proc_macro_hack]
pub fn event(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as KvFields);
    let call_site = Span::call_site();
}

#[derive(Debug)]
struct KvFields {
    fields: Punctuated<KvField, Token![,]>,
}

#[derive(Debug)]
struct KvField {
    name: Punctuated<syn::Ident, Token![.]>,
    value: Option<Box<syn::Expr>>,
}

impl Parse for KvField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = Punctuated::parse_separated_nonempty(input)?;
        let value = if input.lookahead1().peek(Token![=]) {
            let _ = input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { name, value })
    }
}

impl Parse for KvFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fields = Punctuated::<KvField, Token![,]>::parse_terminated(input)?;
        Ok(Self { fields })
    }
}
