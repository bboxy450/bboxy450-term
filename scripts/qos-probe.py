#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Private native MCP round trips under CPU load; no installed app or account.

Default-load latency is comparative evidence, not a required failing control.
--control default-qos and --control delayed-reply are deterministic negatives.
"""
import argparse
import json
import math
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
QOS = ROOT / "scripts/qos.sh"


def line(stream, seconds=30):
    if not select.select([stream], [], [], seconds)[0]:
        raise RuntimeError("private fixture response timed out")
    value = stream.readline()
    if not value:
        raise RuntimeError("private fixture closed unexpectedly")
    return value


def rpc_response(server, expected):
    deadline = time.monotonic() + 30
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("private MCP response deadline exceeded")
        response = json.loads(line(server.stdout, remaining))
        if response.get("id") == expected:
            return response
        if "id" not in response and str(response.get("method", "")).startswith("notifications/"):
            continue
        raise RuntimeError("unexpected private MCP response id")


def samples(server, count, delay_ms=0):
    values = []
    for index in range(count + 10):
        request = {"jsonrpc": "2.0", "id": index, "method": "tools/call",
                   "params": {"name": "self_status", "arguments": {}}}
        start = time.perf_counter_ns()
        server.stdin.write(json.dumps(request).encode() + b"\n")
        server.stdin.flush()
        response = rpc_response(server, index)
        if response.get("id") != index or "error" in response:
            raise RuntimeError("private MCP protocol failure")
        result = response.get("result", {})
        actor = result.get("structuredContent", {}).get("actor", {})
        if result.get("isError") or not actor.get("seat_id"):
            raise RuntimeError("private self_status did not authenticate")
        if delay_ms:
            time.sleep(delay_ms / 1000)
        elapsed = (time.perf_counter_ns() - start) / 1e6
        if index >= 10:
            values.append(elapsed)
    values.sort()
    return {"p50_ms": values[math.ceil(count * .5) - 1],
            "p95_ms": values[math.ceil(count * .95) - 1], "samples": count}


def build(target):
    target.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(["make", "-s", "build-qos-probe", f"TARGET_DIR={target}"],
                            cwd=ROOT, capture_output=True, text=True)
    if result.returncode:
        sys.stderr.write(result.stderr)
        raise RuntimeError(f"private mux build failed ({result.returncode})")
    paths = [r["executable"] for s in (target / "qos-probe-build.jsonl").read_text().splitlines() if s.startswith("{")
             for r in [json.loads(s)] if r.get("reason") == "compiler-artifact" and r.get("executable")]
    if len(paths) != 1:
        raise RuntimeError("private mux executable missing or ambiguous")
    return paths[0]


def load_sample(server, role, count, delay_ms):
    # Read back the real child's priority; argv alone does not establish policy.
    worker = ('import ctypes,json,os,subprocess,sys; '
              'background=ctypes.CDLL(None).getpriority(4,os.getpid()) if sys.platform=="darwin" else None; '
              'print(json.dumps(dict(priority=int(subprocess.check_output(["ps","-o","pri=","-p",str(os.getpid())],text=True)),nice=os.getpriority(os.PRIO_PROCESS,0),darwin_background=background)),flush=True)\n'
              'while True: pass\n')
    children = []
    try:
        for _ in range(os.cpu_count() or 1):
            child = subprocess.Popen(["/bin/sh", str(QOS), role, sys.executable, "-I", "-c", worker], stdout=subprocess.PIPE)
            children.append(child)
        policies = [json.loads(line(child.stdout)) for child in children]
        time.sleep(.2)
        if any(child.poll() is not None for child in children):
            raise RuntimeError("load worker exited before measurement")
        result = samples(server, count, delay_ms)
        if any(child.poll() is not None for child in children):
            raise RuntimeError("load worker exited during measurement")
        return {**result, "workers": len(children), "priorities": [p["priority"] for p in policies],
                "nice": [p["nice"] for p in policies], "darwin_background": [p["darwin_background"] for p in policies]}
    finally:
        for child in children:
            if child.poll() is None:
                child.terminate()
        for child in children:
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--target", type=Path, default=ROOT / "target")
    parser.add_argument("--control", choices=["default-qos", "delayed-reply"])
    args = parser.parse_args()
    if args.samples < 20:
        parser.error("at least 20 measured samples are required")
    executable = build(args.target.resolve())
    with tempfile.TemporaryDirectory(prefix="qos-probe-", dir="/tmp") as own:
        with open(Path(own) / "stderr", "w+") as error:
            server = subprocess.Popen([executable, "--exact", "qos_probe::serve", "--ignored", "--nocapture", "--test-threads=1"],
                                      stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=error, bufsize=0)
            try:
                while True:
                    ready = line(server.stdout)
                    if b"QOS_READY " in ready:
                        witness = json.loads(ready.split(b"QOS_READY ", 1)[1])
                        break
                server.stdin.write(json.dumps({"jsonrpc": "2.0", "id": "init", "method": "initialize",
                    "params": {"protocolVersion": "2025-11-25", "clientInfo": {"name": "qos-probe", "version": "1"}, "capabilities": {}}}).encode() + b"\n")
                server.stdin.flush()
                initialized = rpc_response(server, "init")
                if "error" in initialized:
                    raise RuntimeError("private bridge initialization failed")
                unloaded = samples(server, args.samples)
                before = load_sample(server, "default", args.samples, 0)
                threshold = max(4 * unloaded["p95_ms"], 250)
                role = "default" if args.control == "default-qos" else "background"
                delay = threshold + 10 if args.control == "delayed-reply" else 0
                after = load_sample(server, role, args.samples, delay)
                qos_ok = ((all(p == 1 for p in after["darwin_background"]) and witness["requested_class"] == 0x19)
                          if sys.platform == "darwin" else all(p >= 10 for p in after["nice"]))
                latency_ok = after["p95_ms"] <= threshold
                report = {"unloaded": unloaded, "before_default_load": before, "after_build_load": after,
                          "limit_ms": threshold, "qos_ok": qos_ok, "latency_ok": latency_ok,
                          "mcp_requested_class": witness["requested_class"], "control": args.control}
                print(json.dumps(report, sort_keys=True), flush=True)
                return int(args.check and not (qos_ok and latency_ok))
            finally:
                server.stdin.close()
                try:
                    server.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    server.terminate()
                    server.wait(timeout=5)
                if server.returncode:
                    error.seek(0)
                    sys.stderr.write(error.read()[-4000:])
                    raise RuntimeError(f"private mux failed ({server.returncode})")


if __name__ == "__main__":
    raise SystemExit(main())
