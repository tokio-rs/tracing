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

#[proc_macro]
pub fn event(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as EventInput);
    let call_site = Span::call_site();
    panic!("{:#?}", input);
}

#[derive(Debug)]
struct EventInput {
    level: syn::Path,
    attrs: Attrs,
}

#[derive(Debug)]
struct Attrs {
    target: Option<Target>,
    parent: Option<Parent>,
    fields: KvFields,
}

#[derive(Debug)]
struct Parent {
    parent: syn::Expr,
}

#[derive(Debug)]
struct Target {
    target: syn::LitStr,
}

#[derive(Debug)]
struct KvFields {
    fields: Punctuated<KvField, Token![,]>,
}

#[derive(Debug)]
struct KvField {
    name: Punctuated<syn::Ident, Token![.]>,
    value: Option<syn::Expr>,
}

impl Parse for EventInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let level = input.parse()?;
        input.parse::<Token![,]>()?;
        let attrs = input.parse()?;
        Ok(Self { level, attrs })
    }
}

impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut target = None;
        let mut parent = None;
        loop {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::target) {
                target = Some(input.parse()?);
                input.parse::<Token![,]>()?;
            } else if lookahead.peek(kw::parent) {
                parent = Some(input.parse()?);
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        let fields = input.parse()?;

        Ok(Self {
            target,
            parent,
            fields,
        })
    }
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

impl Parse for Target {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::target>()?;
        let _ = input.parse::<Token![:]>()?;
        input.parse().map(|target| Target { target })
    }
}

impl Parse for Parent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::parent>()?;
        let _ = input.parse::<Token![:]>()?;
        input.parse().map(|parent| Parent { parent })
    }
}

mod kw {
    syn::custom_keyword!(target);
    syn::custom_keyword!(parent);
}
