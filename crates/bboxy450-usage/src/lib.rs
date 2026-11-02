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
    fn decode(&self, body: &[u8]) -> Result<u8, Failure>;
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
    pub left: Option<u8>,
    pub stale: bool,
    pub reason: Option<String>,
}
impl Reading {
    pub fn current(left: u8) -> Self {
        Self {
            left: Some(left.min(100)),
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
        self.stale = self.left.is_some();
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
pub(crate) fn percent_left(used: impl IntoIterator<Item = f64>) -> Result<u8, Failure> {
    let used = used
        .into_iter()
        .filter(|n| n.is_finite())
        .reduce(f64::max)
        .ok_or(Failure::Decode)?;
    Ok((100. - used.clamp(0., 100.)).round() as u8)
}
