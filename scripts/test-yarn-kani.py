#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Inspired by Kani's custom KANI_HOME installation and autoharness interface:
# https://model-checking.github.io/kani/install-guide.html
"""Install pinned, local Kani; require good proofs and the exact broken assertion."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PROOFS = ("initial_invariant", "step_never_panics", "invariant_is_inductive", "refusal_preserves_input",
          "broken_invariant_is_inductive")


def run(argv, env, *, check=True):
    print("yarn-kani: " + " ".join(map(str, argv)), flush=True)
    result = subprocess.run(list(map(str, argv)), cwd=ROOT, env=env, check=False,
                            stdin=subprocess.DEVNULL, timeout=1800)
    print(f"yarn-kani: exit {result.returncode}", flush=True)
    if check and result.returncode:
        raise RuntimeError(f"command failed with exit {result.returncode}: {argv[0]}")
    return result.returncode


def validate(path, name, code):
    """A failed subprocess alone never proves the broken control was detected."""
    data = json.loads(path.read_text())
    verification = data["verification_results"]
    summary = verification["summary"]
    results = verification["results"]
    if (summary["status"] != "completed" or summary["total_harnesses"] != 1
            or summary["executed"] != 1 or len(results) != 1
            or results[0]["harness_id"].split("::")[-1] != name):
        raise ValueError(f"missing, skipped or incomplete harness: {name}")
    checks = results[0]["checks"]
    if not checks or any(c["status"] not in ("Success", "Failure", "Unreachable")
                         for c in checks):
        raise ValueError(f"incomplete verification: {name}")
    failures = [c for c in checks if c["status"] == "Failure"]
    broken = name == PROOFS[-1]
    if broken:
        if (code != 1 or summary["failed"] != 1 or summary["successful"] != 0
                or results[0]["status"] != "Failure" or len(failures) != 1
                or failures[0]["category"] != "assertion"
                or "RESULT-001 broken inductive step" not in failures[0]["description"]):
            raise ValueError("broken control did not fail its freshness assertion alone")
    elif (code or failures or results[0]["status"] != "Success"
          or summary["successful"] != 1 or summary["failed"] != 0):
        raise ValueError(f"proof failed: {name}")
    print(f"yarn-kani: {name}: {'expected assertion failure' if broken else 'proved'}",
          flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, required=True)
    args = parser.parse_args()
    target = args.target.resolve()
    target.relative_to(ROOT)  # Toolchains stay in this repository, even with overrides.
    pins = json.loads((ROOT / "verification/kani-tools.lock.json").read_text())
    version = pins["version"]
    arch = {"arm64": "aarch64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())
    system = {"Darwin": "apple-darwin", "Linux": "unknown-linux-gnu"}.get(platform.system())
    host = f"{arch}-{system}"
    if host not in pins["bundles"]:
        raise ValueError(f"Kani {version}: no pinned bundle for {host}")
    tools = target / "tools" / f"yarn-kani-{version}"
    tools.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_BUILD_JOBS="2", RUSTC_WRAPPER="",
               RUSTC_WORKSPACE_WRAPPER="", CARGO_BUILD_BUILD_DIR=str(tools / "build"))
    launcher = tools / "launcher" / "bin" / "cargo-kani"
    if not launcher.exists():
        run(["cargo", "install", "--locked", "--version", f"={version}", "kani-verifier",
             "--root", tools / "launcher", "--target-dir", tools / "installer-build"], env)
    filename = f"kani-{version}-{host}.tar.gz"
    bundle = tools / filename
    if not bundle.exists():
        staged = bundle.with_suffix(".download")
        run(["curl", "-fSL", "--retry", "2", "--max-time", "300", "-o", staged,
             f"https://github.com/model-checking/kani/releases/download/kani-{version}/{filename}"], env)
        staged.replace(bundle)
    if hashlib.sha256(bundle.read_bytes()).hexdigest() != pins["bundles"][host]:
        raise ValueError(f"Kani {version}: bundle checksum mismatch: {bundle}")
    env.update(KANI_HOME=str(tools / "home"), RUSTUP_HOME=str(tools / "rustup"),
               RUSTUP_PROFILE="minimal", RUSTUP_NO_UPDATE_CHECK="1")
    env.pop("RUSTUP_TOOLCHAIN", None)
    ready = tools / "setup-complete"
    if not ready.exists():
        # Kani invokes rustup without --no-self-update. Disable it in this local
        # RUSTUP_HOME so setup cannot attempt to replace the machine's rustup.
        run(["rustup", "set", "auto-self-update", "disable"], env)
        run(["rustup", "set", "profile", "minimal"], env)
        run([launcher, "setup", "--use-local-bundle", bundle], env)
        ready.write_text(version + "\n")
    run([launcher, "--version"], env)
    output = target / "yarn-kani"
    output.mkdir(parents=True, exist_ok=True)
    # Kani 0.68 selects integration targets with --tests (no --test NAME).
    # Keep default features: existing unrelated test targets require Tokio.
    common = ["--package", "bboxy450-yarn", "--tests",
              "--features", "model", "--target-dir", tools / "build",
              "--default-unwind", "4", "--output-format", "regular", "-j", "1"]
    for name in PROOFS:
        result = output / f"{name}.json"
        result.unlink(missing_ok=True)  # Never accept evidence from a previous run.
        command = [launcher, "autoharness", "-Z", "autoharness", *common,
                   "--include-pattern", f"^kani::proofs::{name}$"]
        code = run([*command,
                    "-Z", "unstable-options", "--export-json", result], env,
                   check=False)
        validate(result, name, code)
    print(f"yarn-kani: Kani {version}: four proofs and freshness-broken control checked", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"yarn-kani: FAILED: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
