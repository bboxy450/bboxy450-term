"""Postsubmit diagnosis, corrective landings and nightly scale evidence (#323).

Mutable state belongs to landing.py. These commands never check out the caller's
branch or push directly; acceptance runs use local, throwaway repositories.
"""
import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import uuid

sys.dont_write_bytecode = True


def load_api():
    spec = importlib.util.spec_from_file_location("landing", Path(__file__).with_name("landing.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def resolve(api, root, revision):
    # --end-of-options prevents a supplied revision becoming a Git option.
    return api.git("rev-parse", "--verify", "--end-of-options", revision + "^{commit}", cwd=root)


def ancestor(api, root, before, after):
    return api.command(["git", "merge-base", "--is-ancestor", before, after],
                       cwd=root, check=False).returncode == 0


def fixture(api, root):
    options = api.fixture_options()
    if os.environ.get("BBOXY_LANDING_STATE_DIR") and not api.local_origin(root):
        raise api.Refusal("fixture_state_requires_local_origin")
    return options


def output_directory(api, name):
    path = api.state_directory() / name / uuid.uuid4().hex
    path.mkdir(parents=True, mode=0o700)
    return path


def retire_target(target):
    """Remove only this command's freshly-created cache, after its owners exit."""
    if not target.exists():
        return
    lsof = shutil.which("lsof")
    if lsof is None:
        print("recovery: cache retained; lsof unavailable: " + str(target), file=sys.stderr)
        return
    result = subprocess.run([lsof, "+D", str(target)], capture_output=True, text=True)
    if result.returncode == 1 and not result.stdout:
        shutil.rmtree(target)
    else:
        print("recovery: cache retained; open owners or unknown lsof result: " + str(target), file=sys.stderr)


@contextmanager
def retiring(target):
    try:
        yield
    finally:
        retire_target(target)


@contextmanager
def checkout(api, root, sha, output, branch=None, retain=False):
    tree = output / "tree"
    arguments = ["worktree", "add", "--detach"] if branch is None else ["worktree", "add", "-b", branch]
    api.git(*arguments, str(tree), sha, cwd=root)
    try:
        yield tree
    finally:
        if not retain:
            api.git("worktree", "remove", "--force", str(tree), cwd=root)


def run_logged(api, argv, tree, output, label, environment=None, timeout=900):
    """Bound and record this invocation's own process group, including on timeout."""
    started = api.now()
    log = output / (label + ".log")
    with log.open("w") as stream:
        child = subprocess.Popen(argv, cwd=tree, env=environment, stdin=subprocess.DEVNULL,
                                 stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        owner = {"pid": child.pid, "process_group": child.pid, "argv": argv,
                 "started": started, "cwd": str(tree)}
        api.atomic_write(output / (label + "-process.json"), json.dumps(owner) + "\n")
        timed_out = False
        try:
            code = child.wait(timeout=timeout)
        except (subprocess.TimeoutExpired, KeyboardInterrupt):
            timed_out = True
            # Only the new session created immediately above, never a name scan.
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=30)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            code = 124
    record = dict(owner, ended=api.now(), exit=code, timed_out=timed_out, log=str(log))
    api.atomic_write(output / (label + ".json"), json.dumps(record, sort_keys=True) + "\n")
    return record, log.read_text(errors="replace")


def cargo_selector(api, root, name):
    supplied = os.environ.get("BBOXY_FAILED_TEST_SELECTOR")
    if supplied:
        selector = json.loads(Path(supplied).read_text())
        if name not in (selector.get("name"), selector.get("selector")) or selector.get("source") != "cargo_rerun":
            raise api.Refusal("failed_test_selector_unavailable_or_mismatched")
        name = selector["name"]
        flags = selector.get("cargo_args")
    elif name.startswith("cargo:"):
        try:
            _, package, target, name = name.split(":", 3)
        except ValueError as error:
            raise api.Refusal("invalid_cargo_test_selector") from error
        flags = ["-p", package]
        if target == "lib":
            flags += ["--lib"]
        elif target.startswith(("test/", "bin/", "example/")):
            kind, target_name = target.split("/", 1)
            flags += ["--" + kind, target_name]
        else:
            raise api.Refusal("invalid_cargo_target")
    else:
        # Direct callers may omit Cargo's rerun flags only when the owner is
        # unique. A duplicate name must not diagnose a different crate's test.
        if not re.fullmatch(r"[\w:]+", name):
            raise api.Refusal("failed_test_selector_unavailable")
        leaf = name.split("::")[-1]
        matches = []
        for package, data in api.metadata(root).items():
            directory = Path(data["manifest_path"]).parent
            if any(re.search(r"\bfn\s+" + re.escape(leaf) + r"\b", p.read_text())
                   for p in directory.rglob("*.rs")):
                matches.append(package)
        if len(matches) != 1:
            raise api.Refusal("failed_test_owner_ambiguous_or_missing")
        flags = ["-p", matches[0]]
    if not isinstance(flags, list) or not all(isinstance(v, str) for v in flags):
        raise api.Refusal("invalid_cargo_rerun_flags")
    package, targets, i = None, 0, 0
    while i < len(flags):
        flag = flags[i]
        i += 1
        if flag in ("-p", "--package", "--test", "--bin", "--example"):
            if i == len(flags) or not re.fullmatch(r"\w[\w.+-]*", flags[i]):
                raise api.Refusal("invalid_cargo_rerun_value")
            if flag in ("-p", "--package"):
                if package is not None:
                    raise api.Refusal("multiple_cargo_packages")
                package = flags[i]
            else:
                targets += 1
            i += 1
        elif flag == "--lib":
            targets += 1
        else:
            raise api.Refusal("unsupported_cargo_rerun_flag: " + flag)
    if package is None or targets > 1 or not re.fullmatch(r"[\w:]+", name):
        raise api.Refusal("invalid_exact_test_selector")
    return name, flags


def diagnose(api, root, green, red, name):
    green, red = resolve(api, root, green), resolve(api, root, red)
    chain = api.git("rev-list", "--first-parent", "--reverse", red, cwd=root).splitlines()
    if green not in chain or green == red:
        raise api.Refusal("last_green_must_precede_red_on_first_parent")
    candidates = chain[chain.index(green) + 1:]
    options = fixture(api, root)
    predicate = options.get("recovery_test")
    exact, flags = (name, []) if predicate else cargo_selector(api, root, name)
    if predicate and (not isinstance(predicate, list) or not predicate or
                      not all(isinstance(v, str) for v in predicate)):
        raise api.Refusal("invalid_fixture_test_command")
    output = output_directory(api, "culprit")
    environment = dict(os.environ, RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="",
                       CARGO_BUILD_JOBS="2", BBOXY_TEST_CUSTODY="memory",
                       CARGO_TARGET_DIR=str(output / "target"))
    trials = []
    cache = {}
    with checkout(api, root, red, output) as tree, retiring(output / "target"):
        def bad(sha):
            if sha in cache:
                return cache[sha]
            api.git("checkout", "--detach", sha, cwd=tree)
            results = []
            for attempt in range(3):
                argv = ([v.replace("{sha}", sha) for v in predicate] if predicate else
                        ["cargo", "test", "--locked", *flags, exact, "--", "--exact", exact])
                record, text = run_logged(api, argv, tree, output,
                                          f"{sha}-{attempt + 1}", environment)
                record.update(sha=sha, attempt=attempt + 1)
                trials.append(record)
                if predicate:
                    valid = record["exit"] in (0, 1)
                else:
                    # A build error, panic before the harness or zero tests is
                    # not a failing test, and cannot identify a culprit.
                    observed = re.findall(r"(?m)^test " + re.escape(exact) + r" \.\.\. (ok|FAILED)$", text)
                    valid = len(observed) == 1 and record["exit"] in (0, 101)
                    valid &= bool(observed) and ((observed[0] == "ok") == (record["exit"] == 0))
                if not valid or record["timed_out"]:
                    raise api.Refusal("test_not_observed_or_infrastructure_failure: " + str(record["log"]))
                results.append(record["exit"] != 0)
            cache[sha] = any(results)
            return cache[sha]

        if not bad(red):
            result = {"verdict": "flake", "culprit": None, "quarantine_proposed": True}
        elif bad(green):
            result = {"verdict": "unresolved", "culprit": None, "reason": "test_fails_at_last_green",
                      "revert_range": green + ".." + red}
        else:
            low, high = 0, len(candidates) - 1
            while low < high:
                middle = (low + high) // 2
                if bad(candidates[middle]):
                    high = middle
                else:
                    low = middle + 1
            result = {"verdict": "culprit", "culprit": candidates[low]}
    result.update(green=green, red=red, test=name, trials=trials, evidence=str(output))
    api.atomic_write(output / "result.json", json.dumps(result, sort_keys=True) + "\n")
    api.append_record("culprit.log", result)
    return result


def corrective_revert(api, root, revision):
    fixture(api, root)
    api.publishing_identity(root)
    api.git("fetch", "origin", "main", cwd=root)
    main = resolve(api, root, "origin/main")
    if ".." in revision:
        before, after = revision.split("..", 1)
        before, after = resolve(api, root, before), resolve(api, root, after)
        chain = api.git("rev-list", "--first-parent", after, cwd=root).splitlines()
        if before not in chain or not ancestor(api, root, after, main):
            raise api.Refusal("revert_range_not_on_main")
        commits = chain[:chain.index(before)]
    else:
        commits = [resolve(api, root, revision)]
        if not ancestor(api, root, commits[0], main):
            raise api.Refusal("culprit_not_on_main")
    if not commits:
        raise api.Refusal("empty_revert_range")
    output = output_directory(api, "reverts")
    branch = "landing-revert-" + output.name
    # Retain on failure, allowing the watcher to report and retry this exact
    # signed cut instead of leaving an unnamed destructive operation behind.
    with checkout(api, root, main, output, branch=branch, retain=True) as tree:
        for sha in commits:
            parents = api.git("show", "-s", "--format=%P", sha, cwd=tree).split()
            argv = ["git", "-c", "commit.gpgsign=true", "revert", "--no-edit"]
            if len(parents) > 1:
                argv += ["-m", "1"]
            result = api.command([*argv, sha], cwd=tree, check=False)
            if result.returncode:
                raise api.Refusal("revert_conflict: " + str(tree) + "\n" + result.stderr)
        landed, _ = run_logged(api, ["sh", str(Path(__file__).with_name("land.sh"))], tree,
                               output, "land", dict(os.environ), timeout=3000)
        answer = {"culprit": revision, "commits": commits, "branch": branch,
                  "sha": api.git("rev-parse", "HEAD", cwd=tree), "exit": landed["exit"],
                  "tree": str(tree), "evidence": str(output)}
        api.atomic_write(output / "result.json", json.dumps(answer, sort_keys=True) + "\n")
        api.append_record("reverts.log", answer)
    if answer["exit"] == 0:
        api.git("worktree", "remove", str(tree), cwd=root)
    return answer


def code_commit(api, root, sha):
    parents = api.git("show", "-s", "--format=%P", sha, cwd=root).split()
    arguments = ["diff", "--name-only", parents[0], sha] if parents else ["diff-tree", "--root", "--no-commit-id", "--name-only", "-r", sha]
    paths = api.git(*arguments, "--", cwd=root).splitlines()
    return any(p == "Cargo.lock" or p.startswith(("crates/", "scripts/")) for p in paths)


def check_main(api, root):
    fixture(api, root)
    records = api.records("postsubmit.log")
    status = api.state_directory() / "main-status"
    fields = status.read_text().split() if status.exists() else []
    if fields and fields[0] == "red":
        if len(fields) < 3:
            raise api.Refusal("invalid_red_main_status")
        if not any(r.get("sha") == fields[1] and r.get("verdict") == "red" for r in records):
            records.append({"sha": fields[1], "verdict": "red", "culprit": None,
                            "failing_test": fields[2], "in_flight": True})
    if not records:
        return {"status": "pending", "reason": "no_postsubmit_yet", "violations": []}
    main = resolve(api, root, "origin/main")
    head = resolve(api, root, "HEAD")
    if ancestor(api, root, main, head):
        subjects = api.git("log", "--format=%s", main + ".." + head, cwd=root).splitlines()
        if subjects and all(s.startswith("Revert") for s in subjects):
            main = head  # check the corrective candidate before land.sh pushes it
    chain = api.git("rev-list", "--first-parent", "--reverse", main, cwd=root).splitlines()
    positions = {sha: n for n, sha in enumerate(chain)}
    violations, pending = [], []
    for row in records:
        if row.get("sha") not in positions or row.get("verdict") not in ("green", "red"):
            raise api.Refusal("postsubmit_not_on_main_or_invalid_verdict")
    for index, row in enumerate(records):
        if row["verdict"] != "red":
            continue
        following = next((r for r in records[index + 1:] if r["verdict"] == "green"), None)
        start = positions[row["sha"]] + 1
        stop = positions[following["sha"]] + 1 if following else len(chain)
        later = [sha for sha in chain[start:stop] if code_commit(api, root, sha)]
        if not later:
            pending.append(row["sha"])
            continue
        culprit = row.get("revert_range") or row.get("culprit")
        for n, sha in enumerate(later):
            subject = api.git("show", "-s", "--format=%s", sha, cwd=root)
            body = api.git("show", "-s", "--format=%B", sha, cwd=root)
            if not subject.startswith("Revert"):
                violations.append({"sha": sha, "reason": "non_revert_while_red", "red": row["sha"]})
            elif n == 0 and culprit not in (None, "-", ""):
                named = culprit.split("..")[-1]
                if named not in body:
                    violations.append({"sha": sha, "reason": "first_revert_does_not_name_culprit", "culprit": culprit})
    greens = [positions[r["sha"]] for r in records if r["verdict"] == "green"]
    return {"status": "violated" if violations else ("pending" if pending else "held"), "main": main,
            "covered_through": chain[max(greens)] if greens else None,
            "pending_red": pending, "violations": violations}


def groups_at(api, root, sha):
    text = api.git("show", sha + ":.bboxy/specs/e2e-acceptance.md", cwd=root)
    section = re.search(r"(?ms)^## 1\..*?\n(.*?)(?=^## |\Z)", text)
    groups = re.findall(r"(?m)^### (.+)$", section.group(1)) if section else []
    if not groups or len(groups) != len(set(groups)):
        raise api.Refusal("night_groups_missing_or_duplicate")
    return groups


def json_lines(text):
    rows = []
    for line in text.splitlines():
        try:
            value = json.loads(line)
        except ValueError:
            continue
        if isinstance(value, dict):
            rows.append(value)
    return rows


def stop_night_apps(api, binary, rows, output, label):
    # A timed-out group may not run its Rust owners' stop path. Only endpoints
    # underneath roots announced by this invocation are eligible for cleanup.
    for n, row in enumerate(r for r in rows if r.get("kind") == "night" and r.get("root")):
        root = Path(row["root"])
        if not root.is_absolute() or not root.is_dir():
            continue
        for k, endpoint in enumerate(root.rglob("gui.endpoint")):
            if endpoint.resolve().is_relative_to(root.resolve()):
                run_logged(api, [str(binary), "ctl", "--run-dir", str(endpoint.parent), "app", "stop"],
                           output, output, f"{label}-stop-{n}-{k}", dict(os.environ), timeout=30)


def night_all(api, root, arguments):
    parser = argparse.ArgumentParser(prog="e2e-night.sh --all")
    parser.add_argument("--sha", default="origin/main")
    args = parser.parse_args(arguments)
    fixture(api, root)
    sha = resolve(api, root, args.sha)
    if not ancestor(api, root, sha, resolve(api, root, "origin/main")):
        raise api.Refusal("night_source_is_not_on_main")
    binary = Path(os.environ.get("BBT", root / "target/debug/bboxy450-term")).resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise api.Refusal("night_requires_prebuilt_app: " + str(binary))
    output = output_directory(api, "nights")
    combined = output / "night.jsonl"
    groups = groups_at(api, root, sha)
    with binary.open("rb") as stream:
        binary_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    header = {"kind": "night-source", "sha": sha, "binary": str(binary), "binary_sha256": binary_hash}
    combined.write_text(json.dumps(header) + "\n")
    results = []
    with checkout(api, root, sha, output) as tree:
        for index, group in enumerate(groups):
            record, text = run_logged(api, [str(binary), "e2e", "night", "--group", group,
                                            "--scripts", "scripts/e2e", "--records", str(output / "records")],
                                       tree, output, f"group-{index + 1}", dict(os.environ), timeout=1200)
            rows = json_lines(text)
            summaries = [r for r in rows if r.get("kind") == "summary" and r.get("group") == group]
            complete = len(summaries) == 1 and not summaries[0].get("unwritten")
            entry = {"kind": "night-group-result", "group": group, "exit": record["exit"],
                     "completed_by_verdict": complete and record["exit"] in (0, 1), "log": record["log"]}
            results.append(entry)
            with combined.open("a") as stream:
                for row in rows:
                    stream.write(json.dumps(row) + "\n")
                stream.write(json.dumps(entry) + "\n")
            stop_night_apps(api, binary, rows, output, f"group-{index + 1}")
        # Completion means every group has ended. Failed/refused groups stay in
        # the record and must not suppress the independent scale measurement.
        with combined.open("a") as stream:
            stream.write(json.dumps({"kind": "night-summary", "sha": sha, "groups": results}) + "\n")
        measured, _ = run_logged(api, ["sh", str(tree / "scripts/scale-night.sh"), "--night-log", str(combined),
                                      "--sha", sha], tree, output, "scale", dict(os.environ), timeout=6000)
    code = measured["exit"]
    if code == 0 and any(r["exit"] != 0 or not r["completed_by_verdict"] for r in results):
        code = 1
    result = {"sha": sha, "exit": code, "groups": results, "scale_exit": measured["exit"],
              "night_log": str(combined), "evidence": str(output)}
    api.atomic_write(output / "result.json", json.dumps(result, sort_keys=True) + "\n")
    api.append_record("nights.log", result)
    return result


def scale_night(api, root, arguments):
    parser = argparse.ArgumentParser(prog="scale-night.sh")
    parser.add_argument("--sha", default="origin/main")
    parser.add_argument("--night-log", type=Path, required=True,
                        help="completed e2e JSONL; the scheduler calls this last")
    args = parser.parse_args(arguments)
    options = fixture(api, root)
    sha = resolve(api, root, args.sha)
    if not ancestor(api, root, sha, resolve(api, root, "origin/main")):
        raise api.Refusal("scale_source_is_not_on_main")
    night = json_lines(args.night_log.read_text())
    summaries = [r for r in night if r.get("kind") == "summary"]
    completed = [r for r in night if r.get("kind") == "night-summary" and r.get("sha") == sha]
    expected = set(groups_at(api, root, sha))
    if completed:
        observed = {r.get("group") for r in completed[-1].get("groups", [])}
    else:
        observed = {r.get("group") for r in summaries}
    if observed != expected:
        raise api.Refusal("scale_requires_completed_e2e_night")
    output = output_directory(api, "scale")
    with checkout(api, root, sha, output) as tree, retiring(output / "target"):
        target = output / "target"
        environment = dict(os.environ, RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="",
                           CARGO_BUILD_JOBS="2", CARGO_TARGET_DIR=str(target),
                           CARGO_INCREMENTAL="0", CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS="true",
                           BBOXY_TEST_CUSTODY="memory", GATE_BUDGET="8.33")
        environment.pop("BBOXY_VERIFY_INPUT", None)
        binary = target / "release/bboxy450-term"
        def expanded(argv):
            return [v.replace("{output}", str(output)).replace("{tree}", str(tree))
                    .replace("{sha}", sha).replace("{binary}", str(binary)) for v in argv]
        build = expanded(options.get("scale_build", ["cargo", "build", "--release", "--locked", "-p", "bboxy450-term"]))
        built, _ = run_logged(api, build, tree, output, "build", environment, timeout=3600)
        argv = [str(binary), "--stall-test", "--stall-real", "--stall-frame-gate",
                "--stall-seats", "967", "--stall-watched", "50", "--stall-seconds", "10", "--stall-idle", "10",
                "--stall-run-dir", str(output / "run"), "--stall-report", str(output / "report.json"),
                "--capture", str(output / "window.png"), "--capture-after", "5"]
        measured = built
        if not built["exit"]:
            measured, _ = run_logged(api, expanded(options.get("scale_run", argv)), tree, output,
                                     "measurement", environment, timeout=1800)
        report_path = output / "report.json"
        try:
            report = json.loads(report_path.read_text()) if report_path.is_file() else {}
        except ValueError:
            report = {}  # preserve malformed raw evidence, never turn it green
        if not isinstance(report, dict):
            report = {}
        report_valid = (report.get("rows") == 967 and report.get("real_ptys") == 967
                        and report.get("watched") == 50 and report.get("timing_gated") is True
                        and report.get("metrics", {}).get("diagnostics", {}).get("enabled") is True)
        result = {"sha": sha, "exit": measured["exit"], "seats": 967, "watched": 50,
                  "night_log": str(args.night_log.resolve()), "evidence": str(output),
                  "report_present": report_path.is_file(), "report_valid": report_valid,
                  "build_exit": built["exit"], "source_mode": "native_frame_gate"}
        if not measured["exit"] and (not report_valid or report.get("failures")
                                      or report.get("timing_failures") or report.get("unproven")):
            result.update(exit=2, reason="scale_report_unproven_or_inconsistent")
        api.atomic_write(output / "result.json", json.dumps(result, sort_keys=True) + "\n")
        # Commit immutable daily evidence on a separate branch; publication stays
        # with records-push.sh, never a side effect of a measurement verdict.
        branch = "scale-evidence-" + output.name
        api.git("switch", "-c", branch, cwd=tree)
        destination = tree / ".bboxy/runs/landing" / (api.now()[:10] + "-scale-" + output.name[:8])
        destination.mkdir(parents=True)
        for name in ("result.json", "report.json", "report.capture.json", "build.log", "build.json", "measurement.log", "measurement.json"):
            path = output / name
            if path.is_file():
                shutil.copy2(path, destination / name)
        shutil.copy2(args.night_log, destination / "e2e-night.jsonl")
        for name in ("landings.log", "postsubmit.log"):
            path = api.state_directory() / name
            if path.is_file():
                shutil.copy2(path, destination.parent / (api.now()[:10] + "-" + name))
        api.git("add", "--", str(destination.relative_to(tree)), cwd=tree)
        daily = [str(p.relative_to(tree)) for p in destination.parent.glob(api.now()[:10] + "-*.log")]
        if daily:
            api.git("add", "--", *daily, cwd=tree)
        api.git("-c", "commit.gpgsign=true", "commit", "-m", "Record nightly 967/50 measurement at " + sha[:12], cwd=tree)
        result.update(evidence_commit=api.git("rev-parse", "HEAD", cwd=tree), evidence_branch=branch)
    api.append_record("scale.log", result)
    return result


def main():
    def interrupted(_signum, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupted)
    api = load_api()
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("culprit", "revert", "main-red-bounded", "scale-night", "night-all"))
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    root = None
    try:
        root = api.repository()
        if args.command == "culprit":
            if len(args.arguments) != 3:
                raise api.Refusal("usage: culprit.sh LAST_GREEN RED TEST")
            result = diagnose(api, root, *args.arguments)
            code = 2 if result["verdict"] == "unresolved" else 0
        elif args.command == "revert":
            if len(args.arguments) != 1:
                raise api.Refusal("usage: revert.sh CULPRIT_OR_RANGE")
            result = corrective_revert(api, root, args.arguments[0])
            code = result["exit"]
        elif args.command == "main-red-bounded":
            if args.arguments:
                raise api.Refusal("main-red-bounded.sh takes no arguments")
            result = check_main(api, root)
            code = int(bool(result["violations"]))
        elif args.command == "night-all":
            result = night_all(api, root, args.arguments)
            code = result["exit"]
        else:
            result = scale_night(api, root, args.arguments)
            code = result["exit"] or (0 if result["report_present"] else 2)
        print(json.dumps(result, sort_keys=True))
        return code
    except (api.Refusal, OSError, ValueError) as error:
        result = {"verdict": "unresolved", "culprit": None, "reason": str(error)}
        if root is not None and args.command == "culprit" and len(args.arguments) == 3:
            # Compilation, unavailable selectors and zero-test runs cannot
            # identify a landing. The watcher still needs the validated batch
            # so it can follow revert-first rather than leave main red forever.
            try:
                fixture(api, root)
                green, red = (resolve(api, root, revision) for revision in args.arguments[:2])
                chain = api.git("rev-list", "--first-parent", red, cwd=root).splitlines()
                if green in chain and green != red:
                    result.update(green=green, red=red, test=args.arguments[2], revert_range=green + ".." + red)
            except (api.Refusal, OSError, ValueError):
                pass  # invalid ranges/fixture authority never instruct a revert
        print(json.dumps(result, sort_keys=True))
        return getattr(error, "code", 2)


if __name__ == "__main__":
    raise SystemExit(main())
