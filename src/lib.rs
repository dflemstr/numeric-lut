//! # `numeric-lut`
//!
//! A library for generating numeric lookup functions.  Currently, it requires the use of the
//! `proc_macro_hygiene` nightly feature.
//!
//! ## Examples
//!
//! ```
//! #![feature(proc_macro_hygiene)]
//! let lut = numeric_lut::lut!(|x @ 0..8, y @ 0..16| -> u32 { x as u32 + y as u32 });
//! let x = lut(3, 10);
//! assert_eq!(13, x);
//! ```
#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

extern crate proc_macro;

struct Lut {
    #[allow(unused)]
    or1_token: syn::Token![|],
    inputs: syn::punctuated::Punctuated<Param, syn::Token![,]>,
    #[allow(unused)]
    or2_token: syn::Token![|],
    #[allow(unused)]
    arrow_token: syn::Token![->],
    return_type: syn::Type,
    body: syn::Expr,
}

struct Param {
    ident: syn::Ident,
    lo: usize,
    exclusive_end: bool,
    hi: usize,
}

/// Generates a numeric lookup function.
///
/// The macro is function-like and accepts an expression that looks like a closure.  Only parameters
/// that use range patterns (like `x @ 0..1`) are accepted.  All parameters are implicitly of type
/// `usize` since they will be used as indices for lookup tables.
#[proc_macro]
pub fn lut(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as Lut);

    let table_data = input.inputs.iter().rev().fold(input.body, |body, param| {
        if param.exclusive_end {
            generate_array(&param.ident, param.lo..param.hi, body)
        } else {
            generate_array(&param.ident, param.lo..=param.hi, body)
        }
    });

    let lut_access = input
        .inputs
        .iter()
        .fold(quote::quote!(__LUT), |expr, param| {
            let ident = &param.ident;
            quote::quote!(#expr[#ident])
        });

    let lut_params = input.inputs.iter().map(|param| {
        let ident = &param.ident;
        quote::quote!(#ident: usize)
    });

    let lut_type = input
        .inputs
        .iter()
        .rev()
        .fold(input.return_type, |ty, param| {
            let count = if param.exclusive_end {
                param.hi - param.lo
            } else {
                param.hi - param.lo + 1
            };
            quote::quote!([#ty; #count]).into()
        });

    let output = quote::quote!({
        static __LUT: #lut_type = #table_data;
        |#(#lut_params),*| #lut_access
    });

    output.into()
}

fn generate_array(
    ident: &syn::Ident,
    range: impl Iterator<Item = usize>,
    body: syn::Expr,
) -> syn::Expr {
    let items = range.map(|n| {
        quote::quote!({
            #[allow(non_upper_case_globals)]
            const #ident: usize = #n;
            #body
        })
    });
    quote::quote!([#(#items),*]).into()
}

impl Param {
    fn from_pat(pat: syn::Pat) -> syn::Result<Self> {
        use syn::spanned::Spanned;
        match pat {
            syn::Pat::Ident(pat_ident) => Self::from_pat_ident(pat_ident),
            other => Err(syn::Error::new(
                other.span(),
                "this parameter must have a range pattern (e.g. `x @ 1..2` or `y @ 3..=4`)",
            )),
        }
    }

    fn from_pat_ident(pat_ident: syn::PatIdent) -> syn::Result<Self> {
        use syn::spanned::Spanned;
        match pat_ident {
            syn::PatIdent {
                ident,
                subpat,
                ..
            } => match subpat {
                Some((_, pat)) => {
                    let pat_span = pat.span();
                    match *pat {
                        syn::Pat::Range(syn::PatRange {
                            lo,
                            limits,
                            hi,
                            ..
                        }) => match *lo {
                            syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Int(lo),
                                ..
                            }) => {
                                let lo = lo.base10_parse()?;
                                match *hi {
                                    syn::Expr::Lit(syn::ExprLit {
                                        lit: syn::Lit::Int(hi),
                                        ..
                                    }) => {
                                        let hi = hi.base10_parse()?;
                                        if hi < lo {
                                            return Err(syn::Error::new(pat_span, format!("range lower bound {} must be less than upper bound {}", lo, hi)));
                                        }
                                        let exclusive_end = match limits {
                                            syn::RangeLimits::Closed(_) => false,
                                            syn::RangeLimits::HalfOpen(_) => true,
                                        };
                                        Ok(Param {
                                            ident,
                                            lo,
                                            exclusive_end,
                                            hi,
                                        })
                                    }
                                    expr => Err(syn::Error::new(
                                        expr.span(),
                                        "must be an integer literal",
                                    )),
                                }
                            }
                            expr => Err(syn::Error::new(expr.span(), "must be an integer literal")),
                        },
                        pat => Err(syn::Error::new(
                            pat.span(),
                            "only range patterns allowed (e.g. `1..2` or `3..=4`)",
                        )),
                    }
                }
                None => Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "this parameter must have a specified range pattern (e.g. `{} @ 1..2`)",
                        ident
                    ),
                )),
            },
        }
    }
}

impl syn::parse::Parse for Lut {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let or1_token: syn::Token![|] = input.parse()?;

        let mut inputs = syn::punctuated::Punctuated::new();
        loop {
            if input.peek(syn::Token![|]) {
                break;
            }
            let value = Param::from_pat(input.parse::<syn::Pat>()?)?;
            inputs.push_value(value);
            if input.peek(syn::Token![|]) {
                break;
            }
            let punct: syn::Token![,] = input.parse()?;
            inputs.push_punct(punct);
        }

        let or2_token: syn::Token![|] = input.parse()?;

        let arrow_token: syn::Token![->] = input.parse()?;
        let return_type: syn::Type = input.parse()?;
        let body: syn::Block = input.parse()?;
        let body = syn::Expr::Block(syn::ExprBlock {
            attrs: Vec::new(),
            label: None,
            block: body,
        });

        Ok(Lut {
            or1_token,
            inputs,
            or2_token,
            arrow_token,
            return_type,
            body,
        })
    }
}
