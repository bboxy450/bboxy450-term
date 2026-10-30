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
parser.add_argument("--runs", type=int, default=3, choices=range(1, 11),
                    help="fresh sessions; failures are retained, never retried away")
args = parser.parse_args()
plugin = Path(args.plugin).resolve()
root = Path(args.workspace).resolve(strict=True)
evidence = {"root": str(root), "plugin_sha256": hashlib.sha256(plugin.read_bytes()).hexdigest(), "operations": []}
shape = json.loads(subprocess.run([str(plugin), "--shape"], capture_output=True, text=True,
                                  timeout=5, check=True).stdout)
for attempt in range(args.runs):
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
                ready = select.select([process.stdout], [], [], 5)[0]
                try:
                    result = json.loads(process.stdout.readline()) if ready else {}
                except (ValueError, UnicodeError):
                    result = {}
                if not isinstance(result, dict):
                    result = {}
                elapsed = time.monotonic() - start
                row = {"run": attempt + 1, "method": params.get("name", method), "elapsed_ms": round(elapsed * 1000, 2)}
                evidence["operations"].append(row)
                value = result.get("result", {})
                if not ready or "error" in result or "result" not in result:
                    row["error"] = "no_reply" if not ready else "protocol_error"
                if value.get("isError"):
                    row["error"] = value.get("structuredContent", {}).get("code", "provider_error")
                if elapsed >= 2:
                    row["deadline_exceeded"] = True
                # Persist every attempt before checking it, including intermittent
                # resolve failures and host/provider timeout failures.
                evidence["result"] = "failed" if row.get("error") or elapsed >= 2 else "incomplete"
                Path(args.evidence).write_text(json.dumps(evidence, indent=2))
                assert not row.get("error") and elapsed < 2, row
                if method == "tools/call":
                    assert json.loads(value["content"][0]["text"]) == value["structuredContent"]
                return value.get("structuredContent", value)

            def call(method, arguments):
                return rpc("tools/call", {"name": "data_source_" + method, "arguments": arguments})

            rpc("initialize", {"protocolVersion": "2025-06-18", "root": str(root)})
            ref = call("resolve", {"locator": "context/immutable/project-brief"})
            evidence["resolved"] = ref
            record = call("fetch", {"ref": ref})
            evidence["source"] = record["gitkb"]["source"]
            evidence["title"] = record["frontmatter"]["title"]
            source = record["gitkb"]["source"]
            native_env = dict(env, GITKB_ROOT=str(root))
            if source["kind"] == "workspace":
                native_env["GITKB_WORKSPACE"] = source["workspace"]
            native = subprocess.run([shape["identity"]["backend"]["path"], "--no-pager", "--color", "never",
                                     "show", "--source", source["kind"], "--raw-content", "--json", "--", ref["slug"]],
                                    cwd=data, env=native_env, capture_output=True, text=True, timeout=5, check=True)
            native = json.loads(native.stdout)
            parity = {"run": attempt + 1,
                      "raw": record["gitkb"]["raw_content"] == native["rawContent"],
                      "body": record["body"] == native["document"]["content"],
                      "id": record["external_id"] == native["document"]["id"],
                      "etag": record["etag"] == native["generation"],
                      "raw_sha256": hashlib.sha256(native["rawContent"].encode()).hexdigest()}
            evidence.setdefault("source_parity", []).append(parity)
            if not all(parity[key] for key in ("raw", "body", "id", "etag")):
                evidence["result"] = "failed; native source parity"
            Path(args.evidence).write_text(json.dumps(evidence, indent=2))
            assert all(parity[key] for key in ("raw", "body", "id", "etag")), "native source parity failed"
            for method, request in [("list", {"filter": {"type": "task"}, "limit": 32}),
                                    ("search", {"query": "knowledge type:task", "limit": 32}),
                                    ("graph", {"scope": {"kind": "type", "graph": "markdown", "type": "task"}, "limit": 32})]:
                first = call(method, request)
                evidence["operations"][-1]["items"] = len(first.get("items", first.get("nodes", [])))
                if first.get("next"):
                    second = call(method, dict(request, cursor=first["next"]))
                    evidence["operations"][-1]["items"] = len(second.get("items", second.get("nodes", [])))
            call("links", {"ref": ref})
            changes = call("changes_since", {"cursor": "0"})
            assert not call("changes_since", {"cursor": changes["next"]})["changed"]
            call("resolve", {"locator": "context/immutable/project-brief"})
            process.stdin.close()
            process.wait(timeout=5)
            assert process.returncode == 0
            evidence["process_exit"] = process.returncode
            evidence.setdefault("sessions", []).append({"run": attempt + 1, "process_exit": process.returncode})
evidence["result"] = "passed; fresh sessions, exact native source parity, all calls read-only and below 2 seconds"
Path(args.evidence).write_text(json.dumps(evidence, indent=2))
print(json.dumps(evidence, indent=2))
