// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Decode fixtures copied from Tachyon (MIT), Tests/TachyonTests/ProviderDecodeTests.swift:
// https://github.com/Gonzih/tachyon; upstream notice in ../LICENSE-TACHYON.
#![allow(
    clippy::disallowed_methods,
    reason = "injected private fixture runtime and synchronization, never UI IO"
)]
use bboxy450_usage::{claude::Claude, codex::Codex, grok::Grok, *};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Reader {
    env: BTreeMap<String, String>,
    files: BTreeMap<PathBuf, Vec<u8>>,
    keychain: Option<Vec<u8>>,
    keychain_calls: AtomicUsize,
    tail: Option<Vec<u8>>,
}
impl CredentialReader for Reader {
    fn env(&self, key: &str) -> Option<String> {
        self.env.get(key).cloned()
    }
    fn home(&self) -> PathBuf {
        "/fixture".into()
    }
    fn exists(&self, path: &Path) -> bool {
        self.files.keys().any(|p| p.starts_with(path))
    }
    fn file(&self, path: &Path) -> Result<Option<Vec<u8>>, Failure> {
        Ok(self.files.get(path).cloned())
    }
    fn keychain(&self, _: &str) -> Result<Option<Vec<u8>>, Failure> {
        self.keychain_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.keychain.clone())
    }
    fn rollout_tail(&self, _: &Path) -> Option<Vec<u8>> {
        self.tail.clone()
    }
}
fn claude_auth(token: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"claudeAiOauth":{"accessToken": token}})).unwrap()
}
fn ready(p: &dyn Provider, r: &dyn CredentialReader) -> Credential {
    let Presence::Ready(c) = p.credential(r).unwrap() else {
        panic!("expected ready");
    };
    c
}

#[test]
fn tachyon_fixtures_select_worst_window_used() {
    let used = |r: Result<Window, Failure>| r.map(|w| w.used.round() as u8);
    assert_eq!(
        used(Claude.decode(include_bytes!("fixtures/claude.json"))),
        Ok(36)
    );
    assert_eq!(used(Codex.decode(include_bytes!("fixtures/codex.json"))), Ok(100));
    let config: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/grok-config.json")).unwrap();
    assert_eq!(
        used(Grok.decode(&serde_json::to_vec(&serde_json::json!({"config":config})).unwrap())),
        Ok(30)
    );
    assert_eq!(
        used(Claude.decode(br#"{"five_hour":{"utilization":10},"seven_day":{"utilization":70}}"#)),
        Ok(70)
    );
    assert_eq!(
        used(Claude.decode(br#"{"limits":[{"kind":"session","percent":62,"is_active":true}]}"#)),
        Ok(62)
    );
    assert_eq!(
        used(Grok.decode(br#"{"config":{"subscriptionTier":"premium"}}"#)),
        Ok(0)
    );
    assert_eq!(
        Grok.decode(br#"{"config":{"creditUsagePercent":"bad"}}"#),
        Err(Failure::Decode)
    );
    assert_eq!(
        Codex.decode(
            br#"{"additional_rate_limits":[{"rate_limit":{"primary_window":{"used_percent":0}}}]}"#
        ),
        Err(Failure::Decode)
    );
    for p in providers() {
        assert_eq!(p.decode(b"{}"), Err(Failure::Decode));
        assert_eq!(
            p.decode(b"not json token-should-not-escape"),
            Err(Failure::Decode)
        );
    }
}

#[test]
fn credential_order_overrides_issuer_and_redaction() {
    let mut r = Reader::default();
    r.env
        .insert("CLAUDE_CODE_OAUTH_TOKEN".into(), "environment".into());
    r.keychain = Some(claude_auth("keychain"));
    r.files.insert(
        "/fixture/.claude/.credentials.json".into(),
        claude_auth("file"),
    );
    assert!(
        Claude.request(&ready(&Claude, &r)).headers[0]
            .1
            .ends_with("environment")
    );
    assert_eq!(r.keychain_calls.load(Ordering::Relaxed), 0);
    r.env.clear();
    assert!(
        Claude.request(&ready(&Claude, &r)).headers[0]
            .1
            .ends_with("keychain")
    );
    r.keychain = None;
    assert!(
        Claude.request(&ready(&Claude, &r)).headers[0]
            .1
            .ends_with("file")
    );
    assert_eq!(
        format!("{:?}", ready(&Claude, &r)),
        "Credential([redacted])"
    );
    r.env.insert("CODEX_HOME".into(), "/custom".into());
    r.files.insert(
        "/custom/auth.json".into(),
        br#"{"tokens":{"access_token":"fixture-secret","account_id":"fixture-account"}}"#.to_vec(),
    );
    let request = Codex.request(&ready(&Codex, &r));
    assert!(
        request
            .headers
            .iter()
            .any(|(k, v)| *k == "ChatGPT-Account-Id" && v == "fixture-account")
    );
    r.files.insert("/fixture/.grok/auth.json".into(), br#"{"foreign":{"oidc_issuer":"https://other.invalid","key":"wrong"},"ours":{"oidc_issuer":"https://auth.x.ai","key":"right","user_id":"user"}}"#.to_vec());
    let request = Grok.request(&ready(&Grok, &r));
    assert!(request.headers[0].1.ends_with("right"));
    assert!(
        request
            .headers
            .iter()
            .any(|(k, v)| *k == "x-userid" && v == "user")
    );
    assert!(matches!(
        Codex.credential(&Reader::default()),
        Ok(Presence::Missing)
    ));
    assert!(Credential::new("bad\r\nheader".into(), None).is_err());
}

struct Fetcher {
    status: u16,
    body: Vec<u8>,
    calls: AtomicUsize,
}
impl Fetch for Fetcher {
    fn get(&self, _: Request) -> FetchFuture<'_> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async {
            Ok(Response {
                status: self.status,
                body: self.body.clone(),
            })
        })
    }
}
#[tokio::test]
async fn fetch_errors_and_auth_hints_are_sanitized() {
    let c = Credential::new("never-in-output".into(), None).unwrap();
    for p in providers() {
        for status in [401, 403, 429, 500, 200] {
            let fetch = Fetcher {
                status,
                body: b"never-in-output".to_vec(),
                calls: AtomicUsize::new(0),
            };
            let r = poller::fetch_reading(p.as_ref(), &fetch, &c, Duration::from_secs(1)).await;
            assert_eq!(r.used, None);
            assert!(!format!("{r:?}").contains("never-in-output"));
            if status == 401 || status == 403 {
                assert_eq!(r.reason.as_deref(), Some(p.login_hint()));
            }
        }
    }
}

struct Hanging;
impl Fetch for Hanging {
    fn get(&self, _: Request) -> FetchFuture<'_> {
        Box::pin(std::future::pending())
    }
}
#[tokio::test(start_paused = true)]
async fn timeout_is_not_current_or_success() {
    let c = Credential::new("fixture".into(), None).unwrap();
    let reading = poller::fetch_reading(&Codex, &Hanging, &c, Duration::from_secs(8)).await;
    assert_eq!(reading, Reading::error("Read timed out"));
}

#[test]
fn rollout_fallback_is_explicitly_historical() {
    let fixture = br#"{"timestamp":"2026-11-01T00:00:00Z","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":27},"secondary":{"used_percent":60}}}}
{"payload":{"type":"other","rate_limits":{"primary":{"used_percent":99}}}}
"#;
    let reading = codex::rollout(fixture).unwrap();
    assert_eq!(reading.used, Some(60));
    assert!(reading.stale);
    assert!(reading.reason.unwrap().contains("account unverified"));
}

#[tokio::test]
async fn workers_hide_missing_providers_and_manual_refresh_reads_again() {
    let mut r = Reader::default();
    r.files.insert(
        "/fixture/.codex/auth.json".into(),
        br#"{"tokens":{"access_token":"test-account"}}"#.to_vec(),
    );
    let fetch = Arc::new(Fetcher {
        status: 200,
        body: include_bytes!("fixtures/codex.json").to_vec(),
        calls: AtomicUsize::new(0),
    });
    let updates = Arc::new(Mutex::new(Vec::new()));
    let sink = updates.clone();
    let worker = poller::Worker::start(
        &tokio::runtime::Handle::current(),
        "/fixture".into(),
        providers(),
        Arc::new(r),
        fetch.clone(),
        Arc::new(move |u| sink.lock().unwrap().push(u)),
        poller::Intervals::default(),
    );
    for _ in 0..100 {
        if updates.lock().unwrap().iter().any(|u| {
            matches!(
                u,
                poller::Update::Provider {
                    id: "codex",
                    reading: Some(Reading { used: Some(100), .. })
                }
            )
        }) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(fetch.calls.load(Ordering::Relaxed), 1);
    assert!(updates.lock().unwrap().iter().any(|u| matches!(
        u,
        poller::Update::Provider {
            id: "grok",
            reading: None
        }
    )));
    worker.refresh();
    for _ in 0..100 {
        if fetch.calls.load(Ordering::Relaxed) == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(fetch.calls.load(Ordering::Relaxed), 2);
}

struct SwitchingReader(Mutex<Reader>);
impl CredentialReader for SwitchingReader {
    fn env(&self, key: &str) -> Option<String> {
        self.0.lock().unwrap().env(key)
    }
    fn home(&self) -> PathBuf {
        "/fixture".into()
    }
    fn exists(&self, path: &Path) -> bool {
        self.0.lock().unwrap().exists(path)
    }
    fn file(&self, path: &Path) -> Result<Option<Vec<u8>>, Failure> {
        self.0.lock().unwrap().file(path)
    }
    fn keychain(&self, service: &str) -> Result<Option<Vec<u8>>, Failure> {
        self.0.lock().unwrap().keychain(service)
    }
}
struct SwitchDuringFetch(Arc<SwitchingReader>);
impl Fetch for SwitchDuringFetch {
    fn get(&self, _: Request) -> FetchFuture<'_> {
        Box::pin(async {
            self.0.0.lock().unwrap().files.insert(
                "/fixture/.codex/auth.json".into(),
                br#"{"tokens":{"access_token":"account-B"}}"#.to_vec(),
            );
            Ok(Response {
                status: 200,
                body: include_bytes!("fixtures/codex.json").to_vec(),
            })
        })
    }
}
#[tokio::test]
async fn worker_revalidates_credentials_before_publishing_an_inflight_reply() {
    let mut r = Reader::default();
    r.files.insert(
        "/fixture/.codex/auth.json".into(),
        br#"{"tokens":{"access_token":"account-A"}}"#.to_vec(),
    );
    let r = Arc::new(SwitchingReader(Mutex::new(r)));
    let updates = Arc::new(Mutex::new(Vec::new()));
    let sink = updates.clone();
    let _worker = poller::Worker::start(
        &tokio::runtime::Handle::current(),
        "/fixture".into(),
        vec![Arc::new(Codex)],
        r.clone(),
        Arc::new(SwitchDuringFetch(r)),
        Arc::new(move |u| sink.lock().unwrap().push(u)),
        Default::default(),
    );
    for _ in 0..100 {
        if updates
            .lock()
            .unwrap()
            .iter()
            .any(|u| matches!(u, poller::Update::Provider { .. }))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let updates = updates.lock().unwrap();
    let reading = updates
        .iter()
        .find_map(|u| match u {
            poller::Update::Provider {
                reading: Some(r), ..
            } => Some(r),
            _ => None,
        })
        .unwrap();
    assert_eq!(reading.used, None);
    assert_eq!(
        reading.reason.as_deref(),
        Some("Account changed; refresh required")
    );
    assert!(!format!("{updates:?}").contains("account-A"));
}

#[test]
fn tachyon_pace_projects_used_over_elapsed_window() {
    let w = Claude
        .decode(br#"{"five_hour":{"utilization":40,"resets_at":"1970-01-01T05:00:00.250+00:00"}}"#)
        .unwrap();
    assert_eq!(w.resets_at, Some(18000.25));
    // Half of the 5 h window elapsed at 40% used: ~80% by reset.
    assert_eq!(Reading::from_window(w, 9000.25).projected, Some(80));
    // Under 10% of the window elapsed: no pace yet.
    assert_eq!(Reading::from_window(w, 1000.).projected, None);
    // On pace past the limit.
    let hot = Window::timed(60., Some(18000.), Some(18000.));
    assert_eq!(Reading::from_window(hot, 9000.).projected, Some(120));
    // A stale reading never drives pace.
    let mut stale = Reading::from_window(hot, 9000.);
    stale.stale("Refresh due");
    assert_eq!(stale.projected, None);
}
