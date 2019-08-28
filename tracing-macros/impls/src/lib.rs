#![doc(html_root_url = "https://docs.rs/tracing-macros/0.1.0")]
#![deny(missing_debug_implementations, unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;
// use proc_macro2::Span;
// use proc_macro_hack::proc_macro_hack;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

pub(crate) mod ast;

#[proc_macro_hack]
pub fn event(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as EventInput);
    // let call_site = Span::call_site();
    let callsite = input.body.gen_callsite(input.level);
    let valueset = input.body.fields.gen_valueset();
    quote!({
        static CALLSITE: &'static dyn tracing::callsite::Callsite = {
            #callsite
        };
        #valueset
    })
    .into()
    // panic!("{:#?}", input);
}

#[derive(Debug)]
struct EventInput {
    level: syn::Path,
    body: ast::EventBody,
}

impl Parse for EventInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let level = input.parse()?;
        input.parse::<Token![,]>()?;
        let body = input.parse()?;
        Ok(Self { level, body })
    }
}
