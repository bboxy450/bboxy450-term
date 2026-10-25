#![allow(
    clippy::disallowed_methods,
    reason = "subprocess integration tests are offline tools, not UI code"
)]

use std::process::Command;
use std::time::Duration;

use bboxy_firehose::options::{Options, Rate};
use bboxy_firehose::{SCENES, ordered_scenes, scene_bytes};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bboxy-firehose"))
        .args(args)
        .output()
        .unwrap()
}

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
#[test]
fn named_scene_and_bounded_loops_are_exact_bytes() {
    let expected = scene_bytes(&SCENES[0]);
    let output = run(&["--scene", "codex", "--rate", "max", "--loop", "3"]);
    assert!(output.status.success());
    assert_eq!(output.stdout, expected.repeat(3));
    assert!(output.stderr.is_empty());
}

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
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

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
#[test]
fn listing_and_checks_never_emit_terminal_control_bytes() {
    let list = run(&["--list"]);
    assert_eq!(
        String::from_utf8(list.stdout).unwrap().lines().count(),
        SCENES.len()
    );
    let check = run(&["--check", "--scene", "codex", "--seed", "42", "--loop"]);
    assert!(check.status.success());
    assert!(!check.stdout.contains(&27));
    let line = String::from_utf8(check.stdout).unwrap();
    assert!(line.starts_with("codex seed=42 80x24 fnv1a64="));
    assert_eq!(line.split("fnv1a64=").nth(1).unwrap().trim().len(), 16);
}

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
#[test]
fn invalid_inputs_are_rejected_by_the_option_parser() {
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
        assert!(Options::parse(args.into_iter().map(str::to_owned)).is_err());
    }
}

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
#[test]
fn a_cli_refusal_exits_two_without_emitting_terminal_bytes() {
    let output = run(&["--other"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
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

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
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
// keeps: spec 2026-10-22-t-terminal-fidelity §T1
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
    // First output proves the handler is installed; signal only this test child.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let mut rest = Vec::new();
    output.read_to_end(&mut rest).unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(130));
    assert!(rest.ends_with(b"\x1b[?2026l\x1b[?1049l\x1b[0m\x1b[r\x1b]8;;\x1b\\\x1b[?25h"));
}

// keeps: spec 2026-10-22-t-terminal-fidelity §T1
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
