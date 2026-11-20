# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Prepare one private resvg oracle; do not build application/render dependencies."""
import json
from pathlib import Path
import sys
import tomllib

base = Path(__file__).resolve().parent
root = base.parents[1]
out = Path(sys.argv[1]).resolve() / "oracle-source"
out.mkdir(parents=True, exist_ok=True)
workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]
packages = tomllib.loads((root / "Cargo.lock").read_text())["package"]
versions = {p["name"]: p["version"] for p in packages if p["name"] in ("resvg", "png", "serde_json")}
assert versions["resvg"] == "0.48.1"
source = f'''[package]
name = "bboxy450-svg-reference"
version = "{workspace['package']['version']}"
edition = "2024"
publish = false

[workspace]

[[bin]]
name = "bboxy450-svg-reference"
path = {json.dumps(str(base / 'reference.rs'))}

[dependencies]
resvg = {{ version = "={versions['resvg']}", default-features = false }}
png = "={versions['png']}"
serde_json = "={versions['serde_json']}"

[profile.dev]
debug = 0
opt-level = 1
'''
path = out / "Cargo.toml"
if not path.exists() or path.read_text() != source:
    path.write_text(source)
