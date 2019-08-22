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
    let fieldset = input.body.fields.gen_fieldset();
    let valueset = input.body.fields.gen_valueset();
    quote!(
        #fieldset
        #valueset
    )
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

// fn callsite<'a>(
//     name: proc_macro2::TokenStream,
//     target: proc_macro2::TokenStream,
//     level: proc_macro2::TokenStream,
//     fields: impl Iterator<Item = &'a str>,
//     kind: proc_macro2::TokenStream,
// ) -> proc_macro2::TokenStream {
//     quote! {{
//         use std::sync::{
//             atomic::{self, AtomicUsize, Ordering},
//             Once,
//         };
//         use tracing::{subscriber::Interest, Metadata, metadata};
//         struct MyCallsite;
//         static META: Metadata<'static> = {
//             metadata! {
//                 name: #name,
//                 target: #target,
//                 level: #level,
//                 fields: &[#(#fields),*],
//                 callsite: &MyCallsite,
//                 kind: #kind,
//             }
//         };
//         // FIXME: Rust 1.34 deprecated ATOMIC_USIZE_INIT. When Tokio's minimum
//         // supported version is 1.34, replace this with the const fn `::new`.
//         #[allow(deprecated)]
//         static INTEREST: AtomicUsize = atomic::ATOMIC_USIZE_INIT;
//         static REGISTRATION: Once = Once::new();
//         impl MyCallsite {
//             #[inline]
//             fn interest(&self) -> Interest {
//                 match INTEREST.load(Ordering::Relaxed) {
//                     0 => Interest::never(),
//                     2 => Interest::always(),
//                     _ => Interest::sometimes(),
//                 }
//             }
//         }
//         impl callsite::Callsite for MyCallsite {
//             fn set_interest(&self, interest: Interest) {
//                 let interest = match () {
//                     _ if interest.is_never() => 0,
//                     _ if interest.is_always() => 2,
//                     _ => 1,
//                 };
//                 INTEREST.store(interest, Ordering::SeqCst);
//             }

//             fn metadata(&self) -> &Metadata {
//                 &META
//             }
//         }
//         REGISTRATION.call_once(|| {
//             callsite::register(&MyCallsite);
//         });
//         &MyCallsite
//     }}
// }
