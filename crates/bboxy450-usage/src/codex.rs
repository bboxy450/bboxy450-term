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
    fn decode(&self, body: &[u8]) -> Result<Vec<Window>, Failure> {
        let root = json(body)?;
        let now = now();
        let mut windows = Vec::new();
        let mut add = |limit: &Value| {
            windows.extend(["primary_window", "secondary_window"].iter().filter_map(|k| {
                let w = &limit[k];
                let used = number(&w["used_percent"])?;
                let seconds = number(&w["limit_window_seconds"]).filter(|s| *s > 0.);
                let resets = number(&w["reset_at"]).or_else(|| {
                    number(&w["reset_after_seconds"])
                        .filter(|s| *s > 0.)
                        .map(|s| now + s)
                });
                Some(Window::timed(&window_label(seconds, k), used, resets, seconds))
            }));
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
        worst_first(windows)
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
        let windows = ["primary", "secondary"].into_iter().filter_map(|k| {
            let w = &p["rate_limits"][k];
            let seconds = number(&w["window_minutes"]).map(|m| m * 60.);
            number(&w["used_percent"])
                .map(|used| Window::timed(&window_label(seconds, k), used, number(&w["resets_at"]), seconds))
        });
        if let Ok(windows) = worst_first(windows) {
            let mut reading = Reading::from_windows(&windows, now());
            // Stale, so no pace (Tachyon).
            reading.stale("Historical rollout; account unverified");
            return Some(reading);
        }
    }
    None
}
/// Tachyon `windowLabel`: name a window by its length, else by its slot.
fn window_label(seconds: Option<f64>, slot: &str) -> String {
    match seconds.map(|s| s.round() as i64) {
        Some(18_000) => "5-hour".into(),
        Some(604_800) => "Weekly".into(),
        Some(86_400) => "Daily".into(),
        Some(s) if s % 86_400 == 0 => format!("{}-day", s / 86_400),
        Some(s) if s % 3600 == 0 => format!("{}-hour", s / 3600),
        _ if slot.starts_with("primary") => "Primary".into(),
        _ => "Secondary".into(),
    }
}
