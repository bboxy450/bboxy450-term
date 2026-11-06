#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Mandatory native/reference differential gate; never accepts partial checking."""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
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


def harness_root(root):
    if not (root / "Cargo.toml").is_file():
        return None  # Contract/hash-only infrastructure fixtures.
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    member = "crates/bboxy450-harness"
    if member in manifest.get("workspace", {}).get("members", []):
        child = root / member
        if not (child / "verification/corpus.toml").is_file():
            raise ValueError("harness formal submodule is missing; run make init-submodules")
        return child
    return None


def corpus(root):
    rows = tomllib.loads((root / "verification/corpus.toml").read_text())["model"]
    child = harness_root(root)
    if child:
        rows += [{**row, "frontend": row.get("frontend", "project")}
                 for row in tomllib.loads((child / "verification/corpus.toml").read_text())["model"]]
    return rows


def harness_contracts(root, models):
    child = harness_root(root)
    if not child:
        return []
    # Reuse the pinned harness's multi-control validation and complete source inventory.
    import importlib.util
    spec = importlib.util.spec_from_file_location("embedded_harness_formal", child / "scripts/formal.py")
    module = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(module)
    module.coverage(child, models)
    rows = module.contracts(child, models)
    prefix = str(child.relative_to(root)) + "/"
    for row in rows:
        for field in ("sources", "implementation_tests"):
            row[field] = [prefix + path for path in row[field]]
    for row in rows:
        if row["name"] in {"HarnessEmbedLifecycle", "HarnessCancelTreeGroup", "HarnessProcessLifecycle"}:
            row["implementation_tests"].append("crates/bbt-mux/tests/mcp.rs")
            row["sources"].extend(["crates/bbt-mux/src/mcp/seats.rs", "crates/bbt-mux/src/adapters/mod.rs"])
            if row["name"] == "HarnessProcessLifecycle":
                row["sources"].append("crates/bbt-supervisor/src/supervisor.rs")
    return rows


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
    """Require executed local project families with concrete source/test mappings."""
    path = root / "verification/contracts.toml"
    if not path.is_file() and not (root / "crates/bboxy450-term").exists():
        return []  # Independently exported checker has its own conformance corpus.
    rows = tomllib.loads(path.read_text())["contract"]
    local = tomllib.loads((root / "verification/corpus.toml").read_text())["model"]
    local_names = [row["name"] for row in local]
    if len(local_names) != len(set(local_names)):
        raise ValueError("duplicate local corpus model")
    inventory = {row["name"]: row for row in local}
    seen = set()
    for row in rows:
        name = row["name"]
        # Adapted from bboxy450-harness scripts/formal.py:102-134 at pinned
        # d06e696f99472ed754e84353baf11a2974589d6d: explicit control/verdict
        # families, specialized here to LOCAL project frontend ownership and
        # Broken* names rather than producer-only BrokenHarness* names.
        # https://github.com/bboxy450/bboxy450-harness/blob/d06e696f99472ed754e84353baf11a2974589d6d/scripts/formal.py
        # Policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
        if "controls" in row:
            controls = row["controls"]
            if (not isinstance(controls, list) or not controls or any(
                    not isinstance(c, dict) or set(c) != {"name", "expected"}
                    or not isinstance(c["name"], str) or not isinstance(c["expected"], str)
                    for c in controls)):
                raise ValueError(f"malformed declared contract controls: {name}")
        else:
            negative = "Broken" + name
            controls = [{"name": negative, "expected": inventory.get(negative, {}).get("expected")}]
        control_names = [control["name"] for control in controls]
        family = {name, *control_names}
        members = row.get("models")
        if (name in seen or not isinstance(members, list) or not members
                or any(not isinstance(member, str) for member in members)
                or len(control_names) != len(set(control_names))
                or len(members) != len(set(members)) or set(members) != family
                or not family.issubset(models)
                or any(inventory.get(member, {}).get("frontend") != "project" for member in family)
                or inventory.get(name, {}).get("expected") != "pass"
                or any(not control["name"].startswith("Broken") or control["name"] == "Broken"
                       or control["expected"] not in {"safety", "liveness", "deadlock"}
                       or inventory.get(control["name"], {}).get("expected") != control["expected"]
                       for control in controls)):
            raise ValueError(f"missing, duplicate, foreign or unexecuted contract pair/family: {name}")
        seen.add(name)
        for field in ["assumptions", "property", "title"]:
            if not isinstance(row.get(field), str) or not row[field].strip():
                raise ValueError(f"missing contract {field}: {name}")
        for field in ["sources", "implementation_tests"]:
            if (not isinstance(row.get(field), list) or not row[field]
                    or any(not isinstance(p, str) or not (root / p).is_file() for p in row[field])):
                raise ValueError(f"missing contract {field}: {name}")
        if row.get("checks") != ["make test-formal", "make test"]:
            raise ValueError(f"missing mandatory checks: {name}")
    if not rows:
        raise ValueError("project contract inventory is empty")
    return rows + harness_contracts(root, models)


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
               "tlc2.TLC", "-workers", "1", "-lncheck", "final", "-cleanup", "-config", f"{name}{suffix}.cfg",
               "-metadir", str(root / f"{name}{suffix}-metadata")]
    if graph:
        command += ["-dump", "dot,actionlabels", f"{name}.dot"]
    command += [name]
    try:
        # TLC extracts standard modules using java.io.tmpdir and deletes them
        # on exit. Concurrent checking slots must never own the same files.
        with tempfile.TemporaryDirectory(prefix="tlc-", dir=root) as own:
            command.insert(1, f"-Djava.io.tmpdir={own}")
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
    child = harness_root(root)
    if child:
        paths.update({"Cargo.toml", "Cargo.lock", ".gitmodules", "scripts/harness-submodule.py"})
        # Hash the complete imported implementation, witnesses, tools and inventories.
        paths.update(str(p.relative_to(root)) for directory in ("src", "tests", "scripts", "verification")
                     for p in (child / directory).rglob("*") if p.is_file() and "__pycache__" not in p.parts)
        paths.update(str((child / p).relative_to(root)) for p in ("Cargo.toml", "Cargo.lock", "Makefile", "clippy.toml"))
    paths.update(p for row in records for p in row["sources"])
    paths.update(p for row in contract_records for field in ["sources", "implementation_tests"] for p in row[field])
    return {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in sorted(paths) if (root / p).is_file()}


def source_dirty(root):
    """Use ci-local's source boundary; unrelated untracked run records are not code."""
    tracked = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=root)
    untracked = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard", "--",
        "crates", "scripts", "verification", "lab", "design", ".cargo", "Cargo.toml", "Cargo.lock", "build.rs"], cwd=root)
    return bool(tracked.strip() or untracked.strip())


def evidence_receipt(target, root=ROOT, *, require_clean=True, expected_source=None):
    """Validate current-source evidence; R5 may name its checked prior source.

    Even that explicit prior-source read must match every current input, tool
    and model hash. Normal verification always requires the current HEAD.
    """
    target, root = Path(target).resolve(), Path(root).resolve()
    path = target / "evidence.json"
    raw = path.read_bytes()
    data = json.loads(raw)
    sha = expected_source or subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    if data["source"] != sha:
        raise ValueError("formal evidence source differs from current HEAD")
    if require_clean and (data.get("dirty") is not False or source_dirty(root)):
        raise ValueError("formal evidence requires clean source")
    required = corpus(root)
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
    if "tool_pins" in data and data["tool_pins"] != pins:
        raise ValueError("formal evidence full tool pins differ")
    checker = checker_sha256(root) if "checker_sha256" in data or any("reuse_key" in m for m in models) else None
    if "checker_sha256" in data and data["checker_sha256"] != checker:
        raise ValueError("formal evidence checker sources differ")
    contract_names = {name for row in contract_records for name in row["models"]}
    for m in models:
        if "checker_sha256" in m and m["checker_sha256"] != checker:
            raise ValueError("formal model checker sources differ")
        if (m["graph_equal"] is not True or m["states"] < 1 or m["edges"] < 0
                or m["tlc_exit"] != {"pass": 0, "safety": 12, "deadlock": 11, "liveness": 13}[m["expected"]]):
            raise ValueError("formal evidence contains incomplete or mismatched verification")
        if hashlib.sha256((target / "models" / (m["name"] + ".tla")).read_bytes()).hexdigest() != m["tla_sha256"]:
            raise ValueError("formal evidence TLA model hash differs")
        for suffix, digest in m.get("graph_sha256", {}).items():
            if suffix not in GRAPH_SUFFIXES or model_digest(target / "models" / (m["name"] + suffix)) != digest:
                raise ValueError("formal evidence decoded graph hash differs")
        if m["name"] in contract_names and m["contract_metadata"] != contract_metadata(target / "models", m["name"]):
            raise ValueError("formal evidence contract metadata differs")
        if "reuse_key" in m and m["reuse_key"] != model_reuse_key(
                target / "models", m["name"], m["expected"], m["contract_metadata"], pins, checker=checker):
            raise ValueError("formal evidence reuse inputs differ")
    return {"source": sha, "checkout": str(root), "evidence": str(path),
            "evidence_sha256": hashlib.sha256(raw).hexdigest(), "dirty": data["dirty"],
            "models": len(models), "crates": len(records), "contracts": len(contract_records)}


def records_rebase_receipt(receipt, root, delta):
    """Rebind unchanged verification inputs after the caller's R5 records gates."""
    root = Path(root).resolve()
    archive = Path(receipt["evidence"]).parent
    checked = evidence_receipt(archive, root, expected_source=receipt["source"])
    if {k: v for k, v in checked.items() if k != "checkout"} != {
            k: v for k, v in receipt.items() if k != "checkout"}:
        raise ValueError("formal receipt differs from evidence")
    data = json.loads((archive / "evidence.json").read_text())
    data["source"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    data["records_rebase"] = {"gate_log_source": receipt["source"], "records_delta": delta,
                              "records_checks_exit": 0, "evidence_sha256": receipt["evidence_sha256"]}
    with tempfile.TemporaryDirectory(prefix="records-formal-") as temporary:
        target = Path(temporary)
        shutil.copytree(archive / "models", target / "models")
        (target / "evidence.json").write_text(json.dumps(data, indent=2) + "\n")
        destination = archive_evidence(target, root)
    return evidence_receipt(destination, root)


GRAPH_SUFFIXES = (".dot", ".states", ".edges")


@contextmanager
def model_stream(path):
    """Read legacy files or zstd objects; all identities bind decoded bytes."""
    path = Path(path)
    if path.exists():
        with path.open("rb") as stream:
            yield stream
    else:
        with tempfile.TemporaryFile() as errors:
            child = subprocess.Popen(["zstd", "-q", "-dc", str(path) + ".zst"],
                                     stdout=subprocess.PIPE, stderr=errors)
            try:
                yield child.stdout
                if child.wait() != 0:
                    raise ValueError(f"corrupt compressed evidence: {path}")
            finally:
                child.stdout.close()
                if child.poll() is None:
                    child.terminate()
                child.wait()


def stream_digest(stream):
    digest = hashlib.sha256()
    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(chunk)
    return digest.hexdigest()


def model_digest(path):
    with model_stream(path) as stream:
        return stream_digest(stream)


def copy_model(source, destination):
    with model_stream(source) as stream, Path(destination).open("wb") as output:
        shutil.copyfileobj(stream, output)


def store_graph(source, destination, store):
    """Immutable hardlinks dedupe graphs without coupling receipt lifetimes.

    Original bboxy450-term storage protocol; keep credit with extracted code:
    https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    """
    digest = model_digest(source)
    objects = store / "by-model" / "graphs"
    objects.mkdir(parents=True, exist_ok=True)
    blob = objects / (digest + ".zst")
    try:
        os.link(blob, str(destination) + ".zst")
    except FileNotFoundError:
        pass
    else:
        # Validate through the receipt's retained inode, never a racing index.
        if model_digest(destination) != digest:
            raise ValueError(f"corrupt graph object: {blob}")
        return
    with tempfile.NamedTemporaryFile(dir=objects, prefix="staging-", suffix=".zst") as temporary:
        staged = Path(temporary.name)
        if source.exists():
            subprocess.run(["zstd", "-q", "-T1", "-3", "-c", str(source)],
                           stdout=temporary, check=True)
        else:
            with Path(str(source) + ".zst").open("rb") as stream:
                shutil.copyfileobj(stream, temporary)
        temporary.flush()
        # Round-trip before publishing or replacing any original.
        if model_digest(Path(str(staged).removesuffix(".zst"))) != digest:
            raise ValueError("compressed graph round-trip differs")
        while True:
            try:
                os.link(staged, blob)
            except FileExistsError:
                pass
            try:
                os.link(blob, str(destination) + ".zst")
                if model_digest(destination) != digest:
                    raise ValueError(f"corrupt graph object: {blob}")
                break
            except FileNotFoundError:
                continue  # An orphan sweep unlinked the index, never our data.


def archive_evidence(target, root=ROOT):
    """Keep reviewable, compressed evidence after checking-slot cleanup."""
    target, root = Path(target).resolve(), Path(root).resolve()
    store = evidence_store(root)
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
                    plain = path.with_suffix("") if path.suffix == ".zst" else path
                    if plain.suffix in GRAPH_SUFFIXES:
                        store_graph(plain, staged / "models" / plain.name, store)
                    else:
                        shutil.copyfile(path, staged / "models" / path.name)
            try:
                staged.rename(destination)
            except OSError:
                if not destination.is_dir():
                    raise
    data = json.loads((destination / "evidence.json").read_text())
    if data.get("dirty") is False and any("reuse_key" in row for row in data["models"]):
        evidence_receipt(destination, root)
        try:
            publish_model_index(store, destination, data)
        except OSError:
            print("formal: reuse index unavailable; checked evidence remains archived", flush=True)
    return destination


def evidence_store(root=ROOT):
    common = Path(subprocess.check_output(["git", "rev-parse", "--git-common-dir"], cwd=root, text=True).strip())
    return (root / common).resolve() / "bboxy-formal-evidence"


def receipts(store):
    return sorted((path for path in store.iterdir() if path.is_dir()
                   and re.fullmatch(r"[a-f0-9]{64}", path.name)),
                  key=lambda path: (path.stat().st_mtime_ns, path.name), reverse=True) if store.exists() else []


def record_scalar(text, field):
    front = text.split("---", 2)[1] if text.startswith("---") else ""
    match = re.search(r"^" + re.escape(field) + r":\s*(.+)$", front, re.M)
    return match.group(1).strip(" \"'") if match else ""


def live_evidence(root, store):
    """Pin remote-main, every extant worktree HEAD and pending report citations.

    Reports whose task/status cannot be resolved are conservatively pending.
    A report can cite an evidence digest, source SHA or receipt directory.
    """
    output = subprocess.check_output(["git", "worktree", "list", "--porcelain", "-z"], cwd=root)
    roots, heads = {Path(root)}, set()
    for field in output.decode().split("\0"):
        if field.startswith("worktree ") and Path(field[9:]).is_dir():
            roots.add(Path(field[9:]))
        if field.startswith("HEAD "):
            heads.add(field[5:])
    remote = subprocess.run(["git", "rev-parse", "--verify", "origin/main"], cwd=root,
                            text=True, capture_output=True)
    if remote.returncode == 0:
        heads.add(remote.stdout.strip())
    pending = set()
    for checkout in roots:
        kb = checkout / ".bboxy"
        tasks = {}
        for task in (kb / "tasks").glob("*.md"):
            text = task.read_text()
            for key in (task.stem, "tasks/" + task.stem, record_scalar(text, "id"),
                        "#" + record_scalar(text, "number")):
                tasks[key] = record_scalar(text, "status")
        for report in (kb / "reports").glob("*.md"):
            text = report.read_text()
            task = record_scalar(text, "task").removeprefix("[[").removesuffix("]]").removesuffix(".md")
            if tasks.get(task) not in {"accepted", "done"}:
                pending.update(re.findall(r"(?<![a-f0-9])[a-f0-9]{40,64}(?![a-f0-9])", text))
    live = set()
    for path in receipts(store):
        data = json.loads((path / "evidence.json").read_text())
        if data["source"] in heads | pending or path.name in pending:
            live.add(path.name)
    return live


def receipt_files(path):
    result = {}
    for file in sorted(path.rglob("*")):
        if file.is_symlink():
            raise ValueError(f"refusing symlink in receipt: {file}")
        if file.is_file():
            with file.open("rb") as stream:
                result[str(file.relative_to(path.parent))] = stream_digest(stream)
    return result


def archive_reader(path):
    return subprocess.Popen(["zstd", "-q", "-dc", str(path)], stdout=subprocess.PIPE)


def verify_archive(path, manifest):
    """Require frame integrity, exact member count, every byte hash and archive hash."""
    subprocess.run(["zstd", "-q", "-t", str(path)], check=True)
    with path.open("rb") as stream:
        if stream_digest(stream) != manifest["archive_sha256"]:
            raise ValueError("archive SHA-256 differs")
    seen = {}
    child = archive_reader(path)
    try:
        with tarfile.open(fileobj=child.stdout, mode="r|") as tar:
            for member in tar:
                if (not (member.isfile() or member.islnk()) or member.name in seen
                        or any(part in {".", ".."} for part in member.name.split("/"))
                        or not re.fullmatch(r"[a-f0-9]{64}/(?:[A-Za-z0-9_.-]+/)*[A-Za-z0-9_.-]+", member.name)):
                    raise ValueError("unsafe or duplicate archive member")
                if member.islnk():
                    if member.linkname not in seen:
                        raise ValueError("archive hardlink lacks an earlier verified member")
                    seen[member.name] = seen[member.linkname]
                else:
                    seen[member.name] = stream_digest(tar.extractfile(member))
        if child.wait() != 0:
            raise ValueError("archive decompression failed")
    finally:
        child.stdout.close()
        if child.poll() is None:
            child.terminate()
        child.wait()
    if len(seen) != manifest["file_count"] or seen != manifest["files"]:
        raise ValueError("archive file count or SHA-256 manifest differs")


def archive_month(store, month, paths):
    directory = store.with_name(store.name + "-archive")
    directory.mkdir(exist_ok=True)
    archive = directory / ("receipts-" + month + ".tar.zst")
    manifest_path = archive.with_suffix(".sha256.json")
    previous = {"files": {}, "file_count": 0}
    if archive.exists() or manifest_path.exists():
        previous = json.loads(manifest_path.read_text())
        verify_archive(archive, previous)  # Never repair over corrupt custody.
    added = {name: digest for path in paths for name, digest in receipt_files(path).items()}
    if any(name in previous["files"] and previous["files"][name] != digest for name, digest in added.items()):
        raise ValueError("archived receipt changed")
    files = previous["files"] | added
    graph_members = {digest: name for name, digest in previous["files"].items()
                     if any(name.endswith(suffix + ".zst") for suffix in GRAPH_SUFFIXES)}
    with tempfile.TemporaryDirectory(prefix="staging-", dir=directory) as own:
        staged = Path(own) / archive.name
        with staged.open("wb") as output:
            compressor = subprocess.Popen(["zstd", "-q", "-T1", "-3", "-c"], stdin=subprocess.PIPE, stdout=output)
            try:
                with tarfile.open(fileobj=compressor.stdin, mode="w|", dereference=True) as tar:
                    if previous["files"]:
                        reader = archive_reader(archive)
                        try:
                            with tarfile.open(fileobj=reader.stdout, mode="r|") as old:
                                for member in old:
                                    tar.addfile(member, None if member.islnk() else old.extractfile(member))
                            if reader.wait() != 0:
                                raise ValueError("old archive read failed")
                        finally:
                            reader.stdout.close()
                            if reader.poll() is None:
                                reader.terminate()
                            reader.wait()
                    for name in sorted(added.keys() - previous["files"].keys()):
                        info = tar.gettarinfo(store / name, arcname=name)
                        graph = any(name.endswith(suffix + ".zst") for suffix in GRAPH_SUFFIXES)
                        digest = added[name]
                        if graph and digest in graph_members:
                            info.type, info.linkname, info.size = tarfile.LNKTYPE, graph_members[digest], 0
                            tar.addfile(info)
                        else:
                            with (store / name).open("rb") as stream:
                                tar.addfile(info, stream)
                            if graph:
                                graph_members[digest] = name
                compressor.stdin.close()
                if compressor.wait() != 0:
                    raise ValueError("archive compression failed")
            finally:
                if not compressor.stdin.closed:
                    compressor.stdin.close()
                if compressor.poll() is None:
                    compressor.terminate()
                compressor.wait()
            output.flush()
            os.fsync(output.fileno())
        with staged.open("rb") as stream:
            manifest = {"archive_sha256": stream_digest(stream), "file_count": len(files), "files": files}
        verify_archive(staged, manifest)
        staged_manifest = Path(own) / manifest_path.name
        with staged_manifest.open("w") as stream:
            stream.write(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
            stream.flush()
            os.fsync(stream.fileno())
        staged.replace(archive)
        staged_manifest.replace(manifest_path)
        # Persist both names and their parent before removing source custody.
        for path in (directory, directory.parent):
            descriptor = os.open(path, os.O_RDONLY)
            try:
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
    # Re-read published custody before permitting a single removal.
    verify_archive(archive, json.loads(manifest_path.read_text()))
    return added


def prune_evidence(root=ROOT, *, keep=10, dry_run=False):
    if keep < 0:
        raise ValueError("KEEP must be nonnegative")
    store = evidence_store(root)
    if not store.exists():
        return
    # Only maintenance competes for this nonblocking lock. Writers/readers never wait.
    if dry_run:
        lock = None
    else:
        lock = (store / ".prune.lock").open("a")
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            lock.close()
            return
    try:
        candidates = receipts(store)
        live = live_evidence(root, store) | {p.name for p in candidates[:keep]}
        months = {}
        for path in candidates:
            if path.name not in live:
                month = datetime.fromtimestamp((path / "evidence.json").stat().st_mtime, timezone.utc).strftime("%Y-%m")
                months.setdefault(month, []).append(path)
        print(f"formal: prune keep={len(live)} cold={sum(map(len, months.values()))} dry_run={dry_run}")
        if dry_run:
            return
        for month, paths in sorted(months.items()):
            verified = archive_month(store, month, paths)
            # Refresh source/report pins after lengthy IO. Newly live receipts stay.
            live = live_evidence(root, store) | {p.name for p in receipts(store)[:keep]}
            for path in paths:
                if path.name not in live and receipt_files(path) == {
                        name: digest for name, digest in verified.items() if name.startswith(path.name + "/")}:
                    shutil.rmtree(path)
        # Hardlinks make an orphan-index removal safe even against a new writer.
        for blob in (store / "by-model" / "graphs").glob("*.zst"):
            if re.fullmatch(r"[a-f0-9]{64}\.zst", blob.name) and blob.stat().st_nlink == 1:
                blob.unlink(missing_ok=True)
    finally:
        if lock is not None:
            lock.close()


def schedule_prune(root=ROOT):
    """Detach at background QoS only above the physical-byte budget; never gate land."""
    store = evidence_store(root)
    budget = int(os.environ.get("FORMAL_EVIDENCE_BUDGET_BYTES", str(5 * 1024**3)))
    if budget < 0:
        raise ValueError("formal evidence budget must be nonnegative")
    if not store.exists():
        return False
    # du counts a shared graph inode once, unlike logical receipt sizes.
    used = int(subprocess.check_output(["du", "-sk", str(store)], text=True).split()[0]) * 1024
    if used <= budget:
        return False
    command = [sys.executable, "-I", str(Path(root) / "scripts/formal.py"), "--prune-evidence"]
    qos = ["taskpolicy", "-b"] if platform.system() == "Darwin" else []
    with (store / "prune.log").open("ab") as log:
        subprocess.Popen(qos + ["nice", "-n", "19"] + command, cwd=root, stdin=subprocess.DEVNULL,
                         stdout=log, stderr=log, start_new_session=True)
    return True


def checker_sha256(root=None):
    """Hash tracked checker sources using their current working-tree bytes."""
    root = Path(root or ROOT)
    paths = ("scripts/formal.py", "scripts/test-formal.py", "crates/bboxy450-tlc", "crates/bbt-yarn")
    files = subprocess.check_output(["git", "ls-files", "-z", "--", *paths], cwd=root, text=True).split("\0")
    files = sorted(set(files) - {""})
    if (not all(p in files for p in paths[:2])
            or not all(any(p.startswith(directory + "/") for p in files) for directory in paths[2:])):
        raise ValueError("checker source inventory is incomplete")
    hashes = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in files}
    return hashlib.sha256(json.dumps(hashes, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def model_reuse_key(root, name, expected, metadata, pins, *, checker=None):
    # Include configurations and freshly exported native graphs as well as the
    # required TLA, expectation, bounds metadata and every pinned host/tool.
    suffixes = (".tla", ".cfg", "Graph.cfg", ".states", ".edges")
    value = {"name": name, "expected": expected, "contract_metadata": metadata,
             "tool_pins": pins, "checker_sha256": checker or checker_sha256(),
             "files": {suffix: model_digest(root / (name + suffix)) for suffix in suffixes}}
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def publish_model_index(store, archive, data):
    index = store / "by-model"
    index.mkdir(exist_ok=True)
    for row in data["models"]:
        key = row.get("reuse_key")
        if key is None:
            continue  # Legacy receipts cannot attest the additional key inputs.
        with tempfile.NamedTemporaryFile(mode="w", dir=index, delete=False) as stream:
            temporary = Path(stream.name)
            json.dump({"evidence": archive.name, "model": row["name"]}, stream)
        try:
            temporary.replace(index / (key + ".json"))
        finally:
            temporary.unlink(missing_ok=True)


def reused_model(store, root, name, expected, metadata, pins, key, checker):
    try:
        pointer = json.loads((store / "by-model" / (key + ".json")).read_text())
        digest = pointer["evidence"]
        if pointer["model"] != name or not re.fullmatch(r"[a-f0-9]{64}", digest):
            return None
        archive = store / digest
        raw = (archive / "evidence.json").read_bytes()
        data = json.loads(raw)
        if (hashlib.sha256(raw).hexdigest() != digest or data.get("dirty") is not False
                or data.get("checker_sha256") != checker
                or data.get("tool_pins") != pins or data.get("tlc") != pins["tlc"]
                or data.get("java") != pins["java"][f"{platform.system()}-{platform.machine()}"]):
            return None
        rows = [row for row in data["models"] if row["name"] == name]
        if len(rows) != 1:
            return None
        row = rows[0]
        if (row.get("reuse_key") != key or row.get("checker_sha256") != checker or row["expected"] != expected
                or row["contract_metadata"] != metadata or row["graph_equal"] is not True
                or row["states"] < 1 or row["edges"] < 0
                or row["tlc_exit"] != {"pass": 0, "safety": 12, "deadlock": 11, "liveness": 13}[expected]
                or row["tla_sha256"] != hashlib.sha256((archive / "models" / (name + ".tla")).read_bytes()).hexdigest()
                or model_reuse_key(archive / "models", name, expected, metadata, pins, checker=checker) != key):
            return None
        for suffix, graph_digest in row.get("graph_sha256", {}).items():
            if suffix not in GRAPH_SUFFIXES or model_digest(archive / "models" / (name + suffix)) != graph_digest:
                return None
        for suffix, exit_code, wanted in ((".log", row["tlc_exit"], expected), ("Graph.log", 0, "pass")):
            output = (archive / "models" / (name + suffix)).read_text()
            if verdict(subprocess.CompletedProcess([], exit_code, output)) != wanted:
                return None
        for suffix in (".log", "Graph.log", ".dot"):
            source = archive / "models" / (name + suffix)
            copy_model(source, root / source.name)
        return {**row, "reused_from": digest}
    except (OSError, ValueError, KeyError, TypeError):
        return None  # Missing, corrupt or stale cache always re-checks.


def check_model(root, row, contract_names, java, jar, pins, store, *, reuse=False, checker=None):
    name, expected, count, width = row
    if not re.fullmatch(r'[A-Za-z][A-Za-z0-9_]*', name) or expected not in {"pass", "safety", "deadlock", "liveness"}:
        raise ValueError("invalid model manifest")
    metadata = contract_metadata(root, name) if name in contract_names else None
    checker = checker or checker_sha256()
    key = model_reuse_key(root, name, expected, metadata, pins, checker=checker)
    cached = reused_model(store, root, name, expected, metadata, pins, key, checker) if reuse else None
    if cached is not None:
        states, edges = native_graph(root, name)
        if (cached["states"] == len(states) == int(count) and cached["edges"] == len(edges)
                and all(len(state) == int(width) for state in states)):
            print(f"formal: {name}: reused_from {cached['reused_from']}", flush=True)
            return cached
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
    print(f"formal: {name}: {actual}, {len(actual_states)} states, {len(actual_edges)} edges match", flush=True)
    return {"name": name, "expected": expected, "states": len(actual_states),
            "edges": len(actual_edges), "tlc_exit": checked.returncode, "graph_equal": True,
            "contract_metadata": metadata, "reuse_key": key, "checker_sha256": checker,
            "graph_sha256": {suffix: model_digest(root / (name + suffix)) for suffix in GRAPH_SUFFIXES},
            "tla_sha256": hashlib.sha256((root / f"{name}.tla").read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path)
    parser.add_argument("--prune-evidence", action="store_true")
    parser.add_argument("--prune-background", action="store_true")
    parser.add_argument("--verify-evidence", action="store_true")
    parser.add_argument("--reuse", action="store_true", help="presubmit-only reuse of identical archived models")
    args = parser.parse_args()
    if args.prune_evidence:
        prune_evidence(keep=int(os.environ.get("KEEP", "10")), dry_run=os.environ.get("DRY_RUN") == "1")
        return
    if args.prune_background:
        schedule_prune()
        return
    if args.target is None:
        parser.error("--target is required for verification")
    target = args.target.resolve()
    if args.verify_evidence:
        print("formal: receipt " + json.dumps(evidence_receipt(target), sort_keys=True))
        return
    root = target / "models"
    rows = [line.split('\t') for line in (root / "manifest.tsv").read_text().splitlines()]
    if not rows or len({r[0] for r in rows}) != len(rows):
        raise ValueError("empty or duplicate formal corpus")
    names = {r[0] for r in rows}
    required = corpus(ROOT)
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
    checker = checker_sha256()
    evidence = {"source": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "tlc": pins["tlc"], "java": pins["java"][f"{platform.system()}-{platform.machine()}"],
                "tool_pins": pins, "checker_sha256": checker,
                "crates": records, "contracts": contract_records, "models": [], "complete_tlc_replacement": False,
                "dirty": source_dirty(ROOT),
                "input_sha256": input_hashes(ROOT, records, contract_records)}
    for row in rows:
        evidence["models"].append(check_model(root, row, contract_names, java, jar, pins,
                                               evidence_store(ROOT), reuse=args.reuse, checker=checker))
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
