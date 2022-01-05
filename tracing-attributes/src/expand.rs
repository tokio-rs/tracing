use std::iter;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, Block, Expr, ExprAsync, ExprCall, FieldPat, FnArg,
    Ident, Item, ItemFn, Pat, PatIdent, PatReference, PatStruct, PatTuple, PatTupleStruct, PatType,
    Path, Signature, Stmt, Token, TypePath,
};

use crate::{
    attr::{Field, Fields, FormatMode, InstrumentArgs},
    MaybeItemFnRef,
};

/// Given an existing function, generate an instrumented version of that function
pub(crate) fn gen_function<'a, B: ToTokens + 'a>(
    input: MaybeItemFnRef<'a, B>,
    args: InstrumentArgs,
    instrumented_function_name: &str,
    self_type: Option<&syn::TypePath>,
) -> proc_macro2::TokenStream {
    // these are needed ahead of time, as ItemFn contains the function body _and_
    // isn't representable inside a quote!/quote_spanned! macro
    // (Syn's ToTokens isn't implemented for ItemFn)
    let MaybeItemFnRef {
        attrs,
        vis,
        sig,
        block,
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

    let warnings = args.warnings();

    let body = gen_block(
        block,
        params,
        asyncness.is_some(),
        args,
        instrumented_function_name,
        self_type,
    );

    quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            #warnings
            #body
        }
    )
}

/// Instrument a block
fn gen_block<B: ToTokens>(
    block: &B,
    params: &Punctuated<FnArg, Token![,]>,
    async_context: bool,
    mut args: InstrumentArgs,
    instrumented_function_name: &str,
    self_type: Option<&syn::TypePath>,
) -> proc_macro2::TokenStream {
    // generate the span's name
    let span_name = args
        // did the user override the span's name?
        .name
        .as_ref()
        .map(|name| quote!(#name))
        .unwrap_or_else(|| quote!(#instrumented_function_name));

    let level = args.level();
    let tracing = args.tracing();

    // generate this inside a closure, so we can return early on errors.
    let span = (|| {
        // Pull out the arguments-to-be-skipped first, so we can filter results
        // below.
        let param_names: Vec<(Ident, (Ident, RecordType))> = params
            .clone()
            .into_iter()
            .flat_map(|param| match param {
                FnArg::Typed(PatType { pat, ty, .. }) => {
                    param_names(*pat, RecordType::parse_from_ty(&*ty))
                }
                FnArg::Receiver(_) => Box::new(iter::once((
                    Ident::new("self", param.span()),
                    RecordType::Debug,
                ))),
            })
            // Little dance with new (user-exposed) names and old (internal)
            // names of identifiers. That way, we could do the following
            // even though async_trait (<=0.1.43) rewrites "self" as "_self":
            // ```
            // #[async_trait]
            // impl Foo for FooImpl {
            //     #[instrument(skip(self))]
            //     async fn foo(&self, v: usize) {}
            // }
            // ```
            .map(|(x, record_type)| {
                // if we are inside a function generated by async-trait <=0.1.43, we need to
                // take care to rewrite "_self" as "self" for 'user convenience'
                if self_type.is_some() && x == "_self" {
                    (Ident::new("self", x.span()), (x, record_type))
                } else {
                    (x.clone(), (x, record_type))
                }
            })
            .collect();

        for skip in &args.skips {
            if !param_names.iter().map(|(user, _)| user).any(|y| y == skip) {
                return quote_spanned! {skip.span()=>
                    compile_error!("attempting to skip non-existent parameter")
                };
            }
        }

        let target = args.target();

        // filter out skipped fields
        let quoted_fields: Vec<_> = param_names
            .iter()
            .filter(|(param, _)| {
                if args.skips.contains(param) {
                    return false;
                }

                // If any parameters have the same name as a custom field, skip
                // and allow them to be formatted by the custom field.
                if let Some(ref fields) = args.fields {
                    fields.0.iter().all(|Field { ref name, .. }| {
                        let first = name.first();
                        first != name.last() || !first.iter().any(|name| name == &param)
                    })
                } else {
                    true
                }
            })
            .map(|(user_name, (real_name, record_type))| match record_type {
                RecordType::Value => quote!(#user_name = #real_name),
                RecordType::Debug => quote!(#user_name = #tracing::field::debug(&#real_name)),
            })
            .collect();

        // replace every use of a variable with its original name
        if let Some(Fields(ref mut fields)) = args.fields {
            let mut replacer = IdentAndTypesRenamer {
                idents: param_names.into_iter().map(|(a, (b, _))| (a, b)).collect(),
                types: Vec::new(),
            };

            // when async-trait <=0.1.43 is in use, replace instances
            // of the "Self" type inside the fields values
            if let Some(self_type) = self_type {
                replacer.types.push(("Self", self_type.clone()));
            }

            for e in fields.iter_mut().filter_map(|f| f.value.as_mut()) {
                syn::visit_mut::visit_expr_mut(&mut replacer, e);
            }
        }

        let custom_fields = &args.fields;

        quote!(#tracing::span!(
            target: #target,
            #level,
            #span_name,
            #(#quoted_fields,)*
            #custom_fields

        ))
    })();

    let err_event = match args.err_mode {
        Some(FormatMode::Default) | Some(FormatMode::Display) => {
            Some(quote!(#tracing::error!(error = %e)))
        }
        Some(FormatMode::Debug) => Some(quote!(#tracing::error!(error = ?e))),
        _ => None,
    };

    let ret_event = match args.ret_mode {
        Some(FormatMode::Display) => Some(quote!(#tracing::event!(#level, return = %x))),
        Some(FormatMode::Default) | Some(FormatMode::Debug) => {
            Some(quote!(#tracing::event!(#level, return = ?x)))
        }
        _ => None,
    };

    // Generate the instrumented function body.
    // If the function is an `async fn`, this will wrap it in an async block,
    // which is `instrument`ed using `tracing-futures`. Otherwise, this will
    // enter the span and then perform the rest of the body.
    // If `err` is in args, instrument any resulting `Err`s.
    // If `ret` is in args, instrument any resulting `Ok`s when the function
    // returns `Result`s, otherwise instrument any resulting values.
    if async_context {
        let mk_fut = match (err_event, ret_event) {
            (Some(err_event), Some(ret_event)) => quote_spanned!(block.span()=>
                async move {
                    match async move { #block }.await {
                        #[allow(clippy::unit_arg)]
                        Ok(x) => {
                            #ret_event;
                            Ok(x)
                        },
                        Err(e) => {
                            #err_event;
                            Err(e)
                        }
                    }
                }
            ),
            (Some(err_event), None) => quote_spanned!(block.span()=>
                async move {
                    match async move { #block }.await {
                        #[allow(clippy::unit_arg)]
                        Ok(x) => Ok(x),
                        Err(e) => {
                            #err_event;
                            Err(e)
                        }
                    }
                }
            ),
            (None, Some(ret_event)) => quote_spanned!(block.span()=>
                async move {
                    let x = async move { #block }.await;
                    #ret_event;
                    x
                }
            ),
            (None, None) => quote_spanned!(block.span()=>
                async move { #block }
            ),
        };

        return quote!(
            let __tracing_attr_span = #span;
            let __tracing_instrument_future = #mk_fut;
            if !__tracing_attr_span.is_disabled() {
                #tracing::Instrument::instrument(
                    __tracing_instrument_future,
                    __tracing_attr_span
                )
                .await
            } else {
                __tracing_instrument_future.await
            }
        );
    }

    let span = quote!(
        // These variables are left uninitialized and initialized only
        // if the tracing level is statically enabled at this point.
        // While the tracing level is also checked at span creation
        // time, that will still create a dummy span, and a dummy guard
        // and drop the dummy guard later. By lazily initializing these
        // variables, Rust will generate a drop flag for them and thus
        // only drop the guard if it was created. This creates code that
        // is very straightforward for LLVM to optimize out if the tracing
        // level is statically disabled, while not causing any performance
        // regression in case the level is enabled.
        let __tracing_attr_span;
        let __tracing_attr_guard;
        if #tracing::level_enabled!(#level) {
            __tracing_attr_span = #span;
            __tracing_attr_guard = __tracing_attr_span.enter();
        }
    );

    match (err_event, ret_event) {
        (Some(err_event), Some(ret_event)) => quote_spanned! {block.span()=>
            #span
            #[allow(clippy::redundant_closure_call)]
            match (move || #block)() {
                #[allow(clippy::unit_arg)]
                Ok(x) => {
                    #ret_event;
                    Ok(x)
                },
                Err(e) => {
                    #err_event;
                    Err(e)
                }
            }
        },
        (Some(err_event), None) => quote_spanned!(block.span()=>
            #span
            #[allow(clippy::redundant_closure_call)]
            match (move || #block)() {
                #[allow(clippy::unit_arg)]
                Ok(x) => Ok(x),
                Err(e) => {
                    #err_event;
                    Err(e)
                }
            }
        ),
        (None, Some(ret_event)) => quote_spanned!(block.span()=>
            #span
            #[allow(clippy::redundant_closure_call)]
            let x = (move || #block)();
            #ret_event;
            x
        ),
        (None, None) => quote_spanned!(block.span() =>
            // Because `quote` produces a stream of tokens _without_ whitespace, the
            // `if` and the block will appear directly next to each other. This
            // generates a clippy lint about suspicious `if/else` formatting.
            // Therefore, suppress the lint inside the generated code...
            #[allow(clippy::suspicious_else_formatting)]
            {
                #span
                // ...but turn the lint back on inside the function body.
                #[warn(clippy::suspicious_else_formatting)]
                #block
            }
        ),
    }
}

/// Indicates whether a field should be recorded as `Value` or `Debug`.
enum RecordType {
    /// The field should be recorded using its `Value` implementation.
    Value,
    /// The field should be recorded using `tracing::field::debug()`.
    Debug,
}

impl RecordType {
    /// Array of primitive types which should be recorded as [RecordType::Value].
    const TYPES_FOR_VALUE: &'static [&'static str] = &[
        "bool",
        "str",
        "u8",
        "i8",
        "u16",
        "i16",
        "u32",
        "i32",
        "u64",
        "i64",
        "f32",
        "f64",
        "usize",
        "isize",
        "NonZeroU8",
        "NonZeroI8",
        "NonZeroU16",
        "NonZeroI16",
        "NonZeroU32",
        "NonZeroI32",
        "NonZeroU64",
        "NonZeroI64",
        "NonZeroUsize",
        "NonZeroIsize",
        "Wrapping",
    ];

    /// Parse `RecordType` from [syn::Type] by looking up
    /// the [RecordType::TYPES_FOR_VALUE] array.
    fn parse_from_ty(ty: &syn::Type) -> Self {
        match ty {
            syn::Type::Path(syn::TypePath { path, .. })
                if path
                    .segments
                    .iter()
                    .last()
                    .map(|path_segment| {
                        let ident = path_segment.ident.to_string();
                        Self::TYPES_FOR_VALUE.iter().any(|&t| t == ident)
                    })
                    .unwrap_or(false) =>
            {
                RecordType::Value
            }
            syn::Type::Reference(syn::TypeReference { elem, .. }) => {
                RecordType::parse_from_ty(&*elem)
            }
            _ => RecordType::Debug,
        }
    }
}

fn param_names(pat: Pat, record_type: RecordType) -> Box<dyn Iterator<Item = (Ident, RecordType)>> {
    match pat {
        Pat::Ident(PatIdent { ident, .. }) => Box::new(iter::once((ident, record_type))),
        Pat::Reference(PatReference { pat, .. }) => param_names(*pat, record_type),
        // We can't get the concrete type of fields in the struct/tuple
        // patterns by using `syn`. e.g. `fn foo(Foo { x, y }: Foo) {}`.
        // Therefore, the struct/tuple patterns in the arguments will just
        // always be recorded as `RecordType::Debug`.
        Pat::Struct(PatStruct { fields, .. }) => Box::new(
            fields
                .into_iter()
                .flat_map(|FieldPat { pat, .. }| param_names(*pat, RecordType::Debug)),
        ),
        Pat::Tuple(PatTuple { elems, .. }) => Box::new(
            elems
                .into_iter()
                .flat_map(|p| param_names(p, RecordType::Debug)),
        ),
        Pat::TupleStruct(PatTupleStruct {
            pat: PatTuple { elems, .. },
            ..
        }) => Box::new(
            elems
                .into_iter()
                .flat_map(|p| param_names(p, RecordType::Debug)),
        ),

        // The above *should* cover all cases of irrefutable patterns,
        // but we purposefully don't do any funny business here
        // (such as panicking) because that would obscure rustc's
        // much more informative error message.
        _ => Box::new(iter::empty()),
    }
}

enum AsyncTraitKind<'a> {
    // old construction. Contains the function
    Function(&'a ItemFn),
    // new construction. Contains a reference to the async block
    Async(&'a ExprAsync),
}

pub(crate) struct AsyncTraitInfo<'block> {
    // statement that must be patched
    source_stmt: &'block Stmt,
    kind: AsyncTraitKind<'block>,
    self_type: Option<syn::TypePath>,
    input: &'block ItemFn,
}

impl<'block> AsyncTraitInfo<'block> {
    /// Get the AST of the inner function we need to hook, if it was generated
    /// by async-trait.
    ///
    /// When we are given a function annotated by async-trait, that function
    /// is only a placeholder that returns a pinned future containing the
    /// user logic, and it is that pinned future that needs to be instrumented.
    /// Were we to instrument its parent, we would only collect information
    /// regarding the allocation of that future, and not its own span of execution.
    /// Depending on the version of async-trait, we inspect the block of the function
    /// to find if it matches the pattern
    ///
    /// `async fn foo<...>(...) {...}; Box::pin(foo<...>(...))` (<=0.1.43), or if
    /// it matches `Box::pin(async move { ... }) (>=0.1.44). We the return the
    /// statement that must be instrumented, along with some other informations.
    /// 'gen_body' will then be able to use that information to instrument the
    /// proper function/future.
    ///
    /// (this follows the approach suggested in
    /// https://github.com/dtolnay/async-trait/issues/45#issuecomment-571245673)
    pub(crate) fn from_fn(input: &'block ItemFn) -> Option<Self> {
        // are we in an async context? If yes, this isn't a async_trait-like pattern
        if input.sig.asyncness.is_some() {
            return None;
        }

        let block = &input.block;

        // list of async functions declared inside the block
        let inside_funs = block.stmts.iter().filter_map(|stmt| {
            if let Stmt::Item(Item::Fn(fun)) = &stmt {
                // If the function is async, this is a candidate
                if fun.sig.asyncness.is_some() {
                    return Some((stmt, fun));
                }
            }
            None
        });

        // last expression of the block (it determines the return value
        // of the block, so that if we are working on a function whose
        // `trait` or `impl` declaration is annotated by async_trait,
        // this is quite likely the point where the future is pinned)
        let (last_expr_stmt, last_expr) = block.stmts.iter().rev().find_map(|stmt| {
            if let Stmt::Expr(expr) = stmt {
                Some((stmt, expr))
            } else {
                None
            }
        })?;

        // is the last expression a function call?
        let (outside_func, outside_args) = match last_expr {
            Expr::Call(ExprCall { func, args, .. }) => (func, args),
            _ => return None,
        };

        // is it a call to `Box::pin()`?
        let path = match outside_func.as_ref() {
            Expr::Path(path) => &path.path,
            _ => return None,
        };
        if !path_to_string(path).ends_with("Box::pin") {
            return None;
        }

        // Does the call take an argument? If it doesn't,
        // it's not gonna compile anyway, but that's no reason
        // to (try to) perform an out of bounds access
        if outside_args.is_empty() {
            return None;
        }

        // Is the argument to Box::pin an async block that
        // captures its arguments?
        if let Expr::Async(async_expr) = &outside_args[0] {
            // check that the move 'keyword' is present
            async_expr.capture?;

            return Some(AsyncTraitInfo {
                source_stmt: last_expr_stmt,
                kind: AsyncTraitKind::Async(async_expr),
                self_type: None,
                input,
            });
        }

        // Is the argument to Box::pin a function call itself?
        let func = match &outside_args[0] {
            Expr::Call(ExprCall { func, .. }) => func,
            _ => return None,
        };

        // "stringify" the path of the function called
        let func_name = match **func {
            Expr::Path(ref func_path) => path_to_string(&func_path.path),
            _ => return None,
        };

        // Was that function defined inside of the current block?
        // If so, retrieve the statement where it was declared and the function itself
        let (stmt_func_declaration, func) = inside_funs
            .into_iter()
            .find(|(_, fun)| fun.sig.ident == func_name)?;

        // If "_self" is present as an argument, we store its type to be able to rewrite "Self" (the
        // parameter type) with the type of "_self"
        let mut self_type = None;
        for arg in &func.sig.inputs {
            if let FnArg::Typed(ty) = arg {
                if let Pat::Ident(PatIdent { ref ident, .. }) = *ty.pat {
                    if ident == "_self" {
                        let mut ty = *ty.ty.clone();
                        // extract the inner type if the argument is "&self" or "&mut self"
                        if let syn::Type::Reference(syn::TypeReference { elem, .. }) = ty {
                            ty = *elem;
                        }

                        if let syn::Type::Path(tp) = ty {
                            self_type = Some(tp);
                            break;
                        }
                    }
                }
            }
        }

        Some(AsyncTraitInfo {
            source_stmt: stmt_func_declaration,
            kind: AsyncTraitKind::Function(func),
            self_type,
            input,
        })
    }

    pub(crate) fn gen_async_trait(
        self,
        args: InstrumentArgs,
        instrumented_function_name: &str,
    ) -> proc_macro::TokenStream {
        // let's rewrite some statements!
        let mut out_stmts: Vec<TokenStream> = self
            .input
            .block
            .stmts
            .iter()
            .map(|stmt| stmt.to_token_stream())
            .collect();

        if let Some((iter, _stmt)) = self
            .input
            .block
            .stmts
            .iter()
            .enumerate()
            .find(|(_iter, stmt)| *stmt == self.source_stmt)
        {
            // instrument the future by rewriting the corresponding statement
            out_stmts[iter] = match self.kind {
                // async-trait <= 0.1.43
                AsyncTraitKind::Function(fun) => gen_function(
                    fun.into(),
                    args,
                    instrumented_function_name,
                    self.self_type.as_ref(),
                ),
                // async-trait >= 0.1.44
                AsyncTraitKind::Async(async_expr) => {
                    let instrumented_block = gen_block(
                        &async_expr.block,
                        &self.input.sig.inputs,
                        true,
                        args,
                        instrumented_function_name,
                        None,
                    );
                    let async_attrs = &async_expr.attrs;
                    quote! {
                        Box::pin(#(#async_attrs) * async move { #instrumented_block })
                    }
                }
            };
        }

        let vis = &self.input.vis;
        let sig = &self.input.sig;
        let attrs = &self.input.attrs;
        quote!(
            #(#attrs) *
            #vis #sig {
                #(#out_stmts) *
            }
        )
        .into()
    }
}

// Return a path as a String
fn path_to_string(path: &Path) -> String {
    use std::fmt::Write;
    // some heuristic to prevent too many allocations
    let mut res = String::with_capacity(path.segments.len() * 5);
    for i in 0..path.segments.len() {
        write!(&mut res, "{}", path.segments[i].ident)
            .expect("writing to a String should never fail");
        if i < path.segments.len() - 1 {
            res.push_str("::");
        }
    }
    res
}

/// A visitor struct to replace idents and types in some piece
/// of code (e.g. the "self" and "Self" tokens in user-supplied
/// fields expressions when the function is generated by an old
/// version of async-trait).
struct IdentAndTypesRenamer<'a> {
    types: Vec<(&'a str, TypePath)>,
    idents: Vec<(Ident, Ident)>,
}

impl<'a> syn::visit_mut::VisitMut for IdentAndTypesRenamer<'a> {
    // we deliberately compare strings because we want to ignore the spans
    // If we apply clippy's lint, the behavior changes
    #[allow(clippy::cmp_owned)]
    fn visit_ident_mut(&mut self, id: &mut Ident) {
        for (old_ident, new_ident) in &self.idents {
            if id.to_string() == old_ident.to_string() {
                *id = new_ident.clone();
            }
        }
    }

    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        for (type_name, new_type) in &self.types {
            if let syn::Type::Path(TypePath { path, .. }) = ty {
                if path_to_string(path) == *type_name {
                    *ty = syn::Type::Path(new_type.clone());
                }
            }
        }
    }
}

// A visitor struct that replace an async block by its patched version
struct AsyncTraitBlockReplacer<'a> {
    block: &'a Block,
    patched_block: Block,
}

impl<'a> syn::visit_mut::VisitMut for AsyncTraitBlockReplacer<'a> {
    fn visit_block_mut(&mut self, i: &mut Block) {
        if i == self.block {
            *i = self.patched_block.clone();
        }
    }
}
