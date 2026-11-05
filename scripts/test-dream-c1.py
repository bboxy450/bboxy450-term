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
import ctypes
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import re
import selectors
import secrets
import shutil
import signal
import stat
import struct
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_SHA256 = "5e3d8a602eeaa1da5cb97d0f6af0b4f331b216a02255d3dff990be84c30d6a30"
LOCK_SHA256 = "31113dd0968ace9daf43b1e610b53a3fc9b586e7ff9a707136ec25ab5e9f2c31"
# Updated only with the Oracle's reviewed producer pin/artifact handoff.
# The accepted legacy 47182320 source does not attest the native/debt profile.
APPROVED_NATIVE_BUILD = None  # (source commit, exact rustc -vV SHA256, executable SHA256)
BOOTSTRAP_VECTORS_SHA256 = "de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307"
PRIVATE_DEVELOPMENT_BUILD = (
    "bab543e2116d0bf08488fbd753a6114f30d615a4",
    "352d6857968e80a6357a926ac685b9b07f77c479aaafde33e73a53945e802909",
    "bacace4bf5e2ab4d4c91bf07e5e22c81885d967448dff87efbe6eaf6f81ca13e",
)
# Decision d93377961365255: probe=true is permanently source-free/non-startable.
PROBE_NULL_CONTEXT_APPROVED = True
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
    "crates/bboxy450-harness-wake-up/src/dream_bootstrap.rs",
    "crates/bboxy450-harness-wake-up/src/rich.rs",
    "crates/bboxy450-harness-wake-up/tests/dream_c1.rs",
    "crates/bboxy450-harness-wake-up/tests/dream_bootstrap.rs",
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
    "crates/bbt-mux/src/mcp/bridge.rs",
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


def strict_json(raw, max_integer=(1 << 53) - 1):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate JSON key")
            result[key] = value
        return result

    def integer(text):
        value = int(text)
        if value < 0 or value > max_integer:
            raise ValueError("unsafe protocol integer")
        return value

    def floating(_):
        raise ValueError("floating protocol number")

    return json.loads(raw, object_pairs_hook=pairs, parse_int=integer,
                      parse_float=floating, parse_constant=floating)


def provision_native_runner(runner, revision, compiler, *, development=False):
    """Canonical build-side provenance; the product never runs Git or rustc."""
    compiler_sha = hashlib.sha256(compiler.encode()).hexdigest()
    binary_sha = sha(runner)
    expected = PRIVATE_DEVELOPMENT_BUILD if development else APPROVED_NATIVE_BUILD
    if expected != (revision, compiler_sha, binary_sha):
        raise ValueError("native/debt source/compiler/executable has no reviewed consumer approval")
    value = {"schema": "bboxy.dream.c1.provision/1", "role": "worker",
                "metrics_profile": "c1-native-debt/1", "source_commit": revision,
             "bootstrap_profile": "dream-bootstrap/1",
             "compiler_sha256": compiler_sha, "build_profile": "release",
             "default_features": True, "explicit_features": [],
             "effective_features": ["acp-agent"], "test_fixtures": False,
             "binary_sha256": binary_sha}
    path = runner.with_suffix(".dream-c1.json")
    raw = canonical(value)
    if len(raw) > 8192:
        raise ValueError("native provision receipt exceeded its fixed bound")
    # This is build output beside the exact newly built binary, never a runtime
    # source/result publication. Replace only this canonical build artifact.
    with tempfile.NamedTemporaryFile(prefix=".dream-c1-provision-", dir=runner.parent,
                                     delete=False) as output:
        temporary = Path(output.name)
        try:
            os.chmod(temporary, 0o600)
            output.write(raw)
            output.flush()
            os.fsync(output.fileno())
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)
    return {"path": str(path), "sha256": sha(path), **value}


def prepare_development(args, evidence):
    """Prepare only the exact signed private milestone authorized by65166."""
    source = args.development_source
    artifact = args.development_worker
    if source is None or artifact is None or not source.is_absolute() or not artifact.is_absolute():
        raise ValueError("private preparation requires explicit absolute signed source and worker")
    revision, compiler_sha, binary_sha = PRIVATE_DEVELOPMENT_BUILD
    signature = subprocess.run(["git", "-C", str(source), "verify-commit", revision],
                               capture_output=True, text=True, check=False)
    if signature.returncode:
        raise ValueError("private milestone signature verification failed")
    if artifact.is_symlink() or not artifact.is_file() or sha(artifact) != binary_sha:
        raise ValueError("private milestone executable differs from signed handoff")
    compiler = (artifact.parent / "compiler.txt").read_text()
    if hashlib.sha256(compiler.encode()).hexdigest() != compiler_sha:
        raise ValueError("private artifact compiler receipt changed")
    # Read immutable Git objects even when the producer continues a correction.
    bootstrap = subprocess.check_output(["git", "-C", str(source), "show", revision + ":src/dream/bootstrap.rs"])
    if hashlib.sha256(bootstrap).hexdigest() != "1c69a2d42142cf046075d97148e2317d0c9594292c24da411be0a44c5225e555":
        raise ValueError("private bootstrap source differs from signed boundary")
    vectors = source / "tests/fixtures/dream-bootstrap/vectors.json"
    if sha(vectors) != BOOTSTRAP_VECTORS_SHA256:
        raise ValueError("private canonical bootstrap corpus differs from signed boundary")
    target = args.target.resolve()
    target.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="dream-c1-development-bab543e2-", dir=target))
    os.chmod(directory, 0o700)
    worker = directory / "bboxy450-harness"
    shutil.copyfile(artifact, worker)
    os.chmod(worker, 0o700)
    if sha(artifact) != binary_sha or sha(worker) != binary_sha:
        raise ValueError("private worker changed while being prepared")
    provision = provision_native_runner(worker, revision, compiler, development=True)
    probe = hello(worker, evidence, provision)
    return {"development_only": True, "consumer_verification_complete": False,
            "production_pin_changed": False, "source_commit": revision,
            "compiler": compiler, "provision": provision, "probe": probe,
            "source_tree": str(source), "bootstrap_vectors_sha256": sha(vectors),
            "known_exclusion": "Producer intermediate native/control clock sticky-unknown correction is pending; no final conformance"}


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


ULID_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"


def ulid_bytes(value):
    if not isinstance(value, str) or len(value) != 26:
        raise ValueError("invalid bootstrap ULID")
    number = 0
    for letter in value:
        if letter not in ULID_ALPHABET:
            raise ValueError("noncanonical bootstrap ULID")
        number = number * 32 + ULID_ALPHABET.index(letter)
    if number >= 1 << 128:
        raise ValueError("overflow bootstrap ULID")
    return number.to_bytes(16, "big")


def fresh_ulid():
    millis = time.time_ns() // 1_000_000
    if not 0 <= millis < 1 << 48:
        raise ValueError("bootstrap birth clock overflow")
    number = (millis << 80) | int.from_bytes(secrets.token_bytes(10), "big")
    return "".join(ULID_ALPHABET[(number >> (5 * shift)) & 31] for shift in range(25, -1, -1))


def bootstrap_record(inputs, birth, deadline):
    """Adapted from bboxy450-harness src/dream/bootstrap.rs's fixed layout.

    Origin: https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/bootstrap.rs
    Policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
    The producer's immutable vectors decide conformance; this creates no corpus.
    """
    required = {"profile", "purpose", "probe", "op_id", "run_id", "nonce",
                "incarnation", "context", "worker_sha256"}
    if not isinstance(inputs, dict) or set(inputs) != required or inputs["profile"] != "dream-bootstrap/1":
        raise ValueError("incomplete bootstrap binding")
    purpose = {"inspect-c1": 1, "read-prepare": 2}.get(inputs["purpose"])
    incarnation = inputs["incarnation"]
    if (purpose is None or type(inputs["probe"]) is not bool
            or type(incarnation) is not int or not 0 < incarnation < 1 << 64
            or type(deadline) is not int or not 0 < deadline < 1 << 64):
        raise ValueError("invalid bootstrap scalar")
    ulid_bytes(inputs["op_id"])
    worker_sha = inputs["worker_sha256"]
    if not isinstance(worker_sha, str) or re.fullmatch("[0-9a-f]{64}", worker_sha) is None:
        raise ValueError("invalid bootstrap worker digest")
    context = inputs["context"]
    if inputs["probe"]:
        if context is not None:
            raise ValueError("probe binding requires explicit context:null")
    else:
        identifiers = {"account", "workspace", "root", "provider"}
        ulids = {"seat", "swarm"}
        numbers = {"account_epoch", "generation", "grant_epoch", "swarm_revision",
                   "workspace_revision", "root_revision", "provider_epoch"}
        if not isinstance(context, dict) or set(context) != identifiers | ulids | numbers:
            raise ValueError("work binding requires complete public Context")
        for name in identifiers:
            value = context[name]
            if (not isinstance(value, str) or not 0 < len(value.encode("utf-8")) <= 256
                    or "\0" in value):
                raise ValueError("invalid bootstrap Context identifier")
        for name in ulids:
            ulid_bytes(context[name])
        for name in numbers:
            value = context[name]
            if type(value) is not int or not 0 <= value < 1 << 53:
                raise ValueError("unsafe bootstrap Context number")
    binding = hashlib.sha256(canonical(inputs)).digest()
    raw = (b"BBDBOOT1" + bytes((purpose, int(inputs["probe"]))) + bytes(6)
           + ulid_bytes(birth) + ulid_bytes(inputs["run_id"]) + ulid_bytes(inputs["nonce"])
           + binding + struct.pack("<QQQ", incarnation, 1, deadline) + bytes(8))
    if len(raw) != 128:
        raise ValueError("bootstrap mirror changed fixed record length")
    return raw, binding.hex()


def bootstrap_corpus(fixtures):
    root = fixtures.parent / "dream-bootstrap"
    vector_path = root / "vectors.json"
    if BOOTSTRAP_VECTORS_SHA256 is None or sha(vector_path) != BOOTSTRAP_VECTORS_SHA256:
        raise ValueError("missing/revised signed canonical bootstrap vectors")
    vectors = strict_json(vector_path.read_bytes(), max_integer=(1 << 64) - 1)
    cases = vectors["cases"]
    bindings = vectors["binding_cases"]
    if (vectors["profile"] != "dream-bootstrap/1" or vectors["record_bytes"] != 128
            or vectors["case_count"] != len(cases) or len(cases) != 34
            or vectors["binding_case_count"] != len(bindings) or len(bindings) != 8):
        raise ValueError("bootstrap case inventory disagrees")

    def decoded(raw):
        if (len(raw) != 128 or raw[:8] != b"BBDBOOT1" or raw[8] not in (1, 2)
                or raw[9] not in (0, 1) or any(raw[10:16]) or any(raw[120:128])):
            raise ValueError("invalid bootstrap record")
        incarnation, sequence, deadline = struct.unpack_from("<QQQ", raw, 96)
        if incarnation == 0 or sequence != 1 or deadline == 0:
            raise ValueError("invalid bootstrap record numbers")
        return (raw[8], raw[9], raw[16:32], raw[32:48], raw[48:64], raw[64:96],
                incarnation, sequence, deadline)

    def environment(token):
        if not isinstance(token, str) or not token.isascii() or len(token) > 128:
            raise ValueError("invalid bootstrap environment")
        fields = token.split(":")
        if (len(fields) != 4 or fields[0] != "1" or not fields[1]
                or any(c not in "0123456789" for c in fields[1])
                or not 3 <= int(fields[1]) <= 65535
                or re.fullmatch("[0-9a-f]{64}", fields[3]) is None):
            raise ValueError("invalid bootstrap environment fields")
        return int(fields[1]), ulid_bytes(fields[2]), bytes.fromhex(fields[3])

    observed = {}
    classes = {}
    for case in cases:
        relative = Path(case["file"])
        if (relative.is_absolute() or len(relative.parts) != 2
                or relative.parts[0] not in ("valid", "invalid")
                or any(part in ("", ".", "..") for part in relative.parts)):
            raise ValueError("foreign bootstrap fixture path")
        path = root / relative
        raw = path.read_bytes()
        if len(raw) > 1024:
            raise ValueError("overbound bootstrap fixture")
        if sha(path) != case["record_sha256"]:
            raise ValueError("canonical bootstrap bytes changed: " + case["name"])
        authority = case["authority"]
        binding = canonical(authority["inputs"])
        if (binding.decode() != authority["binding_canonical_json"]
                or hashlib.sha256(binding).hexdigest() != authority["binding_sha256"]):
            raise ValueError("bootstrap binding canonicalization disagrees")
        encoded, digest = bootstrap_record(authority["inputs"], authority["birth"],
                                           vectors["static_deadline_ns"])
        if digest != authority["binding_sha256"]:
            raise ValueError("consumer bootstrap binding differs from producer")
        try:
            record = decoded(raw)
        except ValueError:
            record = None
        try:
            token = environment(case["environment"])
        except ValueError:
            token = None
        binary = "valid" if record is not None else "invalid"
        env = "valid" if token is not None else "invalid"
        if binary != case["expect"]["binary_format"] or env != case["expect"]["environment_format"]:
            raise ValueError("consumer bootstrap format verdict differs: " + case["name"])
        if record is not None and token is not None:
            expected = decoded(encoded)
            matches = (record[:-1] == expected[:-1]
                       and token == (authority["fd"], expected[2], expected[5]))
            identity = "match" if matches else "mismatch"
            if matches and (raw != encoded or record[-1] != vectors["static_deadline_ns"]):
                raise ValueError("consumer bootstrap encoding differs: " + case["name"])
        else:
            identity = "not_checked"
        if identity != case["expect"]["identity"]:
            raise ValueError("consumer bootstrap identity verdict differs: " + case["name"])
        verdict = (binary, env, identity)
        classes[verdict] = classes.get(verdict, 0) + 1
        if case["name"] in observed:
            raise ValueError("duplicate bootstrap record name")
        observed[case["name"]] = case["record_sha256"]
    if classes != {("invalid", "valid", "not_checked"): 13,
                   ("valid", "invalid", "not_checked"): 9,
                   ("valid", "valid", "mismatch"): 8,
                   ("valid", "valid", "match"): 4}:
        raise ValueError("bootstrap required negative classes changed")
    binding_observed = {}
    accepted = 0
    for case in bindings:
        binding = canonical(case["inputs"])
        if (binding.decode() != case["binding_canonical_json"]
                or hashlib.sha256(binding).hexdigest() != case["binding_sha256"]):
            raise ValueError("bootstrap context vector canonicalization differs")
        try:
            _, digest = bootstrap_record(case["inputs"], cases[0]["authority"]["birth"],
                                         vectors["static_deadline_ns"])
        except (ValueError, KeyError, TypeError):
            valid = False
        else:
            valid = True
            if digest != case["binding_sha256"]:
                raise ValueError("consumer accepted binding hash differs")
        if ("valid" if valid else "invalid") != case["expect"]:
            raise ValueError("consumer context/null binding verdict differs: " + case["name"])
        if case["name"] in binding_observed:
            raise ValueError("duplicate bootstrap binding name")
        binding_observed[case["name"]] = {"sha256": case["binding_sha256"], "accepted": valid}
        accepted += valid
    if accepted != 2:
        raise ValueError("bootstrap context positive/negative classes changed")
    # Producer's independent checker covers malformed bytes/environment and
    # identity mismatches. It never regenerates or repairs the authority corpus.
    result = subprocess.run([sys.executable, "-I", str(root / "check.py")], cwd=ROOT,
                            env={}, capture_output=True, text=True, check=False)
    if result.returncode:
        raise ValueError("canonical bootstrap checker refused: " + result.stdout + result.stderr)
    return {"vectors_sha256": sha(vector_path), "cases": observed, "binding_cases": binding_observed,
            "producer_checker_exit": result.returncode, "consumer_valid_encoding_matches": True}


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


def admitted_api_witnesses(command):
    """Consume actual per-test verdicts, not a blanket claim from one exit."""
    required = [
        "dream_c1_real_admitted_mcp_and_gui_controls_make_zero_native_or_signing_calls",
        "dream_c1_structured_only_success_page_and_foreign_refusal_use_advertised_schema",
        "dream_c1_actual_lost_cancel_reply_keeps_original_unknown_recovery_identity",
        "dream_c1_near_page_bound_historical_text_never_becomes_host_guidance",
        "dream_c1_public_head_revoke_restore_aba_cannot_reexpose_retained_page",
        "dream_c1_actual_issuer_cutover_rejects_retained_authority_without_credentials",
        "dream_c1_actual_control_routes_reject_expired_public_grant_without_credentials",
        "dream_c1_actual_mute_writer_and_unmute_aba_do_not_revive_old_grant_page",
        "dream_c1_actual_root_workdir_aba_and_provider_revision_reject_old_pages",
        "dream_c1_held_provider_selection_refuses_real_mcp_gui_controls_within_250ms",
        "dream_c1_held_provider_selection_refuses_actual_page_owner_before_next_unit",
        "dream_c1_held_provider_refuses_ordinary_mcp_and_gui_read_admission",
        "dream_c1_held_provider_suppresses_actual_successful_ordinary_gui_completion",
    ]
    output = re.sub(r"\x1b\[[0-9;]*m", "", Path(command["log"]).read_text())
    for name in required:
        # --nocapture permits test stderr between libtest's name and its `ok`.
        # Consume nextest's complete per-test verdict instead of guessing from
        # an interleaved libtest line or the command's blanket exit code.
        verdict = (r"^\s*PASS\s+\[[^\]]+\]\s+\(\s*\d+/\d+\)\s+"
                   r"bboxy450-term-mux\s+logic::dream_public_api_tests::"
                   + re.escape(name) + r"\s*$")
        if not re.search(verdict, output, flags=re.MULTILINE):
            raise ValueError("missing executed admitted API witness: " + name)
    return {"log": command["log"], "sha256": sha(Path(command["log"])), "exit_code": command["exit_code"],
            "passed_tests": required, "scope": "actual admitted MCP/GUI authority and recovery; seeded immutable pages, separate real-worker tests"}


def hello(runner, evidence, native_provision):
    """Private source-free smoke; Rust process tests measure the product parent."""
    if (not runner.is_file() or not runner.is_absolute()
            or native_provision.get("bootstrap_profile") != "dream-bootstrap/1"
            or native_provision.get("binary_sha256") != sha(runner)):
        raise ValueError("missing pre-birth bootstrap-capable runner proof")
    if not PROBE_NULL_CONTEXT_APPROVED:
        raise ValueError("startup probe context disposition is not recorded")
    birth, run_id, nonce, operation = (fresh_ulid() for _ in range(4))
    incarnation = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    inputs = {"profile": "dream-bootstrap/1", "purpose": "inspect-c1", "probe": True,
              "op_id": operation, "run_id": run_id, "nonce": nonce,
              "incarnation": incarnation, "context": None,
              "worker_sha256": native_provision["binary_sha256"]}
    binding = hashlib.sha256(canonical(inputs)).hexdigest()
    libc_write = ctypes.CDLL(None, use_errno=True).write
    libc_write.argtypes = (ctypes.c_int, ctypes.c_void_p, ctypes.c_size_t)
    libc_write.restype = ctypes.c_ssize_t
    with tempfile.TemporaryDirectory(prefix="dream-c1-probe-") as own:
        outer = Path(own)
        os.chmod(outer, 0o700)
        job = outer / run_id
        job.mkdir(mode=0o700)
        (job / "tmp").mkdir(mode=0o700)
        read_gate, lease = os.pipe()
        process = None
        raw = bytearray()
        stderr_raw = bytearray()
        first_stderr_ns = None
        permit_attempted = False
        readiness_failed = False
        parent_cpu = debt = control_cpu = written = None
        try:
            if not 3 <= read_gate <= 65535:
                raise ValueError("bootstrap inherited descriptor outside fixed range")
            gate_identity = os.fstat(read_gate)
            writer_identity = os.fstat(lease)
            # Darwin's two endpoints have distinct inode identities. os.pipe
            # creates the linked pair; retain each original endpoint separately.
            if (not stat.S_ISFIFO(gate_identity.st_mode) or not stat.S_ISFIFO(writer_identity.st_mode)
                    or gate_identity.st_uid != os.geteuid() or writer_identity.st_uid != os.geteuid()
                    or fcntl.fcntl(read_gate, fcntl.F_GETFL) & os.O_ACCMODE != os.O_RDONLY
                    or fcntl.fcntl(lease, fcntl.F_GETFL) & os.O_ACCMODE != os.O_WRONLY):
                raise ValueError("bootstrap endpoint type/ownership/access differs")
            os.set_blocking(lease, False)
            token = f"1:{read_gate}:{birth}:{binding}"
            if len(token.encode("ascii")) > 128:
                raise ValueError("bootstrap environment exceeded fixed bound")

            def background():
                os.setpriority(os.PRIO_PROCESS, 0, 10)

            # This Python smoke controller is not the native process_create
            # measurement. The admitted Rust parent has its own exact receipts.
            cpu_start = time.thread_time_ns()
            process = subprocess.Popen(
                [str(runner), "dream", "inspect-c1", "--job-dir", str(job)],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                env={"LANG": "C.UTF-8", "TMPDIR": str(job / "tmp"),
                     "BBOXY_DREAM_BOOTSTRAP": token}, cwd=job,
                pass_fds=(read_gate,), start_new_session=True, preexec_fn=background,
            )
            returned_at = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            readiness = returned_at + 250_000_000
            os.close(read_gate)
            read_gate = None
            parent_cpu = time.thread_time_ns() - cpu_start
            ended_at = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            debt = 9 * max(1_000_000, parent_cpu)
            if parent_cpu < 0 or ended_at + debt >= readiness or readiness >= 1 << 64:
                raise ValueError("private probe cannot repay parent startup before readiness")
            record, record_binding = bootstrap_record(inputs, birth, readiness)
            if record_binding != binding:
                raise ValueError("frozen bootstrap binding changed before grant")
            fixture = evidence / "hello.bootstrap.json"
            fixture.write_bytes(canonical({"argv": [str(runner), "dream", "inspect-c1", "--job-dir", str(job)],
                                           "environment": {"LANG": "C.UTF-8", "TMPDIR": str(job / "tmp"),
                                                           "BBOXY_DREAM_BOOTSTRAP": token},
                                           "binding_inputs": inputs, "record_hex": record.hex(),
                                           "spawn_return_clock_monotonic_ns": returned_at,
                                           "original_readiness_clock_monotonic_ns": readiness}))
            os.chmod(fixture, 0o600)
            permit = ctypes.create_string_buffer(record, 128)
            # All command/pipe/record preparation finishes before this final
            # observed debt. No work-credit accrues from earlier IO or idle.
            parent_cpu = time.thread_time_ns() - cpu_start
            ended_at = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            debt = 9 * max(1_000_000, parent_cpu)
            due = ended_at + debt
            if due >= readiness:
                raise ValueError("private probe grant debt exceeds original readiness")
            while time.clock_gettime_ns(time.CLOCK_MONOTONIC) < due:
                time.sleep(max(0, due - time.clock_gettime_ns(time.CLOCK_MONOTONIC)) / 1_000_000_000)
            # Exactly one syscall. ctypes avoids Python's automatic EINTR retry.
            permit_attempted = True
            control_start = time.thread_time_ns()
            written = libc_write(lease, permit, 128)
            raw_errno = ctypes.get_errno()
            control_cpu = time.thread_time_ns() - control_start
            if written != 128 or control_cpu < 0 or control_cpu > 1_000_000:
                raise ValueError(f"private probe permit failed ({written}, errno {raw_errno})")
            with selectors.DefaultSelector() as ready:
                ready.register(process.stdout, selectors.EVENT_READ, "stdout")
                ready.register(process.stderr, selectors.EVENT_READ, "stderr")
                while not raw.endswith(b"\n"):
                    remaining = (readiness - time.clock_gettime_ns(time.CLOCK_MONOTONIC)) / 1_000_000_000
                    events = ready.select(max(0, remaining)) if remaining > 0 else []
                    if not events:
                        readiness_failed = True
                        raise ValueError("real gated worker missed original250ms Hello window")
                    for key, _ in events:
                        if key.data == "stderr":
                            chunk = os.read(process.stderr.fileno(), 4096)
                            if not chunk:
                                ready.unregister(process.stderr)
                                continue
                            if first_stderr_ns is None:
                                first_stderr_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC) - returned_at
                            stderr_raw.extend(chunk)
                            if len(stderr_raw) > 16 * 1024:
                                raise ValueError("real worker exceeded bounded stderr")
                        else:
                            chunk = os.read(process.stdout.fileno(), min(4096, FRAME + 1 - len(raw)))
                            if not chunk:
                                raise ValueError("real gated worker exited without Hello")
                            raw.extend(chunk)
                            if len(raw) > FRAME or b"\n" in raw[:-1]:
                                raise ValueError("real worker emitted extra/oversized Hello")
            value = strict_json(bytes(raw[:-1]))
            expected = {"protocol": "bboxy.dream.c1/1", "type": "hello",
                        "algorithm": "d0-c1/1", "schema_sha256": SCHEMA_SHA256,
                        "ir_major": 1, "ir_minor_max": 3}
            if value != expected:
                raise ValueError("real built worker Hello is incompatible with approved schema")
            os.close(lease)  # Child retired its read end after paying bootstrap debt.
            lease = None
            process.stdin.close()  # Probe never sends Start or receives source custody.
            try:
                code = process.wait(timeout=4)
            except subprocess.TimeoutExpired as error:
                raise ValueError("Hello probe failed to settle on stdin EOF") from error
            if code != 0:
                raise ValueError(f"real gated probe did not settle successfully: {code}")
            stderr = bytes(stderr_raw) + process.stderr.read(16 * 1024 + 1)
            if len(stderr) > 16 * 1024:
                raise ValueError("real worker exceeded bounded stderr")
            (evidence / "hello.stderr").write_bytes(stderr)
            os.chmod(evidence / "hello.stderr", 0o600)
            return {"runner": str(runner), "runner_sha256": sha(runner),
                    "hello": value, "eof_exit_code": code, "actual_reap": True,
                    "inspection_started": False, "bootstrap_profile": "dream-bootstrap/1",
                    "probe": True, "run_id": run_id, "nonce": nonce, "op_id": operation,
                    "birth": birth, "incarnation": incarnation, "binding": binding,
                    "pipe_dev": gate_identity.st_dev, "pipe_ino": gate_identity.st_ino,
                    "writer_pipe_dev": writer_identity.st_dev, "writer_pipe_ino": writer_identity.st_ino,
                    "permit_attempts": int(permit_attempted), "permit_bytes": written,
                    "private_controller_startup_cpu_ns": parent_cpu,
                    "private_controller_debt_ns": debt, "permit_control_cpu_ns": control_cpu,
                    "scope": "private source-free gated smoke; native parent proof is the Rust API/process witness"}
        finally:
            for descriptor in (lease, read_gate):
                if descriptor is not None:
                    os.close(descriptor)
            if process is not None:
                if readiness_failed and process.poll() is None:
                    # The 250ms verdict is already failed. Revoke the lease and
                    # input before observing this exact owned process; a later
                    # marker/exit cannot turn it into a passing readiness run.
                    process.stdin.close()
                    diagnostic = evidence / "hello.failed-readiness.sample.txt"
                    try:
                        with diagnostic.open("wb") as output:
                            subprocess.run(["/usr/bin/sample", str(process.pid), "1", "1"],
                                           stdin=subprocess.DEVNULL, stdout=output,
                                           stderr=subprocess.STDOUT, timeout=3, check=False)
                        os.chmod(diagnostic, 0o600)
                        process.wait(timeout=2)
                    except (OSError, subprocess.TimeoutExpired):
                        pass
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=2)
                for stream in (process.stdin, process.stdout, process.stderr):
                    if stream is process.stderr and not stream.closed:
                        diagnostic = bytes(stderr_raw) + stream.read(16 * 1024 + 1)
                        path = evidence / "hello.stderr"
                        if not path.exists():
                            path.write_bytes(diagnostic)
                            os.chmod(path, 0o600)
                    stream.close()
                lifecycle = {"scope":"private source-free smoke, not native-parent conformance",
                             "op_id":operation,"run_id":run_id,"nonce":nonce,"birth":birth,
                             "binding":binding,"worker_sha256":native_provision["binary_sha256"],
                             "pid":process.pid,"actual_reap":process.returncode is not None,
                             "returncode":process.returncode,"permit_attempted":permit_attempted,
                             "permit_bytes":written,"controller_cpu_ns":parent_cpu,
                             "controller_debt_ns":debt,"permit_control_cpu_ns":control_cpu,
                             "stdout_frame_bytes":len(raw)}
                lifecycle["original_readiness_failed"] = readiness_failed
                lifecycle["first_stderr_after_spawn_return_ns"] = first_stderr_ns
                path = evidence / "hello.lifecycle.json"
                path.write_bytes(canonical(lifecycle))
                os.chmod(path, 0o600)


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
    "actual_store_fork_frontier": "actual unchanged Store fork bytes refuse at the header before inherited rows or inspection work",
}


def finite_number(value, label):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
        raise ValueError(f"missing/non-finite runtime measurement: {label}")
    return value


def known_duration(value, label):
    # u64::MAX is the native unavailable sentinel, never a measured duration.
    # Exact integer diagnostics stay within the protocol-safe numeric domain.
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= (1 << 53) - 1:
        raise ValueError("unknown/unsafe duration: " + label)
    return value


def latency_bound(metrics, label):
    if (metrics.get("samples") != 200
            or finite_number(metrics.get("p99_ms"), label + " p99") > 100
            or finite_number(metrics.get("max_ms"), label + " maximum") > 250):
        raise ValueError(label + " lacks200 calls within unchanged latency bounds")


def owned_running_proof(value, running_key):
    owned = value["owned_hello_receipt"]
    run = value.get("run", value.get("run_id"))
    if (owned.get("run_id") != run or owned.get("worker_pid") != value.get("worker_pid")
            or owned.get("effect_stage") != "spawned_hello_validated"
            or owned.get("work_started") is not False or owned.get("phase") != "validate"
            or finite_number(owned.get("seq"), "owned Hello receipt sequence") != 0
            or ("worker_start" in value and owned.get("worker_start") != value["worker_start"])):
        raise ValueError("owned pre-Start receipt lacks its exact process/stage identity")
    running = value[running_key]
    if (running.get("run_id") != run or running.get("state") != "running"
            or running.get("settled") is not False):
        raise ValueError("work witness did not observe a validated actual worker Progress")
    return owned


def authority_owner_bounds(metrics):
    for name in ("authority_setup_cpu_ns", "authority_monitor_setup_cpu_ns", "authority_monitor_max_cpu_ns"):
        if finite_number(metrics.get(name), name) > 1_000_000:
            raise ValueError("authority helper exceeded1ms: " + name)
    if metrics.get("authority_monitor_overruns") != 0:
        raise ValueError("authority monitor CPU overrun")


def paced_owner_bounds(metrics):
    """Whole reactor receipts complement compute units and retained cleanup."""
    for name in ("max_unit_cpu_ns", "max_cleanup_unit_cpu_ns", "reactor_max_cpu_ns",
                 "dispatch_guard_max_cpu_ns", "worker_start_cpu_ns",
                 "codec_prepare_max_cpu_ns",
                 "checkpoint_path_max_cpu_ns", "checkpoint_config_max_cpu_ns",
                 "checkpoint_open_max_cpu_ns", "checkpoint_save_max_cpu_ns",
                 "source_copy_unit_max_cpu_ns", "blob_copy_unit_max_cpu_ns",
                 "producer_max_cleanup_unit_cpu_ns", "producer_reactor_max_cpu_ns", "producer_control_max_cpu_ns"):
        if finite_number(metrics.get(name), name) > 1_000_000:
            raise ValueError("owner CPU unit exceeded1ms: " + name)
    for name in ("unit_overruns", "cleanup_overruns", "reactor_overruns", "reactor_clock_failures",
                 "dispatch_guard_overruns", "native_dispatch_overruns", "checkpoint_overruns",
                 "collection_capacity_refusals", "rest_skips", "rest_violations"):
        if metrics.get(name) != 0:
            raise ValueError("owner/cleanup pacing failed: " + name)
    for name in ("reactor_units", "work_unit_count", "cleanup_units"):
        if finite_number(metrics.get(name), name) <= 1:
            raise ValueError("missing exercised owner units: " + name)
    for name in ("reactor_min_gap_ns", "min_inter_unit_gap_ns"):
        if finite_number(metrics.get(name), name) < 9_000_000:
            raise ValueError("owner rest gap fell below9ms: " + name)
    for wall_name, cpu_name in (("reactor_rest_wall_ns", "reactor_rest_cpu_ns"),
                                ("rest_wait_ns", "rest_wait_cpu_ns")):
        wall = finite_number(metrics.get(wall_name), wall_name)
        cpu = finite_number(metrics.get(cpu_name), cpu_name)
        if wall <= 0 or cpu >= wall:
            raise ValueError("owner rest was not an actual nonbusy wait")
    cpu = finite_number(metrics.get("reactor_cpu_ns"), "reactor CPU") + metrics["reactor_rest_cpu_ns"]
    wall = finite_number(metrics.get("reactor_wall_ns"), "reactor wall") + metrics["reactor_rest_wall_ns"]
    if cpu * 10 > wall:
        raise ValueError("whole reactor exceeded10% measured compute/rest duty")
    native_debt_bounds(metrics)


def native_debt_bounds(metrics):
    """65086 class-specific observations; no scalar grants a native allowance."""
    if (metrics.get("native_metrics_profile") != "c1-native-debt/1"
            or metrics.get("cost_known") is not True
            or metrics.get("work_debt_poisoned") is not False
            or metrics.get("work_debt_remaining_ns") != 0
            or metrics.get("metrics_capture_cutoff") != "after_reap_settlement_before_terminal_metrics_write"):
        raise ValueError("missing known-cost parent native/debt terminal profile")
    for name in ("native_overruns", "native_clock_failures", "work_debt_admission_violations"):
        if metrics.get(name) != 0:
            raise ValueError("native/debt failure: " + name)
    if (metrics.get("native_process_calls") != 1
            or not 1 <= finite_number(metrics.get("native_publish_calls"), "parent publication count") <= 2):
        raise ValueError("parent native calls differ from their fixed admission caps")
    for tag in ("process", "publish"):
        if (finite_number(metrics.get("native_" + tag + "_max_cpu_ns"), tag + " CPU") > 2_000_000
                or finite_number(metrics.get("native_" + tag + "_max_wall_ns"), tag + " wall") > 25_000_000):
            raise ValueError("parent tagged native call exceeded its allocation: " + tag)
    if (finite_number(metrics.get("native_app_max_cpu_ns"), "native surrounding app") > 1_000_000
            or finite_number(metrics.get("inclusive_max_cpu_ns"), "parent inclusive CPU") > 3_000_000
            or known_duration(metrics.get("worker_spawn_cpu_ns"), "prepared spawn native CPU") > 2_000_000):
        raise ValueError("parent native inclusive or surrounding application allocation exceeded")
    if (finite_number(metrics.get("work_debt_spans"), "debt spans") < 1
            or finite_number(metrics.get("work_debt_charged_ns"), "charged debt") < 9_000_000):
        raise ValueError("missing exercised parent post-span debt")
    if (metrics.get("producer_native_metrics_profile") != "c1-native-debt/1"
            or metrics.get("producer_terminal_cutoff") != "joined_final_control_pre_telemetry"
            or metrics.get("native_metrics_authorized") is not True
            or metrics.get("native_provision_development") is not False
            or metrics.get("native_provision_control_only") is not False
            or metrics.get("producer_native_profile_verified") is not True
            or metrics.get("producer_cost_known") != 1
            or metrics.get("producer_terminal_remaining_debt_ns") != 0):
        raise ValueError("missing matching producer native/debt terminal proof")
    for tag, cap in (("compute_owner_create", 1), ("publish_noreplace", 4097)):
        prefix = "producer_native_" + tag + "_"
        count = finite_number(metrics.get(prefix + "count"), tag + " count")
        if not 1 <= count <= cap or (tag == "compute_owner_create" and count != 1):
            raise ValueError("producer native count differs from fixed cap: " + tag)
        for field, bound in (("max_cpu_ns", 2_000_000), ("max_wall_ns", 25_000_000),
                             ("inclusive_max_cpu_ns", 3_000_000), ("app_max_cpu_ns", 1_000_000)):
            if finite_number(metrics.get(prefix + field), tag + " " + field) > bound:
                raise ValueError("producer native class allocation exceeded: " + tag + " " + field)
        if metrics.get(prefix + "failures_with_effect") != 0:
            raise ValueError("successful producer retains a failed native effect")
    for field in ("app_max_cpu_ns", "control_max_cpu_ns"):
        if finite_number(metrics.get("producer_terminal_" + field), field) > 1_000_000:
            raise ValueError("producer strict application/control unit exceeded1ms")
    captured = finite_number(metrics.get("producer_terminal_captured_max_cpu_ns"), "descriptor cutoff maximum")
    inclusive = finite_number(metrics.get("producer_terminal_inclusive_max_cpu_ns"), "terminal inclusive maximum")
    if captured > inclusive or inclusive > 3_000_000:
        raise ValueError("producer descriptor cutoff or full terminal maximum disagrees")
    for field in ("input", "protocol", "settlement", "output"):
        name = "producer_control_parts_" + field + "_max_cpu_ns"
        if finite_number(metrics.get(name), name) > 1_000_000:
            raise ValueError("producer control part exceeded1ms: " + field)
    # These observations overlap; do not add process/owner/span CPU totals.
    for field in ("span_cpu_ns", "owner_thread_cpu_ns", "process_user_cpu_ns", "process_system_cpu_ns",
                  "wall_ns", "debt_total_ns", "rest_observed_ns"):
        known_duration(metrics.get("producer_terminal_" + field), "producer " + field)


def observed_parent_heap(value):
    live = finite_number(value.get("observed_full_host_live_heap_peak_bytes"), "full parent sampled heap")
    if value.get("allocator_max_touched_supported") is True:
        return max(live, finite_number(value.get("allocator_all_zones_max_touched_bytes"), "allocator high-water"))
    if (value.get("allocator_max_touched_supported") is not False
            or value.get("allocator_all_zones_max_touched_bytes") is not None):
        raise ValueError("unsupported allocator high-water was not labelled honestly")
    return live


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
    owned_running_proof(value, "public_running_before_hold")
    authority_owner_bounds(host)
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
    started = owned_running_proof(value, "public_running_before_restart")
    if (started.get("run_id") != value["run"]
            or started.get("worker_pid") != value["worker_pid"]
            or started.get("worker_start") != value["worker_start"]
            or started.get("nonce") != value["retained_nonce"]):
        raise ValueError("daemon loss occurred without retained owned Hello and validated Progress")
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
        owned_running_proof(row, "public_running_before_hold")
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

    path, value = runtime_record(evidence, "runtime-production-cpu.json", nonce, runner_sha256)
    owned_running_proof(value, "public_running_before_controls")
    metrics = value["host_metrics"]
    authority_owner_bounds(metrics)
    paced_owner_bounds(metrics)
    if (value.get("established") != ["production_cpu_progress_controls"]
            or value.get("protocol_fake") is not False
            or finite_number(value.get("source_bytes"), "production IR bytes") < 2 * 1024 * 1024
            or finite_number(value.get("child_cpu_during_controls_ns"), "actual child CPU overlap") <= 0
            or value.get("actual_reap") is not True or value.get("source_unchanged") is not True
            or metrics.get("producer_metrics_observed") is not True
            or finite_number(metrics.get("producer_compute_cpu_ns"), "producer compute CPU") <= 0
            or finite_number(metrics.get("producer_max_unit_cpu_ns"), "producer unit CPU") > 1_000_000
            or finite_number(metrics.get("max_unit_cpu_ns"), "parent unit CPU") > 1_000_000
            or metrics.get("producer_overruns") != 0 or metrics.get("unit_overruns") != 0
            or metrics.get("rest_skips") != 0 or metrics.get("rest_violations") != 0):
        raise ValueError("actual production CPU/progress workload lacks bounded overlap/reap")
    for lane in ("input", "message", "status"):
        latency_bound(value["controls"][lane], "production " + lane)
    established.add("production_cpu_progress_controls")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-protocol-progress-flood.json", nonce, runner_sha256)
    owned_running_proof(value, "public_running_before_controls")
    fault = value["fault_receipt"]
    if (value.get("established") != ["protocol_fake_progress_flood_controls"]
            or value.get("protocol_fake") is not True or value.get("production_child_executed") is not False
            or not isinstance(value.get("fault_sha256"), str) or len(value["fault_sha256"]) != 64
            or value["fault_sha256"] == runner_sha256
            or finite_number(fault.get("fully_written_progress_frames"), "fully written fake Progress frames") < 200
            or finite_number(fault.get("max_pending_bytes"), "fake pending bytes") > 66_048
            or not (fault.get("cancel_received") is True or fault.get("eof_received") is True)
            or value.get("actual_reap") is not True or value.get("source_unchanged") is not True
            or finite_number(value.get("fault_process_cpu_during_controls_ns"), "fake process CPU overlap") <= 0):
        raise ValueError("separately labelled protocol stress lacks actual bounded traffic/control/reap")
    for lane in ("input", "message", "status"):
        latency_bound(value["controls"][lane], "protocol stress " + lane)
    latency_bound(value["cancel_controls"], "protocol stress cancel")
    established.update(("protocol_fake_progress_flood_controls", "cpu_heavy_progress_flood"))
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-hot-owner-control.json", nonce, runner_sha256)
    if (value.get("established") != ["hot_owner_custody_causal_negative"]
            or value.get("verdict") != "snapshot_owner_custody_refused"
            or value.get("terminal_state") != "failed" or value.get("error") != "resource_limit"
            or value.get("manifest_published") is not False or value.get("worker_start_observed") is not False
            or any(value.get(flag) is not True for flag in ("actual_reap", "source_unchanged", "original_op_retained"))
            or finite_number(value["host_metrics"].get("source_owner_refusals"), "actual custody refusals") <= 0):
        raise ValueError("actual wrong-owner custody control did not refuse before source effect")
    # The earlier omission-of-REST receipt independently falsifies duty pacing.
    if "busy_wait_causal_negative" not in established:
        raise ValueError("missing independent duty-cycle causal negative")
    established.update(("hot_owner_custody_causal_negative", "hot_owner_busy_wait_controls"))
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})

    path, value = runtime_record(evidence, "runtime-foreground-pause.json", nonce, runner_sha256)
    owned_running_proof(value, "public_running_before_b_turn")
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
        parent = observed_parent_heap(value)
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
            paced_owner_bounds(metrics)
            if (finite_number(metrics.get("snapshot_tiny_records"), "actual tiny source records") <= 0
                    or finite_number(metrics.get("snapshot_staged_records"), "actual staged source records") <= 0):
                raise ValueError("real Store load did not exercise both bounded parsing paths")
            if (finite_number(metrics.get("max_unit_cpu_ns"), "isolated parent unit CPU") > 1_000_000
                    or metrics.get("unit_overruns") != 0
                    or metrics.get("producer_metrics_observed") is not True
                    or finite_number(metrics.get("producer_max_unit_cpu_ns"), "isolated child unit CPU") > 1_000_000
                    or metrics.get("producer_overruns") != 0):
                raise ValueError("isolated memory workload violated unchanged CPU gates")
        # This observer cannot attribute opaque allocations among four jobs.
        # Require the WHOLE isolated parent (baseline included) below one32MiB
        # share, which conservatively proves every individual parent's share.
        if (parent <= 0 or parent > 32 * 1024 * 1024
                or child_sum != value.get("child_heap_sum_conservative_bytes")
                or parent + child_sum != value.get("combined_parent_child_conservative_bytes")
                or parent + child_sum > 128 * 1024 * 1024 * instances):
            raise ValueError("isolated parent/child conservative memory bounds failed")
        records.append({"path": str(path), "sha256": sha(path), "measurements": value})
    established.add("attributed_parent_child_heap")
    established.add("reactor_cpu_units")
    path, value = runtime_record(evidence, "runtime-fork-subset.json", nonce, runner_sha256)
    if (value.get("header_fork_present") is not True or value.get("error") != "unsupported_ir"
            or value.get("inherited_parent_rows_interpreted") != 0
            or value.get("inspection_worker_spawned") is not False
            or value.get("malformed_parent_copy_still_header_refused") is not True
            or value.get("original_source_unchanged") is not True
            or value.get("observed_seq") != 1
            or finite_number(value.get("observed_cursor"), "fork header cursor")
                != finite_number(value.get("header_len"), "fork header length")):
        raise ValueError("actual Store fork did not refuse at the supported frontier")
    source = Path(value["store_root"]) / "sessions" / (value["source_session"] + ".jsonl")
    if (source.stat().st_size != value.get("source_len")
            or sha(source) != value.get("source_sha256")):
        raise ValueError("actual Store fork source/hash chain changed")
    established.add("actual_store_fork_frontier")
    records.append({"path": str(path), "sha256": sha(path), "measurements": value})
    for record in records:
        if (Path(record["path"]).name.startswith("runtime-isolated-")
                or Path(record["path"]).name in ("runtime-parent-overflow.json", "runtime-fork-subset.json")):
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
    parser.add_argument("--prepare-only", action="store_true",
                        help="build/probe the two pinned artifacts; does not verify the consumer")
    parser.add_argument("--worker-only", action="store_true",
                        help="build/provision the ordinary release sibling; does not verify the consumer")
    parser.add_argument("--prepare-development", action="store_true")
    parser.add_argument("--development-source", type=Path)
    parser.add_argument("--development-worker", type=Path)
    args = parser.parse_args()
    evidence = Path(tempfile.mkdtemp(prefix="bboxy-dream-c1-gate-"))
    os.chmod(evidence, 0o700)
    receipt = {"commands": [], "complete": False, "evidence": str(evidence)}
    code = 1
    try:
        if sys.platform != "darwin":
            raise ValueError("C1's required native frame/process witness needs the supported macOS fixture")
        if args.prepare_development:
            receipt["private_development"] = prepare_development(args, evidence)
            receipt["prepared"] = True
            receipt["scope"] = "Signed private development worker only; not a pinned producer or completed consumer"
            code = 0
            return code
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
        if not args.worker_only:
            receipt["bootstrap_corpus"] = bootstrap_corpus(fixtures)
        receipt["joined_pinned_corpus"] = fixtures == PINNED_FIXTURES.resolve()
        source_names = SOURCES + sorted(str(path.relative_to(ROOT)) for path in
                                       (ROOT / "crates/bboxy450-harness/src").rglob("*.rs"))
        receipt["source_hashes"] = {name: sha(ROOT / name) for name in source_names}
        target = args.target.resolve()
        nonce = secrets.token_hex(32)
        # Make exports these existing dev/test defaults for unrelated commands.
        # Neither selected profile inherits dev/test; they cannot change this
        # release/fixture build. Record their presence and refuse every other
        # profile override, including any altered default value.
        unrelated_defaults = {"CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
                              "CARGO_PROFILE_DEV_OPT_LEVEL": "1", "CARGO_PROFILE_TEST_OPT_LEVEL": "1"}
        unrelated = sorted(name for name, expected in unrelated_defaults.items()
                           if os.environ.get(name) == expected)
        conflicting = sorted(name for name, value in os.environ.items() if value and name not in unrelated and
                             (name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS")
                              or name.endswith("_RUSTFLAGS")
                              or name.startswith("CARGO_PROFILE_")))
        receipt["build_overrides"] = {"conflicting_variable_names": conflicting,
                                      "unrelated_make_dev_test_defaults": unrelated,
                                      "fixture_profile": "dream-c1-fixture",
                                      "fixture_debug_assertions": True,
                                      "fixture_overflow_checks": True,
                                      "worker_profile": "release", "worker_test_fixtures": False}
        if conflicting and not args.worker_only:
            raise ValueError("conflicting ambient build overrides: " + ", ".join(conflicting))
        receipt["compiler"] = subprocess.check_output(["rustc", "-vV"], cwd=ROOT, text=True)
        receipt["cargo"] = subprocess.check_output(["cargo", "--version"], cwd=ROOT, text=True).strip()
        environment = dict(os.environ, CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="1",
                           NEXTEST_TEST_THREADS="1", BBOXY_TEST_RUNNER="nextest",
                           DREAM_C1_FIXTURES=str(fixtures), DREAM_C1_EVIDENCE=str(evidence),
                           DREAM_C1_GATE_NONCE=nonce)
        if receipt.get("bootstrap_corpus"):
            environment["DREAM_BOOTSTRAP_FIXTURES"] = str(fixtures.parent / "dream-bootstrap")
            environment["DREAM_BOOTSTRAP_VECTORS_SHA256"] = BOOTSTRAP_VECTORS_SHA256
        run(["cargo", "build", "--locked", "--jobs", "2", "--release", "-p", "bboxy450-harness",
             "--bin", "bboxy450-harness", "--target-dir", str(target)],
            environment, evidence, receipt["commands"])
        runner = target / "release/bboxy450-harness"
        receipt["real_worker_build"] = {"path": str(runner), "sha256": sha(runner),
                                         "source_commit": revision, "profile": "release",
                                         "default_features": True, "effective_features": ["acp-agent"],
                                         "explicit_features": [], "test_fixtures": False,
                                         "compiler_sha256": hashlib.sha256(receipt["compiler"].encode()).hexdigest()}
        actual_build = (revision, receipt["real_worker_build"]["compiler_sha256"], receipt["real_worker_build"]["sha256"])
        if args.worker_only and (conflicting or APPROVED_NATIVE_BUILD != actual_build):
            receipt["native_provision"] = None
            receipt["native_profile_supported"] = False
            runner.with_suffix(".dream-c1.json").unlink(missing_ok=True)
        else:
            receipt["native_provision"] = provision_native_runner(runner, revision, receipt["compiler"])
            receipt["real_worker_probe"] = hello(runner, evidence, receipt["native_provision"])
        environment["DREAM_C1_RUNNER"] = str(runner)
        if args.worker_only:
            receipt["prepared"] = True
            receipt["scope"] = "Ordinary release worker preparation only; consumer verification incomplete"
            code = 0
            return code
        run(["cargo", "build", "--locked", "--jobs", "2", "--profile", "dream-c1-fixture",
             "-p", "bboxy450-harness", "--bin", "bboxy450-harness", "--features", "test-fixtures",
             "--target-dir", str(target)], environment, evidence, receipt["commands"])
        seat_runner = target / "dream-c1-fixture/bboxy450-harness"
        receipt["private_seat_fixture"] = {"path": str(seat_runner), "sha256": sha(seat_runner),
                                           "profile": "dream-c1-fixture", "features": ["test-fixtures"]}
        environment["DREAM_C1_SEAT_RUNNER"] = str(seat_runner)
        environment["DREAM_C1_SEAT_PROFILE"] = "dream-c1-fixture"
        environment["BBOXY_HARNESS_TEST_BIN"] = str(seat_runner)
        if args.prepare_only:
            receipt["prepared"] = True
            receipt["scope"] = "Pinned production worker and private optimized seat preparation only; consumer verification incomplete"
            code = 0
            return code

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
              "test(dream_c1_private_signed_gated_worker_held_parent_paid_probe_and_refusals)",
              "--run-ignored", "only", "--", "--nocapture"])
        tests("bboxy450-term-mux", ["--lib", "-E",
              "test(isolated_actual_store_worker_full_host_heap) | test(isolated_four_actual_store_workers_full_host_heap) | test(isolated_parent_work_overflow_preserves_controls_and_reaps_real_probe) | test(actual_consumer_store_fork_frontier)", "--run-ignored", "only",
              "--", "--nocapture"])
        tests("bboxy450-term-proto", ["--test", "dream_c1", "--test", "gui_wire_golden"])
        tests("bboxy450-harness-wake-up", ["--lib", "-E", "test(dream_bootstrap) | test(dream_c1)"])
        tests("bboxy450-harness-wake-up", ["--test", "dream_c1", "--test", "dream_bootstrap"])
        tests("bboxy450-term-mux", ["--lib", "-E", "test(dream_c1) | test(dream_public_) | test(harness_status)", "--", "--nocapture"])
        receipt["admitted_api_witnesses"] = admitted_api_witnesses(receipt["commands"][-1])
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
        if receipt["admitted_api_witnesses"]["exit_code"] == 0:
            validated.add("authority_aba_read_page_unknown_negatives")
        receipt["runtime"]["validated"] = sorted(validated)
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
