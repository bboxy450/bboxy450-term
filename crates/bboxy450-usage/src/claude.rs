// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// The project license applies to Rust integration additions; translated provider portions retain MIT.
// Translated from Tachyon (MIT), Sources/Tachyon/Providers/ClaudeProvider.swift:
// https://github.com/Gonzih/tachyon — Copyright (c) 2026 Maksim Soltan / Wild Honey on the Porch, LLC.
// Full upstream notice: ../LICENSE-TACHYON. Changes: read-only Rust credential chain and bounded fetch.
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Credit policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::*;

pub struct Claude;
fn parse(bytes: &[u8]) -> Result<Credential, Failure> {
    let root = json(bytes).map_err(|_| Failure::Credentials)?;
    let oauth = &root["claudeAiOauth"];
    if number(&oauth["expiresAt"]).is_some_and(|expiry| expiry <= io::epoch_millis()) {
        return Err(Failure::Auth);
    }
    Credential::new(
        oauth["accessToken"]
            .as_str()
            .ok_or(Failure::Credentials)?
            .into(),
        None,
    )
}
impl Provider for Claude {
    fn id(&self) -> &'static str {
        "claude"
    }
    fn login_hint(&self) -> &'static str {
        "Run `claude` and sign in"
    }
    fn credential(&self, r: &dyn CredentialReader) -> Result<Presence, Failure> {
        if let Some(token) = r
            .env("CLAUDE_CODE_OAUTH_TOKEN")
            .filter(|s| !s.trim().is_empty())
        {
            return Credential::new(token.trim().into(), None).map(Presence::Ready);
        }
        let mut keychain_error = None;
        match r.keychain("Claude Code-credentials") {
            Ok(Some(bytes)) => {
                if let Ok(c) = parse(&bytes) {
                    return Ok(Presence::Ready(c));
                }
            }
            Err(e) => keychain_error = Some(e),
            _ => {}
        }
        let dir = r.home().join(".claude");
        match r.file(&dir.join(".credentials.json"))? {
            Some(bytes) => parse(&bytes).map(Presence::Ready),
            None if r.exists(&dir) => keychain_error.map_or(Ok(Presence::SignedOut), Err),
            None => Ok(Presence::Missing),
        }
    }
    fn request(&self, c: &Credential) -> Request {
        let mut request = Request::bearer("https://api.anthropic.com/api/oauth/usage", c);
        request.headers.extend([
            ("anthropic-beta", "oauth-2025-04-20".into()),
            ("User-Agent", "claude-cli/2.0.0 (external, cli)".into()),
        ]);
        request
    }
    fn decode(&self, body: &[u8]) -> Result<Vec<Window>, Failure> {
        const SESSION: Option<f64> = Some(5. * 3600.);
        const WEEK: Option<f64> = Some(7. * 86_400.);
        let r = json(body)?;
        let mut windows = Vec::new();
        let window = |label: &str, v: &Value, key: &str, seconds| {
            number(&v[key])
                .map(|used| Window::timed(label, used, iso_seconds(&v["resets_at"]), seconds))
        };
        let session = window("Current session", &r["five_hour"], "utilization", SESSION);
        let has_session = session.is_some();
        windows.extend(session);
        let mut weekly = false;
        if let Some(limits) = r["limits"].as_array() {
            for limit in limits {
                let kind = limit["kind"].as_str().unwrap_or_default();
                if kind == "session" && !has_session {
                    windows.extend(window("Current session", limit, "percent", SESSION));
                } else if kind.starts_with("weekly") && limit["is_active"].as_bool() == Some(true) {
                    // weekly_all → "Weekly"; weekly_opus → "Weekly (Opus)".
                    let label = match kind.strip_prefix("weekly_").filter(|s| *s != "all") {
                        Some(model) => {
                            let mut model = model.replace('_', " ");
                            if let Some(first) = model.get_mut(..1) {
                                first.make_ascii_uppercase();
                            }
                            format!("Weekly ({model})")
                        }
                        None => "Weekly".into(),
                    };
                    windows.extend(window(&label, limit, "percent", WEEK));
                    weekly |= kind == "weekly_all";
                }
            }
        }
        if !weekly {
            windows.extend(window("Weekly", &r["seven_day"], "utilization", WEEK));
        }
        worst_first(windows)
    }
}
