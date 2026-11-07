#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Six private PTYs: quiet/occluded CPU and endless retained attention budgets."""
import argparse
import ctypes
import hashlib
import json
import math
import os
from pathlib import Path
import signal
import struct
import subprocess
import tempfile
import time


# Adapted from this project's cockpit/owners.rs ProcessIdentity::read: the
# installed macOS SDK's proc_bsdinfo ABI, never process-name based cleanup.
# Origin: https://github.com/bboxy450/bboxy450-term
_LIBPROC = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
_LIBPROC.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int]
_LIBPROC.proc_pidinfo.restype = ctypes.c_int


def identity(pid):
    data = ctypes.create_string_buffer(136)
    if _LIBPROC.proc_pidinfo(pid, 3, 0, data, len(data)) != len(data):
        return None
    # PROC_PIDTBSDINFO: status at byte 4; start seconds/useconds at byte 120.
    if struct.unpack_from("=I", data, 4)[0] == 5:  # SZOMB
        return None
    return {"pid": pid, "birth": list(struct.unpack_from("=QQ", data, 120))}


def cpu_seconds(pid):
    # ps TIME counts all threads, unlike a UI-thread clock. Resolution is 10ms.
    value = subprocess.check_output(["/bin/ps", "-p", str(pid), "-o", "time="], text=True).strip()
    minutes, seconds = value.split(":")
    return int(minutes) * 60 + float(seconds)


def counters(status):
    render = status["history"]["gui_render"]
    result = {name: render[name] for name in ("full_frames", "pulse_frames", "occluded")}
    result["frame_seq"] = status["frame_seq"]
    assert all(type(result[name]) is int and result[name] >= 0
               for name in ("frame_seq", "full_frames", "pulse_frames")), result
    assert type(result["occluded"]) is bool, result
    return result


# Adapted from bboxy450-term crates/bboxy450-term/tests/wheel.py: private account
# and ctl fixture; adds process CPU, frame counters, profiling and owned cleanup.
# Origin: https://github.com/bboxy450/bboxy450-term
def run(binary, mode, seconds, control, full_scene_control, profile, waiting, occluded, pacing):
    assert not full_scene_control or waiting and mode == "native"
    assert not occluded or waiting and mode == "native"
    root = Path(tempfile.mkdtemp(prefix="bboxy-idle-cpu-", dir="/tmp"))
    account, runtime, repository, shell = [root / n for n in ("account", "run", "repository", "shell")]
    for path in (account, runtime, repository, shell):
        path.mkdir(mode=0o700)
    release = root / "waiting-ready"
    attention = (f"while [[ ! -e '{release}' ]]; do /bin/sleep 0.05; done\n"
                 "printf '\\033]7501;state=blocked:kind=question:msg=SWRsZSBxdWVzdGlvbg==\\007'\n") if waiting else ""
    (shell / ".zshrc").write_text(attention + "exec /bin/cat\n")
    env = {"HOME": str(root), "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "ZDOTDIR": str(shell),
           "BBOXY_RUN_DIR": str(runtime), "BBOXY_TEST_SCRIPTED_HOSTS": "off"}
    args = [str(binary), "--ctl", "--unattended", "--test-account", str(account),
            "--run-dir", str(runtime), "--repository", str(repository), "--spawn-zsh", "6",
            "--notifications", "record", "--idle-cpu-probe"]
    if mode == "headless":
        args.append("--headless")
    else:
        args += ["--pacing", pacing]
    for enabled, flag in ((control, "--idle-redraw-control"),
                          (full_scene_control, "--idle-full-scene-control"),
                          (occluded, "--idle-occluded")):
        if enabled:
            args.append(flag)
    digest = hashlib.sha256()
    with binary.open("rb") as executable:
        for chunk in iter(lambda: executable.read(1024 * 1024), b""):
            digest.update(chunk)
    log = (root / "frontend.log").open("w")
    process = subprocess.Popen(args, env=env, cwd=repository, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
    (root / "owned-pid.json").write_text(json.dumps(process.pid))
    owners = {}

    def remember_owners():
        # These records can only come from this freshly-created private run.
        # Preserve the birth fence before owner files disappear during shutdown.
        for name in ("logic", "supervisor"):
            path = runtime / f"{name}.owner.json"
            try:
                stat = path.lstat()
                assert not path.is_symlink() and stat.st_uid == os.getuid() and stat.st_mode & 0o077 == 0, path
                record = json.loads(path.read_text())
            except FileNotFoundError:
                continue
            assert Path(record["run"]).resolve() == runtime.resolve() and Path(record["account"]).resolve() == account.resolve(), path
            owner = record["process"]
            assert type(owner["pid"]) is int and len(owner["birth"]) == 2, path
            owners[path.name.removesuffix(".owner.json")] = owner
        (root / "owned-backend-identities.json").write_text(json.dumps(owners))

    def alive(owner):
        return identity(owner["pid"]) == owner

    def ctl(*command, optional=False):
        reply = subprocess.run([str(binary), "ctl", *command, "--run-dir", str(runtime)],
                               env=env, cwd=repository, capture_output=True, text=True, timeout=35)
        if optional and reply.returncode:
            return None
        assert reply.returncode == 0, (command, reply.returncode, reply.stdout, reply.stderr, str(root))
        return json.loads(reply.stdout)["data"]

    def status():
        assert process.poll() is None, (process.returncode, (root / "frontend.log").read_text()[-6000:])
        return ctl("app", "status", optional=True)

    try:
        deadline = time.monotonic() + 90
        while time.monotonic() < deadline:
            remember_owners()
            current = status()
            seats = ctl("seat", "ls", optional=True) if current and current["gui_ready"] else None
            if seats is not None:
                (root / "seat-readiness.json").write_text(json.dumps(seats))
            real_seats = seats is not None and len(seats) == 6 and all(isinstance(row.get("state"), dict) for row in seats)
            if real_seats and waiting and not release.exists():
                time.sleep(1)
                release.touch()
            if real_seats and (not waiting or all(
                    row["state"]["activity"] == {"kind": "waiting", "waiting": "question"} for row in seats)):
                break
            time.sleep(0.2)
        else:
            raise AssertionError(f"six-seat readiness timed out: {root}")
        # Initial focus/reveal and resize effects settle. Attention itself must
        # remain animated beyond this interval; it is not a finite reveal cue.
        time.sleep(6)
        remember_owners()
        assert all(name in owners and alive(owners[name]) for name in ("logic", "supervisor")), owners
        backend_pids = {name: owners[name]["pid"] for name in ("logic", "supervisor")}
        if occluded:
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if counters(status())["occluded"]:
                    break
                time.sleep(0.2)
            else:
                raise AssertionError(f"private native window never reported actual occlusion: {root}")
        if profile:
            for label, pid in {"gui": process.pid, **backend_pids}.items():
                command = ["/usr/bin/sample", str(pid), "2", "10", "-file", str(root / f"{label}.sample.txt")]
                result = subprocess.run(command, capture_output=True, text=True, timeout=15)
                (root / f"{label}.sample-status.json").write_text(json.dumps({"command": command, "exit_code": result.returncode}))
            for iteration in range(2):
                result = subprocess.run(["/usr/bin/vmmap", "-summary", str(backend_pids["logic"])],
                                        capture_output=True, text=True, timeout=20)
                (root / f"logic-vmmap-{iteration}.txt").write_text(result.stdout + result.stderr)
                (root / f"logic-vmmap-{iteration}.exit").write_text(str(result.returncode))
                time.sleep(2)
            result = subprocess.run(["/usr/bin/vmmap", "-wide", str(backend_pids["logic"])],
                                    capture_output=True, text=True, timeout=20)
            (root / "logic-vmmap-wide.txt").write_text(result.stdout + result.stderr)
            (root / "logic-vmmap-wide.exit").write_text(str(result.returncode))
        old_cpu = cpu_seconds(process.pid)
        started = time.monotonic()
        samples = []
        for index in range(3):
            if index:
                time.sleep(seconds / 2)
            sample = counters(status())
            sample["seconds"] = time.monotonic() - started
            samples.append(sample)
        elapsed = time.monotonic() - started
        cpu = cpu_seconds(process.pid) - old_cpu
        before, middle, after = samples
        deltas = {name: after[name] - before[name] for name in ("frame_seq", "full_frames", "pulse_frames")}
        assert all(value >= 0 for value in deltas.values()), (before, after)
        assert deltas["frame_seq"] == deltas["full_frames"] + deltas["pulse_frames"], (before, after)
        percent = 100 * cpu / elapsed
        visible_waiting = waiting and not occluded
        # Counter snapshots can straddle one deadline at each sample boundary.
        frame_budget = math.ceil(30 * elapsed) + 1 if visible_waiting else 0
        cpu_budget = 10 if visible_waiting else 5
        recurrent = not visible_waiting or all(b["pulse_frames"] > a["pulse_frames"]
                                                for a, b in ((before, middle), (middle, after)))
        visibility_matches = all(sample["occluded"] == occluded for sample in samples)
        passed = (percent <= cpu_budget and deltas["frame_seq"] <= frame_budget
                  and deltas["full_frames"] == 0 and recurrent and visibility_matches)
        rss = {label: int(subprocess.check_output(["/bin/ps", "-p", str(pid), "-o", "rss="], text=True))
               for label, pid in {"gui": process.pid, **backend_pids}.items()}
        report = dict(root=str(root), binary_sha256=digest.hexdigest(), mode=mode, pacing=pacing if mode == "native" else None, seats=6, pid=process.pid,
                      backend_pids=backend_pids, seconds=elapsed, cpu_seconds=cpu,
                      cpu_percent_one_core=percent, cpu_budget_percent_one_core=cpu_budget,
                      idle_frames=deltas["frame_seq"], full_frames=deltas["full_frames"],
                      pulse_frames=deltas["pulse_frames"], frames_per_second=deltas["frame_seq"] / elapsed,
                      frame_budget=frame_budget, pulse_recurrent=recurrent, samples=samples,
                      rss_kib=rss, forced_continuous_redraw=control,
                      forced_full_scene_pulse=full_scene_control, waiting=waiting, occluded=occluded,
                      passed=passed)
        (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report), flush=True)
        return report
    finally:
        # UI death deliberately preserves owners. Stop this exact private run
        # even after frontend failure, then signal only matching recorded births.
        try:
            remember_owners()
            try:
                ctl("app", "stop", optional=True)
            except (subprocess.TimeoutExpired, json.JSONDecodeError):
                pass
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline and (process.poll() is None or any(map(alive, owners.values()))):
                time.sleep(0.1)
            if process.poll() is None:
                process.terminate()
            for signum in (signal.SIGTERM, signal.SIGKILL):
                for owner in owners.values():
                    if alive(owner):
                        try:
                            os.kill(owner["pid"], signum)
                        except ProcessLookupError:
                            pass
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline and any(map(alive, owners.values())):
                    time.sleep(0.1)
                if not any(map(alive, owners.values())):
                    break
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
            remaining = [owner for owner in owners.values() if alive(owner)]
            assert not remaining, f"private backends survived cleanup: {root} {remaining}"
        finally:
            log.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=lambda v: Path(v).resolve())
    parser.add_argument("--mode", choices=("headless", "native"), default="headless")
    parser.add_argument("--pacing", choices=("metal", "cv", "fifo"), default="metal")
    parser.add_argument("--seconds", type=float, default=10)
    parser.add_argument("--control", action="store_true")
    parser.add_argument("--full-scene-control", action="store_true")
    parser.add_argument("--profile", action="store_true")
    parser.add_argument("--waiting", action="store_true")
    parser.add_argument("--occluded", action="store_true")
    options = parser.parse_args()
    assert 3 <= options.seconds <= 60
    assert not (options.control and options.full_scene_control)
    assert not options.occluded or not (options.control or options.full_scene_control)
    result = run(options.binary, options.mode, options.seconds, options.control, options.full_scene_control,
                 options.profile, options.waiting, options.occluded, options.pacing)
    if options.control:
        assert not result["passed"] and result["idle_frames"] >= 10, "continuous redraw control failed to refute budget"
    elif options.full_scene_control:
        assert not result["passed"] and result["full_frames"] >= 2, "full-scene pulse control failed to refute budget"
    else:
        assert result["passed"], "idle/attention CPU or frame budget exceeded"
