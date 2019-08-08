extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned, ArgCaptured, AttributeArgs, FnArg, FnDecl, Ident, ItemFn, Lit, Meta,
    MetaNameValue, NestedMeta, Pat, PatIdent,
};

#[proc_macro_attribute]
pub fn trace(args: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = syn::parse_macro_input!(item as ItemFn);
    let args = syn::parse_macro_input!(args as AttributeArgs);
    let call_site = Span::call_site();

    // these are needed ahead of time, as ItemFn contains the function body _and_
    // isn't representable inside a quote!/quote_spanned! macro
    // (Syn's ToTokens isn't implemented for ItemFn)
    let ItemFn {
        attrs,
        vis,
        unsafety,
        asyncness,
        constness,
        abi,
        block,
        ident,
        decl,
        ..
    } = input;
    // function name
    let ident_str = ident.to_string();

    let FnDecl {
        output: return_type,
        inputs: params,
        ..
    } = *decl;
    let param_names: Vec<Ident> = params
        .clone()
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

    // Generate the instrumented function body.
    // If the function is an `async fn`, this will wrap it in an async block,
    // which is `instrument`ed using `tracing-futures`, if we are on a new
    // enough Rust version to support this. Otherwise, this will enter the span
    // and then perform the rest of the body.
    let body = if asyncness.is_some() {
        async_await::gen_async_body(block, &call_site)
    } else {
        quote_spanned!(block.span()=>
            let __tracing_attr_guard = __tracing_attr_span.enter();
            #block
        )
    };

    let level = level(&args);

    quote_spanned!(call_site=>
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident(#params) #return_type {
            let __tracing_attr_span = tracing::span!(
                #level,
                #ident_str,
                #(#param_names = tracing::field::debug(&#param_names_clone)),*
            );
            #body
        }
    )
    .into()
}

fn level(args: &AttributeArgs) -> proc_macro2::TokenStream {
    let mut levels = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref ident, ref lit, ..
        })) if ident == "level" => Some(lit.clone()),
        _ => None,
    });
    let level = levels.next();

    // If we found more than one arg named "level", that's a syntax error...
    if let Some(lit) = levels.next() {
        return quote_spanned! {lit.span()=>
            compile_error!("expected only a single `level` argument!")
        };
    }

    match level {
        Some(Lit::Str(ref lit)) if lit.value().eq_ignore_ascii_case("trace") => {
            quote!(tracing::Level::TRACE)
        }
        Some(Lit::Str(ref lit)) if lit.value().eq_ignore_ascii_case("debug") => {
            quote!(tracing::Level::DEBUG)
        }
        Some(Lit::Str(ref lit)) if lit.value().eq_ignore_ascii_case("info") => {
            quote!(tracing::Level::INFO)
        }
        Some(Lit::Str(ref lit)) if lit.value().eq_ignore_ascii_case("warn") => {
            quote!(tracing::Level::WARN)
        }
        Some(Lit::Str(ref lit)) if lit.value().eq_ignore_ascii_case("error") => {
            quote!(tracing::Level::ERROR)
        }
        Some(Lit::Int(ref lit)) if lit.value() == 1 => quote!(tracing::Level::TRACE),
        Some(Lit::Int(ref lit)) if lit.value() == 2 => quote!(tracing::Level::DEBUG),
        Some(Lit::Int(ref lit)) if lit.value() == 3 => quote!(tracing::Level::INFO),
        Some(Lit::Int(ref lit)) if lit.value() == 4 => quote!(tracing::Level::WARN),
        Some(Lit::Int(ref lit)) if lit.value() == 5 => quote!(tracing::Level::ERROR),
        Some(lit) => quote_spanned! {lit.span()=>
            compile_error!(
                "unknown verbosity level, expected one of \"trace\", \
                 \"debug\", \"info\", \"warn\", or \"error\", or a number 1-5"
            )
        },
        None => quote!(tracing::Level::INFO),
    }
}

mod async_await {
    #[cfg(feature = "nightly")]
    pub(crate) use super::nightly::*;

    #[cfg(not(feature = "nightly"))]
    pub(crate) use super::stable::*;
}

#[cfg(feature = "nightly")]
mod nightly;
#[cfg(not(feature = "nightly"))]
mod stable;
