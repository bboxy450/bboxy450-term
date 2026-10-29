#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Mandatory native/reference differential gate; never accepts partial checking."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def artifact(pin, path):
    """Download only a pinned public artifact, with bounded IO and atomic install."""
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="download-", dir=path.parent) as own:
            staged = Path(own) / "artifact"
            subprocess.run(["curl", "-fsSL", "--retry", "2", "--max-time", "180",
                            "-o", str(staged), pin["url"]], check=True, timeout=560)
            verify_digest(staged, pin["sha256"])
            staged.replace(path)
    verify_digest(path, pin["sha256"])
    return path


def verify_digest(path, expected):
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise ValueError(f"artifact checksum mismatch: {path}: {actual} != {expected}")


def tools(target):
    pins = json.loads((ROOT / "verification/tools.lock.json").read_text())
    cache = target / "tools"
    jar = artifact(pins["tlc"], cache / "tla2tools.jar")
    host = f"{platform.system()}-{platform.machine()}"
    if host not in pins["java"]:
        raise ValueError(f"no pinned unattended Java runtime for {host}; add a reviewed tools.lock.json entry")
    pin = pins["java"][host]
    archive = artifact(pin, cache / f"java-{host}.tar.gz")
    runtime = cache / f"java-{host}-{pin['version']}"
    if not runtime.exists():
        with tempfile.TemporaryDirectory(prefix="extract-", dir=cache) as own:
            with tarfile.open(archive) as tar:
                tar.extractall(own, filter="data")
            entries = list(Path(own).iterdir())
            if len(entries) != 1:
                raise ValueError("unexpected Java archive layout")
            entries[0].replace(runtime)
    java = runtime / ("Contents/Home/bin/java" if platform.system() == "Darwin" else "bin/java")
    if not java.is_file():
        raise ValueError(f"Java runtime missing executable: {java}")
    return java, jar, pins


def coverage(root, models):
    config = tomllib.loads((root / "verification/coverage.toml").read_text())
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    workspace = manifest.get("workspace", {})
    packages = {manifest["package"]["name"]: root} if "package" in manifest else {}
    for pattern in workspace.get("members", []):
        for directory in root.glob(pattern):
            package = tomllib.loads((directory / "Cargo.toml").read_text())["package"]["name"]
            packages[package] = directory
    records = config["crate"]
    names = [row["name"] for row in records]
    if len(names) != len(set(names)) or set(names) != set(packages):
        raise ValueError(f"formal coverage inventory differs: missing={set(packages)-set(names)}, stale={set(names)-set(packages)}")
    for row in records:
        if row["status"] not in {"contract-model", "implementation-tests"} or not row["scope"].strip():
            raise ValueError(f"invalid formal scope: {row['name']}")
        if row["status"] == "contract-model" and not row.get("models"):
            raise ValueError(f"missing models: {row['name']}")
        if not row.get("sources") or any(not (root / p).is_file() for p in row["sources"]):
            raise ValueError(f"missing model/source mapping: {row['name']}")
        if not set(row.get("models", [])).issubset(models):
            raise ValueError(f"unexecuted model referenced by {row['name']}")
    return records


def contracts(root, models):
    """Require paired, bounded project contracts with concrete source/test mappings."""
    path = root / "verification/contracts.toml"
    if not path.is_file() and not (root / "crates/bboxy450-term").exists():
        return []  # Independently exported checker has its own conformance corpus.
    rows = tomllib.loads(path.read_text())["contract"]
    seen = set()
    for row in rows:
        name = row["name"]
        pair = {name, "Broken" + name}
        if name in seen or set(row["models"]) != pair or not pair.issubset(models):
            raise ValueError(f"missing, duplicate or unexecuted contract pair: {name}")
        seen.add(name)
        for field in ["assumptions", "property", "title"]:
            if not row.get(field, "").strip():
                raise ValueError(f"missing contract {field}: {name}")
        for field in ["sources", "implementation_tests"]:
            if not row.get(field) or any(not (root / p).is_file() for p in row[field]):
                raise ValueError(f"missing contract {field}: {name}")
        if row.get("checks") != ["make test-formal", "make test"]:
            raise ValueError(f"missing mandatory checks: {name}")
    if not rows:
        raise ValueError("project contract inventory is empty")
    return rows


def contract_metadata(root, name):
    path = root / f"{name}.metadata.json"
    data = json.loads(path.read_text())
    if not data.get("variables") or not data.get("actions") or not (
            data.get("invariants") or data.get("responses")):
        raise ValueError(f"incomplete contract metadata: {name}")
    if data.get("implementation_refinement_proven") is not False:
        raise ValueError(f"unsupported refinement claim: {name}")
    for v in data["variables"]:
        if not v["min"] <= v["initial"] <= v["max"]:
            raise ValueError(f"invalid contract bounds: {name}")
    return data


def dot_graph(path, width):
    """Normalize TLC's labeled DOT graph to exact integer states and edge pairs."""
    nodes = {}
    edges = []
    for line in path.read_text().splitlines():
        edge = re.match(r'\s*(-?\d+)\s*->\s*(-?\d+)\s*\[label="A(\d+)"', line)
        if edge:
            edges.append(tuple(edge.groups()))
            continue
        node = re.match(r'\s*(-?\d+)\s*\[label="((?:\\.|[^"\\])*)"', line)
        if node:
            values = {int(i): int(v) for i, v in re.findall(r'v(\d+) = (-?\d+)', node[2])}
            if set(values) != set(range(width)):
                raise ValueError(f"unrecognized TLC state: {node[2]}")
            nodes[node[1]] = tuple(values[i] for i in range(width))
    if not nodes:
        raise ValueError(f"TLC did not emit states: {path}")
    transitions = {(nodes[a], int(action), nodes[b]) for a, b, action in edges}
    return set(nodes.values()), transitions


def native_graph(root, name):
    states = {tuple(map(int, line.split(','))) for line in (root / f"{name}.states").read_text().splitlines()}
    edges = set()
    for line in (root / f"{name}.edges").read_text().splitlines():
        if not line:
            continue
        a, action, b = line.split('\t')
        edges.add((tuple(json.loads(a)), int(action), tuple(json.loads(b))))
    return states, edges


def tlc(java, jar, root, name, graph):
    suffix = "Graph" if graph else ""
    command = [str(java), "-XX:ActiveProcessorCount=2", "-Xmx256m", "-XX:+UseSerialGC", "-cp", str(jar),
               "tlc2.TLC", "-workers", "1", "-cleanup", "-config", f"{name}{suffix}.cfg",
               "-metadir", str(root / f"{name}{suffix}-metadata")]
    if graph:
        command += ["-dump", "dot,actionlabels", f"{name}.dot"]
    command += [name]
    try:
        result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, timeout=60)
    except subprocess.TimeoutExpired as error:
        (root / f"{name}{suffix}.log").write_bytes(error.stdout or b"")
        raise ValueError(f"TLC timed out: {name}; incomplete is not a pass") from error
    (root / f"{name}{suffix}.log").write_text(result.stdout)
    return result


def verdict(result):
    output = result.stdout
    if result.returncode == 0 and "Model checking completed. No error has been found." in output:
        return "pass"
    # Exit status and named diagnostics must agree; syntax/tool errors are never expected negatives.
    for name, code, diagnostic in [("safety", 12, "is violated"), ("deadlock", 11, "Deadlock reached"),
                                   ("liveness", 13, "Temporal properties were violated")]:
        if result.returncode == code and diagnostic in output:
            return name
    raise ValueError(f"TLC tool failure (exit {result.returncode}):\n{output[-4000:]}")


def input_hashes(root, records, contract_records):
    paths = {"crates/bbt-yarn/src/model.rs", "crates/bbt-yarn/examples/verify.rs",
             "crates/bbt-yarn-macros/src/lib.rs", "crates/bboxy450-tlc/src/lib.rs",
             "crates/bboxy450-tlc/examples/verify.rs", "verification/coverage.toml",
             "verification/corpus.toml", "verification/tools.lock.json", "scripts/formal.py",
             "verification/contracts.toml", "crates/bboxy450-term/tests/formal_contracts.rs",
             "crates/bboxy450-term/tests/formal_contracts/models.rs",
             "crates/bboxy450-term/tests/formal_contracts/witnesses.rs"}
    paths.update(p for row in records for p in row["sources"])
    paths.update(p for row in contract_records for field in ["sources", "implementation_tests"] for p in row[field])
    return {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in sorted(paths) if (root / p).is_file()}


def source_dirty(root):
    """Use ci-local's source boundary; unrelated untracked run records are not code."""
    tracked = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=root)
    untracked = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard", "--",
        "crates", "scripts", "verification", "lab", "design", ".cargo", "Cargo.toml", "Cargo.lock", "build.rs"], cwd=root)
    return bool(tracked.strip() or untracked.strip())


def evidence_receipt(target, root=ROOT, *, require_clean=True):
    """Bind completed native/TLC results to the current source, scopes and model bytes."""
    target, root = Path(target).resolve(), Path(root).resolve()
    path = target / "evidence.json"
    raw = path.read_bytes()
    data = json.loads(raw)
    sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if data["source"] != sha:
        raise ValueError("formal evidence source differs from current HEAD")
    if require_clean and (data.get("dirty") is not False or source_dirty(root)):
        raise ValueError("formal evidence requires clean source")
    required = tomllib.loads((root / "verification/corpus.toml").read_text())["model"]
    expected = {m["name"]: m["expected"] for m in required}
    models = data["models"]
    if len(expected) != len(required) or len(models) != len(expected) or {
            m["name"]: m["expected"] for m in models} != expected:
        raise ValueError("formal evidence has missing, duplicate or stale models")
    records, contract_records = coverage(root, set(expected)), contracts(root, set(expected))
    if data["crates"] != records or data.get("contracts", []) != contract_records:
        raise ValueError("formal evidence coverage differs from current inventory")
    if data["input_sha256"] != input_hashes(root, records, contract_records):
        raise ValueError("formal evidence input hashes differ from current source")
    if data.get("complete_tlc_replacement") is not False:
        raise ValueError("formal evidence cannot claim complete TLC replacement")
    pins = json.loads((root / "verification/tools.lock.json").read_text())
    if data["tlc"] != pins["tlc"] or data["java"] not in pins["java"].values():
        raise ValueError("formal evidence reference tools differ from pins")
    contract_names = {name for row in contract_records for name in row["models"]}
    for m in models:
        if (m["graph_equal"] is not True or m["states"] < 1 or m["edges"] < 0
                or m["tlc_exit"] != {"pass": 0, "safety": 12, "deadlock": 11, "liveness": 13}[m["expected"]]):
            raise ValueError("formal evidence contains incomplete or mismatched verification")
        if hashlib.sha256((target / "models" / (m["name"] + ".tla")).read_bytes()).hexdigest() != m["tla_sha256"]:
            raise ValueError("formal evidence TLA model hash differs")
        if m["name"] in contract_names and m["contract_metadata"] != contract_metadata(target / "models", m["name"]):
            raise ValueError("formal evidence contract metadata differs")
    return {"source": sha, "checkout": str(root), "evidence": str(path),
            "evidence_sha256": hashlib.sha256(raw).hexdigest(), "dirty": data["dirty"],
            "models": len(models), "crates": len(records), "contracts": len(contract_records)}


def archive_evidence(target, root=ROOT):
    """Keep reviewable evidence after an owned checking slot is removed."""
    target, root = Path(target).resolve(), Path(root).resolve()
    common = Path(subprocess.check_output(["git", "rev-parse", "--git-common-dir"], cwd=root, text=True).strip())
    store = (root / common).resolve() / "bboxy-formal-evidence"
    store.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256((target / "evidence.json").read_bytes()).hexdigest()
    destination = store / digest
    if not destination.exists():
        with tempfile.TemporaryDirectory(prefix="staging-", dir=store) as own:
            staged = Path(own) / "evidence"
            (staged / "models").mkdir(parents=True)
            shutil.copyfile(target / "evidence.json", staged / "evidence.json")
            for path in (target / "models").iterdir():
                if path.is_file():
                    shutil.copyfile(path, staged / "models" / path.name)
            try:
                staged.rename(destination)
            except OSError:
                if not destination.is_dir():
                    raise
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--verify-evidence", action="store_true")
    args = parser.parse_args()
    target = args.target.resolve()
    if args.verify_evidence:
        print("formal: receipt " + json.dumps(evidence_receipt(target), sort_keys=True))
        return
    root = target / "models"
    rows = [line.split('\t') for line in (root / "manifest.tsv").read_text().splitlines()]
    if not rows or len({r[0] for r in rows}) != len(rows):
        raise ValueError("empty or duplicate formal corpus")
    names = {r[0] for r in rows}
    required = tomllib.loads((ROOT / "verification/corpus.toml").read_text())["model"]
    expected_corpus = {r["name"]: r["expected"] for r in required}
    if len(expected_corpus) != len(required) or {r[0]: r[1] for r in rows} != expected_corpus:
        raise ValueError("formal corpus differs from required independent expectations")
    records = coverage(ROOT, names)
    contract_records = contracts(ROOT, names)
    contract_names = {name for row in contract_records for name in row["models"]}
    if not {row["name"] for row in contract_records}.issubset(
            {name for row in records for name in row["models"]}):
        raise ValueError("project contract missing from crate coverage")
    java, jar, pins = tools(target)
    evidence = {"source": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "tlc": pins["tlc"], "java": pins["java"][f"{platform.system()}-{platform.machine()}"],
                "crates": records, "contracts": contract_records, "models": [], "complete_tlc_replacement": False,
                "dirty": source_dirty(ROOT),
                "input_sha256": input_hashes(ROOT, records, contract_records)}
    for name, expected, count, width in rows:
        if not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*', name) or expected not in {"pass", "safety", "deadlock", "liveness"}:
            raise ValueError("invalid model manifest")
        graph = tlc(java, jar, root, name, True)
        if verdict(graph) != "pass":
            raise ValueError(f"graph generation failed: {name}")
        actual_states, actual_edges = dot_graph(root / f"{name}.dot", int(width))
        native_states, native_edges = native_graph(root, name)
        if actual_states != native_states or actual_edges != native_edges or len(actual_states) != int(count):
            raise ValueError(f"native/TLC graph mismatch: {name}; states={len(native_states)}/{len(actual_states)}, edges={len(native_edges)}/{len(actual_edges)}")
        checked = tlc(java, jar, root, name, False)
        actual = verdict(checked)
        if actual != expected:
            raise ValueError(f"native/TLC verdict mismatch: {name}: {expected}/{actual}")
        metadata = contract_metadata(root, name) if name in contract_names else None
        evidence["models"].append({"name": name, "expected": expected, "states": len(actual_states),
                                   "edges": len(actual_edges), "tlc_exit": checked.returncode, "graph_equal": True,
                                   "contract_metadata": metadata,
                                   "tla_sha256": hashlib.sha256((root / f"{name}.tla").read_bytes()).hexdigest()})
        print(f"formal: {name}: {actual}, {len(actual_states)} states, {len(actual_edges)} edges match", flush=True)
    (target / "evidence.json").write_text(json.dumps(evidence, indent=2) + '\n')
    archive = archive_evidence(target)
    print("formal: receipt " + json.dumps(evidence_receipt(archive, require_clean=False), sort_keys=True))
    print(f"formal: {len(rows)} models; {len(records)} crates inventoried; native/TLC parity held", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"formal: FAILED: {error}", file=sys.stderr)
        sys.exit(1)
