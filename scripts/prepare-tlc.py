#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Export only the independently releasable checker and canonical verification inputs."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates/bboxy450-tlc"


def run(*args, cwd):
    subprocess.run(args, cwd=cwd, check=True, timeout=60)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, required=True)
    args = parser.parse_args()
    target = args.target.resolve()
    target.mkdir(parents=True, exist_ok=True)
    mirror = Path(tempfile.mkdtemp(prefix="tlc-public-", dir=target))
    inputs = {}
    for name in ["src", "tests", "examples", "README.md", "LICENSE", "NOTICE", "ATTRIBUTION.md", "LICENSE-MIT", "LICENSE-APACHE"]:
        path = CRATE / name
        if path.is_dir():
            shutil.copytree(path, mirror / name)
        else:
            shutil.copyfile(path, mirror / name)
    package = tomllib.loads((CRATE / "Cargo.toml").read_text())["package"]
    inherited = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]
    lines = ["[package]"]
    for key, value in package.items():
        if isinstance(value, dict) and value.get("workspace"):
            value = inherited[key]
        lines.append(f"{key} = {json.dumps(value)}")
    (mirror / "Cargo.toml").write_text('\n'.join(lines) + '\n\n[workspace]\n')
    (mirror / "scripts").mkdir()
    for name in ["formal.py", "test-formal.py", "crates-publisher.py"]:
        shutil.copyfile(ROOT / "scripts" / name, mirror / "scripts" / name)
    (mirror / "verification").mkdir()
    shutil.copyfile(ROOT / "verification/tools.lock.json", mirror / "verification/tools.lock.json")
    catalog = tomllib.loads((ROOT / "verification/corpus.toml").read_text())["model"]
    catalog = [row for row in catalog if not row["name"].startswith("Yarn")]
    (mirror / "verification/corpus.toml").write_text('\n'.join(
        '[[model]]\nname = '+json.dumps(row["name"])+ '\nexpected = '+json.dumps(row["expected"])+ '\n' for row in catalog))
    (mirror / "verification/coverage.toml").write_text('''[[crate]]
name = "bboxy450-tlc"
status = "contract-model"
models = ["Counter", "FairResponse", "WeakFairFlicker", "StutteringIsNotFairProgress", "SimultaneousUpdates"]
scope = "Finite native checker/expression exporter conformance; no complete TLA+ parser or arbitrary implementation proof."
sources = ["src/lib.rs", "examples/verify.rs"]
''')
    (mirror / "Makefile").write_text('''# Generated canonical standalone interface by bboxy450-term/scripts/prepare-tlc.py.
# Policy: https://github.com/bboxy450/bboxy450-tlc/blob/main/ATTRIBUTION.md
SHELL := /bin/sh
TARGET_DIR ?= target
export CARGO_BUILD_JOBS := 2
.PHONY: test test-formal
test:
	cargo test --locked --target-dir '$(TARGET_DIR)'
	cargo clippy --locked --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings
	$(MAKE) test-formal TARGET_DIR='$(TARGET_DIR)'
test-formal:
	python3 -I scripts/test-formal.py
	cargo run --locked --example verify --target-dir '$(TARGET_DIR)' -- '$(abspath $(TARGET_DIR))/formal/models'
	python3 -I scripts/formal.py --target '$(abspath $(TARGET_DIR))/formal'
''')
    (mirror / "AGENTS.md").write_text('''# bboxy450-tlc

Use `make test`; TLA+/TLC differential verification is mandatory for every
change. Native/reference agreement covers only the documented subset and
corpus. Keep exact state and action-labeled transition comparison, positive and
broken controls, explicit fairness/bounds and implementation tests. Java TLC
remains the reference until separately recorded conformance evidence authorizes
retirement. No test requires an operator or global runtime installation.

Read README.md, LICENSE, NOTICE and ATTRIBUTION.md before changes/reuse.
Preserve accurate credit and upstream terms. Document new semantics and unsupported
features honestly. Signed commits/tags and detached artifact signatures preserve
provenance. Never commit credentials, downloaded runtimes or raw private logs.
''')
    (mirror / ".gitignore").write_text('target/\n__pycache__/\n')
    workflows = mirror / ".github/workflows"
    workflows.mkdir(parents=True)
    (workflows / "ci.yml").write_text('''name: Native and TLC conformance
on: [push, pull_request, workflow_dispatch]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: make test
''')
    provenance = {"source": "https://github.com/bboxy450/bboxy450-term",
                  "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                  "publication": "https://github.com/bboxy450/bboxy450-tlc"}
    for path in sorted(mirror.rglob('*')):
        if path.is_file():
            inputs[str(path.relative_to(mirror))] = hashlib.sha256(path.read_bytes()).hexdigest()
    provenance["export_sha256"] = inputs
    (mirror / "SOURCE.json").write_text(json.dumps(provenance, indent=2) + '\n')
    # A generated mirror gets its own signed source commit before its own gates.
    run("git", "init", "-q", "-b", "main", cwd=mirror)
    run("cargo", "generate-lockfile", "--offline", cwd=mirror)
    paths = [str(p.relative_to(mirror)) for p in mirror.rglob('*') if p.is_file() and '.git' not in p.parts]
    run("git", "add", "--", *paths, cwd=mirror)
    run("git", "commit", "-S", "-m", "Release bboxy450-tlc with native and reference conformance", cwd=mirror)
    (target / "tlc-public.path").write_text(str(mirror) + '\n')
    print(f"TLC standalone source: {mirror}")


if __name__ == "__main__":
    main()
