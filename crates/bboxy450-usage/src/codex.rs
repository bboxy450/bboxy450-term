// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// The project license applies to Rust integration additions; translated provider portions retain MIT.
// Translated from Tachyon (MIT), Sources/Tachyon/Providers/CodexProvider.swift:
// https://github.com/Gonzih/tachyon — Copyright (c) 2026 Maksim Soltan / Wild Honey on the Porch, LLC.
// Full upstream notice: ../LICENSE-TACHYON. Changes: no app-server launch/refresh; bounded Rust read-only IO.
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Credit policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::*;

pub struct Codex;
fn home(r: &dyn CredentialReader) -> std::path::PathBuf {
    r.env("CODEX_HOME")
        .filter(|s| !s.is_empty())
        .map(Into::into)
        .unwrap_or_else(|| r.home().join(".codex"))
}
impl Provider for Codex {
    fn id(&self) -> &'static str {
        "codex"
    }
    fn login_hint(&self) -> &'static str {
        "Run `codex login`"
    }
    fn credential(&self, r: &dyn CredentialReader) -> Result<Presence, Failure> {
        let home = home(r);
        let Some(bytes) = r.file(&home.join("auth.json"))? else {
            return Ok(if r.exists(&home) {
                Presence::SignedOut
            } else {
                Presence::Missing
            });
        };
        let root = json(&bytes).map_err(|_| Failure::Credentials)?;
        let tokens = &root["tokens"];
        let Some(token) = tokens["access_token"].as_str() else {
            return Ok(Presence::SignedOut);
        };
        Credential::new(token.into(), tokens["account_id"].as_str().map(Into::into))
            .map(Presence::Ready)
    }
    fn request(&self, c: &Credential) -> Request {
        let mut request = Request::bearer("https://chatgpt.com/backend-api/wham/usage", c);
        if let Some(account) = &c.account {
            request
                .headers
                .push(("ChatGPT-Account-Id", account.clone()));
        }
        request.headers.push(("User-Agent", "codex-cli".into()));
        request
    }
    fn decode(&self, body: &[u8]) -> Result<u8, Failure> {
        let root = json(body)?;
        let mut used = Vec::new();
        let mut add = |limit: &Value| {
            used.extend(
                ["primary_window", "secondary_window"]
                    .iter()
                    .filter_map(|k| number(&limit[k]["used_percent"])),
            );
        };
        add(&root["rate_limit"]);
        if let Some(extra) = root["additional_rate_limits"].as_array() {
            for entry in extra {
                let limit = &entry["rate_limit"];
                if ["primary_window", "secondary_window"]
                    .iter()
                    .any(|k| number(&limit[k]["used_percent"]).is_some_and(|used| used >= 1.))
                {
                    add(limit);
                }
            }
        }
        percent_left(used)
    }
    fn fallback(&self, r: &dyn CredentialReader) -> Option<Reading> {
        // Like Tachyon, anonymous rollouts cannot be attributed to a signed-in account.
        // Only the signed-out branch calls this; the visible reason labels historical data.
        let bytes = r.rollout_tail(&home(r))?;
        rollout(&bytes)
    }
}
pub fn rollout(bytes: &[u8]) -> Option<Reading> {
    let text = std::str::from_utf8(bytes).ok()?;
    for line in text.lines().rev() {
        let Ok(root) = json(line.as_bytes()) else {
            continue;
        };
        let p = &root["payload"];
        if p["type"] != "token_count" {
            continue;
        }
        let used = ["primary", "secondary"]
            .into_iter()
            .filter_map(|k| number(&p["rate_limits"][k]["used_percent"]));
        if let Ok(left) = percent_left(used) {
            return Some(Reading {
                left: Some(left),
                stale: true,
                reason: Some("Historical rollout; account unverified".into()),
            });
        }
    }
    None
}
