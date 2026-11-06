// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public controls use the sole producer corpus; no embedded fixture fallback.
use bbt_proto::dream as w;
use serde_json::{Value, json};
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    let root = PathBuf::from(std::env::var_os("DREAM_C1_FIXTURES").expect(
        "DREAM_C1_FIXTURES must name the deliberately pinned harness tests/fixtures/dream-c1",
    ));
    for required in ["valid", "invalid", "vectors/canonical.json", "lock.json"] {
        assert!(
            root.join(required).exists(),
            "missing producer corpus: {}",
            root.join(required).display()
        );
    }
    root
}

#[allow(clippy::disallowed_methods, reason = "canonical corpus IO runs on the isolated test owner")]
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join("valid").join(format!("{name}.json")))
        .unwrap_or_else(|error| panic!("missing canonical producer {name}: {error}"))
}

fn public(name: &str, bytes: &[u8]) -> Vec<u8> {
    match name {
        "inspect" | "status" | "cancel" => {
            let value: w::Control =
                w::decode(bytes).expect("public control mirror rejects producer fixture");
            value
                .validate()
                .expect("producer control violates public input bounds");
            w::canonical(&value).unwrap()
        }
        "success" | "failure" => {
            w::canonical(&w::decode::<w::ControlReply>(bytes).unwrap()).unwrap()
        }
        "page" => w::canonical(&w::decode::<w::Page>(bytes).unwrap()).unwrap(),
        _ => panic!("not a public C1 fixture: {name}"),
    }
}

#[test]
#[allow(clippy::disallowed_methods, reason = "canonical schema IO runs on the isolated test owner")]
fn public_controls_and_pages_conform_to_pinned_producer_schema_and_bytes() {
    let root = fixtures();
    let producer = root.ancestors().nth(3).expect("producer fixture path");
    let schema: Value = serde_json::from_slice(
        &std::fs::read(producer.join("schema/dream-c1-v1.schema.json"))
            .expect("missing canonical producer schema"),
    )
    .unwrap();
    for (name, definition) in [
        ("inspect", "Control"),
        ("status", "Control"),
        ("cancel", "Control"),
        ("success", "ControlReply"),
        ("failure", "ControlReply"),
        ("page", "Page"),
    ] {
        let bytes = fixture(name);
        assert_eq!(
            public(name, &bytes),
            bytes,
            "canonical mirror disagreement: {name}"
        );
        let isolated = json!({"$defs": schema["$defs"], "$ref": format!("#/$defs/{definition}")});
        let validator = jsonschema::validator_for(&isolated).unwrap();
        assert!(
            validator.is_valid(&serde_json::from_slice::<Value>(&bytes).unwrap()),
            "schema disagreement: {name}"
        );
    }
    // Context crosses the control/worker boundary without authority supplied by an agent.
    let manifest: Value = serde_json::from_slice(&fixture("manifest")).unwrap();
    let context: w::Context = serde_json::from_value(manifest["context"].clone()).unwrap();
    assert_eq!(serde_json::to_value(context).unwrap(), manifest["context"]);
}

#[test]
fn public_mutations_reject_unknown_duplicate_unsafe_and_open_ended_values() {
    let valid: Value = serde_json::from_slice(&fixture("inspect")).unwrap();
    for (field, replacement) in [
        ("v", json!(2)),
        ("v", json!(1.0)),
        ("method", json!("replay")),
        ("op_id", json!("01arz3ndektsv4rrffq69g5fav")),
        ("op_id", json!("8ZZZZZZZZZZZZZZZZZZZZZZZZZ")),
        ("path", json!("/caller/chosen/source")),
    ] {
        let mut changed = valid.clone();
        changed[field] = replacement;
        assert!(
            w::decode::<w::Control>(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "accepted {field}"
        );
    }
    for generation in [json!(0), json!(9007199254740992_u64), json!(1.5), json!(-1)] {
        let mut changed = valid.clone();
        changed["sources"][0]["source_generation"] = generation;
        assert!(
            w::decode::<w::Control>(&serde_json::to_vec(&changed).unwrap())
                .and_then(|c| c.validate())
                .is_err()
        );
    }
    for count in [0, 2, 33] {
        let mut changed = valid.clone();
        changed["sources"] = Value::Array(vec![valid["sources"][0].clone(); count]);
        assert!(
            w::decode::<w::Control>(&serde_json::to_vec(&changed).unwrap())
                .unwrap()
                .validate()
                .is_err()
        );
    }
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("v");
    assert!(w::decode::<w::Control>(&serde_json::to_vec(&missing).unwrap()).is_err());
    let duplicate = format!(
        "{{\"v\":1,{}",
        std::str::from_utf8(&fixture("inspect"))
            .unwrap()
            .trim_start_matches('{')
    );
    assert!(
        w::decode::<w::Control>(duplicate.as_bytes()).is_err(),
        "duplicate public field accepted"
    );
    assert!(w::decode::<w::Control>(&vec![b' '; w::MAX_FRAME + 1]).is_err());
    assert!(w::decode::<w::Control>(&[0xff]).is_err());

    let reply: Value = serde_json::from_slice(&fixture("success")).unwrap();
    for (field, replacement) in [
        ("state", json!("trusted")),
        ("request_sha256", json!("F".repeat(64))),
        ("revision", json!(9007199254740992_u64)),
    ] {
        let mut changed = reply.clone();
        changed["run"][field] = replacement;
        assert!(w::decode::<w::ControlReply>(&serde_json::to_vec(&changed).unwrap()).is_err());
    }
    let mut extra = reply;
    extra["run"]["transcript"] = json!("forbidden");
    assert!(w::decode::<w::ControlReply>(&serde_json::to_vec(&extra).unwrap()).is_err());
    let mut page: Value = serde_json::from_slice(&fixture("page")).unwrap();
    page["rows"][0]["kind"] = json!("memory_write");
    assert!(w::decode::<w::Page>(&serde_json::to_vec(&page).unwrap()).is_err());
}

#[test]
#[allow(clippy::disallowed_methods, reason = "the independent Python checker is an owned test process, never UI work")]
fn canonical_vectors_agree_with_independent_python_stdlib() {
    let output = std::process::Command::new("python3").args(["-I", "-c", r#"
import hashlib,json,pathlib,sys
root=pathlib.Path(sys.argv[1])
lock_raw=(root/'lock.json').read_bytes()
assert hashlib.sha256(lock_raw).hexdigest() == sys.argv[3], 'unpinned fixture lock'
lock=json.loads(lock_raw)
assert lock['schema_sha256'] == sys.argv[2]
assert hashlib.sha256((root.parents[2]/'schema/dream-c1-v1.schema.json').read_bytes()).hexdigest() == sys.argv[2], 'schema changed'
actual={p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p.name != 'lock.json'}
assert actual == set(lock['files']), 'missing or unindexed corpus entry'
for name,digest in lock['files'].items():
    assert hashlib.sha256((root/name).read_bytes()).hexdigest() == digest, name
vectors=json.loads((root/'vectors/canonical.json').read_bytes())
assert vectors, 'empty canonical corpus'
for vector in vectors:
    raw=json.dumps(vector['value'],ensure_ascii=False,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
    assert raw == vector['canonical'].encode()
    assert hashlib.sha256(raw).hexdigest() == vector['sha256']
print('independent canonical vectors:',len(vectors))
"#]).arg(fixtures()).arg(w::SCHEMA_SHA256).arg(w::CORPUS_LOCK_SHA256).output().unwrap();
    assert!(
        output.status.success(),
        "canonical vector disagreement: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
