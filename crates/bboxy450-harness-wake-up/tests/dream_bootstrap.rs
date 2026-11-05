// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Sole-producer-corpus conformance; no locally generated binary vectors.
#![allow(
    clippy::disallowed_methods,
    reason = "private canonical corpus/process tests"
)]
use bboxy450_harness_wake_up::{dream_bootstrap as b, dream_c1};
use serde_json::Value;
use std::{
    path::{Component, PathBuf},
    process::Command,
};
// Signed bboxy450-harness bab543e2116d0bf08488fbd753a6114f30d615a4,
// src/dream/bootstrap.rs SHA256 1c69a2d42142cf046075d97148e2317d0c9594292c24da411be0a44c5225e555.
const VECTORS_SHA256: &str = "de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307";

fn fixtures() -> PathBuf {
    let root = PathBuf::from(
        std::env::var_os("DREAM_BOOTSTRAP_FIXTURES")
            .expect("DREAM_BOOTSTRAP_FIXTURES must name the approved producer corpus; no fallback"),
    );
    for file in ["README.md", "vectors.json", "check.py", "valid", "invalid"] {
        assert!(
            root.join(file).exists(),
            "missing canonical bootstrap corpus {file}"
        );
    }
    let result = Command::new("/usr/bin/python3")
        .env_clear()
        .args(["-I", "-c", "import hashlib,pathlib,sys; assert hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest()==sys.argv[2], 'signed bootstrap inventory mismatch'"])
        .arg(root.join("vectors.json"))
        .arg(VECTORS_SHA256)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "signed bootstrap corpus pin refused"
    );
    root
}
fn binding_bytes(i: &Value) -> Option<Vec<u8>> {
    if i["profile"] != b::PROFILE {
        return None;
    }
    let purpose = match i["purpose"].as_str()? {
        "inspect-c1" => b::Purpose::InspectC1,
        "read-prepare" => b::Purpose::ReadPrepare,
        _ => return None,
    };
    let context = i.get("context")?;
    let context = if context.is_null() {
        None
    } else {
        Some(serde_json::from_value::<dream_c1::wire::Context>(context.clone()).ok()?)
    };
    let op = serde_json::from_value::<dream_c1::wire::Ulid>(i["op_id"].clone()).ok()?;
    let run = serde_json::from_value::<dream_c1::wire::Ulid>(i["run_id"].clone()).ok()?;
    let nonce = serde_json::from_value::<dream_c1::wire::Ulid>(i["nonce"].clone()).ok()?;
    let hash = serde_json::from_value::<dream_c1::wire::Hex>(i["worker_sha256"].clone()).ok()?;
    let binding = b::Binding::new(
        purpose,
        i["probe"].as_bool()?,
        &op,
        &run,
        &nonce,
        i["incarnation"].as_u64()?,
        context.as_ref(),
        &hash,
    )
    .ok()?;
    serde_json::to_vec(&binding).ok()
}
fn frozen(case: &Value) -> Option<b::Frozen> {
    let a = &case["authority"];
    let i = &a["inputs"];
    let probe = i["probe"].as_bool()?;
    let context = i.get("context")?;
    if probe {
        if !context.is_null() {
            return None;
        }
    } else if serde_json::from_value::<dream_c1::wire::Context>(context.clone()).is_err() {
        return None;
    }
    Some(b::Frozen {
        purpose: match i["purpose"].as_str()? {
            "inspect-c1" => b::Purpose::InspectC1,
            "read-prepare" => b::Purpose::ReadPrepare,
            _ => return None,
        },
        probe,
        birth: b::ulid(a["birth"].as_str()?.as_bytes()).ok()?,
        run_id: b::ulid(i["run_id"].as_str()?.as_bytes()).ok()?,
        nonce: b::ulid(i["nonce"].as_str()?.as_bytes()).ok()?,
        binding: b::binding(a["binding_sha256"].as_str()?.as_bytes()).ok()?,
        incarnation: i["incarnation"].as_u64()?,
    })
}

#[test]
fn canonical_bootstrap_vectors_validate_every_binary_environment_and_binding_identity() {
    let root = fixtures();
    let inventory: Value =
        serde_json::from_slice(&std::fs::read(root.join("vectors.json")).unwrap()).unwrap();
    assert_eq!(inventory["profile"], b::PROFILE);
    assert_eq!(inventory["record_bytes"], b::LENGTH);
    assert_eq!(inventory["case_count"], 34);
    assert_eq!(inventory["binding_case_count"], 8);
    let cases = inventory["cases"]
        .as_array()
        .expect("producer case inventory");
    assert_eq!(Some(cases.len() as u64), inventory["case_count"].as_u64());
    let mut counts = [0usize; 5];
    for case in cases {
        let file = case["file"].as_str().unwrap();
        let path = std::path::Path::new(file);
        assert!(
            path.components().all(|p| matches!(p, Component::Normal(_)))
                && (file.starts_with("valid/") || file.starts_with("invalid/"))
        );
        assert!(std::fs::metadata(root.join(file)).unwrap().len() <= 1024);
        let bytes = std::fs::read(root.join(file)).unwrap();
        assert_eq!(
            binding_bytes(&case["authority"]["inputs"]).unwrap(),
            case["authority"]["binding_canonical_json"]
                .as_str()
                .unwrap()
                .as_bytes(),
            "binding {file}"
        );
        let record = b::decode(&bytes);
        let env = b::token(case["environment"].as_str().unwrap().as_bytes());
        match case["expect"]["binary_format"].as_str().unwrap() {
            "valid" => {
                let record = record.as_ref().unwrap_or_else(|e| panic!("{file}: {e:?}"));
                assert_eq!(b::encode(record).as_slice(), bytes);
                counts[0] += 1;
            }
            "invalid" => {
                assert!(record.is_err(), "accepted {file}");
                counts[1] += 1;
            }
            _ => panic!("unknown binary verdict"),
        }
        match case["expect"]["environment_format"].as_str().unwrap() {
            "valid" => {
                assert!(env.is_ok(), "environment {file}");
            }
            "invalid" => {
                assert!(env.is_err(), "environment {file}");
                counts[2] += 1;
            }
            _ => panic!("unknown environment verdict"),
        }
        match case["expect"]["identity"].as_str().unwrap() {
            "match" => {
                assert!(
                    record
                        .as_ref()
                        .ok()
                        .zip(env.as_ref().ok())
                        .zip(frozen(case))
                        .is_some_and(|((r, t), f)| f.matches(r, t)
                            && Some(t.fd as u64) == case["authority"]["fd"].as_u64()),
                    "identity {file}"
                );
                counts[3] += 1;
            }
            "mismatch" => {
                assert!(
                    !record
                        .as_ref()
                        .ok()
                        .zip(env.as_ref().ok())
                        .zip(frozen(case))
                        .is_some_and(|((r, t), f)| f.matches(r, t)
                            && Some(t.fd as u64) == case["authority"]["fd"].as_u64()),
                    "identity {file}"
                );
                counts[4] += 1;
            }
            "not_checked" => assert!(record.is_err() || env.is_err()),
            _ => panic!("unknown identity verdict"),
        }
    }
    assert_eq!(counts, [21, 13, 9, 4, 8]);
    let bindings = inventory["binding_cases"].as_array().unwrap();
    assert_eq!(bindings.len(), 8);
    let mut accepted = 0;
    for case in bindings {
        let bytes = binding_bytes(&case["inputs"]);
        match case["expect"].as_str().unwrap() {
            "valid" => {
                assert_eq!(
                    bytes.unwrap(),
                    case["binding_canonical_json"].as_str().unwrap().as_bytes(),
                    "{}",
                    case["name"]
                );
                accepted += 1;
            }
            "invalid" => assert!(bytes.is_none(), "accepted {}", case["name"]),
            _ => panic!("unknown binding input verdict"),
        }
    }
    assert_eq!(accepted, 2);
}

#[test]
fn independently_pinned_bootstrap_inventory_and_binding_hashes_are_checked_without_repair() {
    let root = fixtures();
    let result=Command::new("/usr/bin/python3").env_clear().args(["-I","-c",r#"
import hashlib,pathlib,runpy,sys
root=pathlib.Path(sys.argv[1])
assert hashlib.sha256((root/'vectors.json').read_bytes()).hexdigest()==sys.argv[2], 'bootstrap authority pin mismatch'
runpy.run_path(str(root/'check.py'),run_name='__main__')
"#]).arg(root).arg(VECTORS_SHA256).output().unwrap();
    assert!(
        result.status.success(),
        "canonical producer checker failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
