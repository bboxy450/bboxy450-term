#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Provision the pinned exporter before publishing its interpreter path."""
import hashlib
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile


def install(destination):
    lock = Path(__file__).with_name("requirements.lock")
    destination.parent.mkdir(parents=True, exist_ok=True)
    version = destination.with_name(
        f"{destination.name}-{hashlib.sha256(lock.read_bytes()).hexdigest()}"
    )
    with tempfile.TemporaryDirectory(prefix=".chart-install-", dir=destination.parent) as staging:
        staging = Path(staging)
        if not (version / "bin/python").is_file():
            uv = os.environ.get("BBOXY_UV", "uv")
            venv = staging / "venv"
            python = venv / "bin/python"
            subprocess.run(
                [uv, "venv", "--python", "3.12", "--relocatable", str(venv)], check=True
            )
            subprocess.run(
                [uv, "pip", "sync", "--python", str(python), "--require-hashes",
                 "--only-binary", ":all:", str(lock)], check=True
            )
            subprocess.run(
                [str(python), "-I", "-c", "import altair, fontTools, matplotlib, vl_convert"],
                check=True,
            )
            try:
                venv.rename(version)
            except OSError:
                # A concurrent install can complete the same lock revision first.
                if not (version / "bin/python").is_file():
                    raise
        link = staging / "current"
        link.symlink_to(version, target_is_directory=True)
        os.replace(link, destination)


if __name__ == "__main__":
    if sys.platform != "darwin" or platform.machine() != "arm64":
        print("chart export unavailable: macOS arm64 required", file=sys.stderr)
        sys.exit(0 if "--optional" in sys.argv[2:] else 1)
    install(Path(sys.argv[1]).absolute())
