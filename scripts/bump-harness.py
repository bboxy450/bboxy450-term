#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Move and stage the harness gitlink plus any resulting workspace lock change."""
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
MEMBER = "crates/bboxy450-harness"
URL = "https://github.com/bboxy450/bboxy450-harness"


def main():
    revision = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else "main"
    if revision != "main" and not re.fullmatch(r"[0-9a-fA-F]{7,40}", revision):
        raise ValueError("REV must be a commit SHA; omit it to use harness origin/main")
    env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GCM_INTERACTIVE": "never"}
    child = ROOT / MEMBER
    subprocess.run(["git", "submodule", "update", "--init", "--recursive", "--", MEMBER], cwd=ROOT, env=env, check=True, timeout=180)
    if subprocess.check_output(["git", "remote", "get-url", "origin"], cwd=child, text=True).strip() != URL:
        raise ValueError("harness origin is not the approved repository")
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=child).strip():
        raise ValueError("refuse to move a dirty harness checkout")
    previous = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=child, text=True).strip()
    lock = ROOT / "Cargo.lock"
    original_lock = lock.read_bytes()
    subprocess.run(["git", "fetch", "--no-tags", "origin", revision], cwd=child, env=env, check=True, timeout=180)
    sha = subprocess.check_output(["git", "rev-parse", "FETCH_HEAD^{commit}"], cwd=child, text=True).strip()
    try:
        subprocess.run(["git", "checkout", "--detach", sha], cwd=child, check=True)
        subprocess.run(["cargo", "metadata", "--format-version", "1"], cwd=ROOT, env=env,
                       stdout=subprocess.DEVNULL, check=True)
        paths = [MEMBER]
        if subprocess.run(["git", "diff", "--quiet", "--", "Cargo.lock"], cwd=ROOT).returncode:
            paths.append("Cargo.lock")
        subprocess.run(["git", "add", "--", *paths], cwd=ROOT, check=True)
    except (OSError, subprocess.SubprocessError):
        subprocess.run(["git", "checkout", "--detach", previous], cwd=child, check=True)
        lock.write_bytes(original_lock)
        raise
    print(f"harness: staged {sha}" + (" and Cargo.lock" if "Cargo.lock" in paths else ""), flush=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        raise SystemExit(f"bump-harness: {error}")
