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
//! tracing-attributes = "0.1.7"
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
//! [span]: https://docs.rs/tracing/latest/tracing/span/index.html
//! [instrument]: attr.instrument.html
#![doc(html_root_url = "https://docs.rs/tracing-attributes/0.1.7")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
// TODO: once `tracing` bumps its MSRV to 1.42, remove this allow.
#![allow(unused)]
extern crate proc_macro;

use std::collections::{HashMap, HashSet};
use std::iter;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    spanned::Spanned, AttributeArgs, Block, Expr, ExprCall, FieldPat, FnArg, Ident, Item, ItemFn,
    Lit, LitInt, Meta, MetaList, MetaNameValue, NestedMeta, Pat, PatIdent, PatReference, PatStruct,
    PatTuple, PatTupleStruct, PatType, Path, Signature, Stmt,
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
/// You can also pass additional fields (key-value pairs with arbitrary data)
/// to the generated span. This is achieved using the `fields` argument on the
/// `#[instrument]` macro. You can use a string, integer or boolean literal as
/// a value for each field. The name of the field must be a single valid Rust
/// identifier, nested (dotted) field names are not supported.
///
/// Note that overlap between the names of fields and (non-skipped) arguments
/// will result in a compile error.
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
/// ```
/// Setting the level for the generated span:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(level = "debug")]
/// pub fn my_function() {
///     // ...
/// }
/// ```
/// Overriding the generated span's target:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(target = "my_target")]
/// pub fn my_function() {
///     // ...
/// }
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
/// ```
///
/// To add an additional context to the span, you can pass key-value pairs to `fields`:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(fields(foo="bar", id=1, show=true))]
/// fn my_function(arg: usize) {
///     // ...
/// }
/// ```
///
/// If the function returns a `Result<T, E>` and `E` implements `std::fmt::Display`, you can add
/// `err` to emit error events when the function returns `Err`:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(err)]
/// fn my_function(arg: usize) -> Result<(), std::io::Error> {
///     Ok(())
/// }
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
/// ```
///
/// It also works with async-trait (and hopefully most of the libraries that does similar things):
///
/// ```
/// # use tracing::instrument;
/// use async_trait::async_trait;
///
/// #[async_trait]
/// pub trait Foo {
///     async fn foo(&self, v: usize) -> ();
/// }
///
/// #[derive(Debug)]
/// struct FooImpl;
///
/// #[async_trait]
/// impl Foo for FooImpl {
///     #[instrument(skip(self))]
///     async fn foo(&self, v: usize) {}
/// }
/// ```

///
/// [span]: https://docs.rs/tracing/latest/tracing/span/index.html
/// [`tracing`]: https://github.com/tokio-rs/tracing
/// [`fmt::Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html
#[proc_macro_attribute]
pub fn instrument(args: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = syn::parse_macro_input!(item as ItemFn);
    let args = syn::parse_macro_input!(args as AttributeArgs);

    // check for async_trait-like patterns in the block and wrap the internal function with Instrument
    // instead of wrapping the async_trait generated wrapper
    if let Some(internal_fun_name) = is_async_trait(&input.block, input.sig.asyncness.is_some()) {
        // let's rewrite some statements !
        let mut stmts: Vec<Stmt> = input.block.stmts.to_vec();
        for stmt in &mut stmts {
            if let Stmt::Item(Item::Fn(fun)) = stmt {
                // instrument the function if considered as the function we truly want to trace
                if fun.sig.ident == internal_fun_name {
                    *stmt = syn::parse2(gen_body(fun, args, Some(input.sig.ident.to_string())))
                        .unwrap();
                    break;
                }
            }
        }

        let sig = &input.sig;
        let attrs = &input.attrs;
        quote!(
            #(#attrs) *
            #sig {
                #(#stmts) *
            }
        )
        .into()
    } else {
        gen_body(&input, args, None).into()
    }
}

fn gen_body(
    input: &ItemFn,
    args: AttributeArgs,
    fun_name: Option<String>,
) -> proc_macro2::TokenStream {
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
    let ident_str = if let Some(x) = &fun_name {
        x.clone()
    } else {
        ident.to_string()
    };

    // generate this inside a closure, so we can return early on errors.
    let span = (|| {
        // Pull out the arguments-to-be-skipped first, so we can filter results below.
        let skips = match skips(&args) {
            Ok(skips) => skips,
            Err(err) => return quote!(#err),
        };

        let param_names: Vec<(Ident, Ident)> = params
            .clone()
            .into_iter()
            .flat_map(|param| match param {
                FnArg::Typed(PatType { pat, .. }) => param_names(*pat),
                FnArg::Receiver(_) => Box::new(iter::once(Ident::new("self", param.span()))),
            })
            // if we are inside a function generated by async-trait, we should take care to rewrite
            // "_self" as "self" for 'user convenience'
            .map(|x| {
                if fun_name.is_some() && x == "_self" {
                    (Ident::new("self", x.span()), x)
                } else {
                    (x.clone(), x)
                }
            })
            .collect();

        // TODO: allow the user to rename fields at will (all the machinery should be here)
        // Little dance with new (user-exposed) names and old (internal) names of identifiers.
        // That way, you can do the following even though async_trait rewrite "self" as "_self":
        // ```
        // #[async_trait]
        // impl Foo for FooImpl {
        //     #[instrument(skip(self))]
        //     async fn foo(&self, v: usize) {}
        // }
        // ```

        for skip in &skips {
            if !param_names.iter().map(|(user, _)| user).any(|y| y == skip) {
                return quote_spanned! {skip.span()=>
                    compile_error!("attempting to skip non-existent parameter")
                };
            }
        }

        // filter out skipped fields
        let param_names: Vec<(Ident, Ident)> = param_names
            .into_iter()
            .filter(|(ident, _)| !skips.contains(ident))
            .collect();

        let new_param_names: Vec<&Ident> = param_names.iter().map(|x| &x.0).collect();

        let fields = match fields(&args, &new_param_names) {
            Ok(fields) => fields,
            Err(err) => return quote!(#err),
        };

        let level = level(&args);
        let target = target(&args);
        let span_name = name(&args, ident_str);

        let mut quoted_fields: Vec<_> = param_names
            .iter()
            .map(|(user_name, real_name)| quote!(#user_name = tracing::field::debug(&#real_name)))
            .collect();
        quoted_fields.extend(fields.into_iter().map(|(key, value)| {
            let value = match value {
                Some(value) => quote!(#value),
                None => quote!(tracing::field::Empty),
            };

            quote!(#key = #value)
        }));
        quote!(tracing::span!(
            target: #target,
            #level,
            #span_name,
            #(#quoted_fields),*
        ))
    })();

    // Generate the instrumented function body.
    // If the function is an `async fn`, this will wrap it in an async block,
    // which is `instrument`ed using `tracing-futures`. Otherwise, this will
    // enter the span and then perform the rest of the body.
    // If `err` is in args, instrument any resulting `Err`s.
    let body = if asyncness.is_some() {
        if instrument_err(&args) {
            quote_spanned!(block.span()=>
                let __tracing_attr_span = #span;
                tracing_futures::Instrument::instrument(async move {
                    match async move { #block }.await {
                        Ok(x) => Ok(x),
                        Err(e) => {
                            tracing::error!(error = %e);
                            Err(e)
                        }
                    }
                }, __tracing_attr_span).await
            )
        } else {
            quote_spanned!(block.span()=>
                let __tracing_attr_span = #span;
                    tracing_futures::Instrument::instrument(
                        async move { #block },
                        __tracing_attr_span
                    )
                    .await
            )
        }
    } else if instrument_err(&args) {
        quote_spanned!(block.span()=>
            let __tracing_attr_span = #span;
            let __tracing_attr_guard = __tracing_attr_span.enter();
            match { #block } {
                Ok(x) => Ok(x),
                Err(e) => {
                    tracing::error!(error = %e);
                    Err(e)
                }
            }
        )
    } else {
        quote_spanned!(block.span()=>
            let __tracing_attr_span = #span;
            let __tracing_attr_guard = __tracing_attr_span.enter();
            #block
        )
    };

    quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            #body
        }
    )
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

fn skips(args: &[NestedMeta]) -> Result<HashSet<Ident>, impl ToTokens> {
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

fn level(args: &[NestedMeta]) -> impl ToTokens {
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

fn target(args: &[NestedMeta]) -> impl ToTokens {
    let mut targets = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
            ref path, ref lit, ..
        })) if path.is_ident("target") => Some(lit.clone()),
        _ => None,
    });
    let target = targets.next();

    // If we found more than one arg named "target", that's a syntax error...
    if let Some(lit) = targets.next() {
        return quote_spanned! {lit.span()=>
            compile_error!("expected only a single `target` argument!")
        };
    }

    match target {
        Some(Lit::Str(ref lit)) => quote!(#lit),
        Some(lit) => quote_spanned! {lit.span()=>
            compile_error!(
                "expected target to be a string literal"
            )
        },
        None => quote!(module_path!()),
    }
}

fn fields(
    args: &[NestedMeta],
    param_names: &[&Ident],
) -> Result<(Vec<(Ident, Option<Lit>)>), impl ToTokens> {
    let mut fields = args.iter().filter_map(|arg| match arg {
        NestedMeta::Meta(Meta::List(MetaList {
            ref path,
            ref nested,
            ..
        })) if path.is_ident("fields") => Some(nested.clone()),
        _ => None,
    });
    let field_holder = fields.next();

    // If we found more than one arg named "fields", that's a syntax error...
    if let Some(lit) = fields.next() {
        return Err(quote_spanned! {lit.span()=>
            compile_error!("expected only a single `fields` argument!")
        });
    }

    match field_holder {
        Some(fields) => {
            let mut parsed = Vec::default();
            let mut visited_keys: HashSet<String> = Default::default();
            let param_set: HashSet<String> = param_names.iter().map(|i| i.to_string()).collect();
            for field in fields.into_iter() {
                let (key, value) = match field {
                    NestedMeta::Meta(meta) => match meta {
                        Meta::NameValue(kv) => (kv.path, Some(kv.lit)),
                        Meta::Path(path) => (path, None),
                        _ => {
                            return Err(quote_spanned! {meta.span()=>
                                compile_error!("each field must be a key with an optional value. Keys must be valid Rust identifiers (nested keys with dots are not supported).")
                            })
                        }
                    },
                    _ => {
                        return Err(quote_spanned! {field.span()=>
                            compile_error!("`fields` argument should be a list of key-value fields")
                        })
                    }
                };

                let key = match key.get_ident() {
                    Some(key) => key,
                    None => {
                        return Err(quote_spanned! {key.span()=>
                            compile_error!("field keys must be valid Rust identifiers (nested keys with dots are not supported).")
                        })
                    }
                };

                let key_str = key.to_string();
                if param_set.contains(&key_str) {
                    return Err(quote_spanned! {key.span()=>
                        compile_error!("field overlaps with (non-skipped) parameter name")
                    });
                }

                if visited_keys.contains(&key_str) {
                    return Err(quote_spanned! {key.span()=>
                        compile_error!("each field key must appear at most once")
                    });
                } else {
                    visited_keys.insert(key_str);
                }

                if let Some(literal) = &value {
                    match literal {
                        Lit::Bool(_) | Lit::Str(_) | Lit::Int(_) => {}
                        _ => {
                            return Err(quote_spanned! {literal.span()=>
                                compile_error!("values can be only strings, integers or booleans")
                            })
                        }
                    }
                }

                parsed.push((key.clone(), value));
            }
            Ok(parsed)
        }
        None => Ok(Default::default()),
    }
}

fn name(args: &[NestedMeta], default_name: String) -> impl ToTokens {
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

fn instrument_err(args: &[NestedMeta]) -> bool {
    args.iter().any(|arg| match arg {
        NestedMeta::Meta(Meta::Path(path)) => path.is_ident("err"),
        _ => false,
    })
}

// terrible machinery to make async_trait-like cases works
// (following the approach suggested in
// https://github.com/dtolnay/async-trait/issues/45#issuecomment-571245673)
fn is_async_trait(block: &Block, block_is_async: bool) -> Option<String> {
    // are we in an async context ? If yes, this isn't a async_trait-like pattern
    if block_is_async {
        return None;
    }

    // list of async functions declared inside the block
    let mut inside_funs = Vec::new();
    // last expression declared in the block (it determines the return value of the block, so
    // that if we are working on a function whose `trait` or `impl` declaration is annotated by
    // async_trait, this is quite likely the point where the future is pinned)
    let mut last_expr = None;

    // obtain the list of direct internal functions and the last expression of the block
    for stmt in &block.stmts {
        if let Stmt::Item(Item::Fn(fun)) = &stmt {
            // is the function declared as async ? If so, this is a good candidate, let's keep it
            // in hand
            if fun.sig.asyncness.is_some() {
                inside_funs.push(fun.sig.ident.to_string());
            }
        } else if let Stmt::Expr(e) = &stmt {
            last_expr = Some(e);
        }
    }

    // let's play with (too much) pattern matching
    // is the last expression a function call ?
    if let Some(Expr::Call(ExprCall {
        func: outside_func,
        args: outside_args,
        ..
    })) = last_expr
    {
        if let Expr::Path(path) = outside_func.as_ref() {
            // is it a call to `Box::pin()` ?
            if "Box::pin"
                == path
                    .path
                    .segments
                    .iter()
                    .map(|x| x.ident.to_string())
                    .collect::<Vec<String>>()
                    .join("::")
            {
                // does it takes at least an argument ? (if it doesn't, it's not gonna compile anyway,
                // but that's no reason to (try to) perform an out of bounds access)
                if outside_args.is_empty() {
                    return None;
                }
                // is the argument to Box::pin a function call itself ?
                if let Expr::Call(ExprCall { func, args, .. }) = &outside_args[0] {
                    if let Expr::Path(inside_path) = func.as_ref() {
                        // "stringify" the path of the function called
                        let func_name = inside_path
                            .path
                            .segments
                            .iter()
                            .map(|x| x.ident.to_string())
                            .collect::<Vec<String>>()
                            .join("::");
                        // is this function directly defined insided the current block ?
                        if inside_funs.contains(&func_name) {
                            // we must hook this function now
                            return Some(func_name);
                        }
                    }
                }
            }
        }
    }
    None
}
