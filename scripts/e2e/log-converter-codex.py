# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

"""Full-run-only lowered Codex witness. An absent client never produces proof."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[2]

def main():
    if shutil.which("codex") is None:
        print("SKIP log-converter lowered Codex: codex absent; no native resume proof",flush=True)
        return 0
    target=os.environ.get("BBOXY_CARGO_TARGET_DIR",str(ROOT/"target"))
    command=["cargo","test","--locked","-p","bboxy450-log-converter","--test","codex_resume",
             "--target-dir",target,"--","--ignored","--nocapture"]
    try:
        return subprocess.run(command,cwd=ROOT,stdin=subprocess.DEVNULL,timeout=600).returncode
    except subprocess.TimeoutExpired:
        print("FAIL log-converter lowered Codex: native witness deadline exceeded",file=sys.stderr)
        return 124

if __name__=="__main__":sys.exit(main())
