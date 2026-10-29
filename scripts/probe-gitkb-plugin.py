#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Read-only Harmony probe: report metadata and timing, never document content."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--plugin", required=True)
parser.add_argument("--evidence", required=True)
parser.add_argument("--workspace", required=True)
args = parser.parse_args()
plugin = Path(args.plugin).resolve()
root = Path(args.workspace).resolve(strict=True)
evidence = {"root": str(root), "plugin_sha256": hashlib.sha256(plugin.read_bytes()).hexdigest(), "operations": []}
with tempfile.TemporaryDirectory(prefix="bboxy-harmony-readonly-") as data:
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "TMPDIR", "TERM") or k.startswith("LC_")}
    with subprocess.Popen([str(plugin)], cwd=data, env=env, text=True,
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
        sequence = 0

        def rpc(method, params):
            global sequence
            sequence += 1
            start = time.monotonic()
            process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": sequence, "method": method, "params": params}) + "\n")
            process.stdin.flush()
            assert select.select([process.stdout], [], [], 5)[0], "provider did not answer"
            result = json.loads(process.stdout.readline())
            assert "error" not in result, result
            elapsed = time.monotonic() - start
            row = {"method": params.get("name", method), "elapsed_ms": round(elapsed * 1000, 2)}
            evidence["operations"].append(row)
            assert elapsed < 2, row
            value = result["result"]
            if value.get("isError"):
                row["error"] = value.get("structuredContent")
                Path(args.evidence).write_text(json.dumps(evidence, indent=2))
            assert not value.get("isError"), value.get("structuredContent")
            return value.get("structuredContent", value)

        def call(method, arguments):
            return rpc("tools/call", {"name": "data_source_" + method, "arguments": arguments})

        rpc("initialize", {"protocolVersion": "2025-06-18", "root": str(root)})
        ref = call("resolve", {"locator": "context/immutable/project-brief"})
        evidence["resolved"] = ref
        record = call("fetch", {"ref": ref})
        evidence["source"] = record["gitkb"]["source"]
        evidence["title"] = record["frontmatter"]["title"]
        for index in range(3):
            for method, request in [("list", {"filter": {"type": "task"}, "limit": 32}),
                                    ("search", {"query": "knowledge type:task", "limit": 32}),
                                    ("graph", {"scope": {"kind": "type", "graph": "markdown", "type": "task"}, "limit": 32})]:
                first = call(method, request)
                evidence["operations"][-1]["items"] = len(first.get("items", first.get("nodes", [])))
                if first.get("next"):
                    second = call(method, dict(request, cursor=first["next"]))
                    evidence["operations"][-1]["items"] = len(second.get("items", second.get("nodes", [])))
        call("links", {"ref": ref})
        call("changes_since", {"cursor": "0"})
        process.stdin.close()
        process.wait(timeout=5)
        assert process.returncode == 0
        evidence["process_exit"] = process.returncode
evidence["result"] = "passed; all calls read-only and below 2 seconds"
Path(args.evidence).write_text(json.dumps(evidence, indent=2))
print(json.dumps(evidence, indent=2))
