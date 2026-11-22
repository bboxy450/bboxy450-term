# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Native malloc witness for the app-memory footprint investigation.

Adapted from the private measurement accompanying the 2026-11-21 app-memory
report. Run the same fixture against both builds. Use --control for a pre-fix
build, which must fail the reclaim budget after producing a complete receipt.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time

opt = argparse.ArgumentParser(description=__doc__)
opt.add_argument("app", type=Path)
opt.add_argument("harness", type=Path)
opt.add_argument("evidence", type=Path)
opt.add_argument("--control", action="store_true")
opt.add_argument("--stack-baseline", action="store_true")
opt.add_argument("--compare-atlas-with", type=Path, metavar="BEFORE_STACKS")
args = opt.parse_args()
if args.compare_atlas_with and not args.stack_baseline:
    opt.error("--compare-atlas-with requires --stack-baseline")
out = args.evidence.resolve()
out.mkdir(mode=0o700)
source = Path(__file__).resolve().parents[2] / "bboxy450-harness/tests/fixtures/acp-agent/fake_agent.py"
APP = "bboxy450-term"
HARNESS = "bboxy450-harness"
# Substitute only the synthetic response; keep the fixture's real ACP protocol.
PROMPT = r'''
def memory_prompt(text):
    prompts = __import__("re").findall(r"\bmemory-[a-z0-9-]+\b", text)
    if not prompts:
        return "end_turn"
    text = prompts[-1]
    tick = chr(96)
    body = f"# Private transcript {text}\n\n"
    for i in range(65):
        body += (f"**Probe {i:03}** measured frame timing for {text}. The window queues "
                 "a retained snapshot and the next compositor pass renders the live tab. "
                 f"{tick}memory_stays_bounded{tick}\n\n")
    body += (f"\n\n{tick*3}mermaid\nflowchart LR\n"
             f"A[Terminal] --> B[Queue]\nB --> C[Measured Frame]\n{tick*3}\n")
    body += "\n\n![Private inline image](images/fixture.png)\n\n"
    for start in range(0, len(body), 1800):
        chunk("agent_message_chunk", body[start:start+1800])
    marker = Path.cwd().parent / ("done-" + text)
    pending = marker.with_suffix(".tmp")
    pending.write_text(str(len(body.encode())))
    pending.replace(marker)
    return "end_turn"
'''
# Private 1x1 opaque raster; chart and retained-image custody are exercised by
# the original footprint fixture. This witness isolates transcript ownership.
PNG = bytes.fromhex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d49444154789c631052f1f80f000248017e96c99d990000000049454e44ae426082")

def icon_atlas_malloc(stacks):
    total = worker = 0
    for line in stacks.splitlines():
        if "IconAtlas" not in line:
            continue
        match = re.match(r"\d+ calls? for (\d+) bytes:", line)
        if match:
            size = int(match[1])
            total += size
            if "PanePreparer" in line:
                worker += size
    return {"total_bytes": total, "worker_bytes": worker}

def profile(root):
    home, repository, run = (root / name for name in ("home", "repository", "run"))
    for folder in (home, repository, root / "bin", root / "tmp", root / "shell"):
        folder.mkdir(mode=0o700)
    image = repository / ".bboxy/images/fixture.png"
    image.parent.mkdir(parents=True)
    image.write_bytes(PNG)
    env = {
        "HOME": str(home), "BBOXY_RUN_DIR": str(run),
        "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "SHELL": "/bin/zsh",
        "ZDOTDIR": str(root / "shell"), "TMPDIR": str(root / "tmp") + "/",
        "TERM": "xterm-256color", "LANG": "en_US.UTF-8", "BBOXY_TEST_SCRIPTED_HOSTS": "off",
    }
    begin = time.monotonic()
    curve = []
    commands = (out / "commands.jsonl").open("w", buffering=1)

    def checked(argv, allowed=(0,)):
        cp = subprocess.run(argv, env=env, cwd=repository, capture_output=True,
                            text=True, timeout=75)
        commands.write(json.dumps({"argv": list(map(str, argv)), "exit": cp.returncode}) + "\n")
        assert cp.returncode in allowed, (argv, cp.returncode, cp.stdout[-2000:], cp.stderr[-2000:])
        return cp

    hashes = {}
    for name, original in ((APP, args.app.resolve()), (HARNESS, args.harness.resolve())):
        with original.open("rb") as f:
            hashes[name] = hashlib.file_digest(f, "sha256").hexdigest()
        dest = root / "bin" / name
        shutil.copy2(original, dest)
        checked(["/usr/bin/codesign", "--force", "--sign", "-", dest])
    script = root / "agent.py"
    original = source.read_text()
    marker = "def prompt(rid, text):"
    assert original.count(marker) == 1
    script.write_text(original.replace(marker, PROMPT + "\n" + marker +
        '\n    if SCENARIO == "memory":\n        return memory_prompt(text)'))
    profile_root = home / ".bboxy450-harness"
    (profile_root / "profiles").mkdir(parents=True)
    (profile_root / "default-profile").write_text("memory\n")
    command = json.dumps([sys.executable, "-I", str(script), "memory"])
    (profile_root / "profiles/memory.md").write_text(
        f"---\nname: memory\nprovider: acp-agent\npolicy: ask\nagent:\n"
        f"  command: {command}\non_unattended: deny\n---\n")
    binary = root / "bin" / APP

    def ctl(*argv, allowed=(0,)):
        reply = checked([binary, "ctl", *argv, "--run-dir", run], allowed)
        row = json.loads(reply.stdout.splitlines()[-1])
        return row.get("data") if row.get("ok") else None

    def until(read, label, seconds=35):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            assert app.poll() is None, (label, app.returncode)
            if result := read():
                return result
            time.sleep(.2)
        raise AssertionError(label)

    def vm_mib(value):
        unit = {"K": 1024, "M": 1024**2, "G": 1024**3}[value[-1]]
        return float(value[:-1]) * unit / 1024**2

    def sample(label):
        result = checked(["/usr/bin/vmmap", "-summary", str(app.pid)])
        (out / f"{label}.vmmap.txt").write_text(result.stdout)
        m = re.search(r"Physical footprint:\s+(\S+).*?Physical footprint \(peak\):\s+(\S+)", result.stdout, re.S)
        assert m
        total = re.search(r"^TOTAL\s+(.*)$", result.stdout.split("MALLOC ZONE")[-1], re.M)
        assert total
        row = {"phase": label, "s": round(time.monotonic()-begin, 2),
               "footprint_mib": vm_mib(m[1]), "peak_mib": vm_mib(m[2]),
               "allocated_mib": vm_mib(total[1].split()[5])}
        curve.append(row)
        print(json.dumps(row), flush=True)
        return row["allocated_mib"]

    def spawn(name):
        row = ctl("seat", "spawn", HARNESS, "--place", "tab", "--role", "executor",
                  "--name", name, "--cwd", str(repository))
        seat = row["seat"]
        ctl("seat", "wait", seat, "--state", "idle", "--timeout", "20000")
        return seat

    def reply(seat, prompt):
        ctl("ui", "focus", seat)
        ctl("seat", "type", seat, prompt)
        ctl("seat", "key", seat, "enter")
        marker = root / ("done-" + prompt)
        until(marker.is_file, prompt)
        assert int(marker.read_text()) > 9000

    def expired(seats):
        wanted = set(seats)
        rows = ctl("seat", "ls")
        return {row["record"]["id"] for row in rows if row["record"]["lifecycle"] ==
                "closed_retained"} >= wanted

    app_env = dict(env)
    if args.stack_baseline:
        app_env["MallocStackLogging"] = "1"
    with (out / "app.log").open("w") as log:
        app = subprocess.Popen([binary, "--headless", "--ctl", "--unattended",
            "--test-account", root / "account", "--run-dir", run, "--repository", repository],
            env=app_env, cwd=repository, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
        try:
            status = until(lambda: ((run / "gui.endpoint").is_file() and
                (s := ctl("app", "status", allowed=(0, 1))) and s["gui_ready"] and s), "private readiness")
            assert status["windows"] == 0 and status["custody_backend"] == "file"
            time.sleep(3)
            baseline = sample("baseline")
            if args.stack_baseline:
                stack = checked(["/usr/bin/malloc_history", str(app.pid), "-allBySize"])
                (out / "baseline.stacks.txt").write_text(stack.stdout)
                receipt = {"gui": status["gui"], "binaries": hashes, "curve": curve,
                           "atlas": icon_atlas_malloc(stack.stdout)}
                if args.compare_atlas_with:
                    receipt["atlas_before"] = icon_atlas_malloc(args.compare_atlas_with.read_text())
                return receipt
            seats = [spawn(f"memory-{n}") for n in range(4)]
            for turn in range(48):
                for n, seat in enumerate(seats):
                    reply(seat, f"memory-warm-{turn}-{n}")
                if turn in (7, 15, 23, 31, 39):
                    sample(f"replies-{4*(turn+1)}")
            frame = until(lambda: (f := ctl("ui", "layout")) and
                (r := f.get("rich_observation", {}).get(seats[-1])) and
                r["ready"] and r["paint_count"] and r["retained_items"] >= 96 and f, "rich frame")
            assert frame["rich_observation"][seats[-1]]["error"] is None
            ctl("seat", "capture", "window", out / "rich.png")
            time.sleep(3)
            peak_live = sample("replies-192")
            for seat in seats:
                ctl("seat", "close", seat)
            until(lambda: expired(seats), "all four seat grace intervals expired", 85)
            time.sleep(3)
            remaining = sample("post-grace")
            assert peak_live > baseline + 8., "fixture never loaded"
            recovered = (peak_live - remaining) / (peak_live - baseline)
            churn = []
            for n in range(12):
                seat = spawn(f"memory-churn-{n}")
                reply(seat, f"memory-churn-{n}")
                ctl("seat", "close", seat)
                churn.append(seat)
            until(lambda: expired(churn), "twelve new seats reached final closure", 85)
            time.sleep(3)
            churn_idle = sample("churn-post-grace")
            return {"gui": status["gui"], "binaries": hashes, "curve": curve,
                    "recovered": recovered, "reclaim_ok": recovered >= .75,
                    "churn_ok": churn_idle - remaining <= 16.}
        finally:
            try:
                if app.poll() is None:
                    ctl("app", "stop", allowed=(0, 1))
            finally:
                try:
                    app.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    app.terminate()
                    app.wait(timeout=10)
                assert app.returncode == 0

with tempfile.TemporaryDirectory(prefix="bbx-ram-", dir="/tmp") as directory:
    receipt = profile(Path(directory).resolve())
(out / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
if args.compare_atlas_with:
    before, after = receipt["atlas_before"], receipt["atlas"]
    assert before["worker_bytes"] >= 16 * 1024**2, receipt
    assert after["worker_bytes"] == 0 and after["total_bytes"] >= 16 * 1024**2, receipt
    assert before["total_bytes"] - after["total_bytes"] >= 16 * 1024**2, receipt
elif not args.stack_baseline:
    if args.control:
        assert not receipt["reclaim_ok"], receipt
    else:
        assert receipt["reclaim_ok"] and receipt["churn_ok"], receipt
