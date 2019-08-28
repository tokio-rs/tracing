#![doc(html_root_url = "https://docs.rs/tracing-macros/0.1.0")]
#![deny(missing_debug_implementations, unreachable_pub)]
#![cfg_attr(test, deny(warnings))]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::Token;

pub(crate) mod ast;

#[proc_macro_hack]
pub fn event(item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as EventInput);
    input.generate().into()
}

#[proc_macro_hack]
pub fn trace(item: TokenStream) -> TokenStream {
    let body = syn::parse_macro_input!(item as ast::EventBody);
    let event = EventInput {
        level: syn::parse_quote! { tracing::Level::TRACE },
        body,
    };
    event.generate().into()
}

#[proc_macro_hack]
pub fn debug(item: TokenStream) -> TokenStream {
    let body = syn::parse_macro_input!(item as ast::EventBody);
    let event = EventInput {
        level: syn::parse_quote! { tracing::Level::DEBUG },
        body,
    };
    event.generate().into()
}

#[proc_macro_hack]
pub fn info(item: TokenStream) -> TokenStream {
    let body = syn::parse_macro_input!(item as ast::EventBody);
    let event = EventInput {
        level: syn::parse_quote! { tracing::Level::INFO },
        body,
    };
    event.generate().into()
}

#[proc_macro_hack]
pub fn warn(item: TokenStream) -> TokenStream {
    let body = syn::parse_macro_input!(item as ast::EventBody);
    let event = EventInput {
        level: syn::parse_quote! { tracing::Level::WARN },
        body,
    };
    event.generate().into()
}

#[proc_macro_hack]
pub fn error(item: TokenStream) -> TokenStream {
    let body = syn::parse_macro_input!(item as ast::EventBody);
    let event = EventInput {
        level: syn::parse_quote! { tracing::Level::ERROR },
        body,
    };
    event.generate().into()
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

impl EventInput {
    fn generate(&self) -> TokenStream {
        let callsite = self.body.gen_callsite(&self.level);
        let (vs_args, valueset) = self.body.fields.gen_valueset();
        let event = if let Some(ast::Parent { ref parent }) = self.body.attrs.parent {
            quote! {
                tracing::Event::child_of(
                    #parent,
                    __tracing__meta,
                    &__tracing__fieldset.value_set(#valueset)
                );
            }
        } else {
            quote! {
                tracing::Event::dispatch(
                    __tracing__meta,
                    &__tracing__fieldset.value_set(#valueset),
                );
            }
        };
        quote!({
            use tracing::callsite::Callsite;
            let __tracing__callsite = {
                #callsite
            };
            if tracing::is_enabled!(__tracing__callsite) {
                let __tracing__meta = __tracing__callsite.metadata();
                let __tracing__fieldset = __tracing__meta.fields();
                let mut __tracing__fields = __tracing__fieldset.iter();
                #vs_args
                #event
            }
        })
        .into()
    }
}
