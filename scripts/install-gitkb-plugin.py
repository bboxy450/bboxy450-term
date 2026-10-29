#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Install only the authorized Harmony-local artifact and its notices."""
import hashlib
import argparse
import json
import os
from pathlib import Path
import tempfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("--workspace", required=True, type=Path)
args = parser.parse_args()
source = args.source
workspace = args.workspace.resolve(strict=True)
if not workspace.is_dir() or not (workspace / ".kb").is_dir() or workspace == Path.home():
    raise SystemExit("install_scope_refused: supply an explicit local GitKB project")
directory = workspace / ".bboxy/plugins"
notices = workspace / ".bboxy/plugin-notices/gitkb"
for parent in (directory, directory.parent, notices, notices.parent):
    if parent.is_symlink():
        raise SystemExit("install_scope_refused: symlinked installation path")
directory.mkdir(parents=True, exist_ok=True)
notices.mkdir(parents=True, exist_ok=True)
for target, data, mode in [(directory / source.name, source.read_bytes(), 0o755),
                         (notices / "LICENSE", (root / "LICENSE").read_bytes(), 0o644),
                         (notices / "NOTICE", (root / "NOTICE").read_bytes() + b"\n" +
                          (root / "plugins/gitkb/NOTICE").read_bytes(), 0o644),
                         (notices / "ATTRIBUTION.md", (root / "ATTRIBUTION.md").read_bytes(), 0o644)]:
    if target.is_symlink():
        raise SystemExit("install_scope_refused: symlinked destination")
    with tempfile.NamedTemporaryFile(dir=target.parent, prefix=".gitkb-install-", delete=False) as file:
        temporary = Path(file.name)
        file.write(data)
    temporary.chmod(mode)
    os.replace(temporary, target)
print(json.dumps({"installed": str(directory / source.name), "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                  "activation": "Discovery only; host trust, check, enable and root selection remain explicit"}))
