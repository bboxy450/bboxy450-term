// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::{
    ir::*,
    sanitize::{CapabilityRefusal, Sanitizer, TOOL_OUTPUT_LIMIT},
};
use serde_json::json;
fn session(provider: Provider) -> Session {
    Session {
        id: SessionId(ulid::Ulid::from(1u128)),
        source: Source {
            provider,
            native_id: "private-session-name".into(),
            cli_version: None,
            parser_revision: 1,
        },
        lineage: vec![],
        cwd: Some("/Users/synthetic/work".into()),
        git: None,
        created_at: None,
    }
}
fn event(provider: Provider, text: String) -> Event {
    Event{seq:0,id:EventId::native(provider,"private-session-name","record"),record:RecordId([9;16]),parent:None,turn:Some(TurnId("private-session-name".into())),at:None,fidelity:Fidelity::Full,raw:Some(Raw{provider,bytes:serde_json::to_vec(&json!({"type":"user","sessionId":"private-session-name","message":{"content":[{"type":"tool_result","content":text.clone()}]},"opaqueUnknown":text})).unwrap()}),kind:EventKind::ToolResult{call_id:"toolu_synthetic".into(),is_error:false,blocks:vec![Block::Text{text}]}}
}
#[test]
fn planted_secrets_and_personal_identifiers_are_absent_from_kind_and_raw() {
    let api = ["sk-", "ant-synthetic-credential-123456789"].concat();
    let github = ["ghp_", "012345678901234567890123456789"].concat();
    let cap = "0123456789abcdef".repeat(4);
    let configured = "synthetic-configured-value";
    let email = ["synthetic", "@", "private.example"].concat();
    let pem = [
        "-----BEGIN ",
        "PRIVATE KEY-----\nSYNTHETICPEMBODY\n-----END PRIVATE KEY-----",
    ]
    .concat();
    let content = format!(
        "{api} {github} {cap} {configured}\n{pem}\nAuthorization: SyntheticAuth\nBearer SyntheticBearer\n/Users/synthetic/work {email} workstation.private 203.0.113.88 [2001:db8::12] 10000000-0000-4000-8000-000000000001 01M6CJ3RQK3Q9S9MJ3VH9Y365J private-session-name"
    );
    for provider in [Provider::Claude, Provider::Codex, Provider::Bboxy] {
        let original = event(provider, content.clone());
        let mut a = original.clone();
        let mut b = original;
        let mut sa = session(provider);
        let mut sb = sa.clone();
        let mut sanitizer = Sanitizer::new(vec![cap.clone(), configured.into()]);
        sanitizer.session(&mut sa);
        sanitizer.event(&mut a);
        let mut second = Sanitizer::new(vec![cap.clone(), configured.into()]);
        second.session(&mut sb);
        second.event(&mut b);
        assert_eq!(a, b);
        assert_eq!(sa, sb);
        let raw = String::from_utf8(a.raw.as_ref().unwrap().bytes.clone()).unwrap();
        let output = format!(
            "{} {} {}",
            serde_json::to_string(&a.kind).unwrap(),
            raw,
            serde_json::to_string(&sa).unwrap()
        );
        for forbidden in [
            &api,
            &github,
            &cap,
            configured,
            &email,
            "SYNTHETICPEMBODY",
            "SyntheticAuth",
            "SyntheticBearer",
            "/Users/",
            "workstation.private",
            "203.0.113.88",
            "2001:db8::12",
            "10000000-0000-4000-8000-000000000001",
            "01M6CJ3RQK3Q9S9MJ3VH9Y365J",
            "private-session-name",
        ] {
            assert!(
                !output.contains(forbidden),
                "SanitizePlantedSecretsRemoved: sensitive category remains"
            );
        }
        assert!(output.contains("<email>"));
        assert!(output.contains("<host>"));
        assert!(output.contains("<ip>"));
    }
}
#[test]
fn output_limit_image_metadata_and_opaque_reasoning() {
    let provider = Provider::Claude;
    let text = "é".repeat(20_000);
    let mut event = event(provider, text);
    let mut sanitizer = Sanitizer::default();
    sanitizer.session(&mut session(provider));
    sanitizer.event(&mut event);
    if let EventKind::ToolResult { blocks, .. } = &event.kind {
        let size: usize = blocks
            .iter()
            .map(|b| {
                if let Block::Text { text } = b {
                    text.len()
                } else {
                    0
                }
            })
            .sum();
        assert!(size <= TOOL_OUTPUT_LIMIT);
        assert!(matches!(&blocks[0],Block::Text{text}if text.contains("truncated")));
    } else {
        panic!()
    }
    let raw: serde_json::Value = serde_json::from_slice(&event.raw.unwrap().bytes).unwrap();
    assert!(
        raw["message"]["content"][0]["content"]
            .as_str()
            .unwrap()
            .len()
            <= TOOL_OUTPUT_LIMIT
    );
    let sealed = Sealed {
        provider,
        form: SealedForm::ThinkingSignature,
        bytes: b"unchanged-opaque-signature".to_vec(),
    };
    let mut e=Event{seq:0,id:EventId([1;16]),record:RecordId([9;16]),parent:None,turn:None,at:None,fidelity:Fidelity::Full,raw:Some(Raw{provider,bytes:serde_json::to_vec(&json!({"type":"assistant","message":{"content":[{"type":"thinking","thinking":"/Users/synthetic/thought","signature":"unchanged-opaque-signature"},{"type":"image","source":{"media_type":"image/png","data":"SYNTHETIC_IMAGE_PAYLOAD"}}]}})).unwrap()}),kind:EventKind::Reasoning{text:Some("/Users/synthetic/thought".into()),summary:None,sealed:Some(sealed.clone())}};
    sanitizer.event(&mut e);
    assert!(matches!(e.kind,EventKind::Reasoning{sealed:Some(s),..}if s==sealed));
    let raw = String::from_utf8(e.raw.unwrap().bytes).unwrap();
    assert!(!raw.contains("SYNTHETIC_IMAGE_PAYLOAD"));
    assert!(!raw.contains("/Users/"));
    assert!(raw.contains("image/png"));
    assert!(raw.contains("unchanged-opaque-signature"));
}
#[test]
fn invalid_raw_is_removed_with_explicit_fidelity_downgrade() {
    let mut e = event(Provider::Claude, "safe".into());
    e.raw.as_mut().unwrap().bytes = b"invalid".to_vec();
    Sanitizer::default().event(&mut e);
    assert!(e.raw.is_none());
    assert_eq!(e.fidelity, Fidelity::Partial);
}
#[test]
fn capability_file_validation_and_exact_value() {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let dir = std::env::temp_dir().join(format!("lc-cap-{}", ulid::Ulid::new()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("capability");
    let value = "abcdef0123456789".repeat(4);
    let mut f = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    writeln!(f, "{value}").unwrap();
    drop(f);
    let sanitizer = Sanitizer::default().with_capability_file(&path).unwrap();
    assert_eq!(sanitizer.text(&value), "<redacted>");
    let link = dir.join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(matches!(
        Sanitizer::default().with_capability_file(&link),
        Err(CapabilityRefusal::Symlink)
    ));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn sanitizer_handles_bare_hosts_urls_spaced_homes_and_all_credential_families() {
    let samples = [
        ["sk-", "proj-synthetic-value-123456789"].concat(),
        ["github_pat_", "synthetic012345678901234567890"].concat(),
        ["AKIA", "ABCDEFGHIJKLMNOP"].concat(),
        ["xoxb-", "synthetic-123456789"].concat(),
        ["AIza", "012345678901234567890123456789012345"].concat(),
    ];
    let sanitizer = Sanitizer::default();
    for sample in samples {
        assert_eq!(
            sanitizer.text(&sample),
            "<redacted>",
            "SanitizeCredentialFamily"
        );
    }
    for sample in [
        "host=bare-workstation",
        "HOSTNAME: bare-workstation",
        "https://bare-workstation/path",
        "http://user:pass@bare-workstation:123/path",
        "'/Users/Synthetic Person/private'",
    ] {
        let safe = sanitizer.text(sample);
        assert!(!safe.contains("bare-workstation"));
        assert!(!safe.contains("Synthetic Person"));
        assert!(!safe.contains("user:pass"));
        assert!(!safe.contains("/Users/"));
    }
    let mut e = event(Provider::Claude, "safe".into());
    e.raw.as_mut().unwrap().bytes =
        serde_json::to_vec(&json!({"host":"bare-workstation","hostname":"another-bare-host"}))
            .unwrap();
    sanitizer.event(&mut e);
    let raw = String::from_utf8(e.raw.unwrap().bytes).unwrap();
    assert!(!raw.contains("bare-workstation"));
    assert!(!raw.contains("another-bare-host"));
}
