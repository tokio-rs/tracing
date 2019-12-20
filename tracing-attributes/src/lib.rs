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
//! *Compiler support: requires rustc 1.39+*
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
//! [span]: https://docs.rs/tracing/0.1.6/tracing/span/index.html
//! [instrument]: attr.instrument.html
#![doc(html_root_url = "https://docs.rs/tracing-attributes/0.1.6")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    legacy_directory_ownership,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    plugin_as_library,
    private_in_public,
    safe_extern_statics,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
extern crate proc_macro;

use std::collections::HashSet;
use std::iter;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    spanned::Spanned, AttributeArgs, FieldPat, FnArg, Ident, ItemFn, Lit, LitInt, Meta, MetaList,
    MetaNameValue, NestedMeta, Pat, PatIdent, PatReference, PatStruct, PatTuple, PatTupleStruct,
    PatType, Signature,
};

/// Instruments a function to create and enter a `tracing` [span] every time
/// the function is called.
///
/// The generated span's name will be the name of the function. Any arguments
/// to that function will be recorded as fields using [`fmt::Debug`]. To skip
/// recording a function's or method's argument, pass the argument's name
/// to the `skip` argument on the `#[instrument]` macro. For example,
/// `skip` can be used when an argument to an instrumented function does
/// not implement [`fmt::Debug`], or to exclude an argument with a verbose
/// or costly Debug implementation. Note that:
/// - multiple argument names can be passed to `skip`.
/// - arguments passed to `skip` do _not_ need to implement `fmt::Debug`.
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
/// To skip recording an argument, pass the argument's name to the `skip`:
///
/// ```
/// # use tracing_attributes::instrument;
/// struct NonDebug;
///
/// #[instrument(skip(non_debug))]
/// fn my_function(arg: usize, non_debug: NonDebug) {
///     // ...
/// }
/// # fn main() {}
/// ```
///
/// If `tracing_futures` is specified as a dependency in `Cargo.toml`,
/// `async fn`s may also be instrumented:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument]
/// pub async fn my_function() -> Result<(), ()> {
///     // ...
///     # Ok(())
/// }
/// # fn main() {}
/// ```
///
/// [span]: https://docs.rs/tracing/0.1.6/tracing/span/index.html
/// [`tracing`]: https://github.com/tokio-rs/tracing
/// [`fmt::Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
#[proc_macro_attribute]
pub fn instrument(args: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = syn::parse_macro_input!(item as ItemFn);
    let args = syn::parse_macro_input!(args as AttributeArgs);

    // these are needed ahead of time, as ItemFn contains the function body _and_
    // isn't representable inside a quote!/quote_spanned! macro
    // (Syn's ToTokens isn't implemented for ItemFn)
    let ItemFn {
        attrs,
        vis,
        block,
        sig,
        ..
    } = input;

    let Signature {
        output: return_type,
        inputs: params,
        unsafety,
        asyncness,
        constness,
        abi,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        ..
    } = sig;

    // function name
    let ident_str = ident.to_string();

    // Pull out the arguments-to-be-skipped first, so we can filter results below.
    let skips = match skips(&args) {
        Ok(skips) => skips,
        Err(err) => return quote!(#err).into(),
    };

    let param_names: Vec<Ident> = params
        .clone()
        .into_iter()
        .flat_map(|param| match param {
            FnArg::Typed(PatType { pat, .. }) => param_names(*pat),
            FnArg::Receiver(_) => Box::new(iter::once(Ident::new("self", param.span()))),
        })
        .filter(|ident| !skips.contains(ident))
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
                #async_kwd move { #block },
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
    let span_name = name(&args, ident_str);

    quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            let __tracing_attr_span = tracing::span!(
                target: #target,
                #level,
                #span_name,
                #(#param_names = tracing::field::debug(&#param_names_clone)),*
            );
            #body
        }
    )
    .into()
}

fn param_names(pat: Pat) -> Box<dyn Iterator<Item = Ident>> {
    match pat {
        Pat::Ident(PatIdent { ident, .. }) => Box::new(iter::once(ident)),
        Pat::Reference(PatReference { pat, .. }) => param_names(*pat),
        Pat::Struct(PatStruct { fields, .. }) => Box::new(
            fields
                .into_iter()
                .flat_map(|FieldPat { pat, .. }| param_names(*pat)),
        ),
        Pat::Tuple(PatTuple { elems, .. }) => Box::new(elems.into_iter().flat_map(param_names)),
        Pat::TupleStruct(PatTupleStruct {
            pat: PatTuple { elems, .. },
            ..
        }) => Box::new(elems.into_iter().flat_map(param_names)),

        // The above *should* cover all cases of irrefutable patterns,
        // but we purposefully don't do any funny business here
        // (such as panicking) because that would obscure rustc's
        // much more informative error message.
        _ => Box::new(iter::empty()),
    }
}

fn skips(args: &AttributeArgs) -> Result<HashSet<Ident>, impl ToTokens> {
    let mut skips = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::List(MetaList {
            ref path,
            ref nested,
            ..
        })) if path.is_ident("skip") => Some(nested),
        _ => None,
    });
    let skip = skips.next();

    // Ensure there's only one skip directive.
    if let Some(list) = skips.next() {
        return Err(quote_spanned! {
            list.span() => compile_error!("expected only a single `skip` argument!")
        });
    }

    // Collect the Idents inside the `skip(...)`, if it exists
    Ok(skip
        .iter()
        .map(|list| list.iter())
        .flatten()
        .filter_map(|meta| match meta {
            NestedMeta::Meta(Meta::Path(p)) => p.get_ident().map(Clone::clone),
            _ => None,
        })
        .collect())
}

fn level(args: &AttributeArgs) -> impl ToTokens {
    let mut levels = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref path, ref lit, ..
        })) if path.is_ident("level") => Some(lit.clone()),
        _ => None,
    });
    let level = levels.next();

    // If we found more than one arg named "level", that's a syntax error...
    if let Some(lit) = levels.next() {
        return quote_spanned! {lit.span()=>
            compile_error!("expected only a single `level` argument!")
        };
    }

    fn is_level(lit: &LitInt, expected: u64) -> bool {
        match lit.base10_parse::<u64>() {
            Ok(value) => value == expected,
            Err(_) => false,
        }
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
        Some(Lit::Int(ref lit)) if is_level(lit, 1) => quote!(tracing::Level::TRACE),
        Some(Lit::Int(ref lit)) if is_level(lit, 2) => quote!(tracing::Level::DEBUG),
        Some(Lit::Int(ref lit)) if is_level(lit, 3) => quote!(tracing::Level::INFO),
        Some(Lit::Int(ref lit)) if is_level(lit, 4) => quote!(tracing::Level::WARN),
        Some(Lit::Int(ref lit)) if is_level(lit, 5) => quote!(tracing::Level::ERROR),
        Some(lit) => quote_spanned! {lit.span()=>
            compile_error!(
                "unknown verbosity level, expected one of \"trace\", \
                 \"debug\", \"info\", \"warn\", or \"error\", or a number 1-5"
            )
        },
        None => quote!(tracing::Level::INFO),
    }
}

fn target(args: &AttributeArgs) -> impl ToTokens {
    let mut levels = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref path, ref lit, ..
        })) if path.is_ident("target") => Some(lit.clone()),
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

fn name(args: &AttributeArgs, default_name: String) -> impl ToTokens {
    let mut names = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref path, ref lit, ..
        })) if path.is_ident("name") => Some(lit.clone()),
        _ => None,
    });

    let name = names.next();

    // If we found more than one arg named "name", that's a syntax error.
    if let Some(lit) = names.next() {
        return quote_spanned! {lit.span() =>
            compile_error!("expected only a single `name` argument!")
        };
    }

    match name {
        Some(Lit::Str(ref lit)) => quote!(#lit),
        Some(lit) => {
            quote_spanned! { lit.span() => compile_error!("expected name to be a string literal") }
        }
        None => quote!(#default_name),
    }
}
