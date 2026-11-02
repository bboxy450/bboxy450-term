// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// The project license applies to Rust integration additions; translated provider portions retain MIT.
// Translated from Tachyon (MIT), Sources/Tachyon/Providers/GrokProvider.swift:
// https://github.com/Gonzih/tachyon — Copyright (c) 2026 Maksim Soltan / Wild Honey on the Porch, LLC.
// Full upstream notice: ../LICENSE-TACHYON. Changes: read-only Rust IO, no anonymous log fallback.
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Credit policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::*;

pub struct Grok;
impl Provider for Grok {
    fn id(&self) -> &'static str {
        "grok"
    }
    fn login_hint(&self) -> &'static str {
        "Run `grok` to sign in"
    }
    fn credential(&self, r: &dyn CredentialReader) -> Result<Presence, Failure> {
        let home = r
            .env("GROK_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| r.home().join(".grok"));
        let path = r
            .env("GROK_AUTH_JSON")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| home.join("auth.json"));
        let Some(bytes) = r.file(&path)? else {
            return Ok(if r.exists(&home) {
                Presence::SignedOut
            } else {
                Presence::Missing
            });
        };
        let root = json(&bytes).map_err(|_| Failure::Credentials)?;
        let account = root
            .as_object()
            .ok_or(Failure::Credentials)?
            .values()
            .find(|entry| {
                entry["oidc_issuer"] == "https://auth.x.ai"
                    && entry["key"].as_str().is_some_and(|s| !s.is_empty())
            });
        let Some(a) = account else {
            return Ok(Presence::SignedOut);
        };
        Credential::new(
            a["key"].as_str().ok_or(Failure::Credentials)?.into(),
            a["user_id"].as_str().map(Into::into),
        )
        .map(Presence::Ready)
    }
    fn request(&self, c: &Credential) -> Request {
        let mut request = Request::bearer(
            "https://cli-chat-proxy.grok.com/v1/billing?format=credits",
            c,
        );
        request.headers.extend([
            ("X-XAI-Token-Auth", "xai-grok-cli".into()),
            ("User-Agent", "GrokCLI/1.0.0".into()),
        ]);
        if let Some(account) = &c.account {
            request.headers.push(("x-userid", account.clone()));
        }
        request
    }
    fn decode(&self, body: &[u8]) -> Result<Vec<Window>, Failure> {
        let root = json(body)?;
        let c = &root["config"];
        let known = [
            "creditUsagePercent",
            "currentPeriod",
            "productUsage",
            "subscriptionTier",
            "onDemandCap",
            "onDemandUsed",
        ];
        if !known.iter().any(|k| c.get(k).is_some()) {
            return Err(Failure::Decode);
        }
        // Proto3 JSON omits zero scalar fields; an otherwise recognized config means 0% used.
        let used = match c.get("creditUsagePercent") {
            Some(v) => number(v).ok_or(Failure::Decode)?,
            None => 0.,
        };
        worst_first([Window::new("Credits", used)])
    }
}
