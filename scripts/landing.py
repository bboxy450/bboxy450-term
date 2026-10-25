"""The landing scripts' shared state and bounded command implementation (#323).

Shell entry points keep the same contract on every worktree. All Git commands
use argument arrays; fixture runs use their own repository and shared-state dir.
"""
import argparse
from contextlib import contextmanager
import fcntl
import hashlib
import importlib.util
import io
from datetime import datetime, timezone
import json
import math
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import subprocess
import sys
import time
import uuid

# Running a gate must not leave bytecode among a candidate's source paths.
sys.dont_write_bytecode = True

_timing_path = None


class Refusal(Exception):
    def __init__(self, message, code=2):
        super().__init__(message)
        self.code = code


def cargo_invocation(argv, environment=None):
    if not argv or argv[0] != "cargo":
        return argv, environment
    environment = dict(os.environ if environment is None else environment)
    target = environment.pop("CARGO_TARGET_DIR", None) or environment.get("BBOXY_CARGO_TARGET_DIR")
    if target:
        environment["BBOXY_CARGO_TARGET_DIR"] = target
    before_harness = argv[:argv.index("--")] if "--" in argv else argv
    if (target and len(argv) > 1 and argv[1] in {"build", "check", "test", "clippy", "run", "bench", "doc", "rustc"}
            and not any(a == "--target-dir" or str(a).startswith("--target-dir=") for a in before_harness)):
        argv = [*argv[:2], "--target-dir", target, *argv[2:]]
    return argv, environment


def command(argv, *, cwd=None, capture=True, check=True, **kwargs):
    argv, environment = cargo_invocation(argv, kwargs.pop("env", None))
    result = subprocess.run(argv, cwd=cwd, text=True, env=environment,
                            capture_output=capture, **kwargs)
    if check and result.returncode:
        raise Refusal(f"command_failed ({result.returncode}): {' '.join(str(a) for a in argv)}\n"
                      f"{result.stderr if capture else ''}".rstrip())
    return result


def git(*args, cwd=None):
    return command(["git", *args], cwd=cwd).stdout.rstrip("\n")


def repository():
    return Path(git("rev-parse", "--show-toplevel")).resolve()


def state_directory():
    override = os.environ.get("BBOXY_LANDING_STATE_DIR")
    if override:
        path = Path(override).expanduser()
        if not path.is_absolute():
            raise Refusal("BBOXY_LANDING_STATE_DIR must be absolute")
        return path.resolve()
    base = Path(os.environ.get("XDG_STATE_HOME") or Path.home() / ".local/state")
    if not base.is_absolute():
        raise Refusal("XDG_STATE_HOME must be absolute")
    return base / "bboxy450-term/landing"


def parity_directory():
    # The one fixture override also keeps render fixtures out of the real cache.
    if os.environ.get("BBOXY_LANDING_STATE_DIR"):
        return state_directory() / "parity"
    base = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    if not base.is_absolute():
        raise Refusal("XDG_CACHE_HOME must be absolute")
    return base / "bboxy450-term/parity"


def changed_paths(range_spec, *, cwd=None):
    value = git("diff", "--name-only", "-z", range_spec, "--", cwd=cwd)
    return sorted(set(value.split("\0")) - {""})


def metadata(root):
    data = json.loads(command(["cargo", "metadata", "--format-version", "1",
                               "--no-deps", "--locked"], cwd=root).stdout)
    members = set(data["workspace_members"])
    packages = {p["name"]: p for p in data["packages"] if p["id"] in members}
    return packages


def affected(range_spec, *, root=None):
    root = Path(root or repository()).resolve()
    paths = changed_paths(range_spec, cwd=root)
    packages = metadata(root)
    directories = {name: Path(p["manifest_path"]).parent.resolve()
                   for name, p in packages.items()}
    by_directory = {p: name for name, p in directories.items()}
    dependencies = {name: set() for name in packages}
    for name, package in packages.items():
        for dep in package["dependencies"]:
            path = dep.get("path")
            if path and Path(path).resolve() in by_directory:
                dependencies[name].add(by_directory[Path(path).resolve()])
            elif dep["name"] in packages:
                dependencies[name].add(dep["name"])
    tier = "lab" if paths and all(p.startswith(("lab/", "design/", ".bboxy/"))
                                 for p in paths) else "code"
    broad = [p for p in paths if p in ("Cargo.toml", "Cargo.lock") or p.startswith(".cargo/")]
    # Validation tools are shared for retry decisions, but are not Rust source
    # dependencies. Their fixture suite and the fast invariants check them.
    tooling = [p for p in paths if p.startswith("scripts/") and not p.startswith("scripts/e2e/")]
    touched = set()
    for path in paths:
        absolute = root / path
        hits = [(len(str(directory)), name) for name, directory in directories.items()
                if absolute.is_relative_to(directory)]
        if hits:
            touched.add(max(hits)[1])
    if any(p.startswith("scripts/e2e/") for p in paths):
        touched.update(n for n in ("bboxy450-term",) if n in packages)
    if broad:
        touched.update(packages)
    if tier == "lab":
        touched.clear()  # lab truth is the last green native set, not a product build
    result = set(touched)
    while True:
        more = {name for name, deps in dependencies.items() if deps & result} - result
        if not more:
            break
        result |= more
    if len(result) * 2 > len(packages):
        tier = "wide"
    ui = {"bbt-ui", "bbt-ui-model", "bbt-render", "bboxybook"}
    parity = any(p.startswith(("lab/", "design/")) for p in paths) or bool(touched & ui)
    shared_lab = {"lab/web/lab.css", "lab/web/tokens.js", "lab/web/glow.js",
                  "lab/web/components.js"}
    shared = sorted(set(broad) | set(tooling) | (set(paths) & shared_lab)
                    | {p for p in paths if p.startswith("design/")})
    return {"tier": tier, "touched": sorted(touched), "affected": sorted(result),
            "parity": "yes" if parity else "no", "paths": paths, "shared": shared}


def now():
    return datetime.now(timezone.utc).isoformat()


def atomic_write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_name(path.name + "." + uuid.uuid4().hex + ".tmp")
    try:
        with temporary.open("x") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def append_record(name, value):
    directory = state_directory()
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    line = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
    fd = os.open(directory / name, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        if os.write(fd, line) != len(line):
            raise Refusal("landing_log_short_write")
        os.fsync(fd)
    finally:
        os.close(fd)


def fixture_options():
    if not os.environ.get("BBOXY_LANDING_STATE_DIR"):
        return {}
    path = state_directory() / "fixture.json"
    return json.loads(path.read_text()) if path.exists() else {}


def cargo_backed(check):
    words = shlex.split(check)
    while words and (words[0] == "env" or re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*=.*", words[0])):
        words.pop(0)
    return bool(words and (Path(words[0]).name == "cargo"
                           or (words[0] in ("sh", "bash", "dash") and len(words) > 1
                               and Path(words[1]).name == "ui-lint.sh")))


def checked(argv, root):
    print("presubmit:", shlex.join(map(str, argv)), flush=True)
    code = measured_command(list(map(str, argv)), root, _timing_path)
    if code:
        raise Refusal(f"check_failed ({code}): {argv[0]}", 1)


def measured_command(argv, root, timing_path, *, stream=None, env=None):
    phase = None
    def switch(value):
        nonlocal phase
        if value != phase and timing_path is not None:
            line = (json.dumps({"at": time.monotonic(), "phase": value}) + "\n").encode()
            fd = os.open(timing_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
            try:
                os.write(fd, line)
            finally:
                os.close(fd)
        phase = value
    cargo = argv[0] == "cargo" or any(Path(arg).name == "quarantine-test.sh" for arg in argv)
    fixtures = any(Path(arg).name == "test-landing.py" for arg in argv)
    switch("compile" if cargo else "test" if fixtures else "other")
    argv, env = cargo_invocation(argv, env)
    child = subprocess.Popen(argv, cwd=root, env=env, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    try:
        for line in child.stdout:
            clean = re.sub(r"\x1b\[[0-9;]*m", "", line)
            if re.match(r"\s*(Compiling|Checking) ", clean):
                switch("compile")
            elif re.match(r"\s*Finished .*(?:profile|target)", clean):
                switch("other")
            elif (re.match(r"\s*(?:Running |Doc-tests |running \d+ tests?\b|test result:)", clean)
                  or clean.startswith("Fast CI fixtures;")):
                switch("test")
            if stream is None:
                print(line, end="", flush=True)
            else:
                stream.write(line)
                stream.flush()
            if clean.startswith("test result:"):
                switch("other")
        return child.wait()
    finally:
        child.stdout.close()
        switch("other")


def phase_totals(path, ended):
    events = []
    if path.exists():
        for line in path.read_text().splitlines():
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                pass  # a killed writer can leave only its final event partial
    totals = {"compile_seconds": 0., "test_seconds": 0.,
              "tests_started": any(e["phase"] == "test" for e in events)}
    for event, following in zip(events, events[1:] + [{"at": ended}]):
        if event["phase"] in ("compile", "test"):
            totals[event["phase"] + "_seconds"] += max(0., min(following["at"], ended) - event["at"])
    return totals


class TwoClocks:
    """Elapsed build and execution time; observations share the host's clock."""
    def __init__(self, started):
        self.at = started
        self.phase = "compile"  # setup cannot run forever outside either cap
        self.compile = self.run = self.test = 0.
        self.tests_started = False

    def event(self, value):
        at = max(self.at, value["at"])
        elapsed = at - self.at
        if self.phase == "compile":
            self.compile += elapsed
        else:
            self.run += elapsed
            if self.phase == "test":
                self.test += elapsed
        self.at, self.phase = at, value["phase"]
        self.tests_started |= self.phase == "test"

    def totals(self, ended):
        tail = max(0., ended - self.at)
        build = self.compile + (tail if self.phase == "compile" else 0.)
        run = self.run + (tail if self.phase != "compile" else 0.)
        test = self.test + (tail if self.phase == "test" else 0.)
        return {"compile_seconds": build, "run_seconds": run, "test_seconds": test,
                "unclassified_seconds": max(0., run - test), "tests_started": self.tests_started}

    def expired(self, ended, build_cap, run_bound):
        totals = self.totals(ended)
        if totals["compile_seconds"] >= build_cap:
            return "build_cap"
        if totals["run_seconds"] >= run_bound:
            return "presubmit_timeout"
        return None


def records_checks(root, paths):
    checked(["cargo", "test", "-p", "bbt-doc", "--locked", "--test", "corpus", "-q"], root)
    checked(["cargo", "test", "-p", "bbt-mermaid", "--locked", "-q"], root)
    if set(paths) & {".bboxy/specs/e2e-acceptance.md", "lab/web/e2e.js"}:
        checked(["cargo", "test", "-p", "bbt-ui", "--locked", "--test", "e2e_checklist", "-q"], root)
    for script in ("layering", "pending-flags", "kill-own-pids", "licenses", "brand",
                   "images", "cross-os", "never-block"):
        checked(["sh", f"scripts/{script}.sh"], root)


def parity_cases(cases, data):
    paths = data["paths"]
    shared_lab = bool(data["shared"]) or any(p.startswith("design/") for p in paths)
    pages = {Path(p).with_suffix(".html").name for p in paths
             if p.startswith("lab/web/") and p.endswith((".html", ".js"))}
    if data["tier"] == "lab" and not shared_lab:
        return [case for case in cases if Path(case["page"]).name in pages]
    components = {re.sub(r"[^a-z0-9]", "", Path(p).stem.lower()) for p in paths
                  if p.startswith(("crates/bbt-ui/src/modules/", "crates/bboxybook/src/"))}
    selected = [case for case in cases
                if re.sub(r"[^a-z0-9]", "", case["component"].lower()) in components]
    if selected and not shared_lab:
        return selected
    # A shared input touches every page: a fixed first case per page, not the
    # full catalogue. Postsubmit always compares the complete catalogue.
    first = {}
    for case in sorted(cases, key=lambda case: json.dumps(case, sort_keys=True)):
        first.setdefault(case["page"], case)
    return [first[page] for page in sorted(first)]


def scoped_parity(root, data, output):
    checked(["sh", "scripts/lab-check.sh"], root)
    pointer = state_directory() / "parity-last-green"
    stored = Path(pointer.read_text().strip()).resolve() if pointer.exists() else None
    if stored is not None and not stored.is_relative_to(parity_directory().resolve()):
        raise Refusal("parity_pointer_outside_cache")
    if data["tier"] == "lab" and stored is not None:
        binary = stored / "bboxybook"
        if not binary.is_file() or not (stored / "cases.json").is_file():
            raise Refusal("parity_store_incomplete")
        cases = json.loads((stored / "cases.json").read_text())
    else:
        checked(["cargo", "build", "-p", "bboxybook", "--locked"], root)
        target = Path(os.environ.get("BBOXY_CARGO_TARGET_DIR") or os.environ.get("CARGO_TARGET_DIR") or root / "target")
        binary = target.resolve() / "debug/bboxybook"
        cases = json.loads(command([str(binary), "manifest"], cwd=root).stdout)
    cases = parity_cases(cases, data)
    if not cases:
        print("presubmit: no catalogued cases for these pages", flush=True)
        return
    output.mkdir(parents=True, exist_ok=True)
    request = output / "requested.json"
    request.write_text(json.dumps(cases) + "\n")
    checked(["node", "crates/bboxybook/capture.mjs", output, request], root)
    captured = output / "cases.json"
    if data["tier"] == "lab" and stored is not None:
        def link_or_copy(source, destination):
            try:
                return os.link(source, destination)
            except OSError:
                return shutil.copy2(source, destination)
        shutil.copytree(stored / "native", output / "native", copy_function=link_or_copy)
    else:
        checked([binary, "render", "--all", "--out", output / "native", "--manifest", captured], root)
    checked([binary, "compare", "--strict-geometry", "--out", output, "--manifest", captured], root)


def presubmit_worker(base, result_path):
    global _timing_path
    _timing_path = result_path.parent / "phases.jsonl"
    root = repository()
    data = affected(base + "..HEAD", root=root)
    atomic_write(result_path, json.dumps(data) + "\n")
    if data["tier"] == "lab":
        records_checks(root, data["paths"])
        checked(["sh", "scripts/invariants.sh", "--no-cargo"], root)
    else:
        packages = [arg for name in data["affected"] for arg in ("-p", name)]
        if packages:
            checked(["sh", "scripts/quarantine-test.sh", "--locked", *packages], root)
            checked(["cargo", "clippy", "--locked", *packages, "--all-targets", "--", "-D", "warnings"], root)
        if any(p.startswith("scripts/") for p in data["paths"]):
            checked(["python3", "-I", "scripts/test-landing.py", "--all"], root)
        checked(["sh", "scripts/invariants.sh", *([] if packages else ["--no-cargo"])], root)
        dispatch = any(Path(p).name in ("ctl.rs", "seat_socket.rs", "logic.rs")
                       or "/src/ctl/" in p or "/src/mcp/" in p for p in data["paths"])
        if dispatch and packages:
            # The wire golden is a target in the protocol crate; its test names
            # deliberately contain no "golden" name filter.
            output = io.StringIO()
            code = measured_command(["cargo", "test", "--locked", "-p", "bbt-proto",
                                     "--test", "gui_wire_golden"],
                                    root, _timing_path, stream=output)
            print(output.getvalue(), end="", flush=True)
            if code or not re.search(r"test result: ok\. [1-9][0-9]* passed", output.getvalue()):
                raise Refusal("golden_dispatch_tests_missing")
    if data["parity"] == "yes":
        scoped_parity(root, data, result_path.parent / "parity")
    return 0


def clean_source(root):
    if git("status", "--porcelain", "--untracked-files=no", cwd=root):
        raise Refusal("dirty_tracked_tree")
    untracked = git("ls-files", "--others", "--exclude-standard", "-z", cwd=root).split("\0")
    source = [p for p in untracked if p.startswith(("crates/", "scripts/", "lab/", "design/", ".cargo/"))
              or p in ("Cargo.toml", "Cargo.lock", "build.rs")]
    if source:
        raise Refusal("untracked_source: " + ", ".join(source))


def presubmit(base):
    root = repository()
    sha = git("rev-parse", "HEAD", cwd=root)
    clean_source(root)
    data = affected(base + "..HEAD", root=root)
    if data["tier"] == "wide":
        with Slot():
            return wide_check(root, data)["exit"]
    options = fixture_options()
    legacy = options.get("timeout_seconds")
    if legacy is not None and (not math.isfinite(float(legacy)) or not 0 < float(legacy) <= 900):
        raise Refusal("invalid_presubmit_timeout: maximum is 900 seconds")
    build_cap = float(options.get("build_cap_seconds", legacy if legacy is not None else 1800))
    run_bound = float(options.get("run_bound_seconds", legacy if legacy is not None else 900))
    if not math.isfinite(build_cap) or not 0 < build_cap <= 1800:
        raise Refusal("invalid_build_cap: maximum is 1800 seconds")
    if not math.isfinite(run_bound) or not 0 < run_bound <= 900:
        raise Refusal("invalid_run_bound: maximum is 900 seconds")
    operation = uuid.uuid4().hex
    check_root, check_environment = candidate_environment(root, sha)
    for attempt in range(2):
        clean_source(root)
        if git("rev-parse", "HEAD", cwd=root) != sha:
            raise Refusal("candidate_changed_during_presubmit")
        record = presubmit_attempt(check_root, sha, base, data, build_cap, run_bound, attempt, operation,
                                   environment=check_environment)
        clean_source(root)
        if git("rev-parse", "HEAD", cwd=root) != sha:
            raise Refusal("candidate_changed_during_presubmit")
        if record["error"] != "build_cap":
            return record["exit"]
        if attempt == 0:
            print("build_cap: retrying once warm at the same checkout and target", flush=True)
        else:
            print("finding: recurring_build_cap at the same checkout and target", file=sys.stderr)
    return record["exit"]


def stop_worker(child):
    try:
        os.killpg(child.pid, signal.SIGTERM)  # recorded, newly-created group only
    except ProcessLookupError:
        pass
    try:
        child.wait(timeout=1)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        child.wait()


def presubmit_attempt(root, sha, base, data, build_cap, run_bound, attempt, operation, *, environment=None):
    output = state_directory() / "presubmits" / (sha + "-" + uuid.uuid4().hex[:8])
    output.mkdir(parents=True, mode=0o700)
    result_path = output / "affected.json"
    started = time.monotonic()
    clocks = TwoClocks(started)
    reason = None
    reader = None
    def read_events():
        nonlocal reader
        path = output / "phases.jsonl"
        if reader is None and path.exists():
            reader = path.open()
        if reader is not None:
            while True:
                position = reader.tell()
                line = reader.readline()
                if not line.endswith("\n"):
                    reader.seek(position)
                    break
                try:
                    clocks.event(json.loads(line))
                except json.JSONDecodeError:
                    pass
    with (output / "commands.log").open("w") as stream:
        child = subprocess.Popen(["sh", str(root / "scripts/presubmit.sh"), "--worker",
                                  base, str(result_path)], cwd=root,
                                 env=build_environment(root) if environment is None else environment, stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        atomic_write(output / "process.json", json.dumps({"pid": child.pid, "pgid": child.pid,
                                                          "sha": sha, "started": now(),
                                                          "attempt": attempt, "operation": operation}) + "\n")
        try:
            while True:
                read_events()
                ended = time.monotonic()
                reason = clocks.expired(ended, build_cap, run_bound)
                code = child.poll()
                if code is not None and code != 0:
                    reason = None  # an observed test failure is never retried as infrastructure
                    break
                if reason:
                    stop_worker(child)
                    code = 124
                    break
                if code is not None:
                    break
                try:
                    child.wait(timeout=0.1)
                except subprocess.TimeoutExpired:
                    pass
        except BaseException:
            stop_worker(child)
            raise
        finally:
            if reader is not None:
                reader.close()
    transcript = (output / "commands.log").read_text()
    record = {"sha": sha, "tier": data.get("tier", "unknown"),
              "affected": data.get("affected", []), "seconds": time.monotonic() - started,
              "verdict": "green" if code == 0 else "red", "exit": code,
              "error": reason, "build_cap_seconds": build_cap, "run_bound_seconds": run_bound,
              "attempt": attempt, "operation": operation,
              "finding": "recurring_build_cap" if reason == "build_cap" and attempt else None,
              "binary_durations": binary_durations(transcript),
              "log": str(output / "commands.log"),
              "timing_basis": "setup_and_visible_compile_build;all_other_execution_run",
              **clocks.totals(ended)}
    append_record("presubmit.log", record)
    print(transcript, end="")
    print(json.dumps(record, sort_keys=True), flush=True)
    if reason:
        print(f"{reason}: BUILD {record['compile_seconds']:.3f}/{build_cap:g}s; "
              f"RUN {record['run_seconds']:.3f}/{run_bound:g}s", file=sys.stderr)
    return record


def build_environment(root, *, target=None):
    environment = dict(os.environ)
    if target is None:
        # Explicit legacy callers (including private fixtures) override an
        # inherited transport; never borrow another checkout's Cargo artifacts.
        parent = Path(environment.get("CARGO_TARGET_DIR")
                      or environment.get("BBOXY_CARGO_TARGET_DIR") or root / "target")
        if not parent.is_absolute():
            parent = root / parent
        key = hashlib.sha256(os.fsencode(root.resolve())).hexdigest()[:20]
        target = parent.resolve() / ("checkout-" + key)
    environment.pop("CARGO_TARGET_DIR", None)
    environment.update(BBOXY_CARGO_TARGET_DIR=str(Path(target).resolve()),
                       CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0",
                       CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0",
                       CARGO_PROFILE_DEV_OPT_LEVEL="1", CARGO_PROFILE_TEST_OPT_LEVEL="1",
                       CARGO_PROFILE_DEV_DEBUG_ASSERTIONS="true", CARGO_PROFILE_TEST_DEBUG_ASSERTIONS="true")
    for name in ("CARGO_LOG", "CARGO_TERM_COLOR", "CARGO_TERM_PROGRESS_WHEN", "CARGO_TERM_PROGRESS_WIDTH"):
        environment.pop(name, None)
    return environment


def gate_proof(path, sha):
    path = Path(path).resolve(strict=True)
    digest = hashlib.sha256()
    sources, results = [], []
    last = ""
    passed = False
    with path.open("rb") as stream:
        for raw in stream:
            digest.update(raw)
            line = raw.decode("utf-8", errors="replace").strip()
            if line.startswith("ci-local: source "):
                sources.append(line.split()[2:])
            if line.startswith("ci-local: result "):
                results.append(line.split()[2:])
            passed |= line == "ci-local: all gates passed"
            if line:
                last = line
    if sources != [[sha, "normal"]]:
        raise Refusal("gate_log_source_mismatch: expected exact candidate " + sha)
    if (results != [[sha, "normal", "0"]] or not passed
            or last != f"ci-local: result {sha} normal 0"):
        raise Refusal("gate_log_not_green: full normal gate completion required")
    return {"gate_log": str(path), "gate_log_sha256": digest.hexdigest(),
            "tested_sha": sha}


def wide_check(root, data, gate_log=None):
    sha = git("rev-parse", "HEAD", cwd=root)
    started = time.monotonic()
    if gate_log is None:
        output = state_directory() / "presubmits" / (sha + "-" + uuid.uuid4().hex[:8])
        output.mkdir(parents=True, mode=0o700)
        gate_log = output / "commands.log"
        check_root, environment = candidate_environment(root, sha)
        environment["BBOXY_GATE_SLOT"] = "none"  # land/presubmit already owns it
        print("land: wide cut; full normal CI, no presubmit deadline", flush=True)
        with gate_log.open("w") as stream:
            code = measured_command(["sh", "scripts/ci-local.sh"], root, output / "phases.jsonl",
                                    stream=stream, env=environment)
        print(gate_log.read_text(), end="", flush=True)
        proof = gate_proof(gate_log, sha) if code == 0 else {"gate_log": str(gate_log)}
    else:
        proof = gate_proof(gate_log, sha)
        code = 0
        print("land: reusing full normal gate on exact candidate " + sha, flush=True)
    if git("rev-parse", "HEAD", cwd=root) != sha:
        raise Refusal("candidate_changed_during_gate")
    clean_source(root)
    record = {"sha": sha, "tier": "wide", "affected": data["affected"],
              "seconds": time.monotonic() - started, "verdict": "green" if code == 0 else "red",
              "exit": code, "binary_durations": binary_durations(Path(gate_log).read_text()),
              **proof}
    if "output" in locals():
        record.update(phase_totals(output / "phases.jsonl", time.monotonic()))
        record["timing_basis"] = "observed_output_phase_wall"
    else:
        transcript = Path(gate_log).read_text()
        durations = re.findall(r"\bFinished .*?\bin ((?:[0-9.]+[hms]\s*)+)", transcript)
        compile_seconds = sum(float(value) * {"h": 3600, "m": 60, "s": 1}[unit]
                              for duration in durations for value, unit in re.findall(r"([0-9.]+)([hms])", duration))
        binaries = binary_durations(transcript)
        fixture_seconds = [float(v) for v in re.findall(r"Ran \d+ fixture tests in ([0-9.]+)s", transcript)]
        record.update(compile_seconds=compile_seconds if durations else None,
                      test_seconds=sum(b["seconds"] for b in binaries) + sum(fixture_seconds)
                      if binaries or fixture_seconds else None,
                      tests_started=bool(binaries or fixture_seconds),
                      timing_basis="provided_log_cargo_and_test_reported_wall")
    append_record("presubmit.log", record)
    return record


def binary_durations(transcript):
    result = []
    binary = None
    for line in transcript.splitlines():
        heading = re.search(r"\bRunning .*\(([^)]+)\)\s*$", line)
        docs = re.search(r"\bDoc-tests (\S+)", line)
        if heading:
            binary = heading.group(1)
        elif docs:
            binary = "doc-tests:" + docs.group(1)
        duration = re.search(r"^test result: .*finished in ([0-9.]+)s", line)
        if duration:
            seconds = float(duration.group(1))
            result.append({"binary": binary, "seconds": seconds,
                           "over_60s": seconds > 60})
            binary = None
    return result


def ancestor(older, newer, root):
    result = command(["git", "merge-base", "--is-ancestor", older, newer], cwd=root, check=False)
    if result.returncode not in (0, 1):
        raise Refusal("cannot_check_fast_forward: " + result.stderr)
    return result.returncode == 0


def configured_author(root):
    email = git("config", "--get", "user.email", cwd=root)
    key = git("config", "--get", "user.signingkey", cwd=root).removesuffix("!")
    program = command(["git", "config", "--get", "gpg.program"], cwd=root, check=False).stdout.strip() or "gpg"
    listing = command([program, "--batch", "--with-colons", "--fingerprint", "--list-keys", key], cwd=root).stdout
    fingerprint = next((line.split(":")[9] for line in listing.splitlines()
                        if line.startswith("fpr:")), None)
    if not fingerprint:
        raise Refusal("configured_signing_key_unavailable")
    return email, fingerprint.upper()


def own_signatures(root, base):
    email, key = configured_author(root)
    commits = git("rev-list", "--reverse", base + "..HEAD", cwd=root).splitlines()
    for sha in commits:
        fields = git("show", "-s", "--format=%G?%x00%GF%x00%GP%x00%ae", sha, cwd=root).split("\0")
        if len(fields) != 4 or fields[0] != "G" or fields[3] != email or key not in (fields[1].upper(), fields[2].upper()):
            raise Refusal("signature_or_author_refused: " + sha)
    return commits, email, key


def main_guard(root, base):
    path = state_directory() / "main-status"
    if not path.exists():
        return
    fields = path.read_text().strip().split()
    if not fields or fields[0] not in ("red", "green"):
        raise Refusal("main_status_invalid")
    if fields[0] == "red":
        subjects = git("log", "--format=%s", base + "..HEAD", cwd=root).splitlines()
        if not subjects or not all(subject.startswith("Revert") for subject in subjects):
            raise Refusal("main_red: only an all-revert cut may land", 76)


def slot_directory():
    return state_directory() / "slots" if os.environ.get("BBOXY_LANDING_STATE_DIR") else Path("/tmp")


def common_repository(root):
    path = Path(git("rev-parse", "--git-common-dir", cwd=root))
    return (root / path).resolve()


@contextmanager
def slot_guard(path, *, blocking=False):
    # Short per-slot filesystem transitions only. Other slots never wait.
    with path.with_name("." + path.name + ".lock").open("a") as stream:
        try:
            fcntl.flock(stream, fcntl.LOCK_EX | (0 if blocking else fcntl.LOCK_NB))
        except BlockingIOError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(stream, fcntl.LOCK_UN)


def remove_slot(path, root):
    if any(p.name not in {"pid", "owner.json", "tree", "target"} for p in path.iterdir()):
        raise Refusal("slot_has_unknown_files: " + str(path))
    tree = path / "tree"
    if tree.exists():
        if tree.is_symlink() or common_repository(tree) != common_repository(root):
            raise Refusal("slot_tree_is_not_this_repository")
        git("worktree", "remove", "--force", str(tree), cwd=root)
    target = path / "target"
    if target.exists():
        if target.is_symlink():
            raise Refusal("slot_target_is_symlink")
        shutil.rmtree(target)
    (path / "pid").unlink(missing_ok=True)
    (path / "owner.json").unlink(missing_ok=True)
    path.rmdir()


class Slot:
    def __init__(self, label="land", preferred=None):
        self.path = None
        self.number = None
        self.token = uuid.uuid4().hex
        self.root = repository()
        self.label, self.preferred = label, preferred
        self.previous_environment = {}

    def __enter__(self):
        directory = slot_directory()
        directory.mkdir(parents=True, exist_ok=True)
        limit = 3 if shutil.disk_usage(Path.home()).free >= 25 * 1024 ** 3 else 2
        numbers = [self.preferred] if self.preferred is not None else range(1, limit + 1)
        for number in numbers:
            if number not in range(1, limit + 1):
                continue
            path = directory / f"bboxy450-gate-slot-{number}"
            if path.is_symlink():
                continue
            with slot_guard(path) as held:
                if not held:
                    continue
                if path.exists():
                    try:
                        pid = int((path / "pid").read_text())
                        if pid <= 0:
                            continue
                        os.kill(pid, 0)
                        continue
                    except ProcessLookupError:
                        try:
                            remove_slot(path, self.root)
                        except (OSError, Refusal):
                            continue
                    except (ValueError, OSError):
                        continue
                path.mkdir()
                owner = {"pid": os.getpid(), "token": self.token,
                         "repository": str(common_repository(self.root))}
                atomic_write(path / "pid", str(os.getpid()) + "\n")
                atomic_write(path / "owner.json", json.dumps(owner) + "\n")
                self.path, self.number = path.resolve(), number
            values = {"BBOXY_GATE_SLOT_ROOT": str(self.path),
                      "BBOXY_GATE_SLOT_TOKEN": self.token,
                      "BBOXY_GATE_SLOT_OWNER": str(os.getpid())}
            self.previous_environment = {k: os.environ.get(k) for k in values}
            os.environ.update(values)
            print(f"{self.label}: {'gate ' if self.label == 'ci-local' else ''}slot {number} of {limit}", flush=True)
            return self
        raise Refusal(f"all {limit} slots are taken", 75)

    def __exit__(self, *unused):
        try:
            if self.path is not None:
                with slot_guard(self.path, blocking=True):
                    data = json.loads((self.path / "owner.json").read_text())
                    if data.get("token") == self.token and data.get("pid") == os.getpid():
                        remove_slot(self.path, self.root)
        finally:
            for key, value in self.previous_environment.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value


def active_slot(root):
    value = os.environ.get("BBOXY_GATE_SLOT_ROOT")
    if not value:
        return None
    path = Path(value)
    # Child fixture repositories inherit environment, never their parent's lease.
    if path.parent.resolve() != slot_directory().resolve():
        return None
    if not re.fullmatch(r"bboxy450-gate-slot-[123]", path.name):
        return None
    try:
        owner = json.loads((path / "owner.json").read_text())
        if (owner.get("token") != os.environ.get("BBOXY_GATE_SLOT_TOKEN")
                or str(owner.get("pid")) != os.environ.get("BBOXY_GATE_SLOT_OWNER")
                or owner.get("repository") != str(common_repository(root))):
            return None
        os.kill(int(owner["pid"]), 0)
    except (OSError, ValueError, KeyError):
        return None
    return path


def candidate_environment(root, sha):
    path = active_slot(root)
    if path is None:
        return root, build_environment(root)
    tree, target = path / "tree", path / "target"
    if tree.exists():
        if common_repository(tree) != common_repository(root):
            raise Refusal("slot_tree_is_not_this_repository")
        git("checkout", "-f", sha, cwd=tree)
        git("clean", "-fdx", cwd=tree)
    else:
        git("worktree", "add", "--detach", str(tree), sha, cwd=root)
    if target.exists():
        if target.is_symlink():
            raise Refusal("slot_target_is_symlink")
        shutil.rmtree(target)
    target.mkdir()
    return tree, build_environment(tree, target=target)


def ci_local(arguments):
    if any(arg not in {"--parity", "--slow", "--after-tests"} for arg in arguments):
        raise Refusal("unknown ci-local flag")
    root = repository()
    clean_source(root)
    sha = git("rev-parse", "HEAD", cwd=root)
    with Slot("ci-local"):
        tree, environment = candidate_environment(root, sha)
        environment["BBOXY_GATE_SLOT"] = "none"
        return command(["sh", "scripts/ci-local.sh", *arguments], cwd=tree,
                       env=environment, capture=False, check=False).returncode


def announcement(sha, subject):
    receipt = state_directory() / "announced" / (sha + ".json")
    if receipt.exists():
        return
    text = f"LANDED {sha} {subject[:180]}"
    preview = json.loads(command(["icq", "--dry-run", "send", "#bboxy450-term", text]).stdout)
    if preview.get("operation") != "send" or preview.get("arguments", {}).get("name") != "#bboxy450-term" or preview["arguments"].get("text") != text:
        raise Refusal("icq_preview_mismatch")
    sent = command(["icq", "send", "#bboxy450-term", text])
    atomic_write(receipt, sent.stdout)
    print(text, flush=True)


def records(name):
    path = state_directory() / name
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()] if path.exists() else []


def local_origin(root):
    value = git("remote", "get-url", "origin", cwd=root)
    if value.startswith("file://"):
        from urllib.parse import unquote, urlsplit
        value = unquote(urlsplit(value).path)
    elif ":" in value:
        return False
    return (root / value).resolve().is_dir()


def publishing_identity(root):
    if local_origin(root):
        return
    if os.environ.get("BBOXY_LANDING_STATE_DIR"):
        raise Refusal("fixture_state_requires_local_origin")
    origin = git("remote", "get-url", "origin", cwd=root)
    if not re.search(r"github\.com[:/]bboxy450/", origin):
        raise Refusal("unapproved_origin")
    actor = command(["gh", "api", "user", "--jq", ".login"], cwd=root).stdout.strip()
    if actor != "bboxy450":
        raise Refusal("unapproved_github_identity: " + actor)


def land(gate_log=None):
    root = repository()
    clean_source(root)
    branch = git("symbolic-ref", "--short", "HEAD", cwd=root)
    if branch == "main":
        raise Refusal("land_from_your_branch")
    publishing_identity(root)
    tested = None
    previous_base = None
    spent = 0.
    landing = None
    proof = {}
    check_timings = []
    with Slot():
        for retry in range(3):
            git("fetch", "origin", "main", cwd=root)
            base = git("rev-parse", "origin/main", cwd=root)
            main_guard(root, base)
            commits, email, key = own_signatures(root, base)
            if not commits:
                sha = git("rev-parse", "HEAD", cwd=root)
                existing = next((r for r in records("landings.log") if r.get("sha") == sha
                                 and r.get("author_key") == key), None)
                if existing:
                    landing = existing
                    break
                raise Refusal("no_own_commits_to_land")
            moved = affected(previous_base + ".." + base, root=root) if previous_base else None
            rebased = command(["git", "rebase", "origin/main"], cwd=root, capture=False, check=False)
            if rebased.returncode:
                raise Refusal("conflict: resolve this rebase before rerunning", 78)
            data = affected(base + "..HEAD", root=root)
            intersection = moved and (set(data["affected"]) & set(moved["affected"])
                                      or set(data["paths"]) & set(moved["paths"])
                                      or moved["shared"]
                                      or (any(p.startswith(("lab/", "design/")) for p in data["shared"])
                                          and moved["parity"] == "yes"))
            candidate = git("rev-parse", "HEAD", cwd=root)
            if (tested is None or intersection
                    or (data["tier"] == "wide" and tested != candidate)):
                started = time.monotonic()
                if data["tier"] == "wide":
                    proof = wide_check(root, data, gate_log)
                    code = proof["exit"]
                else:
                    if gate_log:
                        raise Refusal("gate_log_requires_wide_cut")
                    code = command(["sh", "scripts/presubmit.sh", base], cwd=root,
                                   capture=False, check=False).returncode
                    proof = next((row for row in reversed(records("presubmit.log"))
                                  if row.get("sha") == candidate), {})
                    if proof.get("operation"):
                        attempts = [row for row in records("presubmit.log")
                                    if row.get("operation") == proof["operation"]]
                        for field in ("compile_seconds", "run_seconds", "test_seconds", "unclassified_seconds"):
                            proof[field] = sum(row[field] for row in attempts)
                        proof["build_attempts"] = len(attempts)
                spent += time.monotonic() - started
                check_timings.append({k: proof.get(k) for k in
                                      ("sha", "compile_seconds", "run_seconds", "test_seconds", "unclassified_seconds", "timing_basis")})
                if code:
                    raise Refusal("presubmit_failed", 77)
                tested = git("rev-parse", "HEAD", cwd=root)
            else:
                print("land: moved changes are disjoint; reusing presubmit", flush=True)
            own_signatures(root, base)
            git("fetch", "origin", "main", cwd=root)
            latest = git("rev-parse", "origin/main", cwd=root)
            previous_base = base
            if latest != base:
                continue
            main_guard(root, base)  # a watcher can close the tree during the check
            if not ancestor(base, "HEAD", root):
                raise Refusal("not_a_fast_forward")
            sha = git("rev-parse", "HEAD", cwd=root)
            pushed = command(["git", "push", "origin", "HEAD:main"], cwd=root, check=False)
            if pushed.returncode:
                git("fetch", "origin", "main", cwd=root)
                if not ancestor(sha, "origin/main", root):
                    if git("rev-parse", "origin/main", cwd=root) == base:
                        raise Refusal("push_failed: " + pushed.stderr)
                    continue
            landing = {"sha": sha, "timestamp": now(), "author_key": key, "author": email,
                       "tier": proof.get("tier", data["tier"]), "affected": data["affected"],
                       "presubmit_seconds": spent, "retries": retry, "paths": data["paths"],
                       "tested_sha": tested, "branch": branch}
            landing.update({k: proof[k] for k in ("gate_log", "gate_log_sha256", "compile_seconds",
                                                 "run_seconds", "test_seconds", "unclassified_seconds", "build_attempts",
                                                 "tests_started", "timing_basis") if k in proof})
            landing["check_timings"] = check_timings
            if len(check_timings) > 1:
                landing["timing_basis"] = "sum_of_check_timings"
            for kind in ("compile_seconds", "run_seconds", "test_seconds", "unclassified_seconds"):
                landing[kind] = (sum(row[kind] for row in check_timings)
                                 if all(row[kind] is not None for row in check_timings) else None)
            append_record("landings.log", landing)
            break
        if landing is None:
            raise Refusal("main_moving: three attempts exhausted", 79)
    # Communication happens after slot release; it never holds build capacity.
    announcement(landing["sha"], git("show", "-s", "--format=%s", landing["sha"], cwd=root))
    return 0


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "ci-local":
        return ci_local(sys.argv[2:])
    delegates = {"quarantine": ("quarantine", "administer"),
                 "quarantine-test": ("quarantine", "test"),
                 "main-watch": ("main_watch", "run")}
    if len(sys.argv) > 1 and sys.argv[1] in delegates:
        module_name, function = delegates[sys.argv[1]]
        spec = importlib.util.spec_from_file_location(module_name, Path(__file__).with_name(module_name + ".py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return getattr(module, function)(sys.argv[2:], sys.modules[__name__])
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="verb", required=True)
    p = sub.add_parser("affected")
    p.add_argument("range", nargs="?", default="origin/main..HEAD")
    p.add_argument("--crates", action="store_true")
    sub.add_parser("state-dir")
    sub.add_parser("parity-dir")
    p = sub.add_parser("check-is-cargo")
    p.add_argument("check")
    p = sub.add_parser("presubmit")
    p.add_argument("base", nargs="?", default="origin/main")
    p = sub.add_parser("presubmit-worker")
    p.add_argument("base")
    p.add_argument("result", type=Path)
    p = sub.add_parser("land")
    p.add_argument("--gate-log", type=Path,
                   help="reuse full NORMAL CI on the exact wide candidate SHA")
    args = parser.parse_args()
    if args.verb == "affected":
        result = affected(args.range)
        print(" ".join(result["affected"]) if args.crates else json.dumps(result))
    elif args.verb == "state-dir":
        print(state_directory())
    elif args.verb == "parity-dir":
        print(parity_directory())
    elif args.verb == "check-is-cargo":
        return 0 if cargo_backed(args.check) else 1
    elif args.verb == "presubmit":
        return presubmit(args.base)
    elif args.verb == "presubmit-worker":
        return presubmit_worker(args.base, args.result)
    elif args.verb == "land":
        return land(args.gate_log)


if __name__ == "__main__":
    try:
        raise SystemExit(main() or 0)
    except (Refusal, OSError, ValueError) as error:
        print(f"landing: {error}", file=sys.stderr)
        raise SystemExit(error.code if isinstance(error, Refusal) else 2)
