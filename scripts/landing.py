# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""The landing scripts' shared state and bounded command implementation (#323).

Shell entry points keep the same contract on every worktree. All Git commands
use argument arrays; fixture runs use their own repository and shared-state dir.
"""
import argparse
import ast
from contextlib import contextmanager
import fcntl
import fnmatch
from html.parser import HTMLParser
from urllib.parse import unquote, urlsplit
import hashlib
import importlib.util
import io
from datetime import datetime, timezone
import json
import math
import os
from pathlib import Path
import re
import resource
import shlex
import shutil
import signal
import subprocess
import sys
import time
import tomllib
import uuid

# Running a gate must not leave bytecode among a candidate's source paths.
sys.dont_write_bytecode = True

_timing_path = None
STAGE_ALLOCATIONS = {"formal": 290., "tests": 400., "fixtures": 60., "invariants_misc": 60.}
STAGE_MARGIN = 90.
FORMAL_PHASE_PREFIX = "formal-phase: "


def fixture_copy_scripts(root=None):
    """Read the fixture's one copy tuple as data, without importing its tests."""
    path = Path(root or Path(__file__).resolve().parents[1]) / "scripts/test-landing.py"
    declarations = [node for node in ast.parse(path.read_text()).body
                    if isinstance(node, ast.Assign)
                    and any(isinstance(target, ast.Name) and target.id == "FIXTURE_SCRIPTS" for target in node.targets)]
    if len(declarations) != 1:
        raise Refusal("fixture_copy_tuple_missing_or_ambiguous")
    names = ast.literal_eval(declarations[0].value)
    if not isinstance(names, tuple) or not all(isinstance(name, str) and Path(name).name == name for name in names):
        raise Refusal("fixture_copy_tuple_invalid")
    return set(names)


def landing_machinery(paths, root=None):
    names = {"invariants.sh", "ci-local.sh", "test-landing.py", "test-resource-ledger.py"}
    names.update(fixture_copy_scripts(root))
    return any(p.startswith("scripts/") and
               (Path(p).name in names or Path(p).name.startswith("resource-ledger")) for p in paths)


def load_average():
    try:
        return os.getloadavg()[0]
    except OSError:
        return None


def budget_findings(totals, load_start, load_end):
    findings = [{"kind": "stage_allocation", "stage": stage, "seconds": seconds,
                 "allocation_seconds": STAGE_ALLOCATIONS[stage]}
                for stage, seconds in totals["stage_seconds"].items()
                if seconds > STAGE_ALLOCATIONS[stage]]
    if totals["unclassified_seconds"] > 15:
        findings.append({"kind": "unclassified", "seconds": totals["unclassified_seconds"],
                         "allocation_seconds": 15.})
    return [{**row, "load_average_start": load_start, "load_average_end": load_end}
            for row in findings]


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


DELEGATES = {"quarantine": ("quarantine", "administer"),
             "quarantine-test": ("quarantine", "test"),
             "main-watch": ("main_watch", "run")}


def cargo_parts(arguments):
    """Read Cargo's command/options, excluding test or application argv."""
    arguments = iter(enumerate(arguments))
    command_name = None
    command_index = None
    for index, argument in arguments:
        if argument in {"--config", "--color", "-Z", "-C"}:
            next(arguments, None)
        elif argument.startswith(("+", "-")):
            continue
        else:
            command_name = argument
            command_index = index
            break
    options = []
    for _, argument in arguments:
        if argument == "--":
            break
        options.append(argument)
    return command_name, options, command_index


def cargo_qos(arguments):
    command_name, options, _ = cargo_parts(arguments)
    if command_name == "run":
        # App processes must be able to claim their own responsiveness class.
        return "utility" if any(options[i:i+2] == ["--example", "verify"] for i in range(len(options))) else "default"
    if command_name not in {"test", "nextest", "bench"}:
        return "background"
    if "--no-run" in options or (command_name == "nextest" and options[:1] == ["list"]):
        return "background"
    if command_name == "bench" or "--benches" in options:
        return "default"
    timed = {"gates_seats", "input_latency", "dream_c1", "dream_replay", "gui_budget", "formal_contracts"}
    targets = {options[i+1] for i, value in enumerate(options[:-1]) if value == "--test"}
    targets.update(value.split("=", 1)[1] for value in options if value.startswith("--test="))
    if targets & timed or "dream::c1::" in options or os.environ.get("BBOXY_QOS_TIMED_GATE") == "dream-c1":
        return "default"
    return "utility"


def cargo_prebuild(arguments):
    """Build a test selection as one background tree, before clamping tests."""
    before = arguments[:arguments.index("--")] if "--" in arguments else arguments
    if "--no-run" in before:
        return None
    command_name, options, index = cargo_parts(arguments)
    if command_name == "run":
        return [*before[:index], "build", *before[index+1:]]
    if command_name not in {"test", "nextest", "bench"}:
        return None
    if command_name == "nextest" and options[:1] != ["run"]:
        return None
    if command_name == "test" and "--doc" in before:
        # Cargo cannot combine --doc with --no-run. Prebuild its library
        # dependencies (including native build scripts) as a lib test instead.
        return [*[value for value in before if value != "--doc"], *([] if "--lib" in before else ["--lib"]), "--no-run", *arguments[len(before):]]
    # nextest forbids its execution-only --no-fail-fast with --no-run.
    # Preserve the original arguments for actual execution, including anything
    # after --; only the compile-only invocation omits this runner flag.
    build = [value for value in before if value != "--no-fail-fast"] if command_name == "nextest" else before
    return [*build, "--no-run", *arguments[len(before):]]


def qos_cargo_exec(arguments):
    root = Path(os.environ["BBOXY_QOS_ROOT"])
    wrapper = str(root / "scripts/qos.sh")
    cargo = os.environ["BBOXY_REAL_CARGO"]
    prebuild = cargo_prebuild(arguments)
    if prebuild:
        # Do not duplicate Cargo's machine-readable stdout. On failure retain
        # diagnostics even when Cargo itself emitted them as JSON on stdout.
        result = subprocess.run(["/bin/sh", wrapper, "background", cargo, *prebuild],
                                stdout=subprocess.PIPE)
        if result.returncode:
            sys.stderr.buffer.write(result.stdout)
            return result.returncode if result.returncode >= 0 else 128 - result.returncode
    os.execv("/bin/sh", ["/bin/sh", wrapper, cargo_qos(arguments), cargo, *arguments])


def cost_command(arguments):
    """Account for a complete child tree; preserve its exit status on failure."""
    target, label, *argv = arguments
    if not argv:
        raise Refusal("cost: missing command")
    def disk():
        if not Path(target).exists():
            return 0
        result = subprocess.run(["du", "-sk", target], capture_output=True, text=True)
        if result.returncode:
            raise Refusal("cost: cannot measure target directory")
        return int(result.stdout.split()[0]) * 1024
    before_disk = disk()
    before_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.monotonic()
    result = subprocess.run(argv, check=False)
    elapsed = time.monotonic() - started
    after_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after_cpu.ru_utime + after_cpu.ru_stime - before_cpu.ru_utime - before_cpu.ru_stime
    growth = disk() - before_disk
    print(f"land-cost: {label} wall={elapsed:.3f}s cpu={cpu:.3f}s disk={growth:+d}B exit={result.returncode}",
          flush=True)
    return result.returncode if result.returncode >= 0 else 128 - result.returncode


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


def resource_literals(text):
    """Local resource names, without treating navigation links as includes."""
    quoted = re.findall(r'''["'`]([^"'`\n]+)["'`]''', text)
    result = []
    for value in quoted:
        try:
            suffix = Path(urlsplit(value).path).suffix.lower()
        except ValueError:
            continue
        if suffix in {".js", ".mjs", ".css", ".json", ".toml", ".py", ".md", ".svg", ".png", ".woff", ".woff2", ".ttf"}:
            result.append(value)
    result.extend(re.findall(r"url\(\s*([^\)]+)\)", text))
    result.extend(re.findall(r'''(?:fetch|import)\(\s*["']([^"']+)["']''', text))
    return result

class ResourceHTML(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.references = []
        self.code = None
        self.feed(text)

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        names = {"script": ("src",), "link": ("href",), "img": ("src",),
                 "iframe": ("src",), "source": ("src",), "embed": ("src",),
                 "object": ("data",), "video": ("src", "poster"), "audio": ("src",)}
        self.references.extend(attributes[name] for name in names.get(tag, ()) if attributes.get(name))
        self.references.extend(resource_literals(attributes.get("style", "")))
        if tag in ("script", "style"):
            self.code = tag

    def handle_endtag(self, tag):
        if tag == self.code:
            self.code = None

    def handle_data(self, data):
        if self.code:
            self.references.extend(resource_literals(data))

def affected_lab_pages(root, paths):
    """Reverse the actual local resource/header graph; no network or execution."""
    root = Path(root).resolve()
    graph = {}
    pending = list((root / "lab/web").rglob("*.html"))
    pending += list((root / "lab/web").rglob("*.js"))
    pending += list((root / "lab/web").rglob("*.css"))
    seen = set()
    # Generator code and design/records inputs are leaves. Their declarations
    # name inputs; interpreting generator output filenames as inputs adds cycles.
    text_types = {".html", ".js", ".mjs", ".css", ".json"}

    def local(value, parent):
        value = value.strip().strip("\"'`")
        try:
            url = urlsplit(value)
        except ValueError:
            return None
        if url.scheme or url.netloc or not url.path:
            return None
        name = unquote(url.path).replace("\\/", "/")
        candidates = [Path(os.path.abspath(parent / name)), Path(os.path.abspath(root / name))]
        safe = [p for p in candidates if p.is_relative_to(root) and p.resolve().is_relative_to(root)]
        return next((p for p in safe if p.is_file()), safe[0] if safe else None)

    while pending:
        page = pending.pop()
        if page in seen or not page.is_file() or not page.resolve().is_relative_to(root):
            continue
        seen.add(page)
        if page.suffix.lower() not in text_types:
            continue
        text = page.read_text(errors="replace")
        name = page.relative_to(root).as_posix()
        dependencies = set()
        if page.resolve() != page:
            dependencies.add(page.resolve())
        references = ResourceHTML(text).references if page.suffix == ".html" else resource_literals(text)
        dependencies.update(p for value in references if (p := local(value, page.parent)) is not None)
        first = text.splitlines()[0] if text else ""
        if re.match(r"// GENERATED by ", first, re.I):
            # Same repository-path/glob contract checked by generated-sources.sh.
            patterns = re.findall(r"(?:\.?[\w*.-]+/)+[\w*.-]+", first)
            for pattern in patterns:
                pattern = pattern.rstrip(".,:;")
                dependencies.update(Path(os.path.abspath(p)) for p in root.glob(pattern)
                                    if Path(os.path.abspath(p)).is_relative_to(root) and p.resolve().is_relative_to(root))
                dependencies.update(root / path for path in paths if fnmatch.fnmatchcase(path, pattern))
        graph[name] = {p.relative_to(root).as_posix() for p in dependencies}
        pending.extend(dependencies - seen)
    reverse = {}
    for owner, dependencies in graph.items():
        for dependency in dependencies:
            reverse.setdefault(dependency, set()).add(owner)
    affected_paths = set(paths)
    pending_paths = list(paths)
    while pending_paths:
        for owner in reverse.get(pending_paths.pop(), set()) - affected_paths:
            affected_paths.add(owner)
            pending_paths.append(owner)
    return sorted(p for p in affected_paths if p.startswith("lab/web/") and p.endswith(".html"))

def catalogue_page(case):
    # capture.mjs:67 hosts @component in seat-row.html and removes query strings.
    page = "seat-row.html" if case["page"] == "@component" else urlsplit(case["page"]).path
    return "lab/web/" + page.lstrip("/")

def rust_paths(text):
    """Keep token positions while masking comments and quoted Rust data."""
    pattern = r'''(?s)//[^\n]*|/\*.*?\*/|\br(?P<hashes>\#*)".*?"(?P=hashes)|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])' '''.rstrip()
    mask = list(text)
    literals = []
    for match in re.finditer(pattern, text):
        token = match.group()
        if token.startswith('"') or token.startswith('r'):
            first, last = token.find('"'), token.rfind('"')
            value = token[first + 1:last].replace('\\"', '"').replace('\\\\', '\\')
            literals.append((match.start(), match.end(), value))
        mask[match.start():match.end()] = " " * (match.end() - match.start())
    return "".join(mask), literals

def balanced_end(mask, start, opening="(", closing=")"):
    depth = 1
    for index in range(start, len(mask)):
        if mask[index] == opening:
            depth += 1
        elif mask[index] == closing:
            depth -= 1
            if not depth:
                return index
    return len(mask)

def native_input_crates(root, paths, packages):
    root = Path(root).resolve()
    # The validation-input rule for scripts/ also applies when a test names one.
    wanted = {Path(os.path.abspath(root / path)) for path in paths if not path.startswith("scripts/")}
    if not wanted:
        return set()
    consumers = set()
    for name, package in packages.items():
        directory = Path(package["manifest_path"]).parent
        # Packaged copies keep the repository-relative source path below
        # release-inputs/. A source edit therefore reaches its native owner.
        bundled = directory / "release-inputs"
        if any(copy.is_file() and root / copy.relative_to(bundled) in wanted
               for copy in bundled.rglob("*")):
            consumers.add(name)
            continue
        for source in directory.rglob("*.rs"):
            if not source.resolve().is_relative_to(root):
                continue
            text = source.read_text(errors="replace")
            mask, literals = rust_paths(text)
            areas = []
            if source.name == "build.rs" or "tests" in source.relative_to(directory).parts:
                areas.append((0, len(text), False))
            else:
                for match in re.finditer(r"\binclude(?:_str|_bytes)?\s*!\s*\(", mask):
                    areas.append((match.end(), balanced_end(mask, match.end()), True))
                for match in re.finditer(r"#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|(?:\w+::)?test\b)[^\]]*\]", mask):
                    start = mask.find("{", match.end())
                    if start >= 0:
                        areas.append((match.start(), balanced_end(mask, start + 1, "{", "}"), False))
            values = set()
            for start, end, concatenate in areas:
                parts = [value for first, last, value in literals if start <= first < end
                         and not re.search(r"\b(?:option_)?env!\s*\(\s*$", text[max(0, first - 80):first])]
                values.update(parts)
                if concatenate and len(parts) > 1:
                    values.add("".join(parts))
            for value in values:
                if "\x00" in value or not value:
                    continue
                candidates = [Path(os.path.abspath(base / value.lstrip("/")))
                              for base in (source.parent, directory, root)]
                if Path(value).is_absolute():
                    candidates.append(Path(os.path.abspath(value)))
                if wanted.intersection(candidates):
                    consumers.add(name)
                    break
            if name in consumers:
                break
    return consumers


def lean(range_spec, *, root=None):
    """Lean land plan: one line per touched crate with its test selection, then the
    dependents that only need to compile. A crate whose source changed runs its lib
    and bin tests; a changed integration-test file adds that test target."""
    root = Path(root or repository()).resolve()
    result = affected(range_spec, root=root)
    paths = {(root / p).resolve() for p in changed_paths(range_spec, cwd=root)}
    packages = metadata(root)
    lines = []
    for name in result["touched"]:
        package = packages[name]
        base = Path(package["manifest_path"]).parent.resolve()
        targets = package["targets"]
        tests = sorted(t["name"] for t in targets
                       if "test" in t["kind"] and Path(t["src_path"]).resolve() in paths)
        source = any(p.is_relative_to(base) and not p.is_relative_to(base / "tests")
                     for p in paths)
        selection = []
        if source or not tests:
            selection += ["--lib"] if any(set(t["kind"]) & {"lib", "proc-macro"} for t in targets) else []
            selection += ["--bins"] if any("bin" in t["kind"] for t in targets) else []
        for test in tests:
            selection += ["--test", test]
        if selection:
            lines.append(" ".join(["test", name, *selection]))
    check = sorted(set(result["affected"]) - set(result["touched"]))
    if check:
        lines.append(" ".join(["check", *check]))
    return lines


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
    broad = [p for p in paths if p in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
             or p.startswith(".cargo/")]
    # Validation tools are shared for retry decisions, but are not Rust source
    # dependencies. Their fixture suite and the fast invariants check them.
    tooling = [p for p in paths if (p.startswith("scripts/") and not p.startswith("scripts/e2e/"))
               or p.startswith("verification/") or p == "Makefile"]
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
    native = native_input_crates(root, paths, packages)
    touched.update(native)
    if native:
        tier = "code"
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
    ui = {"bboxy450-ui", "bboxy450-ui-core", "bboxy450-render", "bboxy450-book"}
    parity = any(p.startswith(("lab/", "design/")) for p in paths) or bool(touched & ui)
    lab_pages = affected_lab_pages(root, paths)
    parity |= bool(lab_pages)
    shared = sorted(set(broad) | set(tooling))
    return {"tier": tier, "touched": sorted(touched), "affected": sorted(result),
            "parity": "yes" if parity else "no", "paths": paths, "shared": shared,
            "lab_pages": lab_pages, "native_inputs": sorted(native)}


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


def checked(argv, root, *, env=None):
    print("presubmit:", shlex.join(map(str, argv)), flush=True)
    names = {Path(str(arg)).name for arg in argv}
    stage = ("formal" if list(argv)[:2] == ["make", "test-formal"] else
             "tests" if "quarantine-test.sh" in names else
             "fixtures" if names & {"test-landing.py", "test-resource-ledger.py"} else
             "invariants_misc")
    code = measured_command(list(map(str, argv)), root, _timing_path, stage=stage, env=env)
    if code:
        raise Refusal(f"check_failed ({code}): {argv[0]}", 1)


def ledger_origin(root):
    # Fixed slot checkouts are shared over time; attribution stays with the
    # invoking author's worktree and branch, never the slot's directory name.
    os.environ.setdefault("BBOXY_LEDGER_AGENT", "worktree:" + str(root.resolve()))
    os.environ.setdefault("BBOXY_LEDGER_BRANCH", git("branch", "--show-current", cwd=root))
    os.environ["BBOXY_LEDGER_SHA"] = git("rev-parse", "HEAD", cwd=root)


def ledger_module():
    # The optional observer is never a validation prerequisite (including old
    # private landing fixtures that deliberately omit the helper).
    import importlib.util
    path = Path(__file__).with_name("resource-ledger.py")
    if not path.exists():
        return None
    try:
        sys.dont_write_bytecode = True  # observation must leave the candidate clean
        spec = importlib.util.spec_from_file_location("bboxy_resource_ledger", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    except Exception as error:
        print(f"ledger: unavailable ({type(error).__name__}); continuing", file=sys.stderr)
        return None


def ledger_stage(argv):
    names = {Path(str(arg)).name for arg in argv}
    # These drivers meter their leaf stages, so do not double-count a parent.
    if names & {"invariants.sh", "ci-local.sh"}:
        return None
    if argv[0] == "cargo":
        return "unit" if len(argv) > 1 and argv[1] == "test" else "build"
    if "quarantine-test.sh" in names:
        return "unit"
    if names & {"test-landing.py", "test-resource-ledger.py"}:
        return "integration"
    if names & {"bboxy450-book", "capture-inputs.js"}:
        return "parity"
    return "smoke"


def measured_command(argv, root, timing_path, *, stream=None, env=None, stage=None):
    phase = None
    substep = None
    previous = None
    def switch(value):
        nonlocal phase, previous
        active = stage if stage != "formal" or substep else None
        state = (value, active, substep)
        if state != previous and timing_path is not None:
            line = (json.dumps({"at": time.monotonic(), "phase": value,
                                "stage": active, "substep": substep}) + "\n").encode()
            fd = os.open(timing_path, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
            try:
                os.write(fd, line)
            finally:
                os.close(fd)
        phase, previous = value, state
    cargo = argv[0] == "cargo" or any(Path(arg).name == "quarantine-test.sh" for arg in argv)
    fixtures = any(Path(arg).name in {"test-landing.py", "test-resource-ledger.py"} for arg in argv)
    switch("compile" if cargo else "test" if fixtures else "other")
    argv, env = cargo_invocation(argv, env)
    ledger = ledger_module()
    accounting_stage = ledger_stage(argv)
    accounting = ledger.begin(accounting_stage, root) if ledger is not None and accounting_stage else None
    wrapped = ledger.attempt(accounting, "wrap", argv) if ledger is not None else None
    options = ledger.attempt(accounting, "popen_options") if ledger is not None else None
    child = subprocess.Popen(wrapped or argv, cwd=root, env=env, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT, **(options or {}))
    if ledger is not None:
        ledger.attempt(accounting, "attach", child.pid)
    try:
        for line in child.stdout:
            clean = re.sub(r"\x1b\[[0-9;]*m", "", line)
            if stage == "formal" and clean.startswith(FORMAL_PHASE_PREFIX):
                event = json.loads(clean[len(FORMAL_PHASE_PREFIX):])
                if event.get("event") not in {"start", "end"} or not re.fullmatch(r"[\w.-]+", event.get("step", "")):
                    raise Refusal("invalid_formal_phase")
                substep = event["step"] if event["event"] == "start" else None
                switch("other")
            elif re.match(r"\s*(Compiling|Checking) ", clean):
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
        stage = substep = None
        switch("other")
        if ledger is not None:
            ledger.attempt(accounting, "finish", child.returncode)


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
        self.stage = self.substep = None
        self.stage_seconds = dict.fromkeys(STAGE_ALLOCATIONS, 0.)
        self.formal_step_seconds = {}

    def classified(self, elapsed):
        if self.phase != "compile" and self.stage in self.stage_seconds:
            self.stage_seconds[self.stage] += elapsed
            if self.stage == "formal" and self.substep:
                self.formal_step_seconds[self.substep] = self.formal_step_seconds.get(self.substep, 0.) + elapsed

    def event(self, value):
        at = max(self.at, value["at"])
        elapsed = at - self.at
        self.classified(elapsed)
        if self.phase == "compile":
            self.compile += elapsed
        else:
            self.run += elapsed
            if self.phase == "test":
                self.test += elapsed
        self.at, self.phase = at, value["phase"]
        self.stage, self.substep = value.get("stage"), value.get("substep")
        self.tests_started |= self.phase == "test"

    def totals(self, ended):
        tail = max(0., ended - self.at)
        build = self.compile + (tail if self.phase == "compile" else 0.)
        run = self.run + (tail if self.phase != "compile" else 0.)
        test = self.test + (tail if self.phase == "test" else 0.)
        stages, steps = dict(self.stage_seconds), dict(self.formal_step_seconds)
        if self.phase != "compile" and self.stage in stages:
            stages[self.stage] += tail
            if self.stage == "formal" and self.substep:
                steps[self.substep] = steps.get(self.substep, 0.) + tail
        return {"compile_seconds": build, "run_seconds": run, "test_seconds": test,
                "unclassified_seconds": max(0., run - sum(stages.values())),
                "stage_seconds": stages, "formal_step_seconds": steps,
                "tests_started": self.tests_started}

    def expired(self, ended, build_cap, run_bound):
        totals = self.totals(ended)
        if totals["compile_seconds"] >= build_cap:
            return "build_cap"
        if totals["run_seconds"] >= run_bound:
            return "presubmit_timeout"
        return None


def records_checks(root, paths, *, env=None):
    # Report only the path, never the matching bytes from a record.
    secret = re.compile(r"(?i)(?:\b(?:key|token|secret|password)\s*:\s*[\"']?[a-f0-9]{32,}"
                        r"|sk-[a-z0-9_-]{16,}|gh[pousr]_[a-z0-9]{20,}"
                        r"|-----BEGIN (?:RSA |EC |OPENSSH |PGP )?PRIVATE KEY)")
    tracked = set(git("ls-files", "-z", "--", *paths, cwd=root).split("\0")) if paths else set()
    for name in paths:
        # Inspect stored bytes rather than following a record symlink outside
        # this repository. Deleted paths have no new content to scan.
        if name in tracked and secret.search(git("show", "HEAD:" + name, cwd=root)):
            raise Refusal("records secret scan failed: " + name)
    checked(["cargo", "test", "-p", "bboxy450-doc", "--locked", "--test", "corpus", "-q"], root, env=env)
    checked(["cargo", "test", "-p", "bboxy450-mermaid", "--locked", "-q"], root, env=env)
    if set(paths) & {".bboxy/specs/e2e-acceptance.md", "lab/web/e2e.js"}:
        checked(["cargo", "test", "-p", "bboxy450-ui", "--locked", "--test", "e2e_checklist", "-q"], root, env=env)
    for script in ("layering", "pending-flags", "kill-own-pids", "generated-sources", "licenses", "brand",
                   "images", "cross-os", "never-block"):
        checked(["sh", f"scripts/{script}.sh"], root, env=env)


def command_tools(argv, root):
    """Repo-local script entry points, rather than every file under scripts/."""
    words = list(argv)
    while words and (words[0] == "env" or re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*=.*", words[0])):
        words.pop(0)
    if not words:
        return set()
    executable = Path(words[0]).name
    if executable in {"sh", "bash", "dash"} and "-c" in words:
        index = words.index("-c") + 1
        if index >= len(words):
            return set()
        lexer = shlex.shlex(words[index], posix=True, punctuation_chars=True)
        lexer.whitespace_split = True
        result, part = set(), []
        for token in [*lexer, ";"]:
            if token in {";", "&&", "||", "|", "(", ")"}:
                result |= command_tools(part, root)
                part = []
            else:
                part.append(token)
        return result
    if executable == "make" and "test-formal" in words:
        sources = set()
        coverage = root / "verification/coverage.toml"
        if coverage.is_file():
            sources.update(p for row in tomllib.loads(coverage.read_text())["crate"] for p in row["sources"])
        contracts = root / "verification/contracts.toml"
        if contracts.is_file():
            sources.update(p for row in tomllib.loads(contracts.read_text())["contract"]
                           for field in ["sources", "implementation_tests"] for p in row[field])
        return {"Makefile", "scripts/formal.py", "scripts/test-formal.py",
                "scripts/formal-stage.py", "verification/corpus.toml", "verification/coverage.toml", "verification/tools.lock.json",
                "crates/bboxy450-tlc/src/lib.rs", "crates/bboxy450-tlc/examples/verify.rs",
                "crates/bbt-yarn/src/model.rs", "crates/bbt-yarn/examples/verify.rs",
                "crates/bbt-yarn-macros/src/lib.rs", "verification/contracts.toml",
                "crates/bboxy450-term/tests/formal_contracts.rs",
                "crates/bboxy450-term/tests/formal_contracts/models.rs",
                "crates/bboxy450-term/tests/formal_contracts/witnesses.rs"} | sources
    if executable in {"sh", "bash", "dash", "python", "python3", "node"}:
        entries = [word for word in words[1:] if not word.startswith("-")]
        if not entries:
            return set()
        entry = Path(entries[0])
    else:
        entry = Path(words[0])
    if entry.is_absolute():
        try:
            entry = entry.relative_to(root)
        except ValueError:
            try:
                entry = entry.resolve().relative_to(root.resolve())
            except ValueError:
                return set()
    name = entry.as_posix()
    if (not name.startswith("scripts/") or ".." in entry.parts
            or entry.name in {"land.sh", "landing.py", "presubmit.sh", "cache-env.sh"}
            or entry.name.startswith(("test-landing", "landing_"))):
        return set()
    result = {name}
    delegate = DELEGATES.get(entry.stem)
    if delegate:
        result.add("scripts/" + delegate[0] + ".py")
    return result


def execution_commands(root, sha, transcript, *, fixtures=False):
    commands = {}
    invariants = ["sh", "scripts/invariants.sh"]
    def add(argv, tools=None, fixture=False):
        if tools is None:
            tools = command_tools(argv, root)
            if not fixtures:
                tools = {tool for tool in tools if not Path(tool).name.startswith("test-")}
        if tools or fixture:
            key = tuple(argv)
            row = commands.setdefault(key, {"argv": list(argv), "tools": []})
            row["tools"] = sorted(set(row["tools"]) | tools)
            if fixture:
                row["fixtures"] = True
    for line in transcript.splitlines():
        argv = None
        try:
            if line.startswith("presubmit: "):
                argv = shlex.split(line.removeprefix("presubmit: "))
            elif line.startswith("ci-command: "):
                argv = json.loads(line.removeprefix("ci-command: "))
        except (ValueError, TypeError):
            continue
        if isinstance(argv, list) and argv and all(isinstance(arg, str) for arg in argv):
            add(argv, fixture=fixtures and any(arg.startswith("scripts/test-") for arg in argv))
            if "scripts/invariants.sh" in argv:
                invariants = argv
        match = re.fullmatch(r"invariant ([A-Za-z0-9_-]+): pass", line)
        if not match:
            continue
        gate = match[1]
        source = command(["git", "show", f"{sha}:.bboxy/invariants/{gate}.md"], cwd=root, check=False)
        if source.returncode:
            continue
        lines = source.stdout.splitlines()
        if not lines or lines[0] != "---":
            continue
        for field in lines[1:]:
            if field == "---":
                break
            if field.startswith("check:"):
                check = field.removeprefix("check:").strip()
                if len(check) >= 2 and check[0] == check[-1] and check[0] in "\"'":
                    check = check[1:-1]
                try:
                    tools = command_tools(["sh", "-c", check], root)
                except ValueError:
                    tools = set()
                add([*invariants, *([] if gate in invariants else [gate])], tools)
                break
    return list(commands.values())


def parity_cases(cases, data, root=None):
    paths = data["paths"]
    page_scope = set(data["parity_pages"]) if "parity_pages" in data else None
    all_pages = {catalogue_page(case) for case in cases}
    if data["tier"] == "lab" or data.get("lab_pages"):
        pages = set(data["lab_pages"] if "lab_pages" in data else affected_lab_pages(root or repository(), paths))
        if page_scope is not None:
            pages &= page_scope
        selected = [case for case in cases if catalogue_page(case) in pages]
        if len(pages & all_pages) > len(all_pages) / 2:
            first = {}
            for case in sorted(selected, key=lambda case: json.dumps(case, sort_keys=True)):
                first.setdefault(catalogue_page(case), case)
            selected = [first[page] for page in sorted(first)]
        # A lab dependency must not hide the other half of a mixed cut. Native
        # component edits retain their own selection, including all its cases.
        native_paths = [p for p in paths if p.startswith(("crates/bbt-ui/", "crates/bbt-ui-model/",
                                                        "crates/bbt-render/", "crates/bboxybook/", ".cargo/"))
                        or p in ("Cargo.toml", "Cargo.lock")]
        if native_paths:
            selected += parity_cases(cases, {**data, "tier": "code", "paths": native_paths,
                                            "lab_pages": [], "shared": [p for p in data["shared"]
                                                                         if p in native_paths]}, root)
            unique = {json.dumps(case, sort_keys=True): case for case in selected}
            selected = [unique[key] for key in sorted(unique)]
        return selected
    components = {re.sub(r"[^a-z0-9]", "", Path(p).stem.lower()) for p in paths
                  if p.startswith(("crates/bbt-ui/src/modules/", "crates/bboxybook/src/"))}
    selected = [case for case in cases
                if re.sub(r"[^a-z0-9]", "", case["component"].lower()) in components]
    if selected and not data["shared"]:
        if page_scope is not None:
            selected = [case for case in selected if catalogue_page(case) in page_scope]
        return selected
    if page_scope is not None:
        selected = [case for case in cases if catalogue_page(case) in page_scope]
        if len(page_scope & all_pages) <= len(all_pages) / 2:
            return selected
    else:
        selected = cases
    # A shared input touches every page: a fixed first case per page, not the
    # full catalogue. Postsubmit always compares the complete catalogue.
    first = {}
    for case in sorted(selected, key=lambda case: json.dumps(case, sort_keys=True)):
        first.setdefault(catalogue_page(case), case)
    return [first[page] for page in sorted(first)]


def scoped_parity(root, data, output):
    checked(["sh", "scripts/lab-check.sh"], root)
    pointer = state_directory() / "parity-last-green"
    stored = Path(pointer.read_text().strip()).resolve() if pointer.exists() else None
    if stored is not None and not stored.is_relative_to(parity_directory().resolve()):
        raise Refusal("parity_pointer_outside_cache")
    if data["tier"] == "lab" and stored is not None:
        binary = stored / "bboxy450-book"
        if not binary.is_file() or not (stored / "cases.json").is_file():
            raise Refusal("parity_store_incomplete")
        cases = json.loads((stored / "cases.json").read_text())
    else:
        checked(["cargo", "build", "-p", "bboxy450-book", "--locked"], root)
        target = Path(os.environ.get("BBOXY_CARGO_TARGET_DIR") or os.environ.get("CARGO_TARGET_DIR") or root / "target")
        binary = target.resolve() / "debug/bboxy450-book"
        cases = json.loads(command([str(binary), "manifest"], cwd=root).stdout)
    cases = parity_cases(cases, data)
    if "parity_pages" in data and data["parity_pages"]:
        cases = [case for case in cases if catalogue_page(case) in data["parity_pages"]]
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


def parity_recheck_pages(data, moved):
    if not moved:
        return []
    cut, incoming = set(data.get("lab_pages", [])), set(moved.get("lab_pages", []))
    pages = cut & incoming
    ui = {"bboxy450-ui", "bboxy450-ui-core", "bboxy450-render", "bboxy450-book"}
    if set(data["affected"]) & set(moved["affected"]) & ui:
        return []  # The Rust intersection needs the cut's original parity scope.
    if set(moved["affected"]) & ui:
        pages |= cut
    if set(data["affected"]) & ui:
        pages |= incoming
    return sorted(pages)


def recheck_data(data, path):
    if path is None:
        return data
    plan = json.loads(Path(path).read_text())
    if not isinstance(plan, dict):
        raise Refusal("invalid_recheck_scope", 2)
    selected, commands, pages = plan.get("affected"), plan.get("commands", []), plan.get("parity_pages", [])
    if (not isinstance(selected, list) or not all(isinstance(name, str) for name in selected)
            or not set(selected) <= set(data["affected"])
            or not isinstance(commands, list)
            or any(not isinstance(argv, list) or not argv
                   or not all(isinstance(arg, str) for arg in argv) for argv in commands)
            or not isinstance(pages, list)
            or any(not isinstance(page, str) or not re.fullmatch(r"lab/web/[^/]+\.html", page)
                   for page in pages)):
        raise Refusal("invalid_recheck_scope", 2)
    parity = data["parity"] == "yes" and (bool(pages) or bool(set(selected) &
                        {"bboxy450-ui", "bboxy450-ui-core", "bboxy450-render", "bboxy450-book"}))
    scope = {"affected": sorted(set(selected)), "commands": commands}
    if pages:
        scope["parity_pages"] = sorted(set(pages))
    return {**data, "tier": "lab" if data["tier"] == "lab" else "code",
            "affected": sorted(set(selected)), "parity": "yes" if parity else "no",
            **({"parity_pages": sorted(set(pages))} if pages else {}), "recheck": scope}


def presubmit_worker(base, result_path, recheck=None):
    global _timing_path
    os.environ["SIGNED_RANGE"] = base + "..HEAD"
    _timing_path = result_path.parent / "phases.jsonl"
    root = repository()
    data = recheck_data(affected(base + "..HEAD", root=root), recheck)
    atomic_write(result_path, json.dumps(data) + "\n")
    coverage_path = result_path.parent / "quarantine-coverage.json"
    os.environ["BBOXY_PRESUBMIT_COVERAGE"] = str(coverage_path)
    os.environ["BBOXY_PRESUBMIT_ATTEMPT"] = result_path.parent.name
    checked(["make", "test-formal", "FORMAL_REUSE=1"], root)
    if data["tier"] == "lab":
        if recheck is None:
            records_checks(root, data["paths"])
            checked(["sh", "scripts/invariants.sh", "--no-cargo"], root)
    else:
        if any(p.startswith(".bboxy/") and p.endswith(".md") for p in data["paths"]):
            checked(["cargo", "test", "-p", "bboxy450-doc", "--locked", "--test", "corpus", "-q"], root)
        packages = [arg for name in data["affected"] for arg in ("-p", name)]
        if packages:
            checked(["sh", "scripts/quarantine-test.sh", "--locked", *packages], root)
            checked(["cargo", "clippy", "--locked", *packages, "--all-targets", "--", "-D", "warnings"], root)
        if recheck is None and landing_machinery(data["paths"], root):
            checked(["python3", "-I", "scripts/test-landing.py", "--all"], root)
            if (root / "scripts/test-resource-ledger.py").exists():
                checked(["python3", "-I", "scripts/test-resource-ledger.py"], root)
        if packages or recheck is None:
            checked(["sh", "scripts/invariants.sh", "--affected", " ".join(data["affected"]),
                     "--covered-tests", str(coverage_path)], root)
        dispatch = any(Path(p).name in ("ctl.rs", "seat_socket.rs", "logic.rs")
                       or "/src/ctl/" in p or "/src/mcp/" in p for p in data["paths"])
        if dispatch and packages and (recheck is None or "bboxy450-term-proto" in data["affected"]):
            # The wire golden is a target in the protocol crate; its test names
            # deliberately contain no "golden" name filter.
            output = io.StringIO()
            code = measured_command(["cargo", "test", "--locked", "-p", "bboxy450-term-proto",
                                     "--test", "gui_wire_golden"],
                                    root, _timing_path, stream=output, stage="invariants_misc")
            print(output.getvalue(), end="", flush=True)
            if code or not re.search(r"test result: ok\. [1-9][0-9]* passed", output.getvalue()):
                raise Refusal("golden_dispatch_tests_missing")
    for argv in data.get("recheck", {}).get("commands", []):
        if argv not in (["make", "test-formal"], ["make", "test-formal", "FORMAL_REUSE=1"]):
            checked(argv, root)
    if data["parity"] == "yes":
        scoped_parity(root, data, result_path.parent / "parity")
    # A cut without a Report trailer satisfies the optional link contract without
    # adding a Cargo gate to script/lab-only or disjoint incremental checks.
    messages = git("log", "--format=%B", base + "..HEAD", cwd=root)
    if any(line.startswith("Report:") for line in messages.splitlines()):
        checked(["cargo", "test", "-p", "bboxy450-doc", "--locked", "--test", "corpus", "reports::committed_report_trailers_resolve_in_their_own_trees", "--", "--exact"], root)
    return 0


def clean_source(root):
    initializer = root / "scripts/harness-submodule.py"
    if initializer.is_file():
        checked([sys.executable, "-I", str(initializer)], root)
    if git("status", "--porcelain", "--untracked-files=no", cwd=root):
        raise Refusal("dirty_tracked_tree")
    untracked = git("ls-files", "--others", "--exclude-standard", "-z", cwd=root).split("\0")
    source = [p for p in untracked if p.startswith(("crates/", "scripts/", "verification/", "lab/", "design/", ".cargo/"))
              or p in ("Cargo.toml", "Cargo.lock", "build.rs")]
    if source:
        raise Refusal("untracked_source: " + ", ".join(source))


def presubmit(base, recheck=None):
    root = repository()
    ledger_origin(root)
    sha = git("rev-parse", "HEAD", cwd=root)
    clean_source(root)
    data = recheck_data(affected(base + "..HEAD", root=root), recheck)
    if data["tier"] == "wide":
        with Slot() as slot:
            return slot.result(wide_check(root, data)["exit"])
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
                                   environment=check_environment, recheck=recheck)
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


def presubmit_attempt(root, sha, base, data, build_cap, run_bound, attempt, operation, *, environment=None,
                      recheck=None):
    output = state_directory() / "presubmits" / (sha + "-" + uuid.uuid4().hex[:8])
    output.mkdir(parents=True, mode=0o700)
    result_path = output / "affected.json"
    started = time.monotonic()
    load_start = load_average()
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
                                  base, str(result_path), *(["--recheck", str(recheck)] if recheck else [])], cwd=root,
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
            read_events()
            ended = time.monotonic()
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
    record.update(load_average_start=load_start, load_average_end=load_average(),
                  stage_allocations={**STAGE_ALLOCATIONS, "margin": STAGE_MARGIN})
    record["findings"] = budget_findings(record, load_start, record["load_average_end"])
    if code == 0:
        record["formal"] = formal_receipt(transcript, sha, root)
        record["commands"] = execution_commands(root, sha, transcript,
                                                 fixtures=landing_machinery(data["paths"], root))
    if "recheck" in data:
        record["recheck"] = data["recheck"]
    record["result"] = str(output / "result.json")
    atomic_write(output / "result.json", json.dumps(record, sort_keys=True) + "\n")
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


def verify_formal_receipt(receipt, sha, root=None):
    try:
        if receipt["source"] != sha:
            raise ValueError("formal receipt source differs from candidate")
        spec = importlib.util.spec_from_file_location("landing_formal", Path(__file__).with_name("formal.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        current = module.evidence_receipt(Path(receipt["evidence"]).parent,
                                          root=Path(root or receipt["checkout"]))
        # An immutable checking worktree may differ from the author's worktree.
        # Verify its evidence against the current candidate's source, not its old path.
        if {k: v for k, v in receipt.items() if k != "checkout"} != {
                k: v for k, v in current.items() if k != "checkout"}:
            raise ValueError("formal receipt differs from evidence")
        return receipt
    except (KeyError, TypeError, ValueError, OSError, subprocess.SubprocessError) as error:
        raise Refusal("formal_evidence_invalid: " + str(error)) from error


def formal_receipt(transcript, sha, root=None):
    rows = [line.removeprefix("formal: receipt ") for line in transcript.splitlines()
            if line.startswith("formal: receipt ")]
    if len(rows) != 1:
        raise Refusal("formal_receipt_missing_or_duplicate")
    try:
        receipt = json.loads(rows[0])
    except ValueError as error:
        raise Refusal("formal_receipt_invalid_json") from error
    return verify_formal_receipt(receipt, sha, root)


def refresh_formal(root, sha):
    """A disjoint Rust recheck still needs formal evidence for its new commit."""
    result = command(["make", "test-formal", "FORMAL_REUSE=1"], cwd=root, check=False)
    log = state_directory() / "formal" / (sha + "-" + uuid.uuid4().hex[:8] + ".log")
    atomic_write(log, result.stdout + result.stderr)
    print(result.stdout + result.stderr, end="", flush=True)
    if result.returncode:
        raise Refusal("formal_recheck_failed: " + str(log), 77)
    return formal_receipt(result.stdout, sha, root)


def records_gate_delta(root, source, sha):
    """R5 compares trees, including renames and modes, never commit subjects."""
    different = command(["git", "diff", "--quiet", "--ignore-submodules=none", source, sha, "--", ".", ":!.bboxy"],
                        cwd=root, check=False)
    if different.returncode:
        raise Refusal("gate_log_source_mismatch: non-records tree delta")
    if command(["git", "verify-commit", sha], cwd=root, check=False).returncode:
        raise Refusal("gate_log_source_mismatch: unsigned rebased candidate")
    delta = git("diff", "--no-renames", "--name-only", "-z", source, sha, "--", ".bboxy/", cwd=root)
    paths = sorted(set(delta.split("\0")) - {""})
    for name in paths:
        # invariants.sh globs this directory and executes each record's check.
        if name == ".bboxy/invariants" or name.startswith(".bboxy/invariants/"):
            raise Refusal("gate_log_source_mismatch: executable gate record: " + name)
        named = command(["git", "grep", "-F", "-l", "-e", name, sha, "--",
                         "crates", "scripts", "Makefile", "verification", "lab"], cwd=root, check=False)
        if named.returncode != 1:
            raise Refusal("gate_log_source_mismatch: record is named by code: " + name)
    return paths


def gate_proof(path, sha, root=None):
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
    source, delta = sha, None
    if sources != [[sha, "normal"]]:
        if (len(sources) != 1 or len(sources[0]) != 2 or sources[0][1] != "normal"
                or not re.fullmatch(r"[a-f0-9]{40}", sources[0][0]) or root is None):
            raise Refusal("gate_log_source_mismatch: expected exact candidate " + sha)
        source = sources[0][0]
        delta = records_gate_delta(root, source, sha)
    if (results != [[source, "normal", "0"]] or not passed
            or last != f"ci-local: result {source} normal 0"):
        raise Refusal("gate_log_not_green: full normal gate completion required")
    proof = {"gate_log": str(path), "gate_log_sha256": digest.hexdigest(), "tested_sha": sha}
    if delta is None:
        proof["formal"] = formal_receipt(path.read_text(), sha, root)
    else:
        try:
            rows = [json.loads(line.removeprefix("formal: receipt ")) for line in path.read_text().splitlines()
                    if line.startswith("formal: receipt ")]
            if len(rows) != 1 or rows[0]["source"] != source:
                raise ValueError("formal receipt does not identify tested source")
            spec = importlib.util.spec_from_file_location("records_formal", Path(__file__).with_name("formal.py"))
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            # Validate the original receipt and all unchanged inputs before any gates.
            current = module.evidence_receipt(Path(rows[0]["evidence"]).parent, Path(root), expected_source=source)
            if {k: v for k, v in current.items() if k != "checkout"} != {
                    k: v for k, v in rows[0].items() if k != "checkout"}:
                raise ValueError("formal receipt differs from evidence")
            check_root, check_environment = candidate_environment(root, sha)
            records_checks(check_root, delta, env=check_environment)
            proof["formal"] = module.records_rebase_receipt(rows[0], root, delta)
        except (Refusal, ValueError, KeyError, TypeError, OSError, subprocess.SubprocessError) as error:
            raise Refusal("gate_log_source_mismatch: records rebase checks failed: " + str(error)) from error
        proof.update(tested_sha=source, gate_log_source=source, records_delta=delta, records_checks_exit=0)
    return proof


def wide_check(root, data, gate_log=None, *, canonical=False):
    sha = git("rev-parse", "HEAD", cwd=root)
    started = time.monotonic()
    if gate_log is None:
        output = state_directory() / "presubmits" / (sha + "-" + uuid.uuid4().hex[:8])
        output.mkdir(parents=True, mode=0o700)
        gate_log = output / "commands.log"
        check_root, environment = candidate_environment(root, sha)
        environment["BBOXY_GATE_SLOT"] = "none"  # land/presubmit already owns it
        print("land: full normal CI, no presubmit deadline", flush=True)
        argv = (["make", "test", "CI_ARGS=", "TARGET_DIR=" + environment["BBOXY_CARGO_TARGET_DIR"],
                 "CI_LOG=" + str(gate_log)] if canonical else ["sh", "scripts/ci-local.sh"])
        with (output / "make.log" if canonical else gate_log).open("w") as stream:
            code = measured_command(argv, check_root, output / "phases.jsonl",
                                    stream=stream, env=environment)
        if not gate_log.exists():
            gate_log.write_text((output / "make.log").read_text())
        print(gate_log.read_text(), end="", flush=True)
        proof = gate_proof(gate_log, sha, root) if code == 0 else {"gate_log": str(gate_log)}
    else:
        proof = gate_proof(gate_log, sha, root)
        code = 0
        if "gate_log_source" in proof:
            print("land: reusing full normal gate from " + proof["gate_log_source"]
                  + " after checked records rebase to " + sha, flush=True)
        else:
            print("land: reusing full normal gate on exact candidate " + sha, flush=True)
    if git("rev-parse", "HEAD", cwd=root) != sha:
        raise Refusal("candidate_changed_during_gate")
    clean_source(root)
    record = {"sha": sha, "tier": data["tier"], "affected": data["affected"],
              "seconds": time.monotonic() - started, "verdict": "green" if code == 0 else "red",
              "exit": code, "binary_durations": binary_durations(Path(gate_log).read_text()),
              **proof}
    if code == 0:
        record["commands"] = execution_commands(root, sha, Path(gate_log).read_text(),
                                                 fixtures=landing_machinery(data["paths"], root))
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


def release_slot(path, root):
    # Only ownership is released. The checkout and its Cargo target stay warm, so the
    # next landing rebuilds only what changed since this slot's last candidate.
    if any(p.name not in {"pid", "owner.json", "tree", "target"} for p in path.iterdir()):
        raise Refusal("slot_has_unknown_files: " + str(path))
    # A foreign or symlinked tree/target is refused loudly by candidate_environment.
    (path / "pid").unlink(missing_ok=True)
    (path / "owner.json").unlink(missing_ok=True)


def slot_limit():
    """One gate at a time by default: a queued gate at full width beats three thrashing."""
    try:
        wanted = int(os.environ.get("BBOXY_GATE_SLOTS", "1"))
    except ValueError:
        raise Refusal("BBOXY_GATE_SLOTS must be 1, 2 or 3")
    if wanted not in (1, 2, 3):
        raise Refusal("BBOXY_GATE_SLOTS must be 1, 2 or 3")
    return min(wanted, 3 if shutil.disk_usage(Path.home()).free >= 25 * 1024 ** 3 else 2)


def gate_jobs():
    """Cargo jobs for the gate slot; agents' own builds keep two."""
    try:
        return max(2, int(os.environ.get("BBOXY_GATE_JOBS") or 4))
    except ValueError:
        raise Refusal("BBOXY_GATE_JOBS must be an integer")


def slot_notice(text):
    # Capacity notices stay local; retired messaging tools are never invoked.
    try:
        append_record("notices.log", {"timestamp": now(), "text": text})
    except OSError:
        pass


class Slot:
    def __init__(self, label="land", preferred=None):
        self.path = None
        self.number = None
        self.token = uuid.uuid4().hex
        self.root = repository()
        self.sha = git("rev-parse", "HEAD", cwd=self.root)
        self.return_code = 0
        self.release_notified = False
        self.label, self.preferred = label, preferred
        self.previous_environment = {}

    def __enter__(self):
        waiting = False
        while True:
            try:
                return self.acquire()
            except Refusal as refusal:
                if refusal.code != 75 or os.environ.get("BBOXY_GATE_WAIT") == "0":
                    raise
                if not waiting:
                    waiting = True
                    print(f"{self.label}: {refusal}; queued, waiting for a gate slot", flush=True)
                time.sleep(5)

    def acquire(self):
        directory = slot_directory()
        directory.mkdir(parents=True, exist_ok=True)
        limit = slot_limit()
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
                if (path / "pid").exists():
                    try:
                        pid = int((path / "pid").read_text())
                        if pid <= 0:
                            continue
                        os.kill(pid, 0)
                        continue
                    except ProcessLookupError:
                        pass
                    except (ValueError, OSError):
                        continue
                if path.exists():
                    try:
                        release_slot(path, self.root)
                    except (OSError, Refusal):
                        continue
                path.mkdir(exist_ok=True)
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
            slot_notice(f"SLOT {number} TAKEN {os.getpid()} {self.sha}")
            print(f"{self.label}: {'gate ' if self.label == 'ci-local' else ''}slot {number} of {limit}", flush=True)
            return self
        raise Refusal(f"all {limit} slots are taken", 75)

    def result(self, code):
        self.return_code = code
        return code

    def __exit__(self, kind, error, traceback):
        code = self.return_code if error is None else getattr(error, "code", 130 if isinstance(error, KeyboardInterrupt) else 1)
        if not isinstance(code, int):
            code = 1
        try:
            if self.path is not None:
                with slot_guard(self.path, blocking=True):
                    data = json.loads((self.path / "owner.json").read_text())
                    if data.get("token") == self.token and data.get("pid") == os.getpid():
                        release_slot(self.path, self.root)
        except BaseException as cleanup_error:
            code = getattr(cleanup_error, "code", 1)
            raise
        finally:
            if self.path is not None and not self.release_notified:
                self.release_notified = True
                slot_notice(f"SLOT {self.number} RELEASED {os.getpid()} {code}")
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
        git("worktree", "prune", cwd=root)  # /tmp may have dropped a registered tree
        git("worktree", "add", "--detach", str(tree), sha, cwd=root)
    if target.is_symlink():
        raise Refusal("slot_target_is_symlink")
    target.mkdir(exist_ok=True)  # warm: same path, so Cargo rebuilds only changed crates
    initializer = tree / "scripts/harness-submodule.py"
    if initializer.is_file():
        checked([sys.executable, "-I", str(initializer)], tree)
    environment = build_environment(tree, target=target)
    # Intermediates live in the slot's own target, never a worktree's private build dir.
    environment.pop("CARGO_BUILD_BUILD_DIR", None)
    jobs = str(gate_jobs())
    environment.update(CARGO_BUILD_JOBS=jobs, BBOXY_CARGO_JOBS=jobs)
    return tree, environment


def ci_local(arguments):
    if any(arg not in {"--parity", "--slow", "--after-tests"} for arg in arguments):
        raise Refusal("unknown ci-local flag")
    root = repository()
    ledger_origin(root)
    clean_source(root)
    sha = git("rev-parse", "HEAD", cwd=root)
    with Slot("ci-local") as slot:
        tree, environment = candidate_environment(root, sha)
        environment["BBOXY_GATE_SLOT"] = "none"
        return slot.result(command(["sh", "scripts/ci-local.sh", *arguments], cwd=tree,
                                   env=environment, capture=False, check=False).returncode)


def announcement(sha, subject):
    receipt = state_directory() / "announced" / (sha + ".json")
    if receipt.exists():
        return
    text = f"LANDED {sha} {subject[:180]}"
    atomic_write(receipt, json.dumps({"sha": sha, "subject": subject, "timestamp": now()}) + "\n")
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
    if actor not in {"bboxy450", "Gonzih"}:
        raise Refusal("unapproved_github_identity: " + actor)


def land(gate_log=None, full=False):
    root = repository()
    ledger_origin(root)
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
    incremental_rechecks = 0
    executed_commands = {}
    with Slot():
        retry = -1
        while True:
            retry += 1
            git("fetch", "origin", "main", cwd=root)
            base = git("rev-parse", "origin/main", cwd=root)
            main_guard(root, base)
            commits, email, signing_key = own_signatures(root, base)
            if not commits:
                sha = git("rev-parse", "HEAD", cwd=root)
                existing = next((r for r in records("landings.log") if r.get("sha") == sha
                                 and r.get("author_key") == signing_key), None)
                if existing:
                    landing = existing
                    break
                raise Refusal("no_own_commits_to_land")
            rebased = command(["git", "rebase", "origin/main"], cwd=root, capture=False, check=False)
            if rebased.returncode:
                raise Refusal("conflict: resolve this rebase before rerunning", 78)
            data = affected(base + "..HEAD", root=root)
            moved = affected(previous_base + ".." + base, root=root) if previous_base else None
            common_paths = moved and set(data["paths"]) & set(moved["paths"])
            parity_pages = parity_recheck_pages(data, moved)
            # Each affected set already includes transitive dependents; their
            # intersection therefore includes those dependents within this cut.
            intersection = sorted(set(data["affected"]) & set(moved["affected"])) if moved else []
            changed_tools = set(moved["paths"]) if moved else set()
            scripts_moved = any(path.startswith("scripts/") for path in changed_tools)
            script_cut = any(path.startswith("scripts/") for path in data["paths"])
            replay = [row["argv"] for row in executed_commands.values()
                      if changed_tools & set(row["tools"])
                      or (scripts_moved and script_cut and row.get("fixtures"))]
            recheck = None
            if tested is not None and not common_paths and (intersection or replay or parity_pages):
                if incremental_rechecks >= 5:
                    raise Refusal("main_moving: five incremental rechecks exhausted; finding for Oracle", 79)
                incremental_rechecks += 1
                recheck = state_directory() / "rechecks" / (uuid.uuid4().hex + ".json")
                plan = {"affected": intersection, "commands": replay}
                if parity_pages:
                    plan["parity_pages"] = parity_pages
                atomic_write(recheck, json.dumps(plan) + "\n")
                print("land: incremental recheck:", " ".join(intersection), flush=True)
            candidate = git("rev-parse", "HEAD", cwd=root)
            if tested is None or common_paths or recheck is not None or gate_log:
                started = time.monotonic()
                if gate_log or (recheck is None and (full or data["tier"] == "wide")):
                    proof = wide_check(root, data, gate_log, canonical=full)
                    code = proof["exit"]
                else:
                    code = command(["sh", "scripts/presubmit.sh", base,
                                    *(["--recheck", str(recheck)] if recheck else [])], cwd=root,
                                   capture=False, check=False).returncode
                    proof = next((row for row in reversed(records("presubmit.log"))
                                  if row.get("sha") == candidate), {})
                    if proof.get("operation"):
                        attempts = [row for row in records("presubmit.log")
                                    if row.get("operation") == proof["operation"]]
                        for field in ("compile_seconds", "run_seconds", "test_seconds", "unclassified_seconds"):
                            proof[field] = sum(row[field] for row in attempts)
                        for field in ("stage_seconds", "formal_step_seconds"):
                            proof[field] = {key: sum(row.get(field, {}).get(key, 0.) for row in attempts)
                                            for key in {key for row in attempts for key in row.get(field, {})}}
                        proof["findings"] = [finding for row in attempts for finding in row.get("findings", [])]
                        proof["load_average_start"] = attempts[0].get("load_average_start")
                        proof["load_average_end"] = attempts[-1].get("load_average_end")
                        proof["build_attempts"] = len(attempts)
                spent += time.monotonic() - started
                check_timings.append({k: proof.get(k) for k in
                                      ("sha", "affected", "recheck", "log", "gate_log", "gate_log_sha256", "compile_seconds", "run_seconds",
                                       "test_seconds", "unclassified_seconds", "timing_basis", "stage_seconds",
                                       "formal_step_seconds", "load_average_start", "load_average_end", "findings")})
                if code:
                    raise Refusal("presubmit_failed", 77)
                if tested is None or common_paths:
                    executed_commands.clear()
                for row in proof.get("commands", []):
                    command_key = tuple(row["argv"])
                    prior = executed_commands.get(command_key, {"tools": []})
                    executed_commands[command_key] = {"argv": row["argv"],
                                              "tools": sorted(set(prior["tools"]) | set(row["tools"]))}
                    if row.get("fixtures") or prior.get("fixtures"):
                        executed_commands[command_key]["fixtures"] = True
                tested = git("rev-parse", "HEAD", cwd=root)
            else:
                print("land: moved changes are disjoint; reusing presubmit", flush=True)
                proof["formal"] = refresh_formal(root, candidate)
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
            formal = verify_formal_receipt(proof["formal"], sha, root)
            pushed = command(["git", "push", "origin", "HEAD:main"], cwd=root, check=False)
            if pushed.returncode:
                git("fetch", "origin", "main", cwd=root)
                if not ancestor(sha, "origin/main", root):
                    if git("rev-parse", "origin/main", cwd=root) == base:
                        raise Refusal("push_failed: " + pushed.stderr)
                    continue
            landing = {"sha": sha, "base": base, "timestamp": now(), "author_key": signing_key, "author": email,
                       "tier": data["tier"], "affected": data["affected"],
                       "presubmit_seconds": spent, "retries": retry, "paths": data["paths"],
                       "tested_sha": proof.get("gate_log_source", tested), "branch": branch,
                       "incremental_rechecks": incremental_rechecks}
            landing["formal"] = formal
            landing.update({k: proof[k] for k in ("gate_log", "gate_log_sha256", "gate_log_source", "records_delta", "records_checks_exit", "compile_seconds",
                                                 "run_seconds", "test_seconds", "unclassified_seconds", "build_attempts",
                                                 "tests_started", "timing_basis", "stage_seconds", "formal_step_seconds",
                                                 "load_average_start", "load_average_end", "findings") if k in proof})
            landing["check_timings"] = check_timings
            landing["commands"] = list(executed_commands.values())
            if len(check_timings) > 1:
                landing["timing_basis"] = "sum_of_check_timings"
            for kind in ("compile_seconds", "run_seconds", "test_seconds", "unclassified_seconds"):
                landing[kind] = (sum(row[kind] for row in check_timings)
                                 if all(row[kind] is not None for row in check_timings) else None)
            for kind in ("stage_seconds", "formal_step_seconds"):
                landing[kind] = {key: sum((row.get(kind) or {}).get(key, 0.) for row in check_timings)
                                 for key in {key for row in check_timings for key in (row.get(kind) or {})}}
            landing["findings"] = [finding for row in check_timings for finding in (row.get("findings") or [])]
            append_record("landings.log", landing)
            ledger = ledger_module()
            if ledger is not None:
                ledger.attempt(ledger, "landed", landing)
            break
        if landing is None:
            raise Refusal("main_moving: five incremental rechecks exhausted; finding for Oracle", 79)
    # Communication happens after slot release; it never holds build capacity.
    try:
        spec = importlib.util.spec_from_file_location("prune_formal", Path(__file__).with_name("formal.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.schedule_prune(root)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"land: evidence maintenance unavailable: {error}", file=sys.stderr)
    announcement(landing["sha"], git("show", "-s", "--format=%s", landing["sha"], cwd=root))
    return 0


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "qos-cargo-exec":
        return qos_cargo_exec(sys.argv[2:])
    if len(sys.argv) > 1 and sys.argv[1] == "qos-cargo":
        print(cargo_qos(sys.argv[2:]))
        return 0
    if len(sys.argv) > 1 and sys.argv[1] == "cost":
        return cost_command(sys.argv[2:])
    if len(sys.argv) > 1 and sys.argv[1] == "ci-local":
        return ci_local(sys.argv[2:])
    if len(sys.argv) > 1 and sys.argv[1] in DELEGATES:
        module_name, function = DELEGATES[sys.argv[1]]
        spec = importlib.util.spec_from_file_location(module_name, Path(__file__).with_name(module_name + ".py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return getattr(module, function)(sys.argv[2:], sys.modules[__name__])
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="verb", required=True)
    p = sub.add_parser("affected")
    p.add_argument("range", nargs="?", default="origin/main..HEAD")
    p.add_argument("--crates", action="store_true")
    p = sub.add_parser("lean")
    p.add_argument("range", nargs="?", default="origin/main..HEAD")
    sub.add_parser("state-dir")
    sub.add_parser("parity-dir")
    p = sub.add_parser("check-is-cargo")
    p.add_argument("check")
    p = sub.add_parser("trace-command")
    p.add_argument("command", nargs=argparse.REMAINDER)
    p = sub.add_parser("presubmit")
    p.add_argument("base", nargs="?", default="origin/main")
    p.add_argument("--recheck", type=Path)
    p = sub.add_parser("presubmit-worker")
    p.add_argument("base")
    p.add_argument("result", type=Path)
    p.add_argument("--recheck", type=Path)
    p = sub.add_parser("land")
    proof = p.add_mutually_exclusive_group()
    proof.add_argument("--gate-log", type=Path,
                       help="reuse full NORMAL CI on the candidate SHA or a checked records-only rebase")
    proof.add_argument("--full", action="store_true",
                       help="run make test after rebasing, for any tier; retain incremental rechecks")
    args = parser.parse_args()
    if args.verb == "affected":
        result = affected(args.range)
        print(" ".join(result["affected"]) if args.crates else json.dumps(result))
    elif args.verb == "lean":
        print("\n".join(lean(args.range)))
    elif args.verb == "state-dir":
        print(state_directory())
    elif args.verb == "parity-dir":
        print(parity_directory())
    elif args.verb == "check-is-cargo":
        return 0 if cargo_backed(args.check) else 1
    elif args.verb == "trace-command":
        argv = args.command[1:] if args.command[:1] == ["--"] else args.command
        if not argv:
            raise Refusal("empty_trace_command", 2)
        print("ci-command: " + json.dumps(argv), flush=True)
    elif args.verb == "presubmit":
        return presubmit(args.base, args.recheck)
    elif args.verb == "presubmit-worker":
        return presubmit_worker(args.base, args.result, args.recheck)
    elif args.verb == "land":
        return land(args.gate_log, args.full)


if __name__ == "__main__":
    try:
        raise SystemExit(main() or 0)
    except (Refusal, OSError, ValueError) as error:
        print(f"landing: {error}", file=sys.stderr)
        raise SystemExit(error.code if isinstance(error, Refusal) else 2)
