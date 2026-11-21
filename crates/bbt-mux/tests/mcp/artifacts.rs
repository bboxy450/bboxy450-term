// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! Signed native artifact writes and authenticated reads through the real MCP door.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};

fn code(value: &Value) -> &Value {
    assert_eq!(value["isError"], true, "{value}");
    &value["structuredContent"]["error"]["code"]
}
fn svg(color: &str) -> String {
    let fonts = bboxy450_svg::FontSet::bundled().unwrap();
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" data-bboxy-font-set="{}" width="96" height="64"><rect width="96" height="64" fill="{color}"/></svg>"#,
        fonts.identity(),
    )
}
fn write(f: &Fixture, tool: &str, args: Value) -> Value {
    write_operation(f, Ulid::new(), tool, args)
}
fn write_operation(f: &Fixture, op: Ulid, tool: &str, args: Value) -> Value {
    after_nonadmission(|| f.mutate(f.own.id, op, tool, args.clone()))
}
fn after_nonadmission(mut call: impl FnMut() -> Value) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let value = call();
        if value["structuredContent"]["error"]["code"] != "provider_busy" {
            return value;
        }
        assert!(Instant::now() < deadline, "root remained busy: {value}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn read(f: &Fixture, id: &Value, revision: u64) -> Value {
    data(f.call(
        f.own.id,
        Ulid::new(),
        "artifact_read",
        json!({"id":id,"revision":revision}),
    ))
}

#[test]
fn artifact_mcp_exact_bytes_recovery_revisions_and_lineage_survive_restart() {
    let mut f = Fixture::real();
    let source_path = f.config.repository.join("stage.bin");
    let original = [0, 1, 2, 255].repeat(20_000);
    std::fs::write(&source_path, &original).unwrap();
    let op = Ulid::new();
    let args = json!({"media_type":"application/octet-stream","source_path":"stage.bin"});
    let snapshot = data(write_operation(&f, op, "artifact_store", args.clone()));
    assert_eq!(snapshot["revision"], 1);
    assert_eq!(snapshot["bytes"], original.len());
    assert_eq!(snapshot["capture"]["missing"], json!(["execution_source"]));
    assert!(snapshot["display_hint"].is_null());
    let evidence = data(write(
        &f,
        "artifact_store",
        json!({"media_type":"application/json","content":{"n":1,"rows":[null,"日本語"]}}),
    ));
    let text = data(write(
        &f,
        "artifact_store",
        json!({"media_type":"text/plain","content":"Original text\n"}),
    ));
    let art_a = data(write(
        &f,
        "artifact_store",
        json!({
            "media_type":"image/svg+xml","content":svg("#11f183"),
            "inputs":[{"id":evidence["id"],"revision":1},{"id":snapshot["id"],"revision":1}]
        }),
    ));
    let id = &art_a["id"];
    let a_pin = art_a["snapshot_reference"]
        .as_str()
        .unwrap()
        .trim_start_matches("![[")
        .trim_end_matches("]]");
    assert!(
        art_a["display_hint"]
            .as_str()
            .unwrap()
            .contains("historical")
    );
    assert_ne!(art_a["reference"], art_a["snapshot_reference"]);
    let next = json!({"id":id,"expected_revision":1,"content":svg("#bb2484")});
    let a = std::thread::scope(|s| {
        let b = s.spawn(|| write(&f, "artifact_update", next.clone()));
        [
            write(&f, "artifact_update", next.clone()),
            b.join().unwrap(),
        ]
    });
    assert_eq!(
        a.iter().filter(|result| result["isError"] == false).count(),
        1,
        "{a:?}"
    );
    assert!(
        a.iter().any(|result| matches!(
            result["structuredContent"]["error"]["code"].as_str(),
            Some("artifact_conflict" | "provider_busy" | "unknown" | "workspace_busy")
        )),
        "{a:?}"
    );
    let art_b = read(&f, id, 2);
    let art_c = data(write(
        &f,
        "artifact_update",
        json!({"id":id,"expected_revision":2,"content":svg("#11f183")}),
    ));
    assert_eq!(art_c["revision"], 3);
    assert_eq!(art_c["digest"], art_a["digest"]);
    assert_ne!(art_b["digest"], art_a["digest"]);
    assert!(art_c["capture"]["declared"].as_array().unwrap().is_empty());
    assert_eq!(
        code(&write(
            &f,
            "artifact_update",
            json!({"id":evidence["id"],"expected_revision":1,"content":"no"})
        )),
        "artifact_immutable"
    );
    assert_eq!(
        code(&f.call(f.peer.id, Ulid::new(), "artifact_read", json!({"id":id}))),
        "artifact_forbidden"
    );

    std::fs::write(&source_path, b"changed source is irrelevant").unwrap();
    f.restart();
    let retained = data(write_operation(&f, op, "artifact_store", args));
    assert_eq!(retained["id"], snapshot["id"]);
    assert_eq!(retained["digest"], snapshot["digest"]);
    assert_eq!(
        code(&f.mutate(
            f.own.id,
            op,
            "artifact_store",
            json!({"media_type":"text/plain","content":"different"})
        )),
        "op_id_reused"
    );
    assert_eq!(read(&f, &text["id"], 1)["content"], "Original text\n");
    assert_eq!(
        read(&f, &evidence["id"], 1)["content"]["rows"],
        json!([null, "日本語"])
    );
    assert_eq!(read(&f, id, 1)["capture"], art_a["capture"]);
    assert_eq!(read(&f, id, 1)["content"], svg("#11f183"));
    assert_eq!(read(&f, id, 2)["content"], svg("#bb2484"));

    let fetched = data(f.call(
        f.own.id,
        Ulid::new(),
        "provenance_get",
        json!({"locator":a_pin}),
    ));
    assert_eq!(fetched["entity"]["digest"], art_a["digest"]);
    assert_eq!(fetched["entity"]["capture"], art_a["capture"]);
    let hist = data(f.call(
        f.own.id,
        Ulid::new(),
        "provenance_history",
        json!({"locator":id}),
    ));
    let revs: Vec<_> = hist["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["stored"]["revision"].as_u64())
        .collect();
    assert_eq!(revs, [1, 2, 3]);
    let lineage = data(f.call(
        f.own.id,
        Ulid::new(),
        "provenance_lineage",
        json!({"locator":a_pin}),
    ));
    assert_eq!(lineage["edges"].as_array().unwrap().len(), 2);
    assert!(
        lineage["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["basis"] == "declared"
                && edge["digest"] == snapshot["digest"]
                && edge["output"] == a_pin)
    );
    let found = data(f.call(
        f.own.id,
        Ulid::new(),
        "provenance_search",
        json!({"query":a_pin}),
    ));
    assert_eq!(found["items"][0]["revision"], 1);

    let binary = read(&f, &snapshot["id"], 1);
    let mut next = binary["resource"]["uri"].as_str().map(str::to_owned);
    let mut bytes = Vec::new();
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let mut request = 2;
    while let Some(uri) = next.take() {
        let page = rpc(&mut bridge, request, "resources/read", json!({"uri":uri}));
        request += 1;
        let part = &page["result"]["contents"][0];
        assert_eq!(
            part["_meta"]["bboxy450/digest"], snapshot["digest"],
            "{page}"
        );
        bytes.extend(STANDARD.decode(part["blob"].as_str().unwrap()).unwrap());
        next = part["_meta"]["bboxy450/next"].as_str().map(str::to_owned);
    }
    assert_eq!(bytes, original);

    let claimed = write(
        &f,
        "artifact_claim",
        json!({"kind":"image","ref":format!("artifact:{id}",id=id.as_str().unwrap())}),
    );
    assert_eq!(code(&claimed), "artifact_custody_conflict", "{claimed}");
}

#[test]
fn artifact_mcp_bounds_and_missing_observed_source_refuse_before_revision() {
    let f = Fixture::real();
    let too_large = write(
        &f,
        "artifact_store",
        json!({"media_type":"text/plain","content":"é".repeat(131073)}),
    );
    assert_eq!(code(&too_large), "artifact_too_large");
    let edge = data(write(
        &f,
        "artifact_store",
        json!({"media_type":"text/plain","content":"é".repeat(131072)}),
    ));
    assert_eq!(edge["bytes"], 262144);
    let path = f.config.repository.join("too-large.bin");
    std::fs::File::create(path)
        .unwrap()
        .set_len(16777217)
        .unwrap();
    assert_eq!(
        code(&write(
            &f,
            "artifact_store",
            json!({"media_type":"application/octet-stream","source_path":"too-large.bin"})
        )),
        "artifact_too_large"
    );
    std::os::unix::fs::symlink(
        f.config.repository.join("too-large.bin"),
        f.config.repository.join("outside.bin"),
    )
    .unwrap();
    assert_eq!(
        code(&write(
            &f,
            "artifact_store",
            json!({"media_type":"application/octet-stream","source_path":"outside.bin"})
        )),
        "artifact_source_unavailable"
    );
    let png = |width, name: &str| {
        let file = std::fs::File::create(f.config.repository.join(name)).unwrap();
        let mut encoder = png::Encoder::new(file, width, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&vec![0; width as usize * 4])
            .unwrap();
    };
    png(2047, "edge.png");
    let admissible = data(write(
        &f,
        "artifact_store",
        json!({"media_type":"image/png","source_path":"edge.png"}),
    ));
    assert_eq!(admissible["revision"], 1);
    assert!(
        admissible["reference"]
            .as_str()
            .unwrap()
            .starts_with("![[artifact:")
    );
    assert_eq!(
        code(&write(
            &f,
            "artifact_update",
            json!({"id":admissible["id"],"expected_revision":1,"content":svg("#11f183")})
        )),
        "artifact_immutable"
    );
    png(2048, "oversize.png");
    assert_eq!(
        code(&write(
            &f,
            "artifact_store",
            json!({"media_type":"image/png","source_path":"oversize.png"})
        )),
        "artifact_image_refused"
    );
    let missing_font = r#"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="64"/>"#;
    assert_eq!(
        code(&write(
            &f,
            "artifact_store",
            json!({"media_type":"image/svg+xml","content":missing_font})
        )),
        "artifact_image_refused"
    );
    let foreign_font = svg("#11f183").replace(
        bboxy450_svg::FontSet::bundled().unwrap().identity(),
        "sha256:bad",
    );
    assert_eq!(
        code(&write(
            &f,
            "artifact_store",
            json!({"media_type":"image/svg+xml","content":foreign_font})
        )),
        "artifact_image_refused"
    );
    let context = data(f.call(f.own.id, Ulid::new(), "python_execution_context", json!({})));
    let mut bridge = f.bridge(f.own.id);
    initialize(&mut bridge);
    let request = json!({"name":"artifact_store","arguments":{
        "op_id":Ulid::new(),"media_type":"text/plain","content":"Host did not observe this script."
    },"_meta":{"bboxy450/python_context":context["context"]}});
    let denied = after_nonadmission(|| {
        rpc(&mut bridge, 2, "tools/call", request.clone())["result"].clone()
    });
    assert_eq!(code(&denied), "execution_source_unavailable");
}

#[test]
fn artifact_writer_gate_child() {
    let Some(root) = std::env::var_os("BBOXY_PRIVATE_ARTIFACT_WRITER") else {
        return;
    };
    let root = PathBuf::from(root);
    let id = std::env::var("BBOXY_PRIVATE_ARTIFACT_ID").unwrap();
    let other = Fixture::open_repository(true, false, Some(root.join("repository")), |_, _| {});
    let result = other.mutate(
        other.own.id,
        Ulid::new(),
        "artifact_update",
        json!({"id":id,"expected_revision":1,"source_path":"not-admitted-until-lease.svg"}),
    );
    assert!(
        matches!(
            code(&result).as_str(),
            Some("provider_busy" | "workspace_busy")
        ),
        "{result}"
    );
    println!(
        "independent native owner refused before staging: {}",
        code(&result)
    );
}

#[test]
fn artifact_mcp_refuses_foreign_process_custody_before_reading_stage() {
    let first = Fixture::real();
    let one = data(write(
        &first,
        "artifact_store",
        json!({
            "media_type":"image/svg+xml","content":svg("#11f183")
        }),
    ));
    let root = first._temp.path();
    // Same directory claim used by the app publication owner. Hold it while
    // a second application owner with its own real supervisor attempts CAS.
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("repository/.bboxy/log/.writer-lock"))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("private root custody: {error}"),
        }
    }
    let home = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .args([
            "--exact",
            "mcp::artifacts::artifact_writer_gate_child",
            "--nocapture",
        ])
        .env("BBOXY_PRIVATE_ARTIFACT_WRITER", root)
        .env("BBOXY_PRIVATE_ARTIFACT_ID", one["id"].as_str().unwrap());
    let (status, stdout, stderr) = bounded_client(command, Duration::from_secs(20), &home);
    drop(lock);
    assert!(
        status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    assert!(
        String::from_utf8_lossy(&stdout)
            .contains("independent native owner refused before staging")
    );
    assert_eq!(read(&first, &one["id"], 1)["digest"], one["digest"]);
    std::fs::write(
        first.config.repository.join("not-admitted-until-lease.svg"),
        svg("#bb2484"),
    )
    .unwrap();
    let args =
        json!({"id":one["id"],"expected_revision":1,"source_path":"not-admitted-until-lease.svg"});
    let two = data(write(&first, "artifact_update", args));
    assert_eq!(two["revision"], 2);
    assert_ne!(two["digest"], one["digest"]);
}
