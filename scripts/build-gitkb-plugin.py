#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Build one self-contained executable; embed canonical contract and backend pin."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--gitkb", required=True)
parser.add_argument("--output", required=True)
parser.add_argument("--schema", type=Path, default=ROOT / "crates/bbt-plugin/schemas/data_source-1.0.schema.json")
args = parser.parse_args()
backend = Path(shutil.which(args.gitkb) or args.gitkb).resolve(strict=True)
version = subprocess.run([str(backend), "--version"], capture_output=True, text=True, timeout=5, check=True).stdout.strip()
build = {"path": str(backend), "sha256": hashlib.sha256(backend.read_bytes()).hexdigest(), "version": version}
schema = json.loads(args.schema.read_text())
if not {"CommitRequest", "CommitResult"} <= schema["$defs"].keys():
    raise SystemExit("schema_requires_commit_extension: use the canonical host schema with optional commit")
source = (ROOT / "plugins/gitkb/adapter.py").read_text()
source = source.replace("#!/usr/bin/env python3", "#!" + str(Path(sys.executable).resolve()) + " -I", 1)
source = source.replace("BUILD = {}  # @build", "BUILD = " + repr(build))
source = source.replace("SCHEMA = {}  # @schema", "SCHEMA = " + repr(schema))
compile(source, args.output, "exec")
output = Path(args.output)
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(source)
output.chmod(0o755)
print(json.dumps({"artifact": str(output.resolve()), "sha256": hashlib.sha256(output.read_bytes()).hexdigest(), "backend": build}))
