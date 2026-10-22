#![allow(
    clippy::disallowed_methods,
    reason = "subprocess integration tests are offline tools, not UI code"
)]

use std::process::Command;
use std::time::{Duration, Instant};

use bboxy_firehose::options::Rate;
use bboxy_firehose::{SCENES, ordered_scenes, scene_bytes};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bboxy-firehose"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn named_scene_and_bounded_loops_are_exact_bytes() {
    let expected = scene_bytes(&SCENES[0]);
    let output = run(&["--scene", "codex", "--rate", "max", "--loop", "3"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, expected.repeat(3));
    assert!(output.stderr.is_empty());
}

#[test]
fn seed_is_reproducible_and_reorders_the_complete_corpus() {
    let first = run(&["--seed", "42"]);
    let again = run(&["--seed", "42"]);
    let other = run(&["--seed", "43"]);
    assert!(first.status.success());
    assert_eq!(first.stdout, again.stdout);
    assert_ne!(first.stdout, other.stdout);
    let expected: Vec<_> = ordered_scenes(None, 42)
        .into_iter()
        .flat_map(scene_bytes)
        .collect();
    assert_eq!(first.stdout, expected);
}

#[test]
fn listing_and_checks_never_emit_terminal_control_bytes() {
    let list = run(&["--list"]);
    assert_eq!(String::from_utf8(list.stdout).unwrap().lines().count(), 10);
    let check = run(&["--check", "--scene", "codex", "--seed", "42", "--loop"]);
    assert!(check.status.success());
    assert!(!check.stdout.contains(&27));
    let line = String::from_utf8(check.stdout).unwrap();
    assert!(line.starts_with("codex seed=42 80x24 fnv1a64="));
    assert_eq!(line.split("fnv1a64=").nth(1).unwrap().trim().len(), 16);
}

#[test]
fn invalid_inputs_fail_before_emitting_ansi() {
    for args in [
        vec!["--scene", "missing"],
        vec!["--rate", "0"],
        vec!["--pace", "fast"],
        vec!["--loop", "0"],
        vec!["--seed", "-1"],
        vec!["--rate"],
        vec!["--other"],
        vec!["--rate", "18446744073709551615MiB/s"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn byte_and_line_rate_caps_keep_the_exact_stream() {
    let expected = scene_bytes(&SCENES[9]);
    for (rate, units) in [
        (format!("{}B/s", expected.len() * 4), expected.len()),
        (
            format!(
                "{}lines/s",
                expected.iter().filter(|b| **b == b'\n').count() * 4
            ),
            expected.iter().filter(|b| **b == b'\n').count(),
        ),
    ] {
        assert!(units > 0);
        let started = Instant::now();
        let output = run(&["--scene", "osc7501-codex", "--rate", &rate]);
        assert!(output.status.success());
        assert_eq!(output.stdout, expected);
        assert!(started.elapsed() >= Duration::from_millis(240));
    }
}

#[test]
fn real_pacing_preserves_scene_bytes() {
    let output = run(&["--scene", "alternate", "--pace", "real", "--seed", "42"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, scene_bytes(&SCENES[5]));
}

#[test]
fn rates_have_explicit_units_and_cumulative_deadlines() {
    assert_eq!(Rate::parse("2MiB/s").unwrap(), Rate::Bytes(2 * 1_048_576));
    assert_eq!(Rate::parse("12l/s").unwrap(), Rate::Lines(12));
    assert_eq!(Rate::parse("99").unwrap(), Rate::Bytes(99));
    assert_eq!(
        Rate::Bytes(10).elapsed_for(15, 0),
        Duration::from_millis(1500)
    );
    assert_eq!(
        Rate::Lines(4).elapsed_for(1_000_000, 3),
        Duration::from_millis(750)
    );
}

#[test]
fn playback_needs_no_corpus_files_in_its_working_directory() {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy-firehose"))
        .args(["--scene", "codex"])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, scene_bytes(&SCENES[0]));
}

#[cfg(unix)]
#[test]
fn interrupting_slow_playback_restores_modes_without_waiting_for_the_rate() {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy-firehose"))
        .args(["--scene", "alternate", "--rate", "1B/s", "--loop"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = child.stdout.take().unwrap();
    let mut first = [0];
    output.read_exact(&mut first).unwrap();
    let started = Instant::now();
    // First output proves the handler is installed; signal only this test child.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let mut rest = Vec::new();
    output.read_to_end(&mut rest).unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(130));
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(rest.ends_with(b"\x1b[?2026l\x1b[?1049l\x1b[0m\x1b[r\x1b]8;;\x1b\\\x1b[?25h"));
}

#[test]
fn a_closed_downstream_pipe_stops_an_infinite_flood_cleanly() {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_bboxy-firehose"))
        .args(["--loop"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = child.stdout.take().unwrap();
    let mut first = [0; 64];
    output.read_exact(&mut first).unwrap();
    drop(output);
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert!(result.stderr.is_empty());
}
