# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Install and identify the private SVG reference browser."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile

ADVISORIES = Path(__file__).resolve().parent / "fixtures/advisories.json"
ARCHIVE_SHA256 = "45925b7ea18e09bf0d5acdae4e36b57cb7ed08ada8dfc0e0c6422f86a3384c9d"
PLATFORM = "mac-arm64"
DISTRIBUTION = f"chrome-{PLATFORM}"
EXECUTABLE = Path("Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing")


def frozen_version():
    return json.loads(ADVISORIES.read_text())["chrome"]


def require_version(actual, expected):
    if actual != expected:
        raise ValueError(f"SVG reference browser version mismatch: frozen {expected}, observed {actual}")


def install_dir(out, version):
    return out / f"chrome-{version.removeprefix('Chrome/')}-{PLATFORM}"


def capture_browser(out):
    override = os.environ.get("CHROME")
    version = frozen_version()
    binary = Path(override).resolve() if override else install_dir(out, version) / EXECUTABLE
    if not binary.is_file():
        raise FileNotFoundError(f"SVG reference browser missing at {binary}; run make svg-reference")
    observed = subprocess.check_output([binary, "--version"], text=True, timeout=10).strip().split()
    require_version("Chrome/" + observed[-1] if observed else "(no version)", version)
    return {"binary": str(binary), "override": bool(override), "version": version,
            "archive_sha256": None if override else ARCHIVE_SHA256}


def install(out):
    if os.environ.get("CHROME"):
        return
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise SystemExit("SVG reference browser requires macOS arm64")
    version = frozen_version()
    destination = install_dir(out, version)
    binary = destination / EXECUTABLE
    if binary.is_file():
        return
    out.mkdir(parents=True, exist_ok=True)
    url = ("https://storage.googleapis.com/chrome-for-testing-public/"
           f"{version.removeprefix('Chrome/')}/{PLATFORM}/{DISTRIBUTION}.zip")
    with tempfile.TemporaryDirectory(prefix="chrome-install-", dir=out) as tmp:
        staging = Path(tmp)
        archive = staging / f"{DISTRIBUTION}.zip"
        subprocess.run(["curl", "--fail", "--silent", "--show-error", "--location", "--max-time", "180",
                        "--output", archive, url], check=True)
        with archive.open("rb") as stream:
            actual = hashlib.file_digest(stream, "sha256").hexdigest()
        if actual != ARCHIVE_SHA256:
            raise ValueError(f"Chrome for Testing archive SHA-256 mismatch: expected {ARCHIVE_SHA256}, observed {actual}")
        subprocess.run(["ditto", "-x", "-k", archive, staging], check=True)
        extracted = staging / DISTRIBUTION
        if not (extracted / EXECUTABLE).is_file():
            raise FileNotFoundError(f"Chrome for Testing archive is missing {EXECUTABLE}")
        try:
            extracted.rename(destination)
        except OSError:
            if not binary.is_file():
                raise


if __name__ == "__main__":
    install(Path(sys.argv[1]).resolve())
