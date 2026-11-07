#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Named negative controls for the hook-record and explicit writer-tick fixtures."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", choices=("writer", "hook"), required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    args.out.mkdir(parents=True, exist_ok=True)
    if args.case == "hook":
        path = root / "crates/bbt-mux/tests/plugin_host.rs"
        original = path.read_text()
        begin = original.index("fn read_hooks(")
        end = original.index("\n#[test]", begin)
        broken = original[:begin] + '''fn read_hooks(path: &Path) -> Vec<Value> {
    fs::read_to_string(path).unwrap_or_default().lines()
        .map(|line| serde_json::from_str(line).unwrap()).collect()
}
''' + original[end:]
        command = ["cargo", "test", "--locked", "-p", "bboxy450-term-mux", "--test", "it",
                   "plugin_host::a_hook_fragment_is_ignored_until_its_newline", "--", "--exact", "--nocapture"]
        assertion = "EOF while parsing"
    else:
        path = root / "crates/bbt-supervisor/src/ring/writer.rs"
        original = path.read_text()
        begin = original.index("fn one_writer_tick_orders_two_trickling_seats_without_full_flushes()")
        at = original.index("        tick(&writer);", begin) + len("        tick(&writer);")
        # An extra automatic tick can project an already-ordered idle tail.
        # Backdate the fixture observation; no sleep selects the interleaving.
        broken = original[:at] + '''
        {
            let mut q = writer.shared.queue.lock().unwrap();
            q.manual_ticks = false;
            for p in q.pressure.values_mut() {
                p.last_output = Some(std::time::Instant::now() - Duration::from_secs(1));
            }
        }
        tick(&writer);
        writer.shared.queue.lock().unwrap().manual_ticks = true;
''' + original[at:]
        command = ["cargo", "test", "--locked", "-p", "bboxy450-harness-host", "--lib",
                   "ring::writer::tests::one_writer_tick_orders_two_trickling_seats_without_full_flushes",
                   "--", "--exact", "--nocapture"]
        assertion = "one tick ordered each seat exactly once"
    before = hashlib.sha256(path.read_bytes()).hexdigest()
    results = []
    try:
        path.write_text(broken)
        red = args.out / (args.case + "-red.log")
        with red.open("w") as log:
            code = subprocess.run(command, cwd=root, stdout=log, stderr=subprocess.STDOUT, timeout=1800).returncode
        results.append(code)
        assert code == 101 and assertion in red.read_text(), (code, red)
    finally:
        path.write_text(original)
        assert hashlib.sha256(path.read_bytes()).hexdigest() == before
    green = args.out / (args.case + "-green.log")
    with green.open("w") as log:
        code = subprocess.run(command, cwd=root, stdout=log, stderr=subprocess.STDOUT, timeout=1800).returncode
    results.append(code)
    assert code == 0, (code, green)
    summary = {"case": args.case, "command": command, "assertion": assertion,
               "red_exit": results[0], "green_exit": results[1], "source_sha256": before,
               "logs": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (red, green)}}
    (args.out / (args.case + "-control.json")).write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
