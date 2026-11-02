// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Provider abstraction/value model translated from Tachyon (MIT): https://github.com/Gonzih/tachyon,
// Sources/Tachyon/Providers/Provider.swift and Model/UsageModel.swift; notice in ../LICENSE-TACHYON.
//! Read-only usage providers. No credentials, account IDs or HTTP bodies enter snapshots.
pub mod claude;
pub mod codex;
pub mod grok;
pub mod io;
pub mod machine;
pub mod poller;

use serde_json::Value;
use std::{future::Future, pin::Pin, sync::Arc};

pub const AI_INTERVAL: std::time::Duration = std::time::Duration::from_secs(300);
pub const MACHINE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
pub const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// Provider protocol and worst-active-window selection translated from Tachyon (MIT):
/// https://github.com/Gonzih/tachyon, Sources/Tachyon/Providers/Provider.swift and Model/UsageModel.swift.
/// Upstream notice: ../LICENSE-TACHYON. Modified: bounded Rust workers; no refresh or credential writes.
pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn login_hint(&self) -> &'static str;
    fn credential(&self, reader: &dyn CredentialReader) -> Result<Presence, Failure>;
    fn request(&self, credential: &Credential) -> Request;
    fn decode(&self, body: &[u8]) -> Result<Window, Failure>;
    fn fallback(&self, _reader: &dyn CredentialReader) -> Option<Reading> {
        None
    }
}

/// The interface intentionally has no write, refresh, subprocess or logging operation.
pub trait CredentialReader: Send + Sync {
    fn env(&self, key: &str) -> Option<String>;
    fn home(&self) -> std::path::PathBuf;
    fn exists(&self, path: &std::path::Path) -> bool;
    fn file(&self, path: &std::path::Path) -> Result<Option<Vec<u8>>, Failure>;
    fn keychain(&self, service: &str) -> Result<Option<Vec<u8>>, Failure>;
    fn rollout_tail(&self, _root: &std::path::Path) -> Option<Vec<u8>> {
        None
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Credential {
    token: String,
    account: Option<String>,
}
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credential([redacted])")
    }
}
impl Credential {
    pub fn new(token: String, account: Option<String>) -> Result<Self, Failure> {
        if token.trim().is_empty()
            || token.contains(['\r', '\n'])
            || account.as_ref().is_some_and(|s| s.contains(['\r', '\n']))
        {
            return Err(Failure::Credentials);
        }
        Ok(Self { token, account })
    }
    pub(crate) fn identity(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        // Internal only: never serialized or sent to the UI.
        let mut h = Sha256::new();
        h.update(self.token.as_bytes());
        h.update([0]);
        h.update(self.account.as_deref().unwrap_or_default().as_bytes());
        h.finalize().into()
    }
}

#[derive(Debug)]
pub enum Presence {
    Missing,
    SignedOut,
    Ready(Credential),
}

/// Fixed, non-sensitive errors. Never retain reqwest/JSON/OS error descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Credentials,
    KeychainLocked,
    Auth,
    Network,
    Timeout,
    Decode,
    Http(u16),
    AccountChanged,
    Unsupported,
}
impl Failure {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Credentials => "Cannot read credentials",
            Self::KeychainLocked => "Keychain unavailable (interaction disabled)",
            Self::Auth => "Sign-in expired",
            Self::Network => "Network unavailable",
            Self::Timeout => "Read timed out",
            Self::Decode => "Usage response unavailable",
            Self::Http(429) => "Provider rate limited",
            Self::Http(_) => "Provider request failed",
            Self::AccountChanged => "Account changed; refresh required",
            Self::Unsupported => "Machine stats unavailable on this platform",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reading {
    /// Percent used of the worst active window.
    pub used: Option<u8>,
    /// Tachyon pace: percent this window reaches by its reset at the current rate.
    pub projected: Option<u16>,
    pub stale: bool,
    pub reason: Option<String>,
}
impl Reading {
    pub fn current(used: u8) -> Self {
        Self {
            used: Some(used.min(100)),
            ..Self::default()
        }
    }
    pub fn from_window(window: Window, now: f64) -> Self {
        Self {
            used: Some(window.used.round() as u8),
            projected: window
                .projected(now)
                .map(|p| p.round().clamp(0., 999.) as u16),
            ..Self::default()
        }
    }
    pub fn error(reason: impl Into<String>) -> Self {
        Self {
            reason: Some(reason.into()),
            ..Self::default()
        }
    }
    pub fn stale(&mut self, reason: &str) {
        self.stale = self.used.is_some();
        // Tachyon: a stale reading never drives pace.
        self.projected = None;
        self.reason = Some(reason.into());
    }
}

pub struct Request {
    pub url: &'static str,
    pub headers: Vec<(&'static str, String)>,
}
impl Request {
    fn bearer(url: &'static str, c: &Credential) -> Self {
        Self {
            url,
            headers: vec![("Authorization", format!("Bearer {}", c.token))],
        }
    }
}
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}
pub type FetchFuture<'a> = Pin<Box<dyn Future<Output = Result<Response, Failure>> + Send + 'a>>;
pub trait Fetch: Send + Sync {
    fn get(&self, request: Request) -> FetchFuture<'_>;
}

pub fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(claude::Claude),
        Arc::new(codex::Codex),
        Arc::new(grok::Grok),
    ]
}

pub(crate) fn json(bytes: &[u8]) -> Result<Value, Failure> {
    serde_json::from_slice(bytes).map_err(|_| Failure::Decode)
}
pub(crate) fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|n| n.is_finite())
}
/// One limit window (Tachyon `UsageWindow`): percent used, reset instant and length, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub used: f64,
    pub resets_at: Option<f64>,
    pub seconds: Option<f64>,
}
impl Window {
    pub fn new(used: f64) -> Self {
        Self {
            used,
            resets_at: None,
            seconds: None,
        }
    }
    pub fn timed(used: f64, resets_at: Option<f64>, seconds: Option<f64>) -> Self {
        Self {
            used,
            resets_at,
            seconds,
        }
    }
    /// Tachyon pace: used ÷ elapsed fraction of the window, only once 10% has elapsed and
    /// the reset is in the future and within one window length.
    pub fn projected(&self, now: f64) -> Option<f64> {
        let (seconds, resets) = (self.seconds?, self.resets_at?);
        let remaining = resets - now;
        if !(remaining.is_finite() && remaining > 0. && remaining < seconds) {
            return None;
        }
        let elapsed = 1. - remaining / seconds;
        (elapsed >= 0.1).then(|| self.used / elapsed)
    }
}
/// Tachyon worst-active-window rule: the window closest to blocking the user.
pub(crate) fn worst(windows: impl IntoIterator<Item = Window>) -> Result<Window, Failure> {
    windows
        .into_iter()
        .filter(|w| w.used.is_finite())
        .map(|w| Window {
            used: w.used.clamp(0., 100.),
            ..w
        })
        .reduce(|a, b| if b.used > a.used { b } else { a })
        .ok_or(Failure::Decode)
}
pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0., |d| d.as_secs_f64())
}
/// RFC 3339 instant ("2026-11-06T16:00:00.123+00:00" or "...Z") to unix seconds.
pub(crate) fn iso_seconds(v: &Value) -> Option<f64> {
    let s = v.as_str()?;
    let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    let rest = s.get(19..)?;
    let frac_end = rest.find(|c: char| c != '.' && !c.is_ascii_digit()).unwrap_or(rest.len());
    let frac = rest[..frac_end].parse::<f64>().unwrap_or(0.);
    let offset = match &rest[frac_end..] {
        "" | "Z" | "z" => 0,
        tz => {
            let sign = if tz.starts_with('-') { -1 } else { 1 };
            let oh = tz.get(1..3)?.parse::<i64>().ok()?;
            let om = tz.get(tz.len() - 2..)?.parse::<i64>().ok()?;
            sign * (oh * 3600 + om * 60)
        }
    };
    // Days from civil (Howard Hinnant).
    let (y, m) = if mo <= 2 { (y - 1, mo + 9) } else { (y, mo - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some((days * 86_400 + h * 3600 + mi * 60 + se - offset) as f64 + frac)
}
