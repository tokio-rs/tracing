#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_hack::proc_macro_hack;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::{
    braced, parse_macro_input, punctuated::Punctuated, token, Expr, ExprVerbatim, Ident, LitStr,
    Token,
};

#[proc_macro_hack]
pub fn span(tokens: TokenStream) -> TokenStream {
    let TraceSpan {
        options: Options {
            target,
            parent,
            level,
        },
        name,
        fields,
    } = parse_macro_input!(tokens as TraceSpan);
    let callsite = callsite(
        quote! { #name },
        target.map_or(quote! { module_path!() }, |x| quote! { #x }),
        quote! { #level },
        fields.iter().map(|x| &x.name[..]),
        quote! { tracing::metadata::Kind::SPAN },
    );
    let fields = fields.iter().map(|Field { value, .. }| {
        quote! {
            (
                &field_iter.next().expect("FieldSet corrupted (this is a bug)"),
                Some(&#value as &dyn Value)
            )
        }
    });
    let fields2 = fields.clone();
    let fields3 = fields.clone();
    let dispatch = if let Some(parent) = parent {
        quote! {
            Span::child_of(#parent, meta, &fields.value_set(&[#(#fields),*]))
        }
    } else {
        quote! {
            Span::new(meta, &fields.value_set(&[#(#fields2),*]))
        }
    };

    let disabled = if cfg!(feature = "log") {
        quote! {
            let span = Span::new_disabled(meta);
            span.record_all(&fields.value_set(&[#(#fields3),*]));
            span
        }
    } else {
        quote! {
            Span::new_disabled(meta)
        }
    };
    let expanded = quote! {{
        #[allow(unused_imports)]
        use tracing::{callsite, dispatcher, Span, field::{Value, ValueSet}};
        use tracing::callsite::Callsite;
        let callsite = #callsite;
        let meta = callsite.metadata();
        let fields = meta.fields();
        let mut field_iter = fields.iter();
        if #level <= tracing::level_filters::STATIC_MAX_LEVEL && tracing::is_enabled!(callsite) {
            #dispatch
        } else {
            #disabled
        }
    }};
    TokenStream::from(expanded)
}

#[proc_macro_hack]
pub fn event(tokens: TokenStream) -> TokenStream {
    let Event {
        options: Options {
            target,
            parent,
            level,
        },
        fields,
    } = parse_macro_input!(tokens as Event);
    let callsite = callsite(
        quote! { concat!("event ", file!(), ":", line!()) },
        target.map_or(quote! { module_path!() }, |x| quote! { #x }),
        quote! { #level },
        fields.iter().map(|x| &x.name[..]),
        quote! { tracing::metadata::Kind::EVENT },
    );
    let fields = fields.into_iter().map(|Field { value, .. }| {
        quote! {
            (
                &field_iter.next().expect("FieldSet corrupted (this is a bug)"),
                Some(&#value as &dyn Value)
            )
        }
    });
    let dispatch = if let Some(parent) = parent {
        quote! {
            Event::child_of(#parent, meta, &fields.value_set(&[#(#fields),*]))
        }
    } else {
        quote! {
            Event::dispatch(meta, &fields.value_set(&[#(#fields),*]))
        }
    };
    let expanded = quote! {
        if #level <= tracing::level_filters::STATIC_MAX_LEVEL {
            #[allow(unused_imports)]
            use tracing::{callsite, dispatcher, Event, field::{Value, ValueSet}};
            use tracing::callsite::Callsite;
            let callsite = #callsite;
            if tracing::is_enabled!(callsite) {
                let meta = callsite.metadata();
                let fields = meta.fields();
                let mut field_iter = fields.iter();
                #dispatch;
            }
        }
    };
    TokenStream::from(expanded)
}

fn callsite<'a>(
    name: proc_macro2::TokenStream,
    target: proc_macro2::TokenStream,
    level: proc_macro2::TokenStream,
    fields: impl Iterator<Item = &'a str>,
    kind: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {{
        use std::sync::{
            atomic::{self, AtomicUsize, Ordering},
            Once,
        };
        use tracing::{subscriber::Interest, Metadata, metadata};
        struct MyCallsite;
        static META: Metadata<'static> = {
            metadata! {
                name: #name,
                target: #target,
                level: #level,
                fields: &[#(#fields),*],
                callsite: &MyCallsite,
                kind: #kind,
            }
        };
        // FIXME: Rust 1.34 deprecated ATOMIC_USIZE_INIT. When Tokio's minimum
        // supported version is 1.34, replace this with the const fn `::new`.
        #[allow(deprecated)]
        static INTEREST: AtomicUsize = atomic::ATOMIC_USIZE_INIT;
        static REGISTRATION: Once = Once::new();
        impl MyCallsite {
            #[inline]
            fn interest(&self) -> Interest {
                match INTEREST.load(Ordering::Relaxed) {
                    0 => Interest::never(),
                    2 => Interest::always(),
                    _ => Interest::sometimes(),
                }
            }
        }
        impl callsite::Callsite for MyCallsite {
            fn set_interest(&self, interest: Interest) {
                let interest = match () {
                    _ if interest.is_never() => 0,
                    _ if interest.is_always() => 2,
                    _ => 1,
                };
                INTEREST.store(interest, Ordering::SeqCst);
            }

            fn metadata(&self) -> &Metadata {
                &META
            }
        }
        REGISTRATION.call_once(|| {
            callsite::register(&MyCallsite);
        });
        &MyCallsite
    }}
}

struct TraceSpan {
    options: Options,
    name: Expr,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for TraceSpan {
    fn parse(input: ParseStream) -> Result<Self> {
        let options = Options::parse(input)?;
        let name = input.parse::<Expr>()?;
        let fields = if input.is_empty() {
            Punctuated::new()
        } else {
            input.parse::<Token![,]>()?;
            Punctuated::parse_terminated(&input)?
        };
        Ok(Self {
            options,
            name,
            fields,
        })
    }
}

struct Event {
    options: Options,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Event {
    fn parse(input: ParseStream) -> Result<Self> {
        let options = Options::parse(input)?;
        let mut fields = if input.peek(token::Brace) {
            let body;
            braced!(body in input);
            let fields = Punctuated::parse_terminated(&body)?;
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
            fields
        } else {
            let mut fields = Punctuated::new();
            while !input.peek(LitStr) && !input.is_empty() {
                fields.push_value(Field::parse(input)?);
                if !input.is_empty() {
                    fields.push_punct(input.parse::<Token![,]>()?);
                }
            }
            fields
        };
        if !input.is_empty() {
            let fmt = input.parse::<LitStr>()?;
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
            let args = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
            fields.push(Field {
                name: "message".into(),
                value: ExprVerbatim {
                    tts: quote! { format_args!(#fmt, #(#args),*) },
                }
                .into(),
            });
        };
        if fields.len() > 32 {
            return Err(syn::Error::new(
                Span::call_site(),
                "too many fields (should be <= 32)",
            ));
        }
        Ok(Self { options, fields })
    }
}

struct Options {
    target: Option<Expr>,
    parent: Option<Expr>,
    level: Expr,
}

impl Parse for Options {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut target = None;
        let mut parent = None;
        while input.peek(Ident) && input.peek2(Token![:]) && !input.peek3(Token![:]) {
            let ident = input.parse::<Ident>()?;
            input.parse::<Token![:]>()?;
            match &ident.to_string()[..] {
                "target" => {
                    target = Some(input.parse::<Expr>()?);
                }
                "parent" => {
                    parent = Some(input.parse::<Expr>()?);
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "unknown option"));
                }
            }
            input.parse::<Token![,]>()?;
        }
        let level = input.parse::<Expr>()?;
        input.parse::<Token![,]>()?;

        Ok(Self {
            target,
            parent,
            level,
        })
    }
}

struct Field {
    name: String,
    value: Expr,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut debug = None;
        let mut display = None;
        if input.peek(Token![?]) {
            debug = Some(input.parse::<Token![?]>()?);
        } else if input.peek(Token![%]) {
            display = Some(input.parse::<Token![%]>()?);
        }
        let name = Punctuated::<Ident, Token![.]>::parse_separated_nonempty(input)?;
        let value = if input.peek(Token![=]) {
            if let Some(span) = debug
                .as_ref()
                .map(|x| x.span)
                .or_else(|| display.as_ref().map(|x| x.span))
            {
                return Err(syn::Error::new(
                    span,
                    "debug/display annotations must be on values, not names",
                ));
            }
            input.parse::<Token![=]>()?;
            if input.peek(Token![?]) {
                debug = Some(input.parse::<Token![?]>()?);
            } else if input.peek(Token![%]) {
                display = Some(input.parse::<Token![%]>()?);
            }
            let expr = input.parse::<Expr>()?;
            quote! { #expr }
        } else {
            quote! { #name }
        };
        let value = if debug.is_some() {
            quote! { tracing::field::debug(&#value) }
        } else if display.is_some() {
            quote! { tracing::field::display(&#value) }
        } else {
            value
        };
        let mut name_str = name.first().unwrap().value().to_string();
        for part in name.iter().skip(1) {
            name_str += ".";
            name_str += &part.to_string();
        }
        Ok(Self {
            name: name_str,
            value: ExprVerbatim { tts: value }.into(),
        })
    }
}
