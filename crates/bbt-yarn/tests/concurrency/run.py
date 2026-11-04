#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Inspired by Loom cfg shims and Shuttle schedule replay, independently implemented.
# https://github.com/tokio-rs/loom https://github.com/awslabs/shuttle
"""Unattended real-adapter checks; a mutant must fail its named assertion."""
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    target = Path(sys.argv[1]).resolve() / "yarn-concurrency"
    target.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    base = os.environ.copy()
    # Bounds and seeds are source-controlled, never silently truncated by a shell.
    for key in list(base):
        if key.startswith(("LOOM_", "SHUTTLE_", "YARN_BROKEN", "YARN_SHUTTLE_REPLAY")) or key == "CARGO_ENCODED_RUSTFLAGS":
            del base[key]
    base["BBOXY_TEST_RUNNER"] = "nextest"
    base["RUST_TEST_THREADS"] = "1"
    base["CARGO_BUILD_BUILD_DIR"] = str(target)

    def run(name, backend, mutant=None, assertion=None, test="concurrency::"):
        env = base.copy()
        env["RUSTFLAGS"] = f"--cfg {backend}" + (" --cfg yarn_broken_controls" if backend == "loom" else "")
        env["BBOXY_CARGO_TARGET_DIR"] = str(target / backend)
        if mutant:
            env["YARN_BROKEN"] = mutant
        if not mutant:
            lint = ["cargo", "clippy", "--locked", "-p", "bboxy450-yarn", "--no-default-features", "--features", "macros", "--lib", "--profile", "test", "--target-dir", env["BBOXY_CARGO_TARGET_DIR"], "--", "-D", "warnings"]
            result = subprocess.run(lint, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            (target / f"{name}-clippy.log").write_text(result.stdout)
            print(result.stdout, end="", flush=True)
            if result.returncode:
                raise RuntimeError(f"{name} strict Clippy: exit {result.returncode}")
        command = ["sh", "scripts/quarantine-test.sh", "--locked", "-p", "bboxy450-yarn", "--no-default-features", "--features", "macros", "--lib", "-E", f"test({test})", "--no-capture", "--test-threads", "1"]
        print(f"yarn-concurrency: {name}", flush=True)
        result = subprocess.run(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (target / f"{name}.log").write_text(result.stdout)
        print(result.stdout, end="", flush=True)
        if assertion:
            # nextest: 100 means executed tests failed; compilation/no-tests are
            # different failures and cannot validate a negative control.
            if result.returncode != 100 or assertion not in result.stdout:
                raise RuntimeError(f"{name}: expected test exit 100 with {assertion}, got {result.returncode}")
        elif result.returncode:
            raise RuntimeError(f"{name}: exit {result.returncode}")
        print(f"yarn-concurrency: {name} exit={result.returncode}" + (" (expected assertion failure)" if assertion else ""), flush=True)

    run("loom", "loom")
    for mutant, test, assertion in [
        ("wake", "lost_wake", "WAKE-001 lost wake"),
        ("close", "close_racing_ready", "CLOSE-001 receiver not retired"),
        ("service", "double_service", "SERVICE-001 duplicate delivery"),
        ("waker", "waker_replacement", "WAKER-001 stale waker"),
    ]:
        run(f"broken-{mutant}", "loom", mutant, assertion, test)
    run("shuttle", "shuttle")
    print(f"yarn-concurrency: complete, wall_seconds={time.monotonic() - started:.3f}", flush=True)


if __name__ == "__main__":
    main()
