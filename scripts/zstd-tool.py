# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Origin: P2 task #5084; workspace-local compatibility tool.
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
# License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
"""Build the checksum-pinned upstream compatibility CLI locally, never globally."""
import importlib.util
import json
import os
import signal
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location("formal",ROOT/"scripts/formal.py")
formal=importlib.util.module_from_spec(spec);spec.loader.exec_module(formal)
pin=json.loads((ROOT/"verification/tools.lock.json").read_text())["zstd"]
cache=Path(os.environ.get("BBOXY_CARGO_TARGET_DIR", ROOT/"target"))/"tools";cache.mkdir(parents=True,exist_ok=True)
archive=formal.artifact(pin,cache/("zstd-"+pin["version"]+".tar.gz"))
source=cache/("zstd-"+pin["version"])
if not source.exists():
    with tempfile.TemporaryDirectory(dir=cache,prefix="zstd-unpack-") as own:
        with tarfile.open(archive) as tar:tar.extractall(own,filter="data")
        entries=list(Path(own).iterdir())
        if len(entries)!=1:raise ValueError("unexpected zstd archive")
        entries[0].rename(source)
binary=source/"programs/zstd"
if not binary.exists():
    with (cache/"zstd-build.log").open("w") as log:
        # This is one-time tool compilation, outside the affected-test budget.
        # A private process group prevents orphan compilers on setup failure.
        command=["make","-C",str(source/"programs"),"-j2","zstd","HAVE_ZLIB=0","HAVE_LZMA=0","HAVE_LZ4=0"]
        with subprocess.Popen(command,stdout=log,stderr=subprocess.STDOUT,start_new_session=True) as process:
            try:
                code=process.wait(timeout=600)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid,signal.SIGKILL)
                process.wait()
                raise
            if code:raise subprocess.CalledProcessError(code,command)
version=subprocess.check_output([str(binary),"--version"],stderr=subprocess.STDOUT,text=True)
if "v"+pin["version"] not in version:raise ValueError("wrong zstd CLI version")
print("zstd-tool: pinned source build PASS "+str(binary))
