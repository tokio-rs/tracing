//! A procedural macro attribute for instrumenting functions with [`tracing`].
//!
//! [`tracing`] is a framework for instrumenting Rust programs to collect
//! structured, event-based diagnostic information. This crate provides the
//! [`#[instrument]`][instrument] procedural macro attribute.
//!
//! Note that this macro is also re-exported by the main `tracing` crate.
//!
//! *Compiler support: [requires `rustc` 1.49+][msrv]*
//!
//! [msrv]: #supported-rust-versions
//!
//! ## Usage
//!
//! In the `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tracing = "0.1"
//! ```
//!
//! The [`#[instrument]`][instrument] attribute can now be added to a function
//! to automatically create and enter `tracing` [span] when that function is
//! called. For example:
//!
//! ```
//! use tracing::instrument;
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
//! [instrument]: macro@instrument
//! [span]: https://docs.rs/tracing/latest/tracing/span/index.html
//!
//! ## Supported Rust Versions
//!
//! Tracing is built against the latest stable release. The minimum supported
//! version is 1.49. The current Tracing version is not guaranteed to build on
//! Rust versions earlier than the minimum supported version.
//!
//! Tracing follows the same compiler support policies as the rest of the Tokio
//! project. The current stable Rust compiler and the three most recent minor
//! versions before it will always be supported. For example, if the current
//! stable compiler version is 1.45, the minimum supported version will not be
//! increased past 1.42, three minor versions prior. Increasing the minimum
//! supported compiler version is not considered a semver breaking change as
//! long as doing so complies with this policy.
//!
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/logo-type.png",
    html_favicon_url = "https://raw.githubusercontent.com/tokio-rs/tracing/master/assets/favicon.ico",
    issue_tracker_base_url = "https://github.com/tokio-rs/tracing/issues/"
)]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
    unconditional_recursion,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, ItemFn, Signature, Visibility};

mod attr;
mod expand;
/// Instruments a function to create and enter a `tracing` [span] every time
/// the function is called.
///
/// Unless overriden, a span with `info` level will be generated.
/// The generated span's name will be the name of the function.
/// By default, all arguments to the function are included as fields on the
/// span. Arguments that are `tracing` [primitive types] implementing the
/// [`Value` trait] will be recorded as fields of that type. Types which do
/// not implement `Value` will be recorded using [`std::fmt::Debug`].
///
/// [primitive types]: https://docs.rs/tracing/latest/tracing/field/trait.Value.html#foreign-impls
/// [`Value` trait]: https://docs.rs/tracing/latest/tracing/field/trait.Value.html
///
/// To skip recording a function's or method's argument, pass the argument's name
/// to the `skip` argument on the `#[instrument]` macro. For example,
/// `skip` can be used when an argument to an instrumented function does
/// not implement [`fmt::Debug`], or to exclude an argument with a verbose
/// or costly Debug implementation. Note that:
/// - multiple argument names can be passed to `skip`.
/// - arguments passed to `skip` do _not_ need to implement `fmt::Debug`.
///
/// Additional fields (key-value pairs with arbitrary data) can be passed to
/// to the generated span through the `fields` argument on the
/// `#[instrument]` macro. Strings, integers or boolean literals are accepted values
/// for each field. The name of the field must be a single valid Rust
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
/// # use tracing::Level;
/// #[instrument(level = Level::DEBUG)]
/// pub fn my_function() {
///     // ...
/// }
/// ```
/// Levels can be specified either with [`Level`] constants, literal strings
/// (e.g., `"debug"`, `"info"`) or numerically (1—5, corresponding to [`Level::TRACE`]—[`Level::ERROR`]).
///
/// Overriding the generated span's name:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(name = "my_name")]
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
/// Overriding the generated span's parent:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(parent = None)]
/// pub fn my_function() {
///     // ...
/// }
/// ```
/// ```
/// # use tracing_attributes::instrument;
/// // A struct which owns a span handle.
/// struct MyStruct
/// {
///     span: tracing::Span
/// }
///
/// impl MyStruct
/// {
///     // Use the struct's `span` field as the parent span
///     #[instrument(parent = &self.span, skip(self))]
///     fn my_method(&self) {}
/// }
/// ```
/// Specifying [`follows_from`] relationships:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(follows_from = causes)]
/// pub fn my_function(causes: &[tracing::Id]) {
///     // ...
/// }
/// ```
/// Any expression of type `impl IntoIterator<Item = impl Into<Option<Id>>>`
/// may be provided to `follows_from`; e.g.:
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(follows_from = [cause])]
/// pub fn my_function(cause: &tracing::span::EnteredSpan) {
///     // ...
/// }
/// ```
///
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
/// To add additional context to the span, pass key-value pairs to `fields`:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(fields(foo="bar", id=1, show=true))]
/// fn my_function(arg: usize) {
///     // ...
/// }
/// ```
///
/// Adding the `ret` argument to `#[instrument]` will emit an event with the function's
/// return value when the function returns:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(ret)]
/// fn my_function() -> i32 {
///     42
/// }
/// ```
/// The level of the return value event defaults to the same level as the span generated by `#[instrument]`.
/// By default, this will be `TRACE`, but if the span level is overridden, the event will be at the same
/// level.
///
/// It's also possible to override the level for the `ret` event independently:
///
/// ```
/// # use tracing_attributes::instrument;
/// # use tracing::Level;
/// #[instrument(ret(level = Level::WARN))]
/// fn my_function() -> i32 {
///     42
/// }
/// ```
///
/// **Note**:  if the function returns a `Result<T, E>`, `ret` will record returned values if and
/// only if the function returns [`Result::Ok`].
///
/// By default, returned values will be recorded using their [`std::fmt::Debug`] implementations.
/// If a returned value implements [`std::fmt::Display`], it can be recorded using its `Display`
/// implementation instead, by writing `ret(Display)`:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(ret(Display))]
/// fn my_function() -> i32 {
///     42
/// }
/// ```
///
/// If the function returns a `Result<T, E>` and `E` implements `std::fmt::Display`, adding
/// `err` or `err(Display)` will emit error events when the function returns `Err`:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(err)]
/// fn my_function(arg: usize) -> Result<(), std::io::Error> {
///     Ok(())
/// }
/// ```
///
/// The level of the error value event defaults to `ERROR`.
///
/// Similarly, overriding the level of the `err` event :
///
/// ```
/// # use tracing_attributes::instrument;
/// # use tracing::Level;
/// #[instrument(err(level = Level::INFO))]
/// fn my_function(arg: usize) -> Result<(), std::io::Error> {
///     Ok(())
/// }
/// ```
///
/// By default, error values will be recorded using their `std::fmt::Display` implementations.
/// If an error implements `std::fmt::Debug`, it can be recorded using its `Debug` implementation
/// instead by writing `err(Debug)`:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(err(Debug))]
/// fn my_function(arg: usize) -> Result<(), std::io::Error> {
///     Ok(())
/// }
/// ```
///
/// If a `target` is specified, both the `ret` and `err` arguments will emit outputs to
/// the declared target (or the default channel if `target` is not specified).
///
/// The `ret` and `err` arguments can be combined in order to record an event if a
/// function returns [`Result::Ok`] or [`Result::Err`]:
///
/// ```
/// # use tracing_attributes::instrument;
/// #[instrument(err, ret)]
/// fn my_function(arg: usize) -> Result<(), std::io::Error> {
///     Ok(())
/// }
/// ```
///
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
/// It also works with [async-trait](https://crates.io/crates/async-trait)
/// (a crate that allows defining async functions in traits,
/// something not currently possible in Rust),
/// and hopefully most libraries that exhibit similar behaviors:
///
/// ```
/// # use tracing::instrument;
/// use async_trait::async_trait;
///
/// #[async_trait]
/// pub trait Foo {
///     async fn foo(&self, arg: usize);
/// }
///
/// #[derive(Debug)]
/// struct FooImpl(usize);
///
/// #[async_trait]
/// impl Foo for FooImpl {
///     #[instrument(fields(value = self.0, tmp = std::any::type_name::<Self>()))]
///     async fn foo(&self, arg: usize) {}
/// }
/// ```
///
/// `const fn` cannot be instrumented, and will result in a compilation failure:
///
/// ```compile_fail
/// # use tracing_attributes::instrument;
/// #[instrument]
/// const fn my_const_function() {}
/// ```
///
/// [span]: https://docs.rs/tracing/latest/tracing/span/index.html
/// [`follows_from`]: https://docs.rs/tracing/latest/tracing/struct.Span.html#method.follows_from
/// [`tracing`]: https://github.com/tokio-rs/tracing
/// [`fmt::Debug`]: std::fmt::Debug
/// [`Level`]: https://docs.rs/tracing/latest/tracing/struct.Level.html
/// [`Level::TRACE`]: https://docs.rs/tracing/latest/tracing/struct.Level.html#associatedconstant.TRACE
/// [`Level::ERROR`]: https://docs.rs/tracing/latest/tracing/struct.Level.html#associatedconstant.ERROR
#[proc_macro_attribute]
pub fn instrument(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(args as attr::InstrumentArgs);
    // Cloning a `TokenStream` is cheap since it's reference counted internally.
    instrument_precise(args.clone(), item.clone())
        .unwrap_or_else(|_err| instrument_speculative(args, item))
}

/// Instrument the function, without parsing the function body (instead using the raw tokens).
fn instrument_speculative(
    args: attr::InstrumentArgs,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as MaybeItemFn);
    let instrumented_function_name = input.sig.ident.to_string();
    expand::gen_function(
        input.as_ref(),
        args,
        instrumented_function_name.as_str(),
        None,
    )
    .into()
}

/// Instrument the function, by fully parsing the function body,
/// which allows us to rewrite some statements related to async-like patterns.
fn instrument_precise(
    args: attr::InstrumentArgs,
    item: proc_macro::TokenStream,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let input = syn::parse::<ItemFn>(item)?;
    let instrumented_function_name = input.sig.ident.to_string();

    if input.sig.constness.is_some() {
        return Ok(quote! {
            compile_error!("the `#[instrument]` attribute may not be used with `const fn`s")
        }
        .into());
    }

    // check for async_trait-like patterns in the block, and instrument
    // the future instead of the wrapper
    if let Some(async_like) = expand::AsyncInfo::from_fn(&input) {
        return async_like.gen_async(args, instrumented_function_name.as_str());
    }

    let input = MaybeItemFn::from(input);

    Ok(expand::gen_function(
        input.as_ref(),
        args,
        instrumented_function_name.as_str(),
        None,
    )
    .into())
}

/// This is a more flexible/imprecise `ItemFn` type,
/// which's block is just a `TokenStream` (it may contain invalid code).
#[derive(Debug, Clone)]
struct MaybeItemFn {
    outer_attrs: Vec<Attribute>,
    inner_attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    block: TokenStream,
}

impl MaybeItemFn {
    fn as_ref(&self) -> MaybeItemFnRef<'_, TokenStream> {
        MaybeItemFnRef {
            outer_attrs: &self.outer_attrs,
            inner_attrs: &self.inner_attrs,
            vis: &self.vis,
            sig: &self.sig,
            block: &self.block,
        }
    }
}

/// This parses a `TokenStream` into a `MaybeItemFn`
/// (just like `ItemFn`, but skips parsing the body).
impl Parse for MaybeItemFn {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let outer_attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        let sig: Signature = input.parse()?;
        let inner_attrs = input.call(Attribute::parse_inner)?;
        let block: TokenStream = input.parse()?;
        Ok(Self {
            outer_attrs,
            inner_attrs,
            vis,
            sig,
            block,
        })
    }
}

impl From<ItemFn> for MaybeItemFn {
    fn from(
        ItemFn {
            attrs,
            vis,
            sig,
            block,
        }: ItemFn,
    ) -> Self {
        let (outer_attrs, inner_attrs) = attrs
            .into_iter()
            .partition(|attr| attr.style == syn::AttrStyle::Outer);
        Self {
            outer_attrs,
            inner_attrs,
            vis,
            sig,
            block: block.to_token_stream(),
        }
    }
}

/// A generic reference type for `MaybeItemFn`,
/// that takes a generic block type `B` that implements `ToTokens` (eg. `TokenStream`, `Block`).
#[derive(Debug, Clone)]
struct MaybeItemFnRef<'a, B: ToTokens> {
    outer_attrs: &'a Vec<Attribute>,
    inner_attrs: &'a Vec<Attribute>,
    vis: &'a Visibility,
    sig: &'a Signature,
    block: &'a B,
}
