#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Focused unattended C1 join. Missing producer corpus/real runner is a failure.

Exit zero requires source-bound command, real-process, native and resource
receipts. Missing measurements remain failures; exclusions cannot make a green gate.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import selectors
import secrets
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_SHA256 = "5e3d8a602eeaa1da5cb97d0f6af0b4f331b216a02255d3dff990be84c30d6a30"
LOCK_SHA256 = "31113dd0968ace9daf43b1e610b53a3fc9b586e7ff9a707136ec25ab5e9f2c31"
FRAME = 256 * 1024
PINNED_FIXTURES = ROOT / "crates/bboxy450-harness/tests/fixtures/dream-c1"
SOURCES = [
    "Makefile", "scripts/test-dream-c1.py",
    "Cargo.toml", "Cargo.lock", "scripts/quarantine-test.sh", "scripts/landing.py",
    "scripts/quarantine.py", "scripts/test-landing.py",
    "rust-toolchain.toml", ".cargo/config.toml", ".config/nextest.toml", "scripts/cache-env.sh",
    "crates/bbt-proto/src/lib.rs", "crates/bbt-proto/src/gui/mod.rs",
    "crates/bbt-proto/src/dream.rs", "crates/bbt-proto/src/gui/dream.rs",
    "crates/bbt-proto/tests/dream_c1.rs", "crates/bbt-proto/tests/golden/gui_wire.tsv",
    "crates/bboxy450-harness-wake-up/src/lib.rs", "crates/bboxy450-harness-wake-up/src/launch.rs",
    "crates/bboxy450-harness-wake-up/src/dream_c1.rs",
    "crates/bboxy450-harness-wake-up/src/rich.rs",
    "crates/bboxy450-harness-wake-up/tests/dream_c1.rs",
    "crates/bbt-mux/src/logic.rs", "crates/bbt-mux/src/registry.rs",
    "crates/bbt-mux/src/knowledge/roots.rs", "crates/bbt-mux/src/knowledge/workspaces.rs",
    "crates/bbt-mux/src/logic/ring_batch_tests.rs",
    "crates/bbt-mux/src/identity/mod.rs", "crates/bbt-mux/src/identity/status.rs",
    "crates/bbt-mux/src/identity/chain.rs", "crates/bbt-mux/src/identity/custodied.rs",
    "crates/bboxy450-term/tests/formal_contracts.rs",
    "crates/bboxy450-term/tests/formal_contracts/dream_c1.rs",
    "crates/bboxy450-term/tests/formal_contracts/models.rs",
    "verification/corpus.toml", "verification/contracts.toml", "verification/coverage.toml",
    "scripts/formal.py",
    "crates/bbt-mux/src/mcp/mod.rs", "crates/bbt-mux/src/mcp/tools.rs",
    "crates/bbt-mux/src/mcp/hints.rs", "crates/bbt-mux/src/adapters/mod.rs",
    "crates/bbt-mux/src/seat_socket.rs",
    "crates/bbt-mux/src/adapters/launch.rs", "crates/bbt-mux/src/adapters/mapping.rs",
    "crates/bbt-mux/src/logic/dream_c1.rs", "crates/bbt-mux/src/logic/harness_status.rs",
    "crates/bbt-mux/src/registry/dream_c1.rs", "crates/bbt-mux/src/mcp/dream_c1.rs",
    "crates/bbt-mux/tests/dream_c1.rs", "crates/bboxy450-term/src/ctl/dream.rs",
    "crates/bboxy450-term/src/ctl.rs", "crates/bboxy450-term/src/main.rs",
    "crates/bboxy450-term/src/live.rs",
    "crates/bboxy450-term/tests/dream_c1.rs",
    "crates/bboxy450-term/tests/swarm_workdir/rich_detach.rs",
    "crates/bboxy450-term/src/stall.rs", "crates/bboxy450-term/src/stall/headless.rs", "crates/bboxy450-term/src/stall/load.rs",
]


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False,
                      separators=(",", ":"), allow_nan=False).encode()


def strict_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate JSON key")
            result[key] = value
        return result

    def integer(text):
        value = int(text)
        if value < 0 or value > (1 << 53) - 1:
            raise ValueError("unsafe protocol integer")
        return value

    def floating(_):
        raise ValueError("floating protocol number")

    return json.loads(raw, object_pairs_hook=pairs, parse_int=integer,
                      parse_float=floating, parse_constant=floating)


def corpus(fixtures):
    if not fixtures.is_dir():
        raise ValueError(f"missing producer corpus: {fixtures}; no fallback is admitted")
    lock_path = fixtures / "lock.json"
    if sha(lock_path) != LOCK_SHA256:
        raise ValueError("producer corpus lock differs from approved boundary")
    lock = strict_json(lock_path.read_bytes())
    schema_path = fixtures.parents[2] / "schema/dream-c1-v1.schema.json"
    if lock["schema_sha256"] != SCHEMA_SHA256 or sha(schema_path) != SCHEMA_SHA256:
        raise ValueError("producer schema/hash disagrees with the approved consumer pin")
    actual = {path.relative_to(fixtures).as_posix() for path in fixtures.rglob("*")
              if path.is_file() and path != lock_path}
    if actual != set(lock["files"]):
        raise ValueError("missing or unindexed producer fixture entry")
    for name, digest in lock["files"].items():
        if sha(fixtures / name) != digest:
            raise ValueError(f"producer fixture hash disagrees: {name}")
    valid = [name for name in actual if name.startswith("valid/")]
    negative = [name for name in actual if name.startswith("invalid/")]
    if len(valid) != 14 or len(negative) != 10:
        raise ValueError("approved corpus cardinality changed")
    for name in valid:
        raw = (fixtures / name).read_bytes()
        if canonical(strict_json(raw)) != raw:
            raise ValueError(f"producer valid fixture is not canonical: {name}")
    vectors = strict_json((fixtures / "vectors/canonical.json").read_bytes())
    if not vectors:
        raise ValueError("empty independent canonical/hash vectors")
    for vector in vectors:
        raw = canonical(vector["value"])
        if raw != vector["canonical"].encode() or hashlib.sha256(raw).hexdigest() != vector["sha256"]:
            raise ValueError("independent canonical/hash vector disagreement")
    return {"schema_sha256": SCHEMA_SHA256, "lock_sha256": LOCK_SHA256,
            "valid": len(valid), "negative": len(negative), "vectors": len(vectors)}


def run(command, environment, evidence, rows):
    path = evidence / f"command-{len(rows):02d}.log"
    with path.open("xb") as output:
        os.chmod(path, 0o600)
        started = time.monotonic()
        result = subprocess.run(command, cwd=ROOT, env=environment,
                                stdout=output, stderr=subprocess.STDOUT, check=False)
    rows.append({"command": command, "exit_code": result.returncode,
                 "wall_seconds": time.monotonic() - started, "log": str(path)})
    print(f"Dream C1 command {len(rows)} exit {result.returncode}: {path}", flush=True)
    if result.returncode:
        raise subprocess.CalledProcessError(result.returncode, command)


def hello(runner, evidence):
    """Direct fixed worker argv/minimal env; no source/manifest is supplied/read."""
    if not runner.is_file() or not runner.is_absolute():
        raise ValueError(f"missing built absolute sibling runner: {runner}")
    with tempfile.TemporaryDirectory(prefix="dream-c1-probe-") as own:
        job = Path(own)
        os.chmod(job, 0o700)

        def background():
            os.setpriority(os.PRIO_PROCESS, 0, 10)

        process = subprocess.Popen(
            [str(runner), "dream", "inspect-c1", "--job-dir", str(job)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env={"LANG": "C.UTF-8", "TMPDIR": str(job)}, cwd=job,
            start_new_session=True, preexec_fn=background,
        )
        raw = bytearray()
        try:
            with selectors.DefaultSelector() as ready:
                ready.register(process.stdout, selectors.EVENT_READ)
                deadline = time.monotonic() + 2
                while not raw.endswith(b"\n"):
                    remaining = deadline - time.monotonic()
                    if remaining <= 0 or not ready.select(remaining):
                        raise ValueError("real provisioned worker did not produce bounded Hello")
                    chunk = os.read(process.stdout.fileno(), min(4096, FRAME + 1 - len(raw)))
                    if not chunk:
                        raise ValueError("real provisioned worker exited without Hello")
                    raw.extend(chunk)
                    if len(raw) > FRAME or b"\n" in raw[:-1]:
                        raise ValueError("real worker emitted extra/oversized Hello frame")
            value = strict_json(bytes(raw[:-1]))
            expected = {"protocol": "bboxy.dream.c1/1", "type": "hello",
                        "algorithm": "d0-c1/1", "schema_sha256": SCHEMA_SHA256,
                        "ir_major": 1, "ir_minor_max": 3}
            if value != expected:
                raise ValueError("real built worker Hello is incompatible with approved schema")
            process.stdin.close()  # EOF requests cancellation; never starts an inspection.
            try:
                code = process.wait(timeout=4)
            except subprocess.TimeoutExpired as error:
                raise ValueError("Hello probe failed to settle on stdin EOF") from error
            if code not in (0, 1, 2):
                raise ValueError(f"real worker exited abnormally during EOF settlement: {code}")
            stderr = process.stderr.read(16 * 1024 + 1)
            if len(stderr) > 16 * 1024:
                raise ValueError("real worker exceeded bounded stderr")
            (evidence / "hello.stderr").write_bytes(stderr)
            os.chmod(evidence / "hello.stderr", 0o600)
            return {"runner": str(runner), "runner_sha256": sha(runner),
                    "hello": value, "eof_exit_code": code, "actual_reap": True,
                    "inspection_started": False}
        finally:
            if process.poll() is None:
                # Only this exact new-session child/group belongs to the probe.
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=2)
            for stream in (process.stdin, process.stdout, process.stderr):
                stream.close()


REQUIRED_RUNTIME = {
    "closed_only_real_ctl_admission": "owned durable SessionClosed admits a real worker",
    "source_key_manual_dedup": "original and new manual IDs coalesce with automatic close",
    "durable_close_restart_dedup": "duplicate close and daemon restart cause no second replay",
    "own_seat_real_mcp_status_cancel": "actual own-seat MCP Inspect/Status/Cancel admission",
    "immutable_page_tamper_api": "actual API refuses a valid-JSON immutable-result/page edit",
    "held_real_worker_control_latency": "200 input/message/status and cancel round trips under owned real-worker stall",
    "held_source_read": "same latency bounds during a held source read",
    "cpu_heavy_progress_flood": "same bounds during CPU-heavy work and progress flooding",
    "large_input_over_limit": "multi-MiB and over-limit sources preserve resource/control bounds",
    "native_frame_overlap": "240 actual GPU frames with CPU p99 <=4 ms and input GPU <=16.7 ms",
    "same_app_native_frame_pipeline": "native measurements use the same app rendering pipeline as the admitted job",
    "foreground_pause_100ms": "real turn admission pauses within100ms despite held follower/legacy reads",
    "reactor_cpu_units": "actual owner and compute units use <=1ms thread CPU; duty cycle stays bounded",
    "independent_root_progress": "another ready root makes progress within2s while one root is stalled",
    "four_live_jobs_controls": "four simultaneous actual jobs in one account retain independent controls and reap",
    "attributed_parent_child_heap": "actual attributed parent+child live/reserved/cumulative heap, RSS and CPU enforce fixed budgets",
    "hot_owner_busy_wait_controls": "deliberately broken hot-owner and busy-wait controls fail causally",
    "cancel_actual_exit": "cancelled settlement follows the exact owned process exit/reap",
    "restart_predecessor_quarantine": "original/new op retries never adopt or relaunch an unverified predecessor",
    "authority_aba_read_page_unknown_negatives": "actual API enforces current authority across ABA/read/page/unknown outcomes",
}


def finite_number(value, label):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
        raise ValueError(f"missing/non-finite runtime measurement: {label}")
    return value


def runtime_record(evidence, name, nonce, runner_sha256):
    path = evidence / name
    if (not path.is_file() or path.is_symlink() or path.stat().st_size > 256 * 1024
            or path.stat().st_mode & 0o077):
        raise ValueError(f"missing/unbounded/nonprivate runtime receipt: {name}")
    value = json.loads(path.read_bytes())
    if (value.get("schema") != "bboxy.dream.c1.runtime-evidence/1"
            or value.get("driver_nonce") != nonce
            or value.get("runner_sha256") != runner_sha256):
        raise ValueError("runtime receipt does not bind this gate's actual runner/invocation")
    return path, value


def runtime_receipts(evidence, nonce, runner_sha256, seat_runner_sha256):
    """Consume only this invocation's private, assertion-backed test products.

    New runtime products need an explicit validator here. Arbitrary supplied JSON,
    unsupported property names, a child exit, or a producer hash alone prove none
    of the absent requirements.
    """
    path, value = runtime_record(evidence, "runtime-native-ctl.json", nonce, runner_sha256)
    established = set(value["established"])
    expected = {"closed_only_real_ctl_admission", "source_key_manual_dedup",
                "own_seat_real_mcp_status_cancel", "held_real_worker_control_latency",
                "native_frame_overlap", "same_app_native_frame_pipeline", "independent_root_progress", "cancel_actual_exit"}
    if established != expected or established.intersection(value["not_established"]):
        raise ValueError("native/ctl receipt changes its assertion-backed scope")
    if (value.get("source_unchanged") is not True
            or value.get("actual_original_process_absent") is not True
            or value.get("settled_state") != "cancelled"
            or finite_number(value.get("source_bytes"), "source bytes") < 2 * 1024 * 1024
            or not isinstance(value.get("worker_pid"), int) or value["worker_pid"] <= 0):
        raise ValueError("missing closed-source/cancellation/reap runtime evidence")
    for name in ("input", "message", "status", "cancel"):
        metrics = value["cancel_roundtrips"] if name == "cancel" else value["roundtrips"][name]
        if (metrics.get("samples") != 200
                or finite_number(metrics.get("p99_ms"), name + " p99") > 100
                or finite_number(metrics.get("max_ms"), name + " maximum") > 250):
            raise ValueError(f"{name} runtime round-trip bound failed")
    native = value["native"]
    if (native.get("process_exit") != 0
            or native.get("held_worker_entire_interval") is not True
            or finite_number(native.get("frames"), "GPU frames") < 240
            or finite_number(native.get("cpu_p99_ms"), "UI CPU p99") > 4
            or finite_number(native.get("input_to_gpu_ms"), "input-to-GPU") > 16.7):
        raise ValueError("native runtime frame/latency/owned-worker overlap bound failed")
    overlap = native["overlap"]
    if (native.get("same_app_pipeline") is not True
            or overlap.get("same_logic_owner") is not True
            or overlap.get("run_id") != value["run"]
            or overlap.get("worker_pid") != value["worker_pid"]
            or overlap.get("source_sha256") != value["source_sha256"]
            or overlap.get("overflow") is not False
            or finite_number(overlap.get("gpu_frames"), "same-owner GPU frames") < 240
            or finite_number(overlap.get("cpu_p99_ms"), "same-owner UI CPU p99") > 4
            or finite_number(overlap.get("input_to_gpu_ms"), "same-owner input GPU") > 16.7):
        raise ValueError("missing same Native/Logic/job owner overlap witness")
    independent = value["independent_root"]
    if (finite_number(independent.get("ready_ms"), "independent root ready") > 2000
            or independent.get("root") == independent.get("held_root")
            or any(independent.get(flag) is not True for flag in (
                "completed", "actual_reap", "held_worker_unchanged", "source_unchanged"))):
        raise ValueError("independent actual root progress/reap exceeds2s or lacks a held-worker witness")
    samples = [finite_number(v, "same-owner UI CPU sample") for v in overlap["cpu_samples_ms"]]
    if len(samples) != overlap["gpu_frames"]:
        raise ValueError("same-owner GPU completions do not match actual CPU samples")
    p99 = sorted(samples)[math.ceil(len(samples) * .99) - 1]
    if p99 != overlap["cpu_p99_ms"]:
        raise ValueError("same-owner UI CPU p99 differs from its retained actual samples")
    host = value["host_metrics"]
    if (host.get("schema") != "bboxy.dream.c1.host-metrics/1"
            or host.get("run_id") != value["run"]
            or host.get("child_usage_observed") is not True
            or finite_number(host.get("max_unit_cpu_ns"), "parent compute unit CPU") > 1_000_000
            or host.get("unit_overruns") != 0):
        raise ValueError("actual host compute/child-reap resource receipt failed")
    work = finite_number(host.get("work_limit_bytes"), "tracked work capacity limit")
    control = finite_number(host.get("protected_control_bytes"), "protected control capacity")
    opaque = finite_number(host.get("opaque_helper_headroom_bytes"), "opaque helper reservation")
    if work + control + opaque > 32 * 1024 * 1024 or control > 2 * 1024 * 1024:
        raise ValueError("host resource receipt exceeds fixed parent32/control2 allocation")
    for lane, limit in (("work", work), ("control", control)):
        reserved = finite_number(host.get(f"declared_{lane}_reserved_bytes"), lane + " reserved")
        live = finite_number(host.get(f"tracked_{lane}_live_bytes"), lane + " tracked live")
        peak = finite_number(host.get(f"tracked_{lane}_peak_bytes"), lane + " tracked peak")
        if live > reserved or reserved > limit or peak > limit:
            raise ValueError("tracked host requested-capacity reservation failed")
    # Requested buffer capacities and wait4 process RSS/CPU are retained honestly;
    # they do not establish total attributed heap or per-thread CPU accounting.
    records = [{"path": str(path), "sha256": sha(path), "measurements": value}]
    path, value = runtime_record(evidence, "runtime-page-tamper.json", nonce, runner_sha256)
    if (value.get("established") != ["immutable_page_tamper_api"]
            or value.get("ctl_error") != "protocol_error"
            or value.get("mcp_error") != "protocol_error"
            or value.get("restored_exact_page") is not True
            or value.get("source_unchanged") is not True
            or value.get("actual_original_process_absent") is not True
            or value.get("original_page_sha256") == value.get("altered_page_sha256")):
        raise ValueError("missing actual completed-page API tamper/reap witness")
    established.add("immutable_page_tamper_api")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})
    path, value = runtime_record(evidence, "runtime-restart.json", nonce, runner_sha256)
    if (value.get("established") != ["durable_close_restart_dedup", "restart_predecessor_quarantine"]
            or value.get("original_op_error") != "stale_context"
            or value.get("new_op_error") != "stale_context"
            or value.get("status_error") not in ("not_found", "stale_context")
            or value.get("old_logic_pid") == value.get("new_logic_pid")
            or value.get("supervisor_unchanged") is not True
            or value.get("runs_before") != 1 or value.get("runs_after") != 1
            or value.get("predecessor_state") != "quarantined"
            or value.get("predecessor_settled") is not False
            or value.get("source_unchanged") is not True
            or value.get("manifest_unchanged") is not True
            or value.get("no_new_worker") is not True
            or value.get("no_new_context_result") is not True
            or value.get("fixture_exact_orphan_cleanup") is not True):
        raise ValueError("missing actual daemon-loss/original-op/source-key quarantine witness")
    started = value["actual_start_frame"]
    if (started.get("run_id") != value["run"]
            or started.get("worker_pid") != value["worker_pid"]
            or started.get("worker_start") != value["worker_start"]
            or started.get("nonce") != value["retained_nonce"]
            or finite_number(started.get("seq"), "actual worker Start acknowledgement") < 1):
        raise ValueError("daemon loss occurred without a causal real framed-worker Start receipt")
    established.update(("durable_close_restart_dedup", "restart_predecessor_quarantine"))
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})
    path, value = runtime_record(evidence, "runtime-four-jobs.json", nonce, runner_sha256)
    jobs = value["jobs"]
    if (value.get("established") != ["four_live_jobs_controls"] or len(jobs) != 4
            or value.get("four_simultaneous_worker_incarnations") is not True
            or len({row["account"] for row in jobs}) != 1
            or len({row["root"] for row in jobs}) != 4
            or len({row["worker_pid"] for row in jobs}) != 4):
        raise ValueError("four-job witness does not retain four simultaneous same-account independent roots")
    for row in jobs:
        if row.get("actual_reap") is not True or row.get("source_unchanged") is not True:
            raise ValueError("four-job settlement/source custody witness failed")
        for key in ("status_roundtrips", "cancel_roundtrips"):
            times = row[key]
            if (times.get("samples") != 200
                    or finite_number(times.get("p99_ms"), "four-job p99") > 100
                    or finite_number(times.get("max_ms"), "four-job maximum") > 250):
                raise ValueError("four-job independent control latency bound failed")
    established.add("four_live_jobs_controls")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})
    path, value = runtime_record(evidence, "runtime-source-hold.json", nonce, runner_sha256)
    held = value["source_read_held"]
    if (value.get("established") != ["held_source_read"]
            or held.get("owner") != "snapshot" or held.get("source_fd_validated") is not True
            or held.get("fifo_read") is not True
            or value.get("retained_unsettled_until_source_release") is not True
            or value.get("actual_reap") is not True or value.get("source_unchanged") is not True
            or value.get("foreground_pause_established") is not False):
        raise ValueError("missing actual isolated source-read hold/owner settlement witness")
    for name in ("input", "message", "status", "cancel"):
        times = value["cancel_roundtrips"] if name == "cancel" else value["roundtrips"][name]
        if (times.get("samples") != 200
                or finite_number(times.get("p99_ms"), "source-hold " + name + " p99") > 100
                or finite_number(times.get("max_ms"), "source-hold " + name + " maximum") > 250):
            raise ValueError("actual held-source control latency exceeds100/250ms")
    metrics = value["host_metrics"]
    if (metrics.get("rest_skips") != 0 or metrics.get("rest_violations") != 0
            or finite_number(metrics.get("max_unit_cpu_ns"), "held-source compute unit") > 1_000_000):
        raise ValueError("held-source positive scheduling invariant failed")
    established.add("held_source_read")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-busy-wait-control.json", nonce, runner_sha256)
    metrics = value["host_metrics"]
    if (value.get("established") != ["busy_wait_causal_negative"]
            or value.get("verdict") != "schedule_budget_failed"
            or value.get("actual_reap") is not True or value.get("source_unchanged") is not True
            or finite_number(metrics.get("rest_skips"), "mutant omitted rests") <= 0
            or finite_number(metrics.get("rest_violations"), "mutant duty violations") <= 0
            or finite_number(metrics.get("min_inter_unit_gap_ns"), "mutant inter-unit gap") >= 9_000_000):
        raise ValueError("actual omitted-REST control failed to falsify scheduling invariant")
    established.add("busy_wait_causal_negative")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-over-limit.json", nonce, runner_sha256)
    if (value.get("established") != ["large_input_over_limit"]
            or value.get("error") != "resource_limit"
            or finite_number(value.get("actual_store_blob_bytes"), "actual over-limit Store blob") <= 64 * 1024 * 1024
            or value.get("manifest_published") is not False or value.get("worker_start_observed") is not False
            or any(value.get(flag) is not True for flag in ("actual_reap", "source_unchanged", "blob_metadata_unchanged"))
            or finite_number(value["host_metrics"].get("max_unit_cpu_ns"), "over-limit refusal unit") > 1_000_000):
        raise ValueError("actual over-limit source did not refuse before manifest/Start with bounded compute")
    established.add("large_input_over_limit")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-multimib-blob.json", nonce, runner_sha256)
    metrics = value["host_metrics"]
    if (value.get("established") != ["real_multimib_blob_workload"]
            or finite_number(value.get("blob_bytes"), "actual multi-MiB Store blob") < 2 * 1024 * 1024
            or any(value.get(flag) is not True for flag in ("actual_reap", "source_unchanged", "blob_unchanged"))
            or metrics.get("producer_metrics_observed") is not True
            or metrics.get("rest_skips") != 0 or metrics.get("rest_violations") != 0
            or finite_number(metrics.get("max_unit_cpu_ns"), "multi-MiB host unit") > 1_000_000
            or finite_number(metrics.get("producer_max_unit_cpu_ns"), "multi-MiB child unit") > 1_000_000
            or metrics.get("producer_overruns") != 0):
        raise ValueError("actual completed multi-MiB Store workload/resource receipt failed")
    source = Path(value["source_path"])
    blob = Path(value["blob_path"])
    root = Path(value["fixture_root"])
    if (not root.is_dir() or root.is_symlink() or root.stat().st_mode & 0o077
            or not source.is_relative_to(root) or not blob.is_relative_to(root)
            or sha(source) != value["source_sha256"] or sha(blob) != value["blob_sha256"]):
        raise ValueError("exact retained multi-MiB private producer source/hash chain is unavailable")
    established.add("real_multimib_blob_workload")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-foreground-pause.json", nonce, runner_sha256)
    if (value.get("established") != ["foreground_pause_100ms"]
            or finite_number(value.get("pause_ack_ms"), "actual turn pause acknowledgement") > 100
            or value.get("pause_state") != "paused"
            or any(value.get(flag) is not True for flag in ("same_generation", "fresh_nonfork_b",
                "actual_b_turn_started", "legacy_retained_through_pause", "source_unchanged",
                "actual_worker_reap", "resumed_to_completed", "ordinary_eof_exit_after_b"))
            or value["legacy_read_held"].get("owner") != "legacy-log-read"
            or value["legacy_read_held"].get("real_fifo_read") is not True):
        raise ValueError("actual same-seat retained-Host turn/held-legacy pause witness failed")
    established.add("foreground_pause_100ms")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-parent-overflow.json", nonce, runner_sha256)
    if (any(value.get(flag) is not True for flag in
            ("actual_hello", "overflow_refused_before_allocation", "real_exit_reaped", "not_all_times_proof"))
            or value.get("source_or_manifest_opened") is not False
            or finite_number(value.get("while_exhausted_work_live_bytes"), "exhausted actual working capacity") != 22 * 1024 * 1024
            or finite_number(value.get("while_exhausted_work_reserved_bytes"), "exhausted declared working capacity") != 22 * 1024 * 1024
            or finite_number(value.get("cached_cancel_status_max_ns"), "exhausted component control latency") > 100_000_000
            or finite_number(value.get("observed_full_host_live_heap_peak_bytes"), "exhausted full parent heap") > 32 * 1024 * 1024):
        raise ValueError("actual parent overflow did not preserve its protected control and real process owner")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    # These are full isolated-host upper bounds, including each test baseline;
    # they are neither attributed app RSS deltas nor simultaneous peak sums.
    for name, instances in (("runtime-isolated-heap.json", 1),
                            ("runtime-isolated-four-heap.json", 4)):
        path, value = runtime_record(evidence, name, nonce, runner_sha256)
        if (value.get("instances") != instances or value.get("all_jobs_completed") is not True
                or value.get("conservative_sum_of_noncoincident_peaks") is not True
                or value.get("not_all_times_proof") is not True
                or value.get("unit_cpu_excludes_test_heap_observer") is not True
                or value.get("owner_cpu_includes_test_heap_observer") is not True
                or finite_number(value.get("samples"), "isolated heap samples") <= 100
                or finite_number(value.get("source_len"), "isolated actual IR bytes") < 2 * 1024 * 1024
                or len(value.get("jobs", [])) != instances):
            raise ValueError("missing actual isolated-host memory/exit evidence")
        parent = max(finite_number(value.get("observed_full_host_live_heap_peak_bytes"), "parent live heap"),
                     finite_number(value.get("allocator_all_zones_max_touched_bytes"), "parent allocator high-water"))
        child_sum = 0
        for job in value["jobs"]:
            if any(job.get(flag) is not True for flag in
                   ("completed", "settled", "real_exit_reaped", "child_heap_measured")):
                raise ValueError("isolated memory receipt lacks a completed/reaped actual worker")
            peak = finite_number(job.get("child_heap_peak_bytes"), "full child heap peak")
            if peak <= 0 or peak > 96 * 1024 * 1024:
                raise ValueError("actual full child heap exceeds96MiB")
            child_sum += peak
            for lane, limit in (("work", 22 * 1024 * 1024), ("control", 2 * 1024 * 1024)):
                live = finite_number(job.get(f"parent_{lane}_declared_live_bytes"), lane + " declared live")
                peak = finite_number(job.get(f"parent_{lane}_declared_peak_bytes"), lane + " declared peak")
                cumulative = finite_number(job.get(f"parent_{lane}_declared_cumulative_bytes"), lane + " cumulative credits")
                if live > peak or peak > limit or cumulative < peak:
                    raise ValueError("isolated declared storage credit bound failed")
            metrics = job["host_metrics"]
            if (finite_number(metrics.get("max_unit_cpu_ns"), "isolated parent unit CPU") > 1_000_000
                    or metrics.get("unit_overruns") != 0
                    or metrics.get("producer_metrics_observed") is not True
                    or finite_number(metrics.get("producer_max_unit_cpu_ns"), "isolated child unit CPU") > 1_000_000
                    or metrics.get("producer_overruns") != 0):
                raise ValueError("isolated memory workload violated unchanged CPU gates")
        if (parent <= 0 or parent > 32 * 1024 * 1024 * instances
                or child_sum != value.get("child_heap_sum_conservative_bytes")
                or parent + child_sum != value.get("combined_parent_child_conservative_bytes")
                or parent + child_sum > 128 * 1024 * 1024 * instances):
            raise ValueError("isolated parent/child conservative memory bounds failed")
        records.append({"path": str(path), "sha256": sha(path), "measurements": value})
    established.add("attributed_parent_child_heap")
    for record in records:
        if (Path(record["path"]).name.startswith("runtime-isolated-")
                or Path(record["path"]).name == "runtime-parent-overflow.json"):
            continue
        roles = record["measurements"].get("artifact_roles", {})
        worker, seat = roles.get("worker", {}), roles.get("seat", {})
        if (worker.get("sha256") != runner_sha256 or worker.get("profile") != "release"
                or worker.get("default_features") is not True
                or worker.get("explicit_features") != [] or worker.get("test_fixtures") is not False
                or seat.get("sha256") != seat_runner_sha256 or seat.get("profile") != "dream-c1-fixture"
                or seat.get("default_features") is not True
                or seat.get("explicit_features") != ["test-fixtures"] or seat.get("test_fixtures") is not True):
            raise ValueError("runtime evidence conflates the production worker and private seat artifact")
    return {"validated": sorted(established), "records": records}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", type=Path, default=ROOT / "target")
    args = parser.parse_args()
    evidence = Path(tempfile.mkdtemp(prefix="bboxy-dream-c1-gate-"))
    os.chmod(evidence, 0o700)
    receipt = {"commands": [], "complete": False, "evidence": str(evidence)}
    code = 1
    try:
        if sys.platform != "darwin":
            raise ValueError("C1's required native frame/process witness needs the supported macOS fixture")
        harness = ROOT / "crates/bboxy450-harness"
        gitlink = subprocess.check_output(
            ["git", "rev-parse", "HEAD:crates/bboxy450-harness"], cwd=ROOT, text=True).strip()
        revision = subprocess.check_output(
            ["git", "-C", str(harness), "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        dirty = subprocess.check_output(
            ["git", "-C", str(harness), "status", "--porcelain", "--untracked-files=no"],
            cwd=ROOT, text=True).strip()
        receipt["producer_pin"] = {"gitlink": gitlink, "actual_head": revision,
                                   "tracked_source_clean": not dirty}
        if revision != gitlink or dirty:
            raise ValueError("real runner source differs from the deliberately committed producer pin")
        fixtures = Path(os.environ.get("DREAM_C1_FIXTURES", str(PINNED_FIXTURES))).resolve()
        receipt["corpus"] = corpus(fixtures)
        receipt["joined_pinned_corpus"] = fixtures == PINNED_FIXTURES.resolve()
        source_names = SOURCES + sorted(str(path.relative_to(ROOT)) for path in
                                       (ROOT / "crates/bboxy450-harness/src").rglob("*.rs"))
        receipt["source_hashes"] = {name: sha(ROOT / name) for name in source_names}
        target = args.target.resolve()
        nonce = secrets.token_hex(32)
        conflicting = sorted(name for name, value in os.environ.items() if value and
                             (name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS")
                              or name.endswith("_RUSTFLAGS")
                              or name.startswith("CARGO_PROFILE_")))
        receipt["build_overrides"] = {"conflicting_variable_names": conflicting,
                                      "fixture_profile": "dream-c1-fixture",
                                      "fixture_debug_assertions": True,
                                      "fixture_overflow_checks": True,
                                      "worker_profile": "release", "worker_test_fixtures": False}
        if conflicting:
            raise ValueError("conflicting ambient build overrides: " + ", ".join(conflicting))
        receipt["compiler"] = subprocess.check_output(["rustc", "-vV"], cwd=ROOT, text=True)
        receipt["cargo"] = subprocess.check_output(["cargo", "--version"], cwd=ROOT, text=True).strip()
        environment = dict(os.environ, CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="1",
                           NEXTEST_TEST_THREADS="1", BBOXY_TEST_RUNNER="nextest",
                           DREAM_C1_FIXTURES=str(fixtures), DREAM_C1_EVIDENCE=str(evidence),
                           DREAM_C1_GATE_NONCE=nonce)
        run(["cargo", "build", "--locked", "--jobs", "2", "--release", "-p", "bboxy450-harness",
             "--bin", "bboxy450-harness", "--target-dir", str(target)],
            environment, evidence, receipt["commands"])
        runner = target / "release/bboxy450-harness"
        receipt["real_worker_probe"] = hello(runner, evidence)
        environment["DREAM_C1_RUNNER"] = str(runner)
        run(["cargo", "build", "--locked", "--jobs", "2", "--profile", "dream-c1-fixture",
             "-p", "bboxy450-harness", "--bin", "bboxy450-harness", "--features", "test-fixtures",
             "--target-dir", str(target)], environment, evidence, receipt["commands"])
        seat_runner = target / "dream-c1-fixture/bboxy450-harness"
        receipt["private_seat_fixture"] = {"path": str(seat_runner), "sha256": sha(seat_runner),
                                           "profile": "dream-c1-fixture", "features": ["test-fixtures"]}
        environment["DREAM_C1_SEAT_RUNNER"] = str(seat_runner)
        environment["DREAM_C1_SEAT_PROFILE"] = "dream-c1-fixture"
        environment["BBOXY_HARNESS_TEST_BIN"] = str(seat_runner)

        def tests(package, selection, profile="dream-c1-fixture"):
            run(["sh", "scripts/quarantine-test.sh", "--locked", "--jobs", "2",
                 *(["--release"] if profile == "release" else ["--profile", profile]),
                 "-p", package, "--target-dir", str(target), *selection],
                environment, evidence, receipt["commands"])

        # Export using the pinned producer's actual Store/Blobs test, then read
        # those immutable bytes from isolated consumer hosts. This is a component
        # memory bound; own-seat admission remains the separate real-app gate.
        bundle = evidence / "actual-store-bundle"
        export_environment = dict(environment, DREAM_C1_EXPORT_BUNDLE=str(bundle))
        run(["sh", "scripts/quarantine-test.sh", "--locked", "--jobs", "2",
             "--release",
             "-p", "bboxy450-harness", "--target-dir", str(target), "--test", "dream_c1",
             "-E", "test(real_store_multi_mib_ir_and_typed_blob_bundle)", "--", "--nocapture"],
            export_environment, evidence, receipt["commands"])
        environment["DREAM_C1_STORE_BUNDLE"] = str(bundle)
        fork_bundle = evidence / "actual-store-fork-bundle"
        run(["sh", "scripts/quarantine-test.sh", "--locked", "--jobs", "2", "--release",
             "-p", "bboxy450-harness", "--target-dir", str(target), "--test", "dream_c1",
             "-E", "test(actual_store_fork_is_explicitly_unsupported_and_parent_rows_never_pass)",
             "--", "--nocapture"],
            dict(environment, DREAM_C1_EXPORT_FORK_BUNDLE=str(fork_bundle)),
            evidence, receipt["commands"])
        environment["DREAM_C1_FORK_BUNDLE"] = str(fork_bundle)
        tests("bboxy450-harness", ["--test", "dream_c1", "-E",
              "test(child_declared_storage_overflow_precedes_serde_and_controls_still_settle) | "
              "test(four_children_keep_independent_controls_and_individual_memory_shares)",
              "--", "--nocapture"], profile="release")
        tests("bboxy450-term-mux", ["--lib", "-E",
              "test(isolated_actual_store_worker_full_host_heap) | test(isolated_four_actual_store_workers_full_host_heap) | test(isolated_parent_work_overflow_preserves_controls_and_reaps_real_probe)", "--run-ignored", "only",
              "--", "--nocapture"])
        tests("bboxy450-term-proto", ["--test", "dream_c1", "--test", "gui_wire_golden"])
        tests("bboxy450-harness-wake-up", ["--test", "dream_c1"])
        tests("bboxy450-term-mux", ["--lib", "dream_c1", "--", "--nocapture"])
        tests("bboxy450-term-mux", ["--test", "dream_c1", "--", "--nocapture"])
        tests("bboxy450-term", ["--lib", "ctl::dream::tests", "--", "--nocapture"])
        tests("bboxy450-term", ["--bin", "bboxy450-term", "stall::", "--", "--nocapture"])
        tests("bboxy450-term", ["--test", "dream_c1", "--", "--nocapture"])
        # Reuse current compiled private native routing fixture. Its passing exit
        # is not renamed a 240-frame/heap/CPU performance measurement.
        tests("bboxy450-term", ["--test", "swarm_workdir", "--", "--ignored", "--exact",
                               "rich_detach::rich_app_routes_composer_approval_scroll_and_focus",
                               "--nocapture", "--test-threads=1"])
        if receipt["source_hashes"] != {name: sha(ROOT / name) for name in source_names}:
            raise ValueError("C1 source changed while gate ran; receipt cannot bind current source")
        if (sha(runner) != receipt["real_worker_probe"]["runner_sha256"]
                or sha(seat_runner) != receipt["private_seat_fixture"]["sha256"]):
            raise ValueError("a production worker or private seat artifact changed while the gate ran")
        receipt["runtime"] = runtime_receipts(
            evidence, nonce, receipt["real_worker_probe"]["runner_sha256"],
            receipt["private_seat_fixture"]["sha256"])
        validated = set(receipt["runtime"]["validated"])
        receipt["missing_required_runtime"] = {
            key: meaning for key, meaning in REQUIRED_RUNTIME.items() if key not in validated}
        if not receipt["joined_pinned_corpus"]:
            receipt["missing_required_runtime"]["joined_pinned_corpus"] = (
                "development override cannot establish the final deliberately pinned producer join")
        if receipt["missing_required_runtime"]:
            raise ValueError("required runtime witnesses remain unproven: "
                             + ", ".join(sorted(receipt["missing_required_runtime"])))
        receipt["complete"] = True
        receipt["scope"] = "Pinned corpus, actual runner, admitted APIs and all required runtime witnesses"
        print("Dream C1 source-bound join and required runtime witnesses passed.")
        code = 0
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
        receipt["error"] = str(error)
        print(f"Dream C1 gate failed: {error}", file=sys.stderr)
    finally:
        path = evidence / "receipt.json"
        path.write_bytes(canonical(receipt))
        os.chmod(path, 0o600)
        print(f"Dream C1 source/command receipt: {path}", flush=True)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
