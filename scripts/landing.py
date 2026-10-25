"""The landing scripts' shared state and bounded command implementation (#323).

Shell entry points keep the same contract on every worktree. All Git commands
use argument arrays; fixture runs use their own repository and shared-state dir.
"""
import argparse
import importlib.util
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


class Refusal(Exception):
    def __init__(self, message, code=2):
        super().__init__(message)
        self.code = code


def command(argv, *, cwd=None, capture=True, check=True, **kwargs):
    result = subprocess.run(argv, cwd=cwd, text=True,
                            capture_output=capture, **kwargs)
    if check and result.returncode:
        raise Refusal(f"command_failed ({result.returncode}): {' '.join(argv)}\n"
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
    broad = [p for p in paths if p in ("Cargo.toml", "Cargo.lock")
             or p.startswith(".cargo/")
             or (p.startswith("scripts/") and not p.startswith("scripts/e2e/"))]
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
    ui = {"bbt-ui", "bbt-ui-model", "bbt-render", "bboxybook"}
    parity = any(p.startswith(("lab/", "design/")) for p in paths) or bool(touched & ui)
    shared_lab = {"lab/web/lab.css", "lab/web/tokens.js", "lab/web/glow.js",
                  "lab/web/components.js"}
    shared = sorted(set(broad) | (set(paths) & shared_lab)
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
    result = command(list(map(str, argv)), cwd=root, capture=False, check=False)
    if result.returncode:
        raise Refusal(f"check_failed ({result.returncode}): {argv[0]}", 1)


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
        target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
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
        checked(["sh", "scripts/invariants.sh"], root)
        dispatch = any(Path(p).name in ("ctl.rs", "seat_socket.rs", "logic.rs")
                       or "/src/ctl/" in p or "/src/mcp/" in p for p in data["paths"])
        if dispatch:
            # The dispatch cuts supply their golden tests with the dispatcher.
            # A zero-test filter is not evidence of byte identity.
            test = command(["cargo", "test", "--locked", *packages, "golden"], cwd=root)
            print(test.stdout, end="", flush=True)
            if not re.search(r"test result: ok\. [1-9][0-9]* passed", test.stdout):
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
    seconds = float(fixture_options().get("timeout_seconds", 900))
    if not math.isfinite(seconds) or not 0 < seconds <= 900:
        raise Refusal("invalid_presubmit_timeout: maximum is 900 seconds")
    output = state_directory() / "presubmits" / (sha + "-" + uuid.uuid4().hex[:8])
    output.mkdir(parents=True, mode=0o700)
    result_path = output / "affected.json"
    started = time.monotonic()
    timed_out = False
    with (output / "commands.log").open("w") as stream:
        child = subprocess.Popen(["sh", str(root / "scripts/presubmit.sh"), "--worker",
                                  base, str(result_path)], cwd=root, stdout=stream,
                                 stderr=subprocess.STDOUT, start_new_session=True)
        atomic_write(output / "process.json", json.dumps({"pid": child.pid, "pgid": child.pid,
                                                          "sha": sha, "started": now()}) + "\n")
        try:
            code = child.wait(timeout=seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            try:
                os.killpg(child.pid, signal.SIGTERM)  # our recorded, newly-created group only
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
            code = 124
    data = json.loads(result_path.read_text()) if result_path.exists() else {}
    transcript = (output / "commands.log").read_text()
    record = {"sha": sha, "tier": data.get("tier", "unknown"),
              "affected": data.get("affected", []), "seconds": time.monotonic() - started,
              "verdict": "green" if code == 0 else "red", "exit": code,
              "error": "presubmit_timeout" if timed_out else None,
              "binary_durations": binary_durations(transcript),
              "log": str(output / "commands.log")}
    append_record("presubmit.log", record)
    print(transcript, end="")
    print(json.dumps(record, sort_keys=True), flush=True)
    if timed_out:
        print(f"presubmit_timeout: {seconds:g}s bound enforced (maximum 900s)", file=sys.stderr)
    return code


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


class Slot:
    def __init__(self):
        self.path = None
        self.number = None
        self.token = uuid.uuid4().hex

    def __enter__(self):
        directory = state_directory() / "slots" if os.environ.get("BBOXY_LANDING_STATE_DIR") else Path("/tmp")
        directory.mkdir(parents=True, exist_ok=True)
        limit = 3 if shutil.disk_usage(Path.home()).free >= 25 * 1024 ** 3 else 2
        for number in range(1, limit + 1):
            path = directory / f"bboxy450-gate-slot-{number}"
            if path.is_symlink():
                continue
            if path.exists():
                try:
                    pid = int((path / "pid").read_text())
                    if pid <= 0:
                        continue
                    os.kill(pid, 0)
                    continue
                except ProcessLookupError:
                    # Shared protocol authorizes reclaiming a dead owner's slot,
                    # not deleting arbitrary contents under /tmp.
                    if any(p.name not in ("pid", "owner.json") for p in path.iterdir()):
                        continue
                    (path / "pid").unlink(missing_ok=True)
                    (path / "owner.json").unlink(missing_ok=True)
                    try:
                        path.rmdir()
                    except OSError:
                        continue
                except (ValueError, OSError):
                    continue
            try:
                path.mkdir()
            except FileExistsError:
                continue
            atomic_write(path / "pid", str(os.getpid()) + "\n")
            atomic_write(path / "owner.json", json.dumps({"pid": os.getpid(), "token": self.token}) + "\n")
            self.path, self.number = path, number
            print(f"land: slot {number} of {limit}", flush=True)
            return self
        raise Refusal(f"all {limit} slots are taken", 75)

    def __exit__(self, *unused):
        if self.path is None:
            return
        try:
            data = json.loads((self.path / "owner.json").read_text())
            if data.get("token") == self.token and data.get("pid") == os.getpid():
                (self.path / "pid").unlink()
                (self.path / "owner.json").unlink()
                self.path.rmdir()
        except OSError as error:
            print(f"land: slot_cleanup: {error}", file=sys.stderr)


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


def land():
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
            if tested is None or intersection:
                started = time.monotonic()
                checked_result = command(["sh", "scripts/presubmit.sh", base], cwd=root,
                                         capture=False, check=False)
                spent += time.monotonic() - started
                if checked_result.returncode:
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
                       "tier": data["tier"], "affected": data["affected"],
                       "presubmit_seconds": spent, "retries": retry, "paths": data["paths"],
                       "tested_sha": tested, "branch": branch}
            append_record("landings.log", landing)
            break
        if landing is None:
            raise Refusal("main_moving: three attempts exhausted", 79)
    # Communication happens after slot release; it never holds build capacity.
    announcement(landing["sha"], git("show", "-s", "--format=%s", landing["sha"], cwd=root))
    return 0


def main():
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
    sub.add_parser("land")
    args = parser.parse_args()
    if args.verb == "affected":
        result = affected(args.range)
        print(" ".join(result["affected"]) if args.crates else json.dumps(result, sort_keys=True))
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
        return land()


if __name__ == "__main__":
    try:
        raise SystemExit(main() or 0)
    except (Refusal, OSError, ValueError) as error:
        print(f"landing: {error}", file=sys.stderr)
        raise SystemExit(error.code if isinstance(error, Refusal) else 2)
