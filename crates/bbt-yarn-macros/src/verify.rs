// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Quint's explicit finite verification scope: https://quint.sh/docs/checking-properties
// and TLA-Verus statement preservation: https://arxiv.org/abs/2610.14923
// Independent implementation; no source copied.
//! Required claims and exhaustive verification are separate from protocol semantics.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Error, Ident, LitInt, LitStr, Path, Result, Token, braced, bracketed, parenthesized,
};

pub(super) fn metadata<'a>(
    name: &Ident,
    source: &str,
    domains: impl Iterator<Item = (&'a Ident, &'a [Ident])>,
    claims: impl Iterator<Item = (&'a LitStr, &'a Ident)>,
    cases: impl Iterator<Item = String>,
) -> TokenStream {
    let path = super::kittens_path();
    let span = |at| {
        quote::quote_spanned!(at=> #path::verify::DeclarationSpan {
            file: ::core::file!(), line: ::core::line!(), column: ::core::column!(),
        })
    };
    let declaration_span = span(name.span());
    let name = name.to_string();
    let version = env!("CARGO_PKG_VERSION");
    let domains = domains.map(|(name, variants)| {
        let name = name.to_string();
        let variants = variants.iter().map(Ident::to_string);
        quote!(#path::verify::DomainMetadata { name: #name, variants: &[#(#variants),*] })
    });
    let claims = claims.map(|(id, predicate)| {
        let location = span(predicate.span());
        let predicate = predicate.to_string();
        quote!(#path::verify::ClaimMetadata { id: #id, predicate: #predicate, span: #location })
    });
    quote! {
        /// Stable claims, domains, case labels and declaration provenance.
        pub fn verification_metadata() -> #path::verify::ProtocolMetadata {
            #path::verify::ProtocolMetadata {
                name: #name, source: #source, span: #declaration_span,
                domains: &[#(#domains),*], claims: &[#(#claims),*],
                cases: &[#(#cases),*], macro_version: #version,
            }
        }
    }
}

fn invalid(at: impl syn::spanned::Spanned, message: &str) -> Error {
    super::ktr(
        at.span(),
        "VER001",
        message,
        "use the documented requirements!/verify! grammar",
    )
}

struct Requirements {
    visibility: syn::Visibility,
    name: Ident,
    claims: Vec<(LitStr, Path)>,
}

impl Parse for Requirements {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let visibility = input.parse()?;
        let name: Ident = input.parse()?;
        let body;
        braced!(body in input);
        let mut claims = Vec::new();
        let mut ids = BTreeSet::new();
        while !body.is_empty() {
            let id: LitStr = body.parse()?;
            if id.value().is_empty() || !ids.insert(id.value()) {
                return Err(invalid(&id, "required ids must be nonempty and unique"));
            }
            body.parse::<Token![=>]>()?;
            let claim: Path = body.parse()?;
            if claim.segments.len() < 2 || claim.segments.iter().any(|s| !s.arguments.is_empty()) {
                return Err(invalid(
                    &claim,
                    "name a protocol claim as Protocol::predicate",
                ));
            }
            body.parse::<Token![;]>()?;
            claims.push((id, claim));
        }
        Ok(Self {
            visibility,
            name,
            claims,
        })
    }
}

pub(super) fn requirements(input: TokenStream) -> TokenStream {
    let declaration = match syn::parse2::<Requirements>(input) {
        Ok(d) => d,
        Err(e) => return e.into_compile_error(),
    };
    let path = super::kittens_path();
    let Requirements {
        visibility,
        name,
        claims,
    } = declaration;
    let entries = claims.iter().map(|(id, claim)| {
        let mut segments = claim.segments.iter().rev();
        let predicate = segments.next().unwrap().ident.to_string();
        let protocol = segments.next().unwrap().ident.to_string();
        quote!(#path::verify::Requirement { id: #id, protocol: #protocol, predicate: #predicate })
    });
    let references = claims.iter().map(|(_, claim)| quote!(let _ = #claim;));
    quote! {
        #[doc = "Generated required claims. Origin: https://github.com/bboxy450/bboxy450-term; retain credit. Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md"]
        #visibility struct #name;
        impl #name {
            /// Stable ids and the protocol predicates required by this contract.
            pub const REQUIREMENTS: &'static [#path::verify::Requirement] = &[#(#entries),*];
        }
        const _: () = { #(#references)* };
    }
}

struct Verification {
    attributes: Vec<Attribute>,
    visibility: syn::Visibility,
    name: Ident,
    contract: Path,
    protocol: Path,
    reachable: Vec<Path>,
    states: LitInt,
}

impl Parse for Verification {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;
        let visibility = input.parse()?;
        input.parse::<Token![fn]>()?;
        let name: Ident = input.parse()?;
        if !attributes.iter().any(|a| a.path().is_ident("test")) {
            return Err(invalid(&name, "verification declarations require #[test]"));
        }
        let parameters;
        parenthesized!(parameters in input);
        if !parameters.is_empty() {
            return Err(parameters.error("VER001 verification tests take no arguments"));
        }
        let body;
        braced!(body in input);
        let mut contract = None;
        let mut protocol = None;
        let mut reachable = None;
        let mut states = None;
        let mut sections = BTreeSet::new();
        while !body.is_empty() {
            let key: Ident = body.parse()?;
            if !sections.insert(key.to_string()) {
                return Err(invalid(&key, "duplicate verification section"));
            }
            if key == "limits" {
                let limits;
                braced!(limits in body);
                let state_key: Ident = limits.parse()?;
                limits.parse::<Token![:]>()?;
                let limit: LitInt = limits.parse()?;
                limits.parse::<Token![;]>()?;
                let exhaustion_key: Ident = limits.parse()?;
                limits.parse::<Token![:]>()?;
                let disposition: Ident = limits.parse()?;
                limits.parse::<Token![;]>()?;
                if state_key != "states"
                    || exhaustion_key != "on_exhaustion"
                    || disposition != "inconclusive"
                    || !limits.is_empty()
                {
                    return Err(invalid(
                        &key,
                        "limits require states: N; on_exhaustion: inconclusive;",
                    ));
                }
                if limit.base10_parse::<usize>()? == 0 {
                    return Err(invalid(&limit, "the state limit must be positive"));
                }
                states = Some(limit);
                continue;
            }
            body.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "contract" => contract = Some(body.parse()?),
                "protocol" => protocol = Some(body.parse()?),
                "exploration" => {
                    let method: Ident = body.parse()?;
                    if method != "exhaustive" {
                        return Err(invalid(&method, "only exhaustive exploration is supported"));
                    }
                }
                "require_reachable" => {
                    let cases;
                    bracketed!(cases in body);
                    let paths = cases.parse_terminated(Path::parse, Token![,])?;
                    let mut seen = BTreeSet::new();
                    for case in &paths {
                        if case.segments.len() != 2
                            || case.leading_colon.is_some()
                            || case.segments.iter().any(|s| !s.arguments.is_empty())
                            || !seen.insert(quote!(#case).to_string())
                        {
                            return Err(invalid(
                                case,
                                "reachability requires unique Event::case labels",
                            ));
                        }
                    }
                    reachable = Some(paths.into_iter().collect());
                }
                _ => return Err(invalid(&key, "unknown verification section")),
            }
            body.parse::<Token![;]>()?;
        }
        let missing = || {
            invalid(
                &name,
                "contract, protocol, exhaustive exploration, require_reachable and limits are required",
            )
        };
        if !sections.contains("exploration") {
            return Err(missing());
        }
        Ok(Self {
            attributes,
            visibility,
            name: name.clone(),
            contract: contract.ok_or_else(missing)?,
            protocol: protocol.ok_or_else(missing)?,
            reachable: reachable.ok_or_else(missing)?,
            states: states.ok_or_else(missing)?,
        })
    }
}

pub(super) fn verify(input: TokenStream) -> TokenStream {
    let source = input.to_string();
    let d = match syn::parse2::<Verification>(input) {
        Ok(d) => d,
        Err(e) => return e.into_compile_error(),
    };
    let path = super::kittens_path();
    let Verification {
        attributes,
        visibility,
        name,
        contract,
        protocol,
        reachable,
        states,
    } = d;
    let cases = reachable.iter().map(|case| {
        let event = case.segments[0].ident.to_string();
        let label = case.segments[1].ident.to_string();
        format!("{event}::{label}")
    });
    let contract_name = contract.segments.last().unwrap().ident.to_string();
    let helper = Ident::new(&format!("__yarn_verify_configuration_{name}"), name.span());
    let waivers = attributes
        .iter()
        .filter(|a| !a.path().is_ident("test"))
        .map(|a| quote!(#a).to_string());
    quote! {
        #(#attributes)*
        #visibility fn #name() {
            let result = #name::check();
            let destination = result.write_to_target(
                ::core::env!("CARGO_MANIFEST_DIR"),
                ::core::concat!(::core::module_path!(), "::", ::core::stringify!(#name)),
            ).expect("verification evidence must be written even for a failing verdict");
            ::core::assert!(result.is_established(), "Yarn verification {}: {} (evidence: {})", result.outcome.as_str(), result.message, destination.display());
        }
        #[doc(hidden)]
        fn #helper() -> #path::verify::VerificationResult {
            #path::verify::check(
                #contract_name, #contract::REQUIREMENTS,
                &#protocol::verification_metadata(), &#protocol::model(),
                &[#(#cases),*], #states, #source,
            ).with_waivers(&[#(#waivers),*])
        }
        #[doc = "Generated verification configuration. Origin: https://github.com/bboxy450/bboxy450-term; retain credit. Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md"]
        #visibility mod #name {
            /// Run the exact finite check without asserting or writing evidence.
            pub fn check() -> #path::verify::VerificationResult {
                super::#helper()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_claims_and_unsupported_verification_fail_closed() {
        assert!(
            requirements(quote!(C { "R" => P::ok; "R" => P::ok; }))
                .to_string()
                .contains("VER001")
        );
        assert!(syn::parse2::<Requirements>(quote!(C { "R" => P::ok; "R" => P::ok; })).is_err());
        for bad in [
            quote!(#[test] fn t() { contract: C; protocol: P; exploration: random; require_reachable: []; limits { states: 2; on_exhaustion: inconclusive; } }),
            quote!(#[test] fn t() { contract: C; protocol: P; exploration: exhaustive; require_reachable: []; limits { states: 2; on_exhaustion: pass; } }),
            quote!(#[test] fn t() { contract: C; protocol: P; exploration: exhaustive; require_reachable: [E::c, E::c]; limits { states: 2; on_exhaustion: inconclusive; } }),
            quote!(fn t() { contract: C; protocol: P; exploration: exhaustive; require_reachable: []; limits { states: 2; on_exhaustion: inconclusive; } }),
        ] {
            assert!(syn::parse2::<Verification>(bad.clone()).is_err());
            assert!(verify(bad).to_string().contains("VER001"));
        }
    }
}
