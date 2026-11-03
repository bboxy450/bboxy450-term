// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real private PTY/socket/log witness for ICD #348 phase 1c.
use super::*;

#[test]
#[ignore = "real private seat and 60-second timeout; make test-harness-native"]
fn mcp_harness_seat_phase_1c_reports_and_staleness() {
    isolated_seat_fixture("harness-status-phase");
}

fn read(f: &Fixture, seat: Ulid) -> Value {
    data(f.call(
        f.oracle.id,
        Ulid::new(),
        "seat_get",
        json!({"seat_id":seat}),
    ))
}

fn command(
    f: &Fixture,
    seat: Ulid,
    text: &str,
    state: &str,
    source: &str,
    detail: Option<&str>,
) -> Value {
    let sent = Instant::now();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(PathBuf::from(std::env::var_os("HOME").unwrap()).join("status-command"))
        .unwrap()
        .write_all(format!("{text}\r").as_bytes())
        .unwrap();
    loop {
        let status = read(f, seat);
        let adapter = &status["adapter"];
        if adapter["state"] == state
            && adapter["source"] == source
            && adapter["detail"].as_str() == detail
        {
            assert_eq!(adapter["grade"], "reported", "{status}");
            assert!(
                sent.elapsed() < Duration::from_millis(2500),
                "late report: {status}"
            );
            return status;
        }
        assert!(
            sent.elapsed() < Duration::from_millis(2500),
            "{text} did not report: {status}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn run() {
    use std::os::unix::fs::PermissionsExt;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let bin = home.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let shim = bin.join("bboxy450-harness");
    std::fs::write(&shim, include_str!("../fixtures/harness-status.py")).unwrap();
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(
        home.join(".zprofile"),
        format!("export PATH='{}':$PATH\n", bin.display()),
    )
    .unwrap();
    std::fs::write(home.join("status-source"), "osc").unwrap();
    let f = Fixture::real();
    let spawned = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "seat_spawn",
        json!({
            "swarm_id":f.own.swarm,"approved_profile":"bboxy450-harness","cwd":f._temp.path()
        }),
    ));
    let seat: Ulid = serde_json::from_value(spawned["seat_id"].clone()).unwrap();
    let pid = spawned["pid"].as_i64().unwrap() as i32;
    // SAFETY: this is the exact process created and recorded by this fixture.
    let pgid = unsafe { libc::getpgid(pid) };
    assert!(pgid > 0);
    let _groups = OwnedHarnessGroups(vec![pgid]);
    f.wait_screen(seat, "session ");
    let path = home.join(format!(".bboxy450-harness/sessions/{seat}.jsonl"));
    for source in ["7501", "sessions_file"] {
        std::fs::write(
            home.join("status-source"),
            if source == "7501" { "osc" } else { "log" },
        )
        .unwrap();
        command(&f, seat, "start", "active", source, Some("bash"));
        let blocked = command(&f, seat, "approval", "waiting", source, Some("Allow bash"));
        assert_eq!(blocked["adapter"]["waiting_kind"], "permission");
        command(&f, seat, "resume", "active", source, Some("bash"));
        command(&f, seat, "done", "idle", source, None);
        command(&f, seat, "start", "active", source, Some("bash"));
        command(
            &f,
            seat,
            "error",
            "errored",
            source,
            Some("scripted failure"),
        );
        command(&f, seat, "closed", "idle", source, None);
        if source == "7501" {
            assert_eq!(
                std::fs::metadata(&path).unwrap().len(),
                0,
                "OSC-only proof has no log reports"
            );
        }
    }
    let started = command(&f, seat, "start", "active", "sessions_file", Some("bash"));
    let observed_ms = started["adapter"]["observed_ms"].as_u64().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let last: Value = serde_json::from_slice(
        bytes
            .split(|b| *b == b'\n')
            .rfind(|line| !line.is_empty())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(observed_ms, last["ts_ms"].as_u64().unwrap());
    let elapsed = Instant::now();
    let mut samples = 0;
    loop {
        let status = read(&f, seat);
        let adapter = &status["adapter"];
        assert_eq!(adapter["state"], "active", "quiet state changed: {status}");
        assert_eq!(adapter["grade"], "reported", "{status}");
        assert_eq!(
            adapter["observed_ms"], observed_ms,
            "EOF/Hello must not freshen status"
        );
        assert_eq!(adapter["turn_open"], true);
        if adapter["source"] == "stale" {
            assert!(
                elapsed.elapsed() >= Duration::from_millis(59_500),
                "early expiry: {status}"
            );
            assert_eq!(adapter["detail"], "bash · no word for 1m");
            let gui = f.runtime.block_on(f.client().gui_seat(seat)).unwrap();
            assert!(
                gui.state.stale,
                "the GUI must dim the retained activity rim"
            );
            assert_eq!(gui.state.activity, bbt_proto::Activity::Active);
            assert_eq!(gui.state.grade, bbt_proto::Grade::Reported);
            break;
        }
        assert_eq!(adapter["source"], "sessions_file", "{status}");
        assert!(
            elapsed.elapsed() < Duration::from_millis(62_500),
            "missing expiry: {status}"
        );
        samples += 1;
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(
        samples >= 250,
        "insufficient native silence samples: {samples}"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "term never writes the log"
    );
    command(&f, seat, "done", "idle", "sessions_file", None);
    assert!(
        !f.runtime
            .block_on(f.client().gui_seat(seat))
            .unwrap()
            .state
            .stale
    );
    eprintln!(
        "harness-native: phase_1c=osc-only,log-only,approval,error,closed stale_ms={} samples={samples}",
        elapsed.elapsed().as_millis()
    );
}
