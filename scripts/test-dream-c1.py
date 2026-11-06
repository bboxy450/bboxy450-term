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
APPROVED_NATIVE_BUILD = (  # source, exact rustc -vV SHA256, ordinary executable
    "d06e696f99472ed754e84353baf11a2974589d6d",
    "e003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5",
    "12ba4006b694bdce3c44d199de460d09f0f2414fafb7fd17880921a7d397a3b4",
)
BOOTSTRAP_VECTORS_SHA256 = "de919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307"
PRIVATE_DEVELOPMENT_BUILD = None
LAUNCH_DIAGNOSIS_ID = "65882"
LAUNCH_DIAGNOSIS_BODY_SHA256 = "83e9437d604a0002e9101d39c174be75b97ead5f72bf2608aafcc9af30db9e45"
LAUNCH_DIAGNOSIS_RECORD_DIGEST_PREFIX = "7d8ded"
LAUNCH_DIAGNOSIS_BUILD = (
    "d06e696f99472ed754e84353baf11a2974589d6d",
    "e003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5",
    "12ba4006b694bdce3c44d199de460d09f0f2414fafb7fd17880921a7d397a3b4",
)
_SOURCE_FREE_OWNER = None  # At most one active or quarantined private probe owner.
# Decision d93377961365255: probe=true is permanently source-free/non-startable.
PROBE_NULL_CONTEXT_APPROVED = True
FRAME = 256 * 1024
PINNED_FIXTURES = ROOT / "crates/bboxy450-harness/tests/fixtures/dream-c1"
SOURCES = [
    "Makefile", "scripts/test-dream-c1.py",
    "Cargo.toml", "Cargo.lock", "scripts/quarantine-test.sh", "scripts/landing.py",
    "scripts/quarantine.py", "scripts/test-landing.py",
    "rust-toolchain.toml", ".cargo/config.toml", ".config/nextest.toml", "scripts/cache-env.sh",
    "crates/bboxy450-harness/Cargo.toml", "crates/bboxy450-harness/Cargo.lock",
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


def retain_build_closure(evidence, name, value):
    """Retain the bounded input observation before permitting a candidate birth."""
    if name not in ("build-closure-before.json", "build-closure-after.json"):
        raise ValueError("unknown build-closure record")
    raw = canonical(value)
    if len(raw) > 1024 * 1024:
        raise ValueError("build-closure record exceeded its fixed bound")
    path = evidence / name
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    return {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)}


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


def _build_closure_file(path, limit=128 * 1024 * 1024):
    """Hash one bounded regular file while retaining its exact opened identity."""
    requested = Path(path).absolute()
    path = requested.resolve(strict=True)
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW), "rb") as stream:
        before = os.fstat(stream.fileno())
        if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
            raise ValueError("unbounded/nonregular build-closure input")
        digest = hashlib.sha256()
        total = 0
        while chunk := stream.read(64 * 1024):
            total += len(chunk)
            if total > limit:
                raise ValueError("build-closure input grew beyond its bound")
            digest.update(chunk)
        after = os.fstat(stream.fileno())
        identity = lambda s: (s.st_dev, s.st_ino, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
        if identity(before) != identity(after) or total != before.st_size or identity(path.stat()) != identity(after):
            raise ValueError("build-closure input changed while hashed")
    return {"path": str(requested), "resolved_path": str(path), "sha256": digest.hexdigest(),
            "size_bytes": total, "dev": str(after.st_dev), "ino": str(after.st_ino),
            "mtime_ns": str(after.st_mtime_ns), "ctime_ns": str(after.st_ctime_ns)}


def _build_closure_query(argv, environment):
    """Only called with fixed rustup which or known sccache --version argv."""
    process = subprocess.Popen(argv, cwd=ROOT, env=environment, stdin=subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    output = bytearray()
    deadline = time.monotonic() + 5
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise ValueError("build tool metadata query timed out")
                for key, _ in selector.select(remaining):
                    chunk = os.read(key.fd, 4096)
                    if not chunk:
                        selector.unregister(key.fileobj)
                    else:
                        output.extend(chunk)
                        if len(output) > 8192:
                            raise ValueError("build tool metadata exceeded its bound")
        if process.wait(timeout=max(0.001, deadline - time.monotonic())):
            raise ValueError("build tool metadata query failed")
        return output.decode("utf-8", errors="strict").strip()
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        process.stdout.close()


def build_closure_before(*, cargo_argv, target, source_hashes, compiler_version, cargo_version,
                         entry_argv=None, environment=None):
    """65736 prebuild observation, never an environment dump or hermeticity claim.

    Main must persist this receipt and refuse admission when admissible is false.
    No candidate or compiler is executed here; metadata queries are allowlisted.
    """
    environment = os.environ if environment is None else environment
    refusals = []

    def argv(values):
        values = list(values)
        if (not 0 < len(values) <= 64 or any(not isinstance(v, str) or len(v.encode()) > 4096
                or any(c in v for c in "\0\r\n") for v in values)
                or sum(len(v.encode()) for v in values) > 16 * 1024):
            raise ValueError("unbounded build-closure argv")
        return values

    def program(value):
        found = shutil.which(value, path=environment.get("PATH"))
        if found is None:
            raise ValueError("required build metadata tool is unavailable")
        return Path(found).absolute()

    def state(name, allowed):
        if name not in environment:
            return {"present": False}
        value = environment[name]
        if value in allowed:
            return {"present": True, "value": value}
        refusals.append("unexpected_" + name)
        return {"present": True, "value_redacted": True}

    cargo_argv = argv(cargo_argv)
    target = Path(target).resolve()
    entry = environment.get("BBOXY_DREAM_BUILD_ENTRY")
    entries = {"bundle-files", "prepare-dream-c1-worker", "prepare-dream-c1", "test-dream-c1", "diagnose-dream-c1-launch"}
    make_entry = {"present": entry is not None, "recognized": entry in entries}
    if entry in entries:
        make_entry["value"] = entry
    else:
        make_entry["value_redacted"] = entry is not None
        refusals.append("unrecognized_canonical_Make_entry")
    argv_targets = [cargo_argv[i + 1] for i, item in enumerate(cargo_argv[:-1]) if item == "--target-dir"]
    argv_targets.extend(item.split("=", 1)[1] for item in cargo_argv if item.startswith("--target-dir="))
    if len(argv_targets) != 1 or Path(argv_targets[0]).resolve() != target:
        refusals.append("Cargo_argv_target_differs_from_recorded_target")
    if (len(cargo_argv) < 2 or cargo_argv[1] != "build" or "--release" not in cargo_argv
            or any(item == "--profile" or item.startswith("--profile=") for item in cargo_argv)
            or any(item in ("--features", "-F", "--all-features", "--no-default-features")
                   or item.startswith(("--features=", "-F")) for item in cargo_argv)):
        refusals.append("Cargo_argv_is_not_ordinary_release_default_features")
    sources = dict(source_hashes)
    if len(sources) > 2048 or len(canonical(sources)) > 512 * 1024:
        raise ValueError("unbounded build-closure source inventory")
    for name, digest in sources.items():
        if not isinstance(name, str):
            raise ValueError("invalid build-closure source path")
        path = Path(name)
        if (path.is_absolute() or not path.parts
                or any(part in (".", "..") for part in path.parts)
                or not isinstance(digest, str) or re.fullmatch("[0-9a-f]{64}", digest) is None):
            raise ValueError("invalid build-closure source inventory")
    for version in (compiler_version, cargo_version):
        if not isinstance(version, str) or not version.isascii() or len(version) > 8192:
            raise ValueError("unbounded build tool version")
    if not compiler_version.startswith("rustc ") or not cargo_version.startswith("cargo "):
        raise ValueError("unexpected compiler/Cargo version record")

    defaults = {"CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
                "CARGO_PROFILE_DEV_OPT_LEVEL": "1", "CARGO_PROFILE_TEST_OPT_LEVEL": "1"}
    profiles = {name: state(name, {expected}) for name, expected in defaults.items()}
    flags = {name: state(name, {""}) for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS")}
    for name in sorted(environment):
        if name.startswith("CARGO_PROFILE_") and name not in defaults:
            if re.fullmatch(r"[A-Z0-9_]{1,128}", name) is None:
                raise ValueError("invalid profile-variable name")
            profiles[name] = state(name, {""})
        elif name.endswith("_RUSTFLAGS") and name not in flags:
            if re.fullmatch(r"[A-Z0-9_]{1,128}", name) is None:
                raise ValueError("invalid flag-variable name")
            flags[name] = state(name, {""})
    if len(profiles) + len(flags) > 64:
        raise ValueError("unbounded build flag/profile inventory")
    incremental = state("CARGO_INCREMENTAL", {"0", ""})
    other_overrides = {name: state(name, {""}) for name in
                       ("RUSTC_BOOTSTRAP", "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")}

    rustup = _build_closure_file(program("rustup"))
    tools = {"rustup_resolver": rustup}
    for name, version in (("rustc", compiler_version), ("cargo", cargo_version)):
        resolved = _build_closure_query([rustup["resolved_path"], "which", name], environment)
        if not Path(resolved).is_absolute() or len(resolved) > 4096 or any(c in resolved for c in "\0\r\n"):
            raise ValueError("invalid resolved compiler/Cargo path")
        tools[name] = {**_build_closure_file(resolved), "version": version,
                       "version_sha256": hashlib.sha256(version.encode()).hexdigest(),
                       "invocation": _build_closure_file(program(name))}
    wrappers = {}
    for name in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
        value = environment.get(name)
        if not value:
            wrappers[name] = {"present": name in environment, "enabled": False,
                              **({"value": ""} if name in environment else {})}
        elif Path(value).name == "sccache":
            identity = _build_closure_file(program(value))
            version = _build_closure_query([identity["resolved_path"], "--version"], environment)
            if re.fullmatch(r"sccache [0-9][A-Za-z0-9._+ -]{0,255}", version) is None:
                raise ValueError("unexpected known-wrapper version record")
            wrappers[name] = {"present": True, "enabled": True, "kind": "sccache", **identity,
                              "version": version}
        else:
            wrappers[name] = {"present": True, "enabled": True, "value_redacted": True}
            refusals.append("unknown_" + name)
    for name in ("RUSTC", "CARGO_BUILD_RUSTC"):
        value = environment.get(name)
        if value:
            # Unknown overrides are never executed or copied into the receipt.
            if value not in ("rustc", tools["rustc"]["path"], tools["rustc"]["resolved_path"]):
                refusals.append("unexpected_" + name)
                tools[name] = {"present": True, "value_redacted": True}
            else:
                tools[name] = {"present": True, "resolved_path": tools["rustc"]["resolved_path"]}

    directories = {"target": str(target), "cwd": str(ROOT.resolve())}
    for name in ("CARGO_BUILD_BUILD_DIR", "CARGO_BUILD_TARGET_DIR", "CARGO_TARGET_DIR", "BBOXY_CARGO_TARGET_DIR"):
        value = environment.get(name)
        if value:
            if len(value) > 4096 or any(c in value for c in "\0\r\n"):
                raise ValueError("invalid build directory override")
            directories[name] = {"present": True, "resolved_path": str((ROOT / value).resolve())}
        else:
            directories[name] = {"present": name in environment, "empty": True}
    import tomllib
    with (ROOT / ".cargo/config.toml").open("rb") as stream:
        workspace_raw = stream.read(64 * 1024 + 1)
    if len(workspace_raw) > 64 * 1024:
        raise ValueError("unbounded workspace build configuration")
    workspace_build = tomllib.loads(workspace_raw.decode())["build"]
    configured_incremental = workspace_build.get("incremental")
    configured_build_dir = workspace_build.get("build-dir")
    if (type(configured_incremental) is not bool or not isinstance(configured_build_dir, str)
            or len(configured_build_dir) > 4096 or any(c in configured_build_dir for c in "\0\r\n")):
        raise ValueError("unknown workspace incremental/build-directory disposition")
    directories["workspace_configured_build_dir"] = str((ROOT / configured_build_dir).resolve())
    directories["effective_build_dir"] = directories.get("CARGO_BUILD_BUILD_DIR", {}).get(
        "resolved_path", directories["workspace_configured_build_dir"])
    incremental = {"environment": incremental, "workspace_configured": configured_incremental,
                   "effective": (None if incremental.get("value_redacted") else
                                 False if environment.get("CARGO_INCREMENTAL") == "0" else configured_incremental)}
    config_paths = {ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "rust-toolchain.toml",
                    ROOT / "crates/bboxy450-harness/rust-toolchain.toml", ROOT / "scripts/cache-env.sh"}
    cargo_home = Path(environment.get("CARGO_HOME") or Path.home() / ".cargo")
    for config_dir in [*(directory / ".cargo" for directory in [ROOT, *ROOT.parents]), cargo_home]:
        for name in ("config", "config.toml"):
            config_paths.add(config_dir / name)
    if len(config_paths) > 32:
        raise ValueError("unbounded Cargo config inventory")
    configs = []
    for path in sorted(config_paths):
        configs.append(_build_closure_file(path, 2 * 1024 * 1024) if path.exists()
                       else {"path": str(path.absolute()), "present": False})
    cache = {"mode": state("BBOXY_CACHE_MODE", {"auto", "off", ""}),
             "wrapper_enabled": any(v["enabled"] for v in wrappers.values()),
             "server_contacted_by_evidence_helper": False}
    cache["selected_mode"] = (environment.get("BBOXY_CACHE_MODE") or "auto") if not cache["mode"].get("value_redacted") else "unknown_redacted"
    for name in ("SCCACHE_DIR", "SCCACHE_SERVER_PORT", "SCCACHE_CACHE_SIZE"):
        # Presence is sufficient to label inherited cache configuration; raw
        # cache environment values are deliberately not copied here.
        cache[name] = {"present": name in environment, "nonempty": bool(environment.get(name))}
    receipt = {"scope": "bounded observed build closure; no causal equivalence or hermeticity claim",
               "entry_argv": argv(getattr(sys, "orig_argv", [sys.executable, *sys.argv])
                                  if entry_argv is None else entry_argv),
               "make_entry": make_entry, "script_argv": argv(sys.argv), "cargo_argv": cargo_argv,
               "tools": tools, "wrappers": wrappers, "profiles": profiles, "flags": flags,
               "other_build_overrides": other_overrides,
               "incremental": incremental, "directories": directories, "cache": cache,
               "config_inputs": configs, "source_hashes": sources,
               "source_manifest_sha256": hashlib.sha256(canonical(sources)).hexdigest(),
               "admissible": not refusals, "refusals": refusals}
    if len(canonical(receipt)) > 1024 * 1024:
        raise ValueError("build-closure receipt exceeded its fixed bound")
    return receipt


def build_closure_after(before, *, runner):
    """Bind the actual release binary to its exact Cargo fingerprint, not features alone."""
    refusals = list(before["refusals"])
    target = Path(before["directories"]["target"])
    runner = _build_closure_file(runner)
    if Path(runner["resolved_path"]) != (target / "release/bboxy450-harness").resolve():
        refusals.append("runner_is_not_requested_release_binary")
    for name, expected in before["source_hashes"].items():
        if _build_closure_file(ROOT / name, 16 * 1024 * 1024)["sha256"] != expected:
            refusals.append("source_changed_" + name)
    for item in before["config_inputs"]:
        path = Path(item["path"])
        if (path.exists() != (item.get("present") is not False)
                or (path.exists() and _build_closure_file(path, 2 * 1024 * 1024)["sha256"] != item["sha256"])):
            refusals.append("cargo_config_input_changed")
    for item in [*before["tools"].values(), *before["wrappers"].values()]:
        if "sha256" in item and _build_closure_file(item["path"])["sha256"] != item["sha256"]:
            refusals.append("build_tool_changed")
        if "invocation" in item:
            old = item["invocation"]
            if _build_closure_file(old["path"])["sha256"] != old["sha256"]:
                refusals.append("build_tool_invocation_changed")
    candidates = []
    selected = []
    for path in (target / "release/.fingerprint").glob("bboxy450-harness-*/bin-bboxy450-harness.json"):
        if len(candidates) >= 32:
            raise ValueError("more than32 worker Cargo fingerprints")
        identity = _build_closure_file(path, 64 * 1024)
        with path.open("rb") as stream:
            raw = stream.read(64 * 1024 + 1)
        if len(raw) > 64 * 1024 or hashlib.sha256(raw).hexdigest() != identity["sha256"]:
            raise ValueError("Cargo fingerprint changed while read")
        value = strict_json(raw, max_integer=(1 << 64) - 1)
        if (not isinstance(value, dict) or not {"rustc", "target", "profile", "path", "config",
                "compile_kind", "features", "declared_features", "deps", "rustflags"} <= value.keys()):
            raise ValueError("incomplete Cargo fingerprint metadata")
        metadata = {}
        for key in ("rustc", "target", "profile", "path", "config", "compile_kind"):
            number = value[key]
            if type(number) is not int or not 0 <= number < 1 << 64:
                raise ValueError("invalid Cargo fingerprint scalar")
            metadata[key] = {"decimal": str(number), "hex": hex(number)}
        for key in ("features", "declared_features"):
            if not isinstance(value[key], str):
                raise ValueError("invalid Cargo feature encoding")
            features = strict_json(value[key])
            if (not isinstance(features, list) or len(features) > 32
                    or any(not isinstance(f, str) or re.fullmatch(r"[A-Za-z0-9_+.-]{1,128}", f) is None for f in features)):
                raise ValueError("unbounded Cargo feature metadata")
            metadata[key] = features
        dependencies = value["deps"]
        if not isinstance(dependencies, list) or len(dependencies) > 256:
            raise ValueError("unbounded Cargo dependency fingerprint inventory")
        metadata["deps"] = []
        for dependency in dependencies:
            if (not isinstance(dependency, list) or len(dependency) != 4
                    or type(dependency[0]) is not int or not 0 <= dependency[0] < 1 << 64
                    or not isinstance(dependency[1], str)
                    or re.fullmatch(r"[A-Za-z0-9_]{1,128}", dependency[1]) is None
                    or type(dependency[2]) is not bool
                    or type(dependency[3]) is not int or not 0 <= dependency[3] < 1 << 64):
                raise ValueError("invalid Cargo dependency fingerprint")
            metadata["deps"].append([str(dependency[0]), dependency[1], dependency[2], str(dependency[3])])
        if (not isinstance(value["rustflags"], list) or len(value["rustflags"]) > 256
                or any(not isinstance(flag, str) for flag in value["rustflags"])):
            raise ValueError("invalid Cargo rustflags metadata")
        metadata["rustflags"] = {"empty": value["rustflags"] == [],
                                  "nonempty_values_redacted": value["rustflags"] != []}
        suffix = path.parent.name.removeprefix("bboxy450-harness-")
        if re.fullmatch("[0-9a-f]{16}", suffix) is None:
            raise ValueError("invalid Cargo bin fingerprint suffix")
        binary = target / "release/deps" / ("bboxy450_harness-" + suffix)
        artifact = _build_closure_file(binary) if binary.is_file() else None
        candidate = {"fingerprint": identity, "metadata": metadata, "deps_executable": artifact,
                     "matches_runner_sha256": artifact is not None and artifact["sha256"] == runner["sha256"]}
        candidates.append(candidate)
        if candidate["matches_runner_sha256"]:
            selected.append(candidate)
    candidates.sort(key=lambda candidate: candidate["fingerprint"]["path"])
    if len(selected) != 1:
        refusals.append("runner_has_no_unique_matching_Cargo_fingerprint")
    elif (sorted(selected[0]["metadata"]["features"]) != ["acp-agent", "default"]
          or not selected[0]["metadata"]["rustflags"]["empty"]):
        refusals.append("selected_fingerprint_has_unapproved_features_or_flags")
    receipt = {"before_sha256": hashlib.sha256(canonical(before)).hexdigest(), "runner": runner,
               "requested_profile": "release", "fingerprint_selection": "exact deps executable SHA256 equals runner SHA256",
               "candidates": candidates, "selected": selected[0] if len(selected) == 1 else None,
               "admissible": not refusals, "refusals": refusals}
    if len(canonical(receipt)) > 1024 * 1024:
        raise ValueError("postbuild closure receipt exceeded its fixed bound")
    return receipt


def check_build_closure_controls(canonical_receipt=None):
    """Private filesystem/validator controls; no compiler, candidate or Cargo execution.

    The metadata-query boundary alone is substituted. Normal closure functions
    still hash actual private files and parse/select their actual metadata.
    These plain bytes are helper inputs, not another worker/schema corpus.
    """
    from unittest import mock
    accepted, refused = [], []
    with tempfile.TemporaryDirectory(prefix="dream-c1-closure-controls-") as directory:
        root = Path(directory).resolve()
        os.chmod(root, 0o700)
        target, tools = root / "target", root / "tools"
        tools.mkdir(mode=0o700)

        def put(path, raw, executable=False):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
            os.chmod(path, 0o700 if executable else 0o600)

        for name in ("rustup", "rustc", "cargo", "sccache"):
            put(tools / name, ("NEVER EXECUTED private metadata input " + name).encode(), True)
        put(root / ".cargo/config.toml", b'[build]\nbuild-dir="target"\nincremental=false\n')
        put(root / "Cargo.toml", b"[workspace]\n")
        put(root / "Cargo.lock", b"version = 4\n")
        put(root / "rust-toolchain.toml", b'[toolchain]\nchannel="1.99.1"\n')
        put(root / "scripts/cache-env.sh", b"# private helper input, never sourced\n")
        put(root / "source.rs", b"// source input before build\n")
        versions = {"rustc": "rustc 1.99.1 (private helper metadata)\n", "cargo": "cargo 1.99.1"}
        environment = {"PATH": str(tools), "CARGO_HOME": str(root / "cargo-home"),
                       "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
                       "CARGO_PROFILE_DEV_OPT_LEVEL": "1", "CARGO_PROFILE_TEST_OPT_LEVEL": "1",
                       "CARGO_INCREMENTAL": "0", "RUSTC_WRAPPER": "sccache",
                       "RUSTC_WORKSPACE_WRAPPER": "", "BBOXY_CACHE_MODE": "auto",
                       "BBOXY_DREAM_BUILD_ENTRY": "test-dream-c1",
                       "BBOXY_CAPABILITY_FILE": "PRIVATE_CAPABILITY_MARKER_NEVER_READ",
                       "UNRELATED_SECRET": "PRIVATE_UNRELATED_MARKER_NEVER_RECORDED"}
        command = ["cargo", "build", "--release", "--locked", "-p", "bboxy450-harness",
                   "--bin", "bboxy450-harness", "--target-dir", str(target)]
        parameters = {"cargo_argv": command, "target": target,
                      "source_hashes": {"source.rs": sha(root / "source.rs")},
                      "compiler_version": versions["rustc"], "cargo_version": versions["cargo"],
                      "entry_argv": ["private-helper-control", "--target", str(target)]}
        query_calls = []

        def query(argv, _environment):
            query_calls.append(argv)
            if argv[:2] == [str(tools / "rustup"), "which"] and argv[2:] in (["rustc"], ["cargo"]):
                return str(tools / argv[2])
            if argv == [str(tools / "sccache"), "--version"]:
                return "sccache 0.0.0-private-helper"
            raise AssertionError("unexpected metadata query in private closure control")

        with mock.patch.dict(globals(), {"ROOT": root, "_build_closure_query": query}):
            before = build_closure_before(**parameters, environment=environment)
            assert before["admissible"] and not before["refusals"]
            encoded = canonical(before)
            assert all(value.encode() not in encoded for value in
                       (environment["BBOXY_CAPABILITY_FILE"], environment["UNRELATED_SECRET"]))
            assert b"BBOXY_CAPABILITY_FILE" not in encoded and b"UNRELATED_SECRET" not in encoded
            accepted.append("bounded_allowlisted_prebuild_without_secret_capture")

            def denied_before(label, *, changes=None, overrides=None, reason=None):
                args = {**parameters, **(overrides or {})}
                env = {**environment, **(changes or {})}
                try:
                    result = build_closure_before(**args, environment=env)
                except ValueError:
                    assert reason is None, label
                else:
                    assert not result["admissible"], label
                    if reason is not None:
                        assert reason in result["refusals"], (label, result["refusals"])
                    for value in (changes or {}).values():
                        if value.startswith("PRIVATE_"):
                            assert value.encode() not in canonical(result), label
                refused.append(label)

            denied_before("nonempty_flags_redacted", changes={"RUSTFLAGS": "PRIVATE_FLAG_MARKER"}, reason="unexpected_RUSTFLAGS")
            denied_before("unknown_profile_redacted", changes={"CARGO_PROFILE_RELEASE_DEBUG": "PRIVATE_PROFILE_MARKER"}, reason="unexpected_CARGO_PROFILE_RELEASE_DEBUG")
            denied_before("changed_Make_default_redacted", changes={"CARGO_PROFILE_DEV_DEBUG": "PRIVATE_DEFAULT_MARKER"}, reason="unexpected_CARGO_PROFILE_DEV_DEBUG")
            denied_before("unknown_wrapper_never_queried", changes={"RUSTC_WRAPPER": "PRIVATE_WRAPPER_MARKER"}, reason="unknown_RUSTC_WRAPPER")
            denied_before("unknown_entry_redacted", changes={"BBOXY_DREAM_BUILD_ENTRY": "PRIVATE_ENTRY_MARKER"}, reason="unrecognized_canonical_Make_entry")
            denied_before("oversized_argv", overrides={"cargo_argv": ["cargo"] * 65})
            denied_before("foreign_source_inventory", overrides={"source_hashes": {"../foreign": "0" * 64}})

            runner = target / "release/bboxy450-harness"
            original, current = b"old helper artifact", b"current helper artifact"
            put(runner, current)
            fingerprint = {"rustc": (1 << 64) - 1, "target": (1 << 53) + 1,
                           "profile": 854319013279132213, "path": 15914529373502866653,
                           "config": 9396254390672932401, "compile_kind": 0,
                           "features": '["acp-agent","default"]',
                           "declared_features": '["acp-agent","default","test-fixtures"]',
                           "deps": [[(1 << 53) + 1, "helper_dependency", False, (1 << 64) - 1]],
                           "rustflags": []}
            old_suffix, current_suffix = "a" * 16, "b" * 16

            def candidate(suffix):
                return target / "release/.fingerprint" / ("bboxy450-harness-" + suffix) / "bin-bboxy450-harness.json"

            for suffix, artifact in ((old_suffix, original), (current_suffix, current)):
                put(candidate(suffix), canonical(fingerprint))
                put(target / "release/deps" / ("bboxy450_harness-" + suffix), artifact)
            after = build_closure_after(before, runner=runner)
            assert after["admissible"] and len(after["candidates"]) == 2
            assert current_suffix in after["selected"]["fingerprint"]["path"]
            assert sum(c["matches_runner_sha256"] for c in after["candidates"]) == 1
            metadata = after["selected"]["metadata"]
            assert metadata["rustc"]["decimal"] == str((1 << 64) - 1)
            assert metadata["rustc"]["hex"] == hex((1 << 64) - 1)
            assert metadata["deps"][0][0] == str((1 << 53) + 1)
            assert metadata["deps"][0][3] == str((1 << 64) - 1)
            accepted.extend(["same_features_different_bytes_select_exact_deps_SHA", "u64_fingerprints_preserve_exact_string_values"])

            def denied_after(label, reason=None):
                try:
                    result = build_closure_after(before, runner=runner)
                except ValueError:
                    assert reason is None, label
                else:
                    assert not result["admissible"], label
                    if reason is not None:
                        assert reason in result["refusals"], (label, result["refusals"])
                    assert b"PRIVATE_FINGERPRINT_FLAG_MARKER" not in canonical(result)
                refused.append(label)

            for path, replacement, label, reason in (
                (runner, b"unmatched artifact", "no_matching_artifact", "runner_has_no_unique_matching_Cargo_fingerprint"),
                (root / "source.rs", b"changed source", "changed_source", "source_changed_source.rs"),
                (root / "Cargo.toml", b"changed Cargo config", "changed_config", "cargo_config_input_changed"),
                (root / ".cargo/config.toml", b"changed workspace config", "changed_workspace_config", "cargo_config_input_changed"),
                (tools / "rustc", b"changed tool", "changed_tool", "build_tool_changed"),
                (target / "release/deps" / ("bboxy450_harness-" + old_suffix), current,
                 "ambiguous_matching_artifacts", "runner_has_no_unique_matching_Cargo_fingerprint"),
            ):
                retained = path.read_bytes()
                put(path, replacement, executable=path.parent == tools)
                denied_after(label, reason)
                put(path, retained, executable=path.parent == tools)
            for label, value, reason in (
                ("selected_feature_change", {**fingerprint, "features": '["default","test-fixtures"]'}, "selected_fingerprint_has_unapproved_features_or_flags"),
                ("selected_flags_redacted", {**fingerprint, "rustflags": ["PRIVATE_FINGERPRINT_FLAG_MARKER"]}, "selected_fingerprint_has_unapproved_features_or_flags"),
                ("noninteger_fingerprint", {**fingerprint, "profile": True}, None),
                ("u64_overflow", {**fingerprint, "rustc": 1 << 64}, None),
                ("malformed_feature_encoding", {**fingerprint, "features": ["acp-agent", "default"]}, None),
                ("missing_fingerprint_fields", {"features": fingerprint["features"]}, None),
            ):
                put(candidate(current_suffix), canonical(value))
                denied_after(label, reason)
            put(candidate(current_suffix), b'{"profile":1,"profile":2}')
            denied_after("duplicate_fingerprint_keys")
            put(candidate(current_suffix), b" " * (64 * 1024 + 1))
            denied_after("oversized_fingerprint")
            put(candidate(current_suffix), canonical(fingerprint))
            for index in range(31):
                put(candidate(format(index, "016x")), canonical(fingerprint))
            denied_after("more_than_32_fingerprints")
            assert query_calls and all(len(call) <= 3 for call in query_calls)

    audit = None
    if canonical_receipt is not None:
        path = Path(canonical_receipt)
        identity = _build_closure_file(path, 2 * 1024 * 1024)
        with path.open("rb") as stream:
            raw = stream.read(2 * 1024 * 1024 + 1)
        assert len(raw) <= 2 * 1024 * 1024 and hashlib.sha256(raw).hexdigest() == identity["sha256"]
        value = json.loads(raw)
        before, after = value["build_closure_before"], value["build_closure_after"]
        assert before["admissible"] and after["admissible"] and not before["refusals"] and not after["refusals"]
        assert after["before_sha256"] == hashlib.sha256(canonical(before)).hexdigest()
        assert before["source_manifest_sha256"] == hashlib.sha256(canonical(before["source_hashes"])).hexdigest()
        source, compiler, worker = APPROVED_NATIVE_BUILD
        assert value["producer_pin"]["actual_head"] == source
        assert before["tools"]["rustc"]["version_sha256"] == compiler
        assert after["runner"]["sha256"] == worker
        assert _build_closure_file(after["runner"]["path"])["sha256"] == worker
        matches = [c for c in after["candidates"] if c["matches_runner_sha256"]]
        assert len(matches) == 1 and matches[0] == after["selected"]
        for candidate in after["candidates"]:
            artifact = candidate["deps_executable"]
            assert candidate["matches_runner_sha256"] == (artifact is not None and artifact["sha256"] == worker)
            for item in (candidate["fingerprint"], candidate["deps_executable"]):
                if item is not None:
                    assert _build_closure_file(item["path"])["sha256"] == item["sha256"]
        audit = {"receipt_sha256": identity["sha256"], "runner_sha256": worker,
                 "matching_candidates": len(matches), "candidates_rehashed": len(after["candidates"]),
                 "recorded_complete": value["complete"],
                 "scope": "historical canonical build-input audit only; runtime verdict unchanged"}
    return {"scope": "private build-closure helper controls, not worker/runtime conformance",
            "accepted": accepted, "refused": refused, "canonical_record_audit": audit}


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
    if PRIVATE_DEVELOPMENT_BUILD is None:
        raise ValueError("no active signed development-only milestone")
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
    if hashlib.sha256(bootstrap).hexdigest() != "14b3155d23b47e1ac45b9d023bbea00cab730c3043fbf954dc95402d9ae63b05":
        raise ValueError("private bootstrap source differs from signed boundary")
    vectors = source / "tests/fixtures/dream-bootstrap/vectors.json"
    if sha(vectors) != BOOTSTRAP_VECTORS_SHA256:
        raise ValueError("private canonical bootstrap corpus differs from signed boundary")
    target = args.target.resolve()
    target.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix=f"dream-c1-development-{revision[:8]}-", dir=target))
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
            "known_exclusion": "Signed development milestone has no reviewed production pin or final joined conformance"}


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


def _retain_launch_bytes(evidence, name, raw, limit):
    if name not in ("hello.bootstrap.json", "hello.stdout", "hello.stderr", "hello.lifecycle.json", "hello.quarantine.json") or len(raw) > limit:
        raise ValueError("unbounded/unknown private launch record")
    path = evidence / name
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    return {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)}


class _SourceFreeCustody:
    """Explicit private job custody; unknown outcome has no implicit cleanup."""
    def __init__(self, evidence, parent=None):
        global _SOURCE_FREE_OWNER
        if _SOURCE_FREE_OWNER is not None:
            raise ValueError("an active/quarantined source-free owner is already retained; no new birth")
        self.path = Path(tempfile.mkdtemp(prefix="dream-c1-probe-", dir=parent))
        os.chmod(self.path, 0o700)
        self.evidence, self.job, self.claim = evidence, None, None
        self.attempted, self.settled, self.process, self.drain = False, False, None, None
        _SOURCE_FREE_OWNER = self

    def __enter__(self):
        return self

    def __exit__(self, _type, _value, _traceback):
        global _SOURCE_FREE_OWNER
        if not self.attempted or self.settled:
            # Only our own newly created directory, after known no-birth or
            # actual reap and retirement of every captured output owner.
            shutil.rmtree(self.path)
            _SOURCE_FREE_OWNER = None
            return False
        # Keep the exact Child/PID, IO selector, job and claim reachable. There
        # is no periodic poll, adoption, second attempt, destructor or retry.
        _SOURCE_FREE_OWNER = self
        record = {"scope": "private source-free unknown owner quarantine; no runtime conformance",
                  "diagnostic_only": self.claim is not None, "runtime_conformance": False,
                  "state": "quarantined", "settled": False, "attempted": True,
                  "actual_reap": self.process is not None and self.process.returncode is not None,
                  "pid": None if self.process is None else self.process.pid,
                  "returncode": None if self.process is None else self.process.returncode,
                  "retained_process_handle": self.process is not None,
                  "retained_output_owners": self.drain is not None,
                  "outer_directory": str(self.path), "job_directory": None if self.job is None else str(self.job),
                  "claim": self.claim, "automatic_adoption": False, "automatic_retry": False}
        try:
            self.quarantine_record = _retain_launch_bytes(self.evidence, "hello.quarantine.json", canonical(record), 8192)
        except OSError:
            self.quarantine_record = None  # Owners and paths remain retained even when storage fails.
        return False


def _claim_launch_diagnosis(target, evidence, provision, before, after, inputs, birth, pipe_identity):
    """One decision, one durable consumed attempt; failure never removes this claim."""
    target = Path(target).resolve()
    expected = LAUNCH_DIAGNOSIS_BUILD
    if (APPROVED_NATIVE_BUILD != expected or before is None or after is None
            or target != (ROOT / "target").resolve()
            or before.get("admissible") is not True or after.get("admissible") is not True
            or before.get("refusals") or after.get("refusals")
            or before.get("make_entry", {}).get("value") != "diagnose-dream-c1-launch"
            or Path(before["directories"]["target"]) != target
            or after.get("before_sha256") != hashlib.sha256(canonical(before)).hexdigest()
            or (provision.get("source_commit"), provision.get("compiler_sha256"), provision.get("binary_sha256")) != expected
            or provision.get("bootstrap_profile") != "dream-bootstrap/1"
            or provision.get("metrics_profile") != "c1-native-debt/1"
            or provision.get("build_profile") != "release" or provision.get("default_features") is not True
            or provision.get("explicit_features") != [] or provision.get("effective_features") != ["acp-agent"]
            or provision.get("test_fixtures") is not False
            or after.get("runner", {}).get("sha256") != expected[2]
            or ((after.get("selected") or {}).get("deps_executable") or {}).get("sha256") != expected[2]
            or inputs.get("probe") is not True or "context" not in inputs or inputs["context"] is not None):
        raise ValueError("diagnosis lacks exact approved canonical pre-birth proof")
    directory = target / ".dream-c1-launch-diagnosis"
    directory.mkdir(mode=0o700, exist_ok=True)
    identity = directory.lstat()
    if (not stat.S_ISDIR(identity.st_mode) or stat.S_ISLNK(identity.st_mode)
            or identity.st_uid != os.geteuid() or identity.st_mode & 0o077):
        raise ValueError("diagnostic claim directory is not private and owned")
    record = {"decision_id": LAUNCH_DIAGNOSIS_ID,
              "decision_body_sha256": LAUNCH_DIAGNOSIS_BODY_SHA256,
              "authenticated_record_digest_prefix": LAUNCH_DIAGNOSIS_RECORD_DIGEST_PREFIX,
              "diagnostic_only": True, "runtime_conformance": False,
              "attempt_consumed_before_Popen": True, "retry_authorized": False,
              "approved_build": list(expected), "evidence": str(evidence),
              "binding_inputs": inputs, "binding": hashlib.sha256(canonical(inputs)).hexdigest(),
              "birth": birth, "read_pipe": pipe_identity,
              "before_sha256": hashlib.sha256(canonical(before)).hexdigest(),
              "after_sha256": hashlib.sha256(canonical(after)).hexdigest()}
    raw = canonical(record)
    if len(raw) > 8192:
        raise ValueError("diagnostic launch claim exceeded its bound")
    # Fixed decision ID, not a changing source/body hash or a fresh run ID.
    path = directory / ("decision-" + LAUNCH_DIAGNOSIS_ID + ".claim.json")
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    except FileExistsError as error:
        raise ValueError("decision65882 launch attempt already consumed; no second child authorized") from error
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    return {"path": str(path), "sha256": hashlib.sha256(raw).hexdigest(), "bytes": len(raw)}


def _prepare_launch_preexec(read_gate, expected):
    """Diagnostic-only Python preexec, never the product controller/native proof.

    Resolve libc functions, layouts, format, views and storage BEFORE fork. The
    child only invokes fixed clock/FD/priority/write syscalls and fills bounded
    scalar slots. Python/ctypes preexec is not claimed generally async-signal-safe.
    """
    if sys.platform != "darwin":
        raise ValueError("diagnostic preexec layout supports only approved Darwin")
    if (not 3 <= read_gate <= 65535 or not stat.S_ISFIFO(expected.st_mode)
            or expected.st_uid != os.geteuid()
            or fcntl.fcntl(read_gate, fcntl.F_GETFL) & os.O_ACCMODE != os.O_RDONLY):
        raise ValueError("diagnostic preexec requires the original owned read pipe")

    class Clock(ctypes.Structure):
        _fields_ = [("seconds", ctypes.c_long), ("nanoseconds", ctypes.c_long)]

    class StatHead(ctypes.Structure):
        _fields_ = [("dev", ctypes.c_int32), ("mode", ctypes.c_uint16), ("links", ctypes.c_uint16),
                    ("ino", ctypes.c_uint64), ("uid", ctypes.c_uint32), ("gid", ctypes.c_uint32)]

    library = ctypes.CDLL(None, use_errno=True)
    fstat = library.fstat
    fstat.argtypes, fstat.restype = (ctypes.c_int, ctypes.c_void_p), ctypes.c_int
    fcntl_call = library.fcntl
    fcntl_call.argtypes, fcntl_call.restype = (ctypes.c_int, ctypes.c_int, ctypes.c_int), ctypes.c_int
    setpriority = library.setpriority
    setpriority.argtypes, setpriority.restype = (ctypes.c_int, ctypes.c_uint32, ctypes.c_int), ctypes.c_int
    getpriority = library.getpriority
    getpriority.argtypes, getpriority.restype = (ctypes.c_int, ctypes.c_uint32), ctypes.c_int
    getpid = library.getpid
    getpid.argtypes, getpid.restype = (), ctypes.c_int
    clock_call = library.clock_gettime
    clock_call.argtypes, clock_call.restype = (ctypes.c_int, ctypes.POINTER(Clock)), ctypes.c_int
    write_call = library.write
    write_call.argtypes, write_call.restype = (ctypes.c_int, ctypes.c_void_p, ctypes.c_size_t), ctypes.c_ssize_t
    exit_call = library._exit
    exit_call.argtypes, exit_call.restype = (ctypes.c_int,), None
    read_storage, stderr_storage = ctypes.create_string_buffer(256), ctypes.create_string_buffer(256)
    read_head, stderr_head = StatHead.from_buffer(read_storage), StatHead.from_buffer(stderr_storage)
    read_pointer, stderr_pointer = ctypes.addressof(read_storage), ctypes.addressof(stderr_storage)
    if fstat(read_gate, read_pointer) != 0 or (read_head.dev, read_head.ino, read_head.uid) != (expected.st_dev, expected.st_ino, expected.st_uid):
        raise ValueError("Darwin diagnostic stat layout failed its pre-birth identity check")
    clock = Clock()
    clock_pointer = ctypes.pointer(clock)
    marker = bytearray(b"dream-launch-preexec mono_ns=" + b"0" * 20
                       + b" clock_ok=0 pid=" + b"0" * 10 + b" fd=" + b"0" * 5
                       + b" type=" + b"0" * 6 + b" access=9 identity=0 stderr_pipe=0 priority=+00 priority_ok=0\n")
    if len(marker) > 256:
        raise ValueError("preexec diagnostic marker exceeded256bytes")
    slots = {key: marker.index(key + b"=") + len(key) + 1 for key in
             (b"mono_ns", b"clock_ok", b"pid", b"fd", b"type", b"access", b"identity", b"stderr_pipe", b"priority", b"priority_ok")}
    marker_view = (ctypes.c_ubyte * len(marker)).from_buffer(marker)
    marker_pointer, marker_length = ctypes.addressof(marker_view), len(marker)
    expected_identity = (expected.st_dev, expected.st_ino, expected.st_uid)
    expected_uid = os.geteuid()

    def number(offset, width, value):
        for index in range(offset + width - 1, offset - 1, -1):
            marker[index] = 48 + value % 10
            value //= 10

    def preexec():
        priority_rc = setpriority(os.PRIO_PROCESS, 0, 10)
        priority = getpriority(os.PRIO_PROCESS, 0)
        read_rc, stderr_rc = fstat(read_gate, read_pointer), fstat(2, stderr_pointer)
        flags, stderr_flags = fcntl_call(read_gate, fcntl.F_GETFL, 0), fcntl_call(2, fcntl.F_GETFL, 0)
        owned_stderr = (stderr_rc == 0 and stderr_head.mode & 0o170000 == stat.S_IFIFO
                        and stderr_head.uid == expected_uid and stderr_flags >= 0
                        and stderr_flags & os.O_ACCMODE == os.O_WRONLY)
        if not owned_stderr or fcntl_call(2, fcntl.F_SETFL, stderr_flags | os.O_NONBLOCK) != 0:
            exit_call(126)
        number(slots[b"pid"], 10, getpid())
        number(slots[b"fd"], 5, read_gate)
        number(slots[b"type"], 6, read_head.mode & 0o170000 if read_rc == 0 else 0)
        number(slots[b"access"], 1, flags & os.O_ACCMODE if flags >= 0 else 9)
        valid_read = (read_rc == 0 and (read_head.dev, read_head.ino, read_head.uid) == expected_identity
                      and read_head.mode & 0o170000 == stat.S_IFIFO
                      and flags >= 0 and flags & os.O_ACCMODE == os.O_RDONLY)
        marker[slots[b"identity"]] = 49 if valid_read else 48
        marker[slots[b"stderr_pipe"]] = 49
        marker[slots[b"priority"]] = 45 if priority < 0 else 43
        number(slots[b"priority"] + 1, 2, abs(priority))
        marker[slots[b"priority_ok"]] = 49 if priority_rc == 0 and priority == 10 else 48
        clock_rc = clock_call(time.CLOCK_MONOTONIC, clock_pointer)
        now = clock.seconds * 1_000_000_000 + clock.nanoseconds
        known = (clock_rc == 0 and clock.seconds >= 0 and 0 <= clock.nanoseconds < 1_000_000_000
                 and now <= (1 << 64) - 1)
        number(slots[b"mono_ns"], 20, now if known else 0)
        marker[slots[b"clock_ok"]] = 49 if known else 48
        written = write_call(2, marker_pointer, marker_length)  # One bounded nonblocking attempt; no retry.
        # This diagnostic must not leave a different producer stderr policy.
        # No second marker/retry: restoration is not retrospectively claimed by
        # the preceding bytes. Refuse exec if the one restoration attempt fails.
        restored = fcntl_call(2, fcntl.F_SETFL, stderr_flags)
        if (restored != 0 or priority_rc != 0 or priority != 10 or not valid_read
                or not known or written != marker_length):
            exit_call(126)

    return preexec, {"marker_bytes": marker_length, "marker_limit_bytes": 256,
                     "storage_and_format_prepared_before_fork": True,
                     "clock_scope": "last child CLOCK_MONOTONIC sample before fixed marker write",
                     "stderr_flags_policy": "one restoration to original flags after marker; required-check failure exits126 before exec",
                     "restoration_return_exported": False,
                     "python_preexec_general_async_signal_safety_claimed": False}


class _LaunchDrain:
    """Bounded nonblocking output observation, also used after lease retirement."""
    def __init__(self, process, readiness):
        self.process, self.readiness = process, readiness
        self.stdout, self.stderr = bytearray(), bytearray()
        self.selector = selectors.DefaultSelector()
        self.events, self.seen, self.cursor, self.error = [], set(), 0, None
        self.preexec = None
        for name, stream in (("stdout", process.stdout), ("stderr", process.stderr)):
            os.set_blocking(stream.fileno(), False)
            self.selector.register(stream, selectors.EVENT_READ, name)

    def note(self, kind, observed=None, **fields):
        if kind in self.seen:
            return
        if len(self.events) >= 32:
            self.error = self.error or "observation_event_limit"
            return
        self.seen.add(kind)
        observed = time.clock_gettime_ns(time.CLOCK_MONOTONIC) if observed is None else observed
        self.events.append({"kind": kind, "observed_clock_monotonic_ns": observed,
                            "after_original_readiness": observed >= self.readiness, **fields})

    def stderr_lines(self, observed):
        while (end := self.stderr.find(b"\n", self.cursor)) >= 0:
            line = bytes(self.stderr[self.cursor:end])
            self.cursor = end + 1
            for state in (b"waiting", b"paid_hello"):
                if line.startswith(b"dream-bootstrap state=" + state + b" "):
                    self.note("gate_" + state.decode() + "_first_observed", observed)
            if line.startswith(b"dream-launch-preexec "):
                try:
                    if self.preexec is not None:
                        raise ValueError("duplicate preexec marker")
                    fields = line.removeprefix(b"dream-launch-preexec ").split(b" ")
                    values = dict(field.split(b"=", 1) for field in fields)
                    if len(line) + 1 > 256 or len(values) != len(fields) or set(values) != {
                            b"mono_ns", b"clock_ok", b"pid", b"fd", b"type", b"access", b"identity", b"stderr_pipe", b"priority", b"priority_ok"}:
                        raise ValueError("unexpected fixed preexec marker")
                    for key, value in values.items():
                        pattern = rb"[+-][0-9]{2}" if key == b"priority" else rb"[0-9]{1,20}"
                        if re.fullmatch(pattern, value) is None:
                            raise ValueError("invalid preexec scalar")
                    decoded = {key.decode(): int(value) for key, value in values.items()}
                    if (any(value > (1 << 64) - 1 for value in decoded.values())
                            or any(decoded[key] not in (0, 1) for key in
                                   ("clock_ok", "identity", "stderr_pipe", "priority_ok"))):
                        raise ValueError("out-of-range preexec marker")
                    self.preexec = decoded
                    self.note("preexec_marker_first_observed", observed,
                              child_emission_sample_clock_monotonic_ns=self.preexec["mono_ns"],
                              marker_pid_matches_owned_child=self.preexec["pid"] == self.process.pid)
                except ValueError:
                    self.error = self.error or "malformed_preexec_marker"

    def pump(self, timeout):
        self.note("selector_first_entered")
        if not self.selector.get_map():
            if timeout:
                time.sleep(min(timeout, 0.01))
            return
        try:
            events = self.selector.select(max(0, min(timeout, 0.01)))
        except OSError as error:
            self.note("selector_error", errno=error.errno)
            self.error = self.error or "selector_error"
            return
        for key, _ in events:
            observed = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            name = key.data
            self.note(name + "_first_selector_ready", observed)
            buffer, limit = (self.stdout, FRAME) if name == "stdout" else (self.stderr, 16 * 1024)
            try:
                chunk = os.read(key.fd, min(4096, limit + 1 - len(buffer)))
            except BlockingIOError:
                continue
            except OSError as error:
                self.note(name + "_read_error", errno=error.errno)
                self.error = self.error or name + "_read_error"
                self.selector.unregister(key.fileobj)
                continue
            if not chunk:
                self.note(name + "_eof", observed)
                self.selector.unregister(key.fileobj)
                continue
            self.note(name + "_first_bytes", observed)
            if len(buffer) + len(chunk) > limit:
                buffer.extend(chunk[:limit - len(buffer)])
                self.error = self.error or name + "_byte_limit"
                self.selector.unregister(key.fileobj)
                continue
            buffer.extend(chunk)
            if name == "stderr":
                self.stderr_lines(observed)
            elif b"\n" in buffer:
                self.note("hello_line_first_observed", observed)

    def close(self):
        self.selector.close()


def _settle_source_free(process, drain, timing):
    """No sample and no blocking output read; revoke before entering this owner."""
    if not process.stdin.closed:
        try:
            process.stdin.close()  # Never written, therefore no buffered payload to flush.
        except OSError:
            pass
    timing["stdin_retired_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    for stage, duration in ((None, 2), (signal.SIGTERM, 2), (signal.SIGKILL, 2)):
        if process.poll() is not None:
            break
        if stage is not None:
            try:
                os.killpg(process.pid, stage)  # Only this still-unreaped, own new session.
            except ProcessLookupError:
                process.poll()
            timing["term_sent_ns" if stage == signal.SIGTERM else "kill_sent_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        until = time.monotonic() + duration
        while process.poll() is None and time.monotonic() < until:
            drain.pump(min(0.01, max(0, until - time.monotonic())))
    timing["settlement_observed_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    if process.poll() is not None:
        drain.note("owned_child_actual_reap")
        until = time.monotonic() + 0.2
        while drain.selector.get_map() and time.monotonic() < until:
            drain.pump(min(0.01, max(0, until - time.monotonic())))
    return process.returncode


def check_launch_diagnosis_controls():
    """Private claim/pipe controls only. Never spend the real65882 claim or fork."""
    from types import SimpleNamespace
    from unittest import mock
    import copy
    accepted, refused = [], []
    with tempfile.TemporaryDirectory(prefix="dream-c1-diagnostic-controls-") as own:
        root = Path(own).resolve()
        target, evidence = root / "target", root / "evidence"
        target.mkdir(mode=0o700)
        evidence.mkdir(mode=0o700)
        source, compiler, worker = LAUNCH_DIAGNOSIS_BUILD
        provision = {"source_commit": source, "compiler_sha256": compiler, "binary_sha256": worker,
                     "bootstrap_profile": "dream-bootstrap/1", "metrics_profile": "c1-native-debt/1",
                     "build_profile": "release", "default_features": True,
                     "explicit_features": [], "effective_features": ["acp-agent"], "test_fixtures": False}
        inputs = {"profile": "dream-bootstrap/1", "purpose": "inspect-c1", "probe": True,
                  "context": None, "op_id": fresh_ulid(), "run_id": fresh_ulid(), "nonce": fresh_ulid(),
                  "incarnation": 1, "worker_sha256": worker}
        before = {"admissible": True, "refusals": [], "make_entry": {"value": "diagnose-dream-c1-launch"},
                  "directories": {"target": str(target)}}
        after = {"admissible": True, "refusals": [], "before_sha256": hashlib.sha256(canonical(before)).hexdigest(),
                 "runner": {"sha256": worker}, "selected": {"deps_executable": {"sha256": worker}}}
        claim_path = target / ".dream-c1-launch-diagnosis" / ("decision-" + LAUNCH_DIAGNOSIS_ID + ".claim.json")
        with mock.patch.dict(globals(), {"ROOT": root, "APPROVED_NATIVE_BUILD": LAUNCH_DIAGNOSIS_BUILD}):
            for label, area, field, value in (
                ("unapproved_source", "provision", "source_commit", "0" * 40),
                ("unapproved_compiler", "provision", "compiler_sha256", "0" * 64),
                ("unapproved_artifact", "provision", "binary_sha256", "0" * 64),
                ("fixture_build", "provision", "test_fixtures", True),
                ("refused_before_closure", "before", "admissible", False),
                ("refused_after_closure", "after", "admissible", False),
                ("unbound_closure", "after", "before_sha256", "0" * 64),
                ("missing_selected_artifact", "after", "selected", None),
                ("nonnull_probe", "inputs", "context", {}),
            ):
                values = copy.deepcopy({"provision": provision, "before": before, "after": after, "inputs": inputs})
                values[area][field] = value
                if area == "before":
                    values["after"]["before_sha256"] = hashlib.sha256(canonical(values["before"])).hexdigest()
                try:
                    _claim_launch_diagnosis(target, evidence, values["provision"], values["before"], values["after"], values["inputs"], fresh_ulid(), {})
                except ValueError:
                    pass
                else:
                    raise AssertionError("invalid diagnostic proof accepted: " + label)
                assert not claim_path.exists()
                refused.append(label)
            claim = _claim_launch_diagnosis(target, evidence, provision, before, after, inputs, fresh_ulid(), {"fd": 7})
            original = claim_path.read_bytes()
            value = json.loads(original)
            assert hashlib.sha256(original).hexdigest() == claim["sha256"]
            assert value["diagnostic_only"] is True and value["runtime_conformance"] is False
            assert value["binding_inputs"]["context"] is None and claim_path.stat().st_mode & 0o777 == 0o600
            accepted.append("private_O_EXCL_claim_records_frozen_identity_before_birth")
            replacement = {**inputs, "run_id": fresh_ulid(), "nonce": fresh_ulid()}
            try:
                _claim_launch_diagnosis(target, evidence, provision, before, after, replacement, fresh_ulid(), {"fd": 8})
            except ValueError:
                pass
            else:
                raise AssertionError("a second diagnosis attempt was accepted")
            assert claim_path.read_bytes() == original
            refused.append("repeat_with_fresh_ids_cannot_replace_consumed_claim")

        # All process/clock/signal effects below are stubs; only private paths
        # are real. The production owner has no test release/adoption API.
        with mock.patch.dict(globals(), {"_SOURCE_FREE_OWNER": None}):
            with _SourceFreeCustody(evidence, parent=root) as known:
                known_path = known.path
            assert not known_path.exists() and _SOURCE_FREE_OWNER is None
            accepted.append("known_no_birth_cleans_only_its_private_job")
            private_evidence = root / "quarantine-evidence"
            private_evidence.mkdir(mode=0o700)
            stdin = SimpleNamespace(closed=False)
            stdin.close = lambda: setattr(stdin, "closed", True)
            process = SimpleNamespace(pid=4321, returncode=None, stdin=stdin, poll=lambda: None)
            drain = SimpleNamespace(pump=mock.Mock(), note=mock.Mock())
            import itertools
            with _SourceFreeCustody(private_evidence, parent=root) as unknown:
                unknown.attempted, unknown.process, unknown.drain = True, process, drain
                unknown.job = unknown.path / "private-job"
                unknown.job.mkdir(mode=0o700)
                unknown.claim = {"path": str(claim_path)}  # Private control claim only.
                with mock.patch.object(time, "monotonic", side_effect=itertools.count(0, 10).__next__), \
                        mock.patch.object(time, "clock_gettime_ns", return_value=1), \
                        mock.patch.object(os, "killpg") as signals:
                    assert _settle_source_free(process, drain, {}) is None
                    assert signals.call_args_list == [mock.call(4321, signal.SIGTERM), mock.call(4321, signal.SIGKILL)]
            assert _SOURCE_FREE_OWNER is unknown and unknown.path.exists() and unknown.job.exists()
            assert unknown.process is process and unknown.drain is drain and claim_path.exists()
            record = json.loads((private_evidence / "hello.quarantine.json").read_bytes())
            assert record["actual_reap"] is False and record["settled"] is False and record["state"] == "quarantined"
            assert record["retained_process_handle"] and record["retained_output_owners"]
            try:
                _SourceFreeCustody(private_evidence, parent=root)
            except ValueError:
                pass
            else:
                raise AssertionError("quarantined owner allowed a second source-free birth")
            accepted.append("unknown_settlement_retains_original_handle_IO_job_and_private_claim")
            refused.append("retained_unknown_owner_forbids_new_birth")

        for overflow in (False, True):
            stdout_read, stdout_write = os.pipe()
            stderr_read, stderr_write = os.pipe()
            process = SimpleNamespace(pid=1234, stdout=os.fdopen(stdout_read, "rb", buffering=0),
                                      stderr=os.fdopen(stderr_read, "rb", buffering=0))
            observer = _LaunchDrain(process, 0)
            try:
                if overflow:
                    os.set_blocking(stderr_write, False)
                    for _ in range(5):
                        os.write(stderr_write, b"x" * 4096)
                        observer.pump(0)
                    assert len(observer.stderr) == 16 * 1024 and observer.error == "stderr_byte_limit"
                    refused.append("stderr_growth_refused_at_16KiB")
                else:
                    marker = (b"dream-launch-preexec mono_ns=00000000000000000123 clock_ok=1 pid=0000001234 "
                              b"fd=00007 type=004096 access=0 identity=1 stderr_pipe=1 priority=+10 priority_ok=1\n")
                    os.write(stderr_write, marker + b"dream-bootstrap state=waiting profile=dream-bootstrap/1 argv_admissions=0 channel_admissions=0 owner_admissions=0\n")
                    os.write(stdout_write, b"private-observer-bytes\n")
                    observer.pump(0)
                    assert observer.preexec["mono_ns"] == 123
                    assert observer.readiness == 0 and all(event["after_original_readiness"] for event in observer.events)
                    assert {"preexec_marker_first_observed", "gate_waiting_first_observed", "hello_line_first_observed"} <= observer.seen
                    accepted.append("emission_and_late_observation_clocks_remain_distinct")
                os.close(stdout_write)
                stdout_write = None
                os.close(stderr_write)
                stderr_write = None
                for _ in range(4):
                    observer.pump(0)
                if not overflow:
                    assert {"stdout_eof", "stderr_eof"} <= observer.seen
                    accepted.append("nonblocking_pipe_EOF_is_timestamped")
                    for index in range(40):
                        observer.note("private-capacity-control-" + str(index))
                    assert len(observer.events) == len(observer.seen) == 32
                    refused.append("event_and_seen_growth_refused_at32")
            finally:
                observer.close()
                for descriptor in (stdout_write, stderr_write):
                    if descriptor is not None:
                        os.close(descriptor)
                process.stdout.close()
                process.stderr.close()
        if sys.platform == "darwin":
            read_fd, write_fd = os.pipe()
            try:
                callback, metadata = _prepare_launch_preexec(read_fd, os.fstat(read_fd))
                assert callable(callback) and metadata["marker_bytes"] <= 256
                # Deliberately never invoke callback/change this process's priority.
                accepted.append("preexec_format_storage_and_Darwin_layout_prepared_without_fork")
                expected_stat = os.fstat(read_fd)

                class RefusedExec(Exception):
                    pass

                class Stub:
                    def __init__(self, call):
                        self.call = call

                    def __call__(self, *arguments):
                        return self.call(*arguments)

                for fault in (None, "priority_set", "priority_value", "read_access", "stderr_type", "restore", "clock", "partial_marker"):
                    events, writes = [], []

                    def fstat_stub(fd, pointer):
                        kind = stat.S_IFREG if fd == 2 and fault == "stderr_type" else stat.S_IFIFO
                        packed = struct.pack("=iHHQII", expected_stat.st_dev, kind | 0o600, 0,
                                             expected_stat.st_ino, expected_stat.st_uid, expected_stat.st_gid)
                        ctypes.memmove(pointer, packed, len(packed))
                        return 0

                    def fcntl_stub(fd, operation, argument):
                        if operation == fcntl.F_GETFL:
                            return os.O_WRONLY if fd == 2 or fault == "read_access" else os.O_RDONLY
                        assert fd == 2 and operation == fcntl.F_SETFL
                        events.append("restore" if argument == os.O_WRONLY else "nonblocking")
                        return -1 if fault == "restore" and argument == os.O_WRONLY else 0

                    def clock_stub(_which, pointer):
                        pointer.contents.seconds, pointer.contents.nanoseconds = 100, 20
                        return -1 if fault == "clock" else 0

                    def write_stub(fd, pointer, length):
                        assert fd == 2 and length <= 256
                        events.append("write")
                        writes.append(ctypes.string_at(pointer, length))
                        return length - 1 if fault == "partial_marker" else length

                    def exit_stub(code):
                        raise RefusedExec(code)

                    library = SimpleNamespace(
                        fstat=Stub(fstat_stub), fcntl=Stub(fcntl_stub),
                        setpriority=Stub(lambda *_args: -1 if fault == "priority_set" else 0),
                        getpriority=Stub(lambda *_args: 5 if fault == "priority_value" else 10),
                        getpid=Stub(lambda: 1234), clock_gettime=Stub(clock_stub),
                        write=Stub(write_stub), _exit=Stub(exit_stub))
                    # Every native callback target is replaced BEFORE preparation.
                    # Never invoke the actual priority/_exit preexec in this process.
                    with mock.patch.object(ctypes, "CDLL", return_value=library):
                        prepared, _ = _prepare_launch_preexec(read_fd, expected_stat)
                        try:
                            prepared()
                        except RefusedExec as error:
                            assert fault is not None and error.args == (126,)
                        else:
                            assert fault is None
                    if fault == "stderr_type":
                        assert not writes
                    else:
                        assert len(writes) == 1 and events[-2:] == ["write", "restore"]
                    if fault is None:
                        accepted.append("stubbed_preexec_restores_original_flags_after_single_marker")
                    else:
                        refused.append("stubbed_preexec_refuses_" + fault)
            finally:
                os.close(read_fd)
                os.close(write_fd)
        marker = (b"dream-launch-preexec mono_ns=00000000000000000123 clock_ok=1 pid=0000001234 "
                  b"fd=00007 type=004096 access=0 identity=1 stderr_pipe=1 priority=+10 priority_ok=1\n")
        for label, raw in (
            ("invalid_preexec_clock_flag", marker.replace(b"clock_ok=1", b"clock_ok=9")),
            ("overflow_preexec_clock", marker.replace(b"00000000000000000123", b"18446744073709551616")),
            ("duplicate_preexec_scalar", marker.replace(b"fd=00007", b"fd=00007 fd=00008")),
        ):
            observer = object.__new__(_LaunchDrain)
            observer.stderr, observer.cursor, observer.error, observer.preexec = bytearray(raw), 0, None, None
            observer.stderr_lines(456)
            assert observer.error == "malformed_preexec_marker" and observer.preexec is None
            refused.append(label)
        import contextlib
        import io
        for other in ("--prepare-only", "--worker-only", "--seat-only", "--prepare-development", "--check-profile-only"):
            with mock.patch.object(sys, "argv", ["test-dream-c1.py", "--diagnose-launch", other]), \
                    contextlib.redirect_stderr(io.StringIO()), \
                    mock.patch.object(subprocess, "Popen", side_effect=AssertionError("CLI control must not spawn")):
                try:
                    main()
                except SystemExit as error:
                    assert error.code == 2
                else:
                    raise AssertionError("diagnostic mode combination was accepted")
            refused.append("mutually_exclusive_diagnosis_" + other.removeprefix("--"))
    return {"scope": "private helper/pipe controls only; no child, candidate, real claim or runtime evidence",
            "accepted": accepted, "refused": refused}


def hello(runner, evidence, native_provision, *, diagnostic_target=None, closure_before=None, closure_after=None):
    """Ordinary source-free probe, with one separately claimed diagnostic mode."""
    diagnostic_only = diagnostic_target is not None
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
    with _SourceFreeCustody(evidence) as custody:
        outer = custody.path
        os.chmod(outer, 0o700)
        job = outer / run_id
        job.mkdir(mode=0o700)
        custody.job = job
        (job / "tmp").mkdir(mode=0o700)
        read_gate, lease = os.pipe()
        process = drain = claim = preexec_details = None
        timing = {}
        permit_attempted = readiness_failed = False
        parent_cpu = debt = control_cpu = written = value = None
        ready_ok = False
        try:
            if not 3 <= read_gate <= 65535:
                raise ValueError("bootstrap inherited descriptor outside fixed range")
            gate_identity, writer_identity = os.fstat(read_gate), os.fstat(lease)
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

            preexec = background
            if diagnostic_only:
                preexec, preexec_details = _prepare_launch_preexec(read_gate, gate_identity)
                claim = _claim_launch_diagnosis(
                    diagnostic_target, evidence, native_provision, closure_before, closure_after,
                    inputs, birth, {"fd": read_gate, "dev": str(gate_identity.st_dev),
                                    "ino": str(gate_identity.st_ino), "uid": gate_identity.st_uid})
                custody.claim = claim
            # This private Python process boundary is not Rust native-process
            # conformance. Diagnostic preexec is never installed in ordinary mode.
            cpu_start = time.thread_time_ns()
            job_deadline = time.clock_gettime_ns(time.CLOCK_MONOTONIC) + 30_000_000_000
            custody.attempted = True
            process = subprocess.Popen(
                [str(runner), "dream", "inspect-c1", "--job-dir", str(job)],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                env={"LANG": "C.UTF-8", "TMPDIR": str(job / "tmp"), "BBOXY_DREAM_BOOTSTRAP": token},
                cwd=job, pass_fds=(read_gate,), start_new_session=True, preexec_fn=preexec,
            )
            custody.process = process  # Preserve the effect before timing/verdict code.
            returned_at = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            timing["spawn_return_ns"] = returned_at
            readiness = min(returned_at + 250_000_000, job_deadline)
            timing["original_readiness_deadline_ns"] = readiness
            os.close(read_gate)
            read_gate = None
            drain = _LaunchDrain(process, readiness)
            custody.drain = drain
            drain.pump(0)
            if b"\n" in drain.stdout:
                raise ValueError("worker emitted Hello before the sole parent permit attempt")
            record, record_binding = bootstrap_record(inputs, birth, readiness)
            if record_binding != binding:
                raise ValueError("frozen bootstrap binding changed before grant")
            _retain_launch_bytes(evidence, "hello.bootstrap.json", canonical({
                "argv": [str(runner), "dream", "inspect-c1", "--job-dir", str(job)],
                "environment": {"LANG": "C.UTF-8", "TMPDIR": str(job / "tmp"), "BBOXY_DREAM_BOOTSTRAP": token},
                "binding_inputs": inputs, "record_hex": record.hex(),
                "spawn_return_clock_monotonic_ns": returned_at,
                "original_readiness_clock_monotonic_ns": readiness,
                "diagnostic_only": diagnostic_only, "runtime_conformance": False,
                "preexec": preexec_details, "one_shot_claim": claim}), 16 * 1024)
            permit = ctypes.create_string_buffer(record, 128)
            parent_cpu = time.thread_time_ns() - cpu_start
            ended_at = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            debt = 9 * max(1_000_000, parent_cpu)
            due = ended_at + debt
            timing["parent_debt_due_ns"] = due
            if parent_cpu < 0 or due >= readiness or readiness >= 1 << 64:
                readiness_failed = True
                raise ValueError("private probe grant debt exceeds original readiness")
            while time.clock_gettime_ns(time.CLOCK_MONOTONIC) < due:
                time.sleep(max(0, due - time.clock_gettime_ns(time.CLOCK_MONOTONIC)) / 1_000_000_000)
            timing["parent_wait_end_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            control_start = time.thread_time_ns()
            timing["permit_before_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            if timing["permit_before_ns"] >= readiness:
                readiness_failed = True
                raise ValueError("private probe parent missed original readiness before permit; no grant written")
            permit_attempted = True
            written = libc_write(lease, permit, 128)  # Exactly one syscall, including EINTR/partial failure.
            raw_errno = ctypes.get_errno()
            timing["permit_after_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            control_cpu = time.thread_time_ns() - control_start
            if written != 128 or control_cpu < 0 or control_cpu > 1_000_000:
                raise ValueError(f"private probe permit failed ({written}, errno {raw_errno})")
            if timing["permit_after_ns"] >= readiness:
                readiness_failed = True
                raise ValueError("private probe permit crossed original readiness; retaining actual write effect")
            while b"\n" not in drain.stdout:
                now = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
                if now >= readiness:
                    readiness_failed = True
                    timing["readiness_failure_observed_ns"] = now
                    raise ValueError("real gated worker missed original250ms Hello window")
                drain.pump((readiness - now) / 1_000_000_000)
                if drain.error:
                    raise ValueError("bounded launch observer refused: " + drain.error)
                if "stdout_eof" in drain.seen and b"\n" not in drain.stdout:
                    readiness_failed = True
                    raise ValueError("real gated worker exited without Hello")
            hello_event = next(event for event in drain.events if event["kind"] == "hello_line_first_observed")
            if hello_event["observed_clock_monotonic_ns"] >= readiness:
                readiness_failed = True
                raise ValueError("Hello was observed after original250ms readiness; late output is diagnostic only")
            if not drain.stdout.endswith(b"\n") or b"\n" in drain.stdout[:-1]:
                raise ValueError("real worker emitted extra/oversized Hello")
            value = strict_json(bytes(drain.stdout[:-1]))
            expected = {"protocol": "bboxy.dream.c1/1", "type": "hello", "algorithm": "d0-c1/1",
                        "schema_sha256": SCHEMA_SHA256, "ir_major": 1, "ir_minor_max": 3}
            if value != expected:
                raise ValueError("real built worker Hello is incompatible with approved schema")
            ready_ok = True
        finally:
            # Retire the scheduling lease BEFORE output diagnostics or settlement.
            if lease is not None:
                timing["lease_retirement_before_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
                os.close(lease)
                lease = None
                timing["lease_retirement_observed_ns"] = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
            if read_gate is not None:
                os.close(read_gate)
                read_gate = None
            if process is not None:
                if drain is None:
                    drain = _LaunchDrain(process, timing.get("original_readiness_deadline_ns", 0))
                    custody.drain = drain
                code = _settle_source_free(process, drain, timing)
                if process.returncode is not None:
                    drain.close()
                    for stream in (process.stdin, process.stdout, process.stderr):
                        stream.close()
                    custody.settled = True
                stdout_record = _retain_launch_bytes(evidence, "hello.stdout", bytes(drain.stdout), FRAME)
                stderr_record = _retain_launch_bytes(evidence, "hello.stderr", bytes(drain.stderr), 16 * 1024)
                lifecycle = {"scope": "private source-free controller; not native-parent conformance",
                             "diagnostic_only": diagnostic_only, "runtime_conformance": False,
                             "op_id": operation, "run_id": run_id, "nonce": nonce, "birth": birth,
                             "binding": binding, "worker_sha256": native_provision["binary_sha256"],
                             "pid": process.pid, "actual_reap": process.returncode is not None,
                             "settled": custody.settled, "quarantined": not custody.settled,
                             "outer_directory": str(outer), "job_directory": str(job),
                             "returncode": code, "permit_attempted": permit_attempted, "permit_bytes": written,
                             "controller_cpu_ns": parent_cpu, "controller_debt_ns": debt,
                             "permit_control_cpu_ns": control_cpu, "stdout_frame_bytes": len(drain.stdout),
                             "stdout_record": stdout_record,
                             "stderr_bytes": len(drain.stderr), "stderr_record": stderr_record,
                             "original_readiness_failed": readiness_failed,
                             "readiness_verdict": "passed" if ready_ok else "failed",
                             "controller_clock_monotonic_ns": timing, "first_observations": drain.events,
                             "preexec": drain.preexec, "preexec_format": preexec_details,
                             "one_shot_claim": claim, "observation_error": drain.error,
                             "output_eof_observed": {"stdout_eof", "stderr_eof"} <= drain.seen,
                             "late_output_cannot_change_readiness_verdict": True}
                _retain_launch_bytes(evidence, "hello.lifecycle.json", canonical(lifecycle), 32 * 1024)
        if process.returncode is None:
            raise ValueError("source-free child remains unproved stopped after owned escalation")
        if process.returncode != 0:
            raise ValueError(f"real gated probe did not settle successfully: {process.returncode}")
        if drain.error or not drain.stdout.endswith(b"\n") or b"\n" in drain.stdout[:-1]:
            raise ValueError("source-free probe emitted malformed/extra bounded output")
        return {"runner": str(runner), "runner_sha256": sha(runner), "hello": value,
                "eof_exit_code": process.returncode, "actual_reap": True, "inspection_started": False,
                "diagnostic_only": diagnostic_only, "runtime_conformance": False,
                "bootstrap_profile": "dream-bootstrap/1", "probe": True,
                "run_id": run_id, "nonce": nonce, "op_id": operation, "birth": birth,
                "incarnation": incarnation, "binding": binding,
                "pipe_dev": gate_identity.st_dev, "pipe_ino": gate_identity.st_ino,
                "writer_pipe_dev": writer_identity.st_dev, "writer_pipe_ino": writer_identity.st_ino,
                "permit_attempts": int(permit_attempted), "permit_bytes": written,
                "private_controller_startup_cpu_ns": parent_cpu, "private_controller_debt_ns": debt,
                "permit_control_cpu_ns": control_cpu, "one_shot_claim": claim,
                "scope": "instrumented source-free diagnosis only" if diagnostic_only else "ordinary uninstrumented source-free probe"}


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
        if not 0 <= count <= cap or (tag == "compute_owner_create" and count != 1):
            raise ValueError("producer native count differs from fixed cap: " + tag)
        if count == 0:
            for field in ("cpu_ns", "wall_ns", "max_cpu_ns", "max_wall_ns", "inclusive_max_cpu_ns",
                          "inclusive_max_wall_ns", "app_max_cpu_ns", "effects", "failures_with_effect"):
                if metrics.get(prefix + field) != 0:
                    raise ValueError("unattempted producer native operation has nonzero cost/effect: " + field)
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
    compute_max = known_duration(metrics.get("producer_max_unit_cpu_ns"), "producer raw compute maximum")
    if not captured <= compute_max <= inclusive:
        raise ValueError("producer raw compute maximum is outside its verified capture cutoffs")
    for field in ("input", "protocol", "settlement", "output"):
        name = "producer_control_parts_" + field + "_max_cpu_ns"
        if finite_number(metrics.get(name), name) > 1_000_000:
            raise ValueError("producer control part exceeded1ms: " + field)
    # These observations overlap; do not add process/owner/span CPU totals.
    for field in ("span_cpu_ns", "owner_thread_cpu_ns", "process_user_cpu_ns", "process_system_cpu_ns",
                  "wall_ns", "debt_total_ns", "rest_observed_ns"):
        known_duration(metrics.get("producer_terminal_" + field), "producer " + field)


def check_native_profile_controls():
    """Private numeric-validator controls; never process/timing evidence or a corpus.

    Each refusal changes one field of an already accepted input and invokes the
    same native_debt_bounds validator used by the final receipt gate.
    """
    baseline = {
        "native_metrics_profile": "c1-native-debt/1", "cost_known": True,
        "work_debt_poisoned": False, "work_debt_remaining_ns": 0,
        "metrics_capture_cutoff": "after_reap_settlement_before_terminal_metrics_write",
        "native_overruns": 0, "native_clock_failures": 0, "work_debt_admission_violations": 0,
        "native_process_calls": 1, "native_publish_calls": 2,
        "native_process_max_cpu_ns": 1_750_000, "native_process_max_wall_ns": 2_000_000,
        "native_publish_max_cpu_ns": 1_000_000, "native_publish_max_wall_ns": 2_000_000,
        "native_app_max_cpu_ns": 750_000, "inclusive_max_cpu_ns": 2_500_000,
        "worker_spawn_cpu_ns": 1_750_000, "work_debt_spans": 3,
        "work_debt_charged_ns": 67_500_000,
        "producer_native_metrics_profile": "c1-native-debt/1",
        "producer_terminal_cutoff": "joined_final_control_pre_telemetry",
        "native_metrics_authorized": True, "native_provision_development": False,
        "native_provision_control_only": False, "producer_native_profile_verified": True,
        "producer_cost_known": 1, "producer_terminal_remaining_debt_ns": 0,
        "producer_terminal_app_max_cpu_ns": 750_000,
        "producer_terminal_control_max_cpu_ns": 500_000,
        "producer_terminal_captured_max_cpu_ns": 2_500_000,
        "producer_terminal_inclusive_max_cpu_ns": 2_500_000,
        "producer_max_unit_cpu_ns": 2_500_000,
    }
    for tag, count in (("compute_owner_create", 1), ("publish_noreplace", 2)):
        prefix = "producer_native_" + tag + "_"
        baseline.update({prefix + "count": count, prefix + "cpu_ns": count * 1_750_000,
                         prefix + "wall_ns": count * 2_000_000,
                         prefix + "max_cpu_ns": 1_750_000, prefix + "max_wall_ns": 2_000_000,
                         prefix + "inclusive_max_cpu_ns": 2_500_000,
                         prefix + "inclusive_max_wall_ns": 3_000_000,
                         prefix + "app_max_cpu_ns": 750_000,
                         prefix + "effects": count, prefix + "failures_with_effect": 0})
    for part in ("input", "protocol", "settlement", "output"):
        baseline["producer_control_parts_" + part + "_max_cpu_ns"] = 500_000
    for field in ("span_cpu_ns", "owner_thread_cpu_ns", "process_user_cpu_ns",
                  "process_system_cpu_ns", "wall_ns", "debt_total_ns", "rest_observed_ns"):
        baseline["producer_terminal_" + field] = 67_500_000
    native_debt_bounds(baseline)

    zero_publication = dict(baseline)
    publication = "producer_native_publish_noreplace_"
    publication_fields = ("count", "cpu_ns", "wall_ns", "max_cpu_ns", "max_wall_ns",
                          "inclusive_max_cpu_ns", "inclusive_max_wall_ns", "app_max_cpu_ns",
                          "effects", "failures_with_effect")
    zero_publication.update({publication + field: 0 for field in publication_fields})
    native_debt_bounds(zero_publication)

    rejected = []

    def refuses(label, field, value=None, *, remove=False, source=baseline):
        changed = dict(source)
        if remove:
            del changed[field]
        else:
            changed[field] = value
        try:
            native_debt_bounds(changed)
        except ValueError:
            rejected.append(label)
        else:
            raise AssertionError("numeric native/debt control was accepted: " + label)

    for label, field, value in (
        ("raw_above_terminal", "producer_max_unit_cpu_ns", 2_500_001),
        ("raw_below_capture", "producer_max_unit_cpu_ns", 2_499_999),
        ("parent_app_over_1ms", "native_app_max_cpu_ns", 1_000_001),
        ("child_app_over_1ms", "producer_terminal_app_max_cpu_ns", 1_000_001),
        ("child_control_over_1ms", "producer_terminal_control_max_cpu_ns", 1_000_001),
        ("child_protocol_over_1ms", "producer_control_parts_protocol_max_cpu_ns", 1_000_001),
        ("parent_native_over_2ms", "native_process_max_cpu_ns", 2_000_001),
        ("child_native_over_2ms", "producer_native_compute_owner_create_max_cpu_ns", 2_000_001),
        ("child_publication_over_2ms", publication + "max_cpu_ns", 2_000_001),
        ("parent_unknown_cost", "cost_known", False),
        ("child_unknown_cost", "producer_cost_known", 0),
        ("parent_remaining_debt", "work_debt_remaining_ns", 1),
        ("child_remaining_debt", "producer_terminal_remaining_debt_ns", 1),
        ("unauthorized_profile", "native_metrics_authorized", False),
        ("unverified_profile", "producer_native_profile_verified", False),
        ("private_development_class", "native_provision_development", True),
        ("control_only_class", "native_provision_control_only", True),
        ("wrong_parent_cutoff", "metrics_capture_cutoff", "before_settlement"),
        ("wrong_child_cutoff", "producer_terminal_cutoff", "pre_descriptor"),
        ("sentinel_raw_cpu", "producer_max_unit_cpu_ns", (1 << 64) - 1),
        ("sentinel_child_usage", "producer_terminal_process_user_cpu_ns", (1 << 64) - 1),
        ("sentinel_spawn_cpu", "worker_spawn_cpu_ns", (1 << 64) - 1),
        ("missing_compute_owner", "producer_native_compute_owner_create_count", 0),
    ):
        refuses(label, field, value)
    for field in ("native_metrics_authorized", "producer_native_profile_verified",
                  "producer_native_metrics_profile", "producer_terminal_cutoff"):
        refuses("missing_" + field, field, remove=True)
    for field in publication_fields[1:]:
        refuses("zero_publication_nonzero_" + field, publication + field, 1,
                source=zero_publication)
    return {"scope": "private numeric-validator causal controls; no runtime/process evidence",
            "accepted": ["verified_2_5ms_inclusive", "cancelled_before_publication"],
            "refused": rejected}


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
    paced_owner_bounds(metrics)
    if (value.get("established") != ["real_multimib_blob_workload"]
            or finite_number(value.get("blob_bytes"), "actual multi-MiB Store blob") < 2 * 1024 * 1024
            or any(value.get(flag) is not True for flag in ("actual_reap", "source_unchanged", "blob_unchanged"))
            or metrics.get("producer_metrics_observed") is not True
            or metrics.get("rest_skips") != 0 or metrics.get("rest_violations") != 0
            or finite_number(metrics.get("max_unit_cpu_ns"), "multi-MiB host unit") > 1_000_000
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
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--prepare-only", action="store_true",
                        help="build/probe the two pinned artifacts; does not verify the consumer")
    mode.add_argument("--worker-only", action="store_true",
                        help="build/provision the ordinary release sibling; does not verify the consumer")
    mode.add_argument("--seat-only", action="store_true",
                        help="build the private optimized seat fixture without executing any worker")
    mode.add_argument("--prepare-development", action="store_true")
    mode.add_argument("--check-profile-only", action="store_true")
    mode.add_argument("--diagnose-launch", action="store_true",
                      help="decision65882 one-shot instrumented source-free launch; never runtime conformance")
    parser.add_argument("--development-source", type=Path)
    parser.add_argument("--development-worker", type=Path)
    args = parser.parse_args()
    evidence = Path(tempfile.mkdtemp(prefix="bboxy-dream-c1-gate-"))
    os.chmod(evidence, 0o700)
    receipt = {"commands": [], "complete": False, "evidence": str(evidence)}
    if args.diagnose_launch:
        receipt.update(diagnostic_only=True, runtime_conformance=False, diagnostic_decision_id=LAUNCH_DIAGNOSIS_ID)
    code = 1
    try:
        receipt["build_closure_controls"] = check_build_closure_controls()
        receipt["native_profile_controls"] = check_native_profile_controls()
        receipt["launch_diagnosis_controls"] = check_launch_diagnosis_controls()
        if args.check_profile_only:
            receipt["scope"] = "Private build-closure, numeric profile and launch-diagnosis controls only; no artifact execution or runtime conformance"
            code = 0
            return code
        if sys.platform != "darwin":
            raise ValueError("C1's required native frame/process witness needs the supported macOS fixture")
        if args.diagnose_launch:
            claim = ROOT / "target/.dream-c1-launch-diagnosis" / ("decision-" + LAUNCH_DIAGNOSIS_ID + ".claim.json")
            if claim.exists() or claim.is_symlink():
                raise ValueError("decision65882 launch attempt already consumed; no second invocation authorized")
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
        # Record Make's exact existing dev/test defaults as part of this build
        # closure. Host/build dependencies can make otherwise identical release
        # commands differ; only the exact approved executable hash qualifies.
        # Refuse every other profile override, including an altered default.
        make_defaults = {"CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
                              "CARGO_PROFILE_DEV_OPT_LEVEL": "1", "CARGO_PROFILE_TEST_OPT_LEVEL": "1"}
        make_default_names = sorted(name for name, expected in make_defaults.items()
                           if os.environ.get(name) == expected)
        conflicting = sorted(name for name, value in os.environ.items() if value and name not in make_default_names and
                             (name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS")
                              or name.endswith("_RUSTFLAGS")
                              or name.startswith("CARGO_PROFILE_")))
        receipt["build_overrides"] = {"conflicting_variable_names": conflicting,
                                      "make_dev_test_defaults": make_default_names,
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
        if args.seat_only:
            run(["cargo", "build", "--locked", "--jobs", "2", "--profile", "dream-c1-fixture",
                 "-p", "bboxy450-harness", "--bin", "bboxy450-harness", "--features", "test-fixtures",
                 "--target-dir", str(target)], environment, evidence, receipt["commands"])
            seat_runner = target / "dream-c1-fixture/bboxy450-harness"
            receipt["private_seat_fixture"] = {
                "path": str(seat_runner), "sha256": sha(seat_runner),
                "source_commit": revision, "profile": "dream-c1-fixture",
                "features": ["test-fixtures"], "worker_executed": False}
            receipt["prepared"] = True
            receipt["scope"] = "Private optimized seat fixture build only; no worker probe or consumer conformance"
            code = 0
            return code
        worker_command = ["cargo", "build", "--locked", "--jobs", "2", "--release", "-p", "bboxy450-harness",
                          "--bin", "bboxy450-harness", "--target-dir", str(target)]
        receipt["build_closure_before"] = build_closure_before(
            cargo_argv=worker_command, target=target, source_hashes=receipt["source_hashes"],
            compiler_version=receipt["compiler"], cargo_version=receipt["cargo"], environment=environment)
        receipt["build_closure_before_record"] = retain_build_closure(
            evidence, "build-closure-before.json", receipt["build_closure_before"])
        if not receipt["build_closure_before"]["admissible"]:
            raise ValueError("unapproved build input closure: " + ", ".join(receipt["build_closure_before"]["refusals"]))
        run(worker_command, environment, evidence, receipt["commands"])
        runner = target / "release/bboxy450-harness"
        receipt["build_closure_after"] = build_closure_after(receipt["build_closure_before"], runner=runner)
        receipt["build_closure_after_record"] = retain_build_closure(
            evidence, "build-closure-after.json", receipt["build_closure_after"])
        if not receipt["build_closure_after"]["admissible"]:
            raise ValueError("changed or unmatched build closure: " + ", ".join(receipt["build_closure_after"]["refusals"]))
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
            if args.diagnose_launch:
                receipt["launch_diagnosis"] = hello(
                    runner, evidence, receipt["native_provision"], diagnostic_target=target,
                    closure_before=receipt["build_closure_before"], closure_after=receipt["build_closure_after"])
                receipt["scope"] = "One instrumented source-free launch diagnosis only; ordinary uninstrumented readiness/conformance remains unproven"
                code = 0
                return code
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
