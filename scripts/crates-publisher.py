#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Publish only with the operator's approved identity bound to this exact token.

crates.io /api/v1/me rejects API-token authentication (cookie-only). The local
receipt must come from an actual operator confirmation, never an agent's guess.
It contains a token fingerprint, never the token, and stays in ignored target/.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib


def verify_identity(token, receipt):
    if not token or receipt.get("publisher") != "bboxy450" or receipt.get("source") != "operator-confirmed":
        raise ValueError("approved crates.io publisher identity is not confirmed")
    if receipt.get("token_sha256") != hashlib.sha256(token.encode()).hexdigest():
        raise ValueError("Cargo token changed; the publisher identity binding is stale")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    args = parser.parse_args()
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    credentials = cargo_home / "credentials.toml"
    token = os.environ.get("CARGO_REGISTRY_TOKEN")
    if not token and credentials.is_file():
        token = tomllib.loads(credentials.read_text()).get("registry", {}).get("token")
    verify_identity(token, json.loads(args.receipt.read_text()))
    # stdin avoids leaking the token in process arguments or tool logs.
    login = subprocess.run(["cargo", "login"], input=token + '\n', text=True, capture_output=True, timeout=30)
    if login.returncode:
        raise ValueError(f"cargo login failed (exit {login.returncode}); credential output withheld")
    print("crates publisher: bboxy450; exact-token operator binding verified", flush=True)
    subprocess.run(["cargo", "publish", "--locked", "-p", "bboxy450-tlc", "--target-dir", str(args.target)], check=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"crates publication refused: {error}", file=sys.stderr)
        sys.exit(1)
