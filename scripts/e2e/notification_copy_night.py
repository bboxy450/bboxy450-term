"""Canonical private app/native request proof with production-field mutations.

Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from settings_night import run

PACKAGE = "bboxy450-desktop-notifications-macos"
CASE = "tests::native_copy_fields_cross_actual_request_submission"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    target = args.target_dir.resolve()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    source = root / "crates/bboxy450-desktop-notifications-macos/src/lib.rs"
    original = source.read_bytes()
    env = dict(os.environ, CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0")
    calls = []

    def cargo(label, *extra):
        command = ["cargo", "test", "--locked", "-p", PACKAGE, "--target-dir", str(target), "--lib", *extra]
        result = subprocess.run(command, env=env, cwd=root, stdin=subprocess.DEVNULL,
                                capture_output=True, text=True, timeout=180)
        (out / (label + ".log")).write_text(result.stdout + result.stderr)
        calls.append({"label": label, "command": command, "exit_code": result.returncode})
        return result

    def build(label):
        result = cargo(label, "--no-run", "--message-format=json")
        assert result.returncode == 0, result.stderr[-2000:]
        artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
        binaries = [Path(row["executable"]) for row in artifacts
                    if row.get("reason") == "compiler-artifact" and row.get("executable")
                    and row.get("profile", {}).get("test") and row["target"]["name"] == PACKAGE.replace("-", "_")]
        assert len(binaries) == 1, binaries
        return binaries[0]

    def interrupted(signum, _frame):
        raise RuntimeError("private mutation fixture interrupted by signal " + str(signum))

    previous = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGTERM, signal.SIGINT)}
    mutations = [
        ("title", 'content.setTitle(&NSString::from_str(title));',
         'content.setTitle(&NSString::from_str(&format!("{title} wrong")));'),
        ("body", 'content.setBody(&NSString::from_str(body));',
         'content.setBody(&NSString::from_str(&format!("header: {body}")));'),
        ("identifier", '&NSString::from_str(id),', '&NSString::from_str(&format!("{id}-wrong")),')]
    try:
        baseline = cargo("native-baseline", CASE, "--", "--exact", "--nocapture")
        assert baseline.returncode == 0 and "1 passed" in baseline.stdout, baseline.stdout[-2000:] + baseline.stderr[-2000:]
        for name, before, after in mutations:
            text = original.decode()
            assert text.count(before) == 1, (name, "production assignment is not unique")
            source.write_text(text.replace(before, after))
            result = cargo("mutation-" + name, CASE, "--", "--exact", "--nocapture")
            assert result.returncode != 0 and "running 1 test" in result.stdout and "FAILED" in result.stdout, result.stdout[-2000:] + result.stderr[-2000:]
            # A compiler error cannot stand in for a failing native-field assertion.
            assert "assertion `left == right` failed" in result.stderr, result.stdout[-1000:] + result.stderr[-1500:]
            source.write_bytes(original)
    finally:
        source.write_bytes(original)
        for sig, handler in previous.items():
            signal.signal(sig, handler)
        (out / "mutation-commands.json").write_text(json.dumps(calls, indent=2) + "\n")
    assert sha(source) == hashlib.sha256(original).hexdigest(), "production source was not restored"
    native_test = build("native-restored-build")
    frozen = out / "native-copy-test"
    shutil.copy2(native_test, frozen)
    positive = subprocess.run([str(frozen), "--exact", CASE, "--nocapture"], cwd=root,
                              stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=30)
    (out / "native-restored-positive.log").write_text(positive.stdout + positive.stderr)
    assert positive.returncode == 0, positive.stdout + positive.stderr
    verdict = run(args.binary.resolve(strict=True), Path(__file__).with_name("notification_copy.sh"), out,
                  native_copy_test=frozen)
    scripts = Path(__file__).resolve().parent
    inputs = [scripts / name for name in (
        "notification_copy.sh", "notification_copy.py", "notification_copy_night.py",
        "settings.py", "settings_night.py", "ui.py", "rim_more.py", "rim_clock.py", "lib.sh")]
    result = {"source_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
              "native_source_sha256": sha(source), "binary_sha256": sha(args.binary),
              "native_test_sha256": sha(frozen), "app": verdict,
              "inputs": {str(path.relative_to(root)): sha(path) for path in inputs},
              "mutations": calls, "restored_positive_exit_code": positive.returncode,
              "pass": verdict["status"] == "pass" and verdict["exit_code"] == 0}
    (out / "verification.json").write_text(json.dumps(result, indent=2) + "\n")
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
