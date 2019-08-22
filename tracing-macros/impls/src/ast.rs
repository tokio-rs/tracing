use lazy_static::lazy_static;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use regex::Regex;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Ident, LitStr, Token};

use std::{fmt, iter};

#[derive(Debug)]
pub(crate) struct EventBody {
    pub(crate) attrs: Attrs,
    pub(crate) fields: KvFields,
}

#[derive(Debug)]
pub(crate) struct Attrs {
    target: Option<Target>,
    parent: Option<Parent>,
}

#[derive(Debug)]
pub(crate) struct Parent {
    parent: Expr,
}

#[derive(Debug)]
pub(crate) struct Target {
    target: LitStr,
}

#[derive(Debug)]
pub(crate) struct KvFields {
    fmt_str: Option<LitStr>,
    fields: Punctuated<KvField, Token![,]>,
}

#[derive(Debug)]
pub(crate) struct KvField {
    name: Punctuated<syn::Ident, Token![.]>,
    value: Option<Expr>,
}

// === impl Parse for EventBody ===

impl Parse for EventBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.parse()?,
            fields: input.parse()?,
        })
    }
}

// === impl Attrs ===
impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut target = None;
        let mut parent = None;
        loop {
            let lookahead = input.lookahead1();
            if target.is_none() && lookahead.peek(kw::target) {
                target = Some(input.parse()?);
                input.parse::<Token![,]>()?;
            } else if parent.is_none() && lookahead.peek(kw::parent) {
                parent = Some(input.parse()?);
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(Self { target, parent })
    }
}

// === impl KvFields ===

impl Parse for KvFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fmt_str = if input.fork().parse::<LitStr>().is_ok() {
            let s = input
                .parse::<LitStr>()
                .expect("parsing succeeded for fork, it should not fail now");
            let _ = input.parse::<Token![,]>();
            Some(s)
        } else {
            None
        };
        let fields = Punctuated::<KvField, Token![,]>::parse_terminated(input)?;
        // let fields = if let Some(ref fmt) = fmt_str {
        //     let n_formats = FORMAT_RE.find_iter(&fmt.value()).count();
        //     let mut formatted_fields = fields.iter().map(|field| &field.name);
        //     let mut fields = Punctuated::new();
        //     fields.push(syn::parse_quote!(
        //         message = &format_args!(#fmt, #(formatted_fields),+)
        //     ))

        // } else {
        //     fields
        // };

        Ok(Self { fmt_str, fields })
    }
}

impl KvFields {
    pub(crate) fn has_message(&self) -> bool {
        self.fmt_str.is_some()
    }

    pub(crate) fn gen_valueset(&self) -> impl ToTokens {
        lazy_static! {
            static ref FORMAT_RE: Regex =
                Regex::new(r"\{[^\{\}]*\}").expect("regex should comnpile");
        }
        let arg_names = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, _)| Ident::new(&format!("__tracing__arg{}", i), Span::call_site()))
            .collect::<Vec<_>>();
        let values = if let Some(ref fmt) = self.fmt_str {
            let n_formats = FORMAT_RE.find_iter(&fmt.value()).count();
            let formatted_fields = arg_names.iter().take(n_formats);
            quote!(format_args!(#fmt, #(#formatted_fields),*))
        } else {
            quote!()
        };
        let exprs = self.fields.iter().map(|field| field.value.clone().unwrap());
        let args = arg_names
            .iter()
            .zip(exprs)
            .map(|(arg_name, expr)| quote!(let #arg_name = &#expr;));
        quote!(
            #(
                #args
            )*
            #values
        )
    }

    pub(crate) fn gen_fieldset(&self) -> impl ToTokens {
        let message = if self.has_message() {
            Some(Ident::new("message", Span::call_site()))
        } else {
            None
        };
        let field_names = message
            .iter()
            .map(|m| m as &dyn ToTokens)
            .chain(self.fields.iter().map(|field| &field.name as &dyn ToTokens));
        quote!(
            &[ #(stringify!(#field_names)),* ]
        )
    }
}

// === impl KvField ===

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
// === impl Target ===

impl Parse for Target {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::target>()?;
        let _ = input.parse::<Token![:]>()?;
        input.parse().map(|target| Target { target })
    }
}

// impl fmt::Display for Target {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self.target.value())
//     }
// }

impl Parse for Parent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ = input.parse::<kw::parent>()?;
        let _ = input.parse::<Token![:]>()?;
        input.parse().map(|parent| Parent { parent })
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "target: {:?}", self.target.value())
    }
}

mod kw {
    syn::custom_keyword!(target);
    syn::custom_keyword!(parent);
}
