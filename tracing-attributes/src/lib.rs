#![doc(html_root_url = "https://docs.rs/tracing-attributes/0.1.0")]
#![deny(missing_debug_implementations, unreachable_pub)]
#![cfg_attr(test, deny(warnings))]

//! A procedural macro attribute for instrumenting functions with [`tracing`].
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate provides the
//! [`#[instrument]`][instrument] procedural macro attribute.
//!
//! Note that this macro is also re-exported by the main `tracing` crate.
//!
//! ## Usage
//!
//! First, add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tracing-attributes = "0.1.0"
//! ```
//!
//! The [`#[instrument]`][instrument] attribute can now be added to a function
//! to automatically create and enter `tracing` [span] when that function is
//! called. For example:
//!
//! ```
//! use tracing_attributes::instrument;
//!
//! #[instrument]
//! pub fn my_function(my_arg: usize) {
//!     // ...
//! }
//!
//! # fn main() {}
//! ```
//!
//! [`tracing`]: https://crates.io/crates/tracing
//! [span]: https://docs.rs/tracing/0.1.3/tracing/span/index.html
//! [instrument]: attr.instrument.html
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned, ArgCaptured, AttributeArgs, FnArg, FnDecl, Ident, ItemFn, Lit, Meta,
    MetaNameValue, NestedMeta, Pat, PatIdent,
};

/// Instruments a function to create and enter a `tracing` [span] every time
/// the function is called.
///
/// The generated span's name will be the name of the function, and any
/// arguments to that function will be recorded as fields using `fmt::Debug`.
///
/// # Examples
/// Instrumenting a function:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument]
/// pub fn my_function(my_arg: usize) {
///     // This event will be recorded inside a span named `my_function` with the
///     // field `my_arg`.
///     tracing::info!("inside my_function!");
///     // ...
/// }
/// # fn main() {}
/// ```
/// Setting the level for the generated span:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(level = "debug")]
/// pub fn my_function() {
///     // ...
/// }
/// # fn main() {}
/// ```
/// Overriding the generated span's target:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(target = "my_target")]
/// pub fn my_function() {
///     // ...
/// }
/// # fn main() {}
/// ```
///
/// When the `async-await` feature flag is enabled, `async fn`s may also be
/// instrumented:
///
/// ```compile_fail
/// // this currently only compiles on nightly.
/// #![feature(async-await)]
/// # use tracing_attributes::instrument;
///
/// #[instrument]
/// pub async fn my_function() -> Result<(), ()> {
///     // ...
///     # Ok(())
/// }
/// # fn main() {}
/// ```
///
/// # Notes
/// - All argument types must implement `fmt::Debug`
/// - When using `#[instrument]` on an `async fn`, the `tracing_futures` must
///   also be specified as a dependency in `Cargo.toml`.
///
/// [span]: https://docs.rs/tracing/0.1.3/tracing/span/index.html
/// [`tracing`]: https://github.com/tokio-rs/tracing
#[proc_macro_attribute]
pub fn instrument(args: TokenStream, item: TokenStream) -> TokenStream {
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
    // which is `instrument`ed using `tracing-futures`. Otherwise, this will
    // enter the span and then perform the rest of the body.
    let body = if asyncness.is_some() {
        // We can't quote these keywords in the `quote!` macro, since their
        // presence in the file will make older Rust compilers fail to build
        // this crate. Instead, we construct token structs for them so the
        // strings "async" and "await" never actually appear in the source code
        // of this file.
        let async_kwd = syn::token::Async { span: block.span() };
        let await_kwd = syn::Ident::new("await", block.span());
        quote_spanned! {block.span()=>
            tracing_futures::Instrument::instrument(
                #async_kwd { #block },
                __tracing_attr_span
            )
                .#await_kwd
        }
    } else {
        quote_spanned!(block.span()=>
            let __tracing_attr_guard = __tracing_attr_span.enter();
            #block
        )
    };

    let level = level(&args);
    let target = target(&args);

    quote_spanned!(call_site=>
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident(#params) #return_type {
            let __tracing_attr_span = tracing::span!(
                target: #target,
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

fn target(args: &AttributeArgs) -> proc_macro2::TokenStream {
    let mut levels = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref ident, ref lit, ..
        })) if ident == "target" => Some(lit.clone()),
        _ => None,
    });
    let level = levels.next();

    // If we found more than one arg named "level", that's a syntax error...
    if let Some(lit) = levels.next() {
        return quote_spanned! {lit.span()=>
            compile_error!("expected only a single `target` argument!")
        };
    }

    match level {
        Some(Lit::Str(ref lit)) => quote!(#lit),
        Some(lit) => quote_spanned! {lit.span()=>
            compile_error!(
                "expected target to be a string literal"
            )
        },
        None => quote!(module_path!()),
    }
}
