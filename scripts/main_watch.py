"""Batched postsubmit, shared main verdict, and retained native parity truth."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import subprocess
import time
import uuid


def relevant(paths):
    return any(p in ("Cargo.lock", "Cargo.toml", ".bboxy/quarantine.md") or p.startswith(
        ("crates/", "scripts/", "lab/", "design/", ".cargo/")) for p in paths)


def history_rows(rows):
    result = []
    kinds = {"green": "green", "red": "red", "manual-green": "green", "manual-revert": "green"}
    for value in rows:
        row = dict(value)
        if row.get("kind") == "invariant_finding":
            continue  # historical findings never alter CI admission or render truth
        if "verdict" not in row and "correction" in row:
            prior = next((r for r in reversed(result) if r["sha"] == row["sha"]), None)
            if prior is None:
                raise ValueError("postsubmit_correction_without_a_prior_verdict")
            prior.update(correction=row["correction"], full_checks_passed=False)
            continue
        if row.get("verdict") == "green" and row.get("manual"):
            row["verdict"] = "manual-green"
        if row.get("verdict") not in kinds:
            raise ValueError("invalid_postsubmit_verdict")
        row["effective_verdict"] = kinds[row["verdict"]]
        if row["verdict"].startswith("manual-"):
            row["full_checks_passed"] = False
        result.append(row)
    return result


def retain_parity(tree, target, sha, api):
    native = target / "parity/native"
    cases = target / "parity/cases.json"
    binary = target / "debug/bboxy450-book"
    if not native.is_dir() or not cases.is_file() or not binary.is_file():
        raise api.Refusal("postsubmit_green_without_native_parity_store")
    destination = api.parity_directory() / sha
    if not destination.exists():
        temporary = destination.with_name(sha + "." + uuid.uuid4().hex + ".tmp")
        temporary.mkdir(parents=True, mode=0o700)
        try:
            shutil.copytree(native, temporary / "native")
            shutil.copy2(cases, temporary / "cases.json")
            shutil.copy2(binary, temporary / "bboxy450-book")
            api.atomic_write(temporary / "source.json", json.dumps({"sha": sha, "stored": api.now()}) + "\n")
            temporary.rename(destination)
        finally:
            if temporary.exists():
                shutil.rmtree(temporary)  # this invocation's unique, recorded directory
    api.atomic_write(api.state_directory() / "parity-last-green", str(destination.resolve()) + "\n")


def fail_name(text):
    match = re.search(r"(?m)^test (.+) \.\.\. FAILED$", text)
    return match.group(1) if match else "exit_or_parity"


def test_selector(text, parity_report=None):
    gates = re.findall(r"(?m)^invariant ([a-zA-Z0-9_-]+): fail\b", text)
    if gates:
        gate = gates[0]
        if gate == "parity-geometry" and parity_report is not None:
            try:
                rows = json.loads(parity_report.read_text())
                cases = sorted({r["key"] for r in rows if not r["policy"]["historical"]
                                and r["status"] in ("review geometry", "unproven geometry")})
                if cases:
                    return {"name": gate, "selector": "parity:" + json.dumps(cases),
                            "source": "parity_report"}
            except (OSError, ValueError, KeyError, TypeError):
                pass
        return {"name": gate, "selector": "invariant:" + gate, "source": "invariant_output"}
    name = fail_name(text)
    failure = text.find("test " + name + " ... FAILED")
    rerun = re.search(r"error: test failed, to rerun pass [`']([^`']+)[`']",
                      text[max(0, failure):])
    args = shlex.split(rerun.group(1)) if rerun else None
    selector = "test:" + name if name != "exit_or_parity" else "unknown:exit_or_parity"
    if args:
        package = next((args[n + 1] for n, arg in enumerate(args[:-1]) if arg in ("-p", "--package")), None)
        kind = "lib" if "--lib" in args else next(
            (arg[2:] + "/" + args[n + 1] for n, arg in enumerate(args[:-1]) if arg in ("--test", "--bin")), None)
        if package and kind:
            selector = f"test:cargo:{package}:{kind}:{name}"
    return {"name": name, "selector": selector, "cargo_args": args,
            "source": "cargo_rerun" if args else "unavailable"}


class Lease:
    def __init__(self, api):
        self.api = api
        self.path = api.state_directory() / "main-watch.lock"
        self.token = uuid.uuid4().hex

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        if self.path.exists():
            try:
                owner = json.loads((self.path / "owner.json").read_text())
                pid = int(owner["pid"])
                if pid <= 0:
                    raise ValueError("nonpositive owner")
                os.kill(pid, 0)
            except ProcessLookupError:
                if set(p.name for p in self.path.iterdir()) != {"owner.json"}:
                    raise self.api.Refusal("watcher_lock_has_unknown_files")
                (self.path / "owner.json").unlink()
                self.path.rmdir()
            except (ValueError, OSError, KeyError) as error:
                raise self.api.Refusal("watcher_lock_unavailable: " + str(error))
            else:
                raise self.api.Refusal("main_watch_already_running", 75)
        try:
            self.path.mkdir()
        except FileExistsError:
            raise self.api.Refusal("main_watch_already_running", 75)
        self.api.atomic_write(self.path / "owner.json", json.dumps(
            {"pid": os.getpid(), "token": self.token}) + "\n")
        return self

    def __exit__(self, *unused):
        owner = json.loads((self.path / "owner.json").read_text())
        if owner.get("token") == self.token:
            (self.path / "owner.json").unlink()
            self.path.rmdir()


def run_ci(api, tree, output, environment, sha, label="ci", after_tests=False):
    log = output / (label + ".log")
    argv = ["sh", "scripts/ci-local.sh"]
    if after_tests:
        argv.append("--after-tests")
    argv.append("--parity")
    with log.open("w") as stream:
        child = subprocess.Popen(argv, cwd=tree, env=environment, stdout=stream,
                                 stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, start_new_session=True)
        process = "process.json" if label == "ci" else label + "-process.json"
        api.atomic_write(output / process, json.dumps(
            {"pid": child.pid, "pgid": child.pid, "sha": sha, "argv": argv}) + "\n")
        try:
            code = child.wait()
        except BaseException:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
            raise
    return code, log.read_text()


def failure_selections(text, report):
    gates = list(dict.fromkeys(re.findall(r"(?m)^invariant ([a-zA-Z0-9_-]+): fail\b", text)))
    if gates:
        return [test_selector("invariant " + gate + ": fail", report) for gate in gates]
    names = list(dict.fromkeys(re.findall(r"(?m)^test (.+) \.\.\. FAILED$", text)))
    if names:
        return [test_selector(text[text.index("test " + name + " ... FAILED"):], report) for name in names]
    return [test_selector(text, report)]


def diagnose_predicate(api, tree, output, green, sha, selection, label):
    selector = output / (label + "-selector.json")
    api.atomic_write(selector, json.dumps(selection) + "\n")
    diagnosis = api.command(["sh", "scripts/culprit.sh", green or "-", sha, selection["selector"]],
                            cwd=tree, check=False,
                            env=dict(os.environ, BBOXY_FAILED_TEST_SELECTOR=str(selector),
                                     BBOXY_POSTSUBMIT_ISOLATE="1"))
    raw = diagnosis.stdout + diagnosis.stderr
    api.atomic_write(output / (label + ".log"), raw)
    with (output / "culprit.log").open("a") as stream:
        stream.write(raw)
    try:
        answer = json.loads(diagnosis.stdout)
        if not isinstance(answer, dict):
            raise ValueError("not an object")
    except ValueError:
        answer = {"verdict": "unresolved", "reason": "invalid_culprit_json"}
    return diagnosis.returncode, answer


def status_publication(api, color, sha):
    try:
        with (api.state_directory() / "main-status").open() as stream:
            fields = stream.read().split(maxsplit=2)
            modified = os.fstat(stream.fileno()).st_mtime
        if fields[:2] == [color, sha]:
            return datetime.fromtimestamp(modified, timezone.utc).isoformat()
    except OSError:
        pass
    return None


def check_watch_invariant(root, api, row, destination):
    command = ["sh", str(Path(__file__).with_name("main-red-bounded.sh"))]
    try:
        result = api.command(command, cwd=root, check=False)
        code = result.returncode
        try:
            report = json.loads(result.stdout)
        except ValueError:
            report = {"unavailable": "invalid_checker_json", "stdout": result.stdout}
        evidence = {"exit": code, "report": report, "stderr": result.stderr, "checked_at": api.now()}
    except OSError as error:
        code = 127
        evidence = {"exit": code, "unavailable": str(error), "checked_at": api.now()}
    api.atomic_write(destination, json.dumps(evidence, sort_keys=True) + "\n")
    if code:
        finding = {"kind": "invariant_finding", "check": "main-red-bounded", "sha": row["sha"],
                   "exit": code, "ended": api.now(), "log": str(destination), "evidence": evidence}
        api.append_record("postsubmit.log", finding)
        print(json.dumps(finding, sort_keys=True), flush=True)
    return code


def publish_verdict(root, api, row):
    color = "red" if row["verdict"] == "red" else "green"
    # Diagnosis closes admission before bisect/revert. Preserve that first write.
    first_red = status_publication(api, "red", row["sha"]) if color == "red" else None
    status = color + " " + row["sha"]
    if color == "red":
        status += " " + row["failing_test"]
    api.atomic_write(api.state_directory() / "main-status", status + "\n")
    row["ended"] = first_red or status_publication(api, color, row["sha"])
    row["completed_at"] = api.now()
    destination = api.state_directory() / "watch-checks" / (uuid.uuid4().hex + ".json")
    row["watch_check_log"] = str(destination)
    api.append_record("postsubmit.log", row)
    code = check_watch_invariant(root, api, row, destination)
    print(json.dumps(row, sort_keys=True), flush=True)
    return code or int(color == "red")


def one(root, api):
    api.git("fetch", "origin", "main", cwd=root)
    sha = api.git("rev-parse", "origin/main", cwd=root)
    history = history_rows(api.records("postsubmit.log"))
    last = history[-1]["sha"] if history else None
    if last == sha and not history[-1]["verdict"].startswith("manual-"):
        return 0 if history[-1]["effective_verdict"] == "green" else 1
    paths = api.changed_paths(last + ".." + sha, cwd=root) if last else ["crates/"]
    if last and not history[-1]["verdict"].startswith("manual-") and not relevant(paths):
        previous = history[-1]
        record = dict(previous, sha=sha, started=api.now(), ended=api.now(),
                      carried_from=last, skipped="records_only")
        if record["effective_verdict"] == "red":
            record["verdict"] = "red"
        return publish_verdict(root, api, record)
    work = root / "scratchpad/landing-watch"
    work.mkdir(parents=True, exist_ok=True)
    tree = work / "tree"
    target = work / "target"
    output = api.state_directory() / "postsubmits" / (sha + "-" + uuid.uuid4().hex[:8])
    output.mkdir(parents=True, mode=0o700)
    api.atomic_write(output / "paths.json", json.dumps({"tree": str(tree), "target": str(target)}) + "\n")
    if tree.exists():
        if tree.is_symlink() or api.common_repository(tree) != api.common_repository(root):
            raise api.Refusal("watch_tree_is_not_this_repository")
        api.git("worktree", "remove", "--force", str(tree), cwd=root)
    api.git("worktree", "add", "--detach", str(tree), sha, cwd=root)
    started = api.now()
    try:
        marker = target / ".bboxy-checkout.json"
        binding = {"tree": str(tree.resolve()), "target": str(target.resolve())}
        try:
            bound = json.loads(marker.read_text()) == binding
        except (OSError, ValueError):
            bound = False
        if target.exists() and not bound:
            if target.is_symlink():
                raise api.Refusal("watch_target_is_symlink")
            shutil.rmtree(target)
        target.mkdir(exist_ok=True)
        api.atomic_write(marker, json.dumps(binding) + "\n")
        environment = api.build_environment(tree, target=target)
        environment.update(BBOXY_GATE_SLOT="none", PARITY_OUT=str(target / "parity"))
        code, text = run_ci(api, tree, output, environment, sha)
        selection = test_selector(text, target / "parity/report.json") if code else None
        failure = selection["name"] if code else "-"
        culprit = "-"
        verdict = "red" if code else "green"
        if not code:
            try:
                retain_parity(tree, target, sha, api)
            except api.Refusal as error:
                code, verdict, failure = 1, "red", "parity_store_missing"
                selection = {"name": failure, "selector": "unknown:" + failure, "source": "unavailable"}
                api.atomic_write(output / "store-error.log", str(error) + "\n")
        record = {"sha": sha, "verdict": verdict, "started": started, "ended": api.now(),
                  "failing_test": failure, "culprit": culprit, "exit": code,
                  "full_checks_passed": not code, "log": str(output / "ci.log")}
        green = next((r["sha"] for r in reversed(history)
                      if r["verdict"] == "green" and r.get("full_checks_passed", True)), None)
        if code:
            # Every failing predicate gets isolated reruns before main closes.
            # A flake is not a full CI verdict: complete the remaining checks.
            for resume in range(4):
                pending = ([selection] if failure == "parity_store_missing" else
                           failure_selections(text, target / "parity/report.json"))
                survivor = None
                for index, selected in enumerate(pending):
                    record.update(test_selector=selected["selector"], predicate=selected["selector"])
                    label = f"rerun-{resume}-{index}"
                    diagnostic_exit, answer = diagnose_predicate(
                        api, tree, output, green, sha, selected, label)
                    if diagnostic_exit == 0 and answer.get("verdict") == "flake":
                        proposal = {"sha": sha, "predicate": selected["selector"], "timestamp": api.now(),
                                    "evidence": str(output / (label + ".log")),
                                    "log": str(output / (label + ".log")),
                                    "reruns": answer.get("trials", []),
                                    "owner_and_task_required": True}
                        api.append_record("quarantine-proposals.log", proposal)
                        record.setdefault("flaky_predicates", []).append(proposal)
                        record["flake"] = True
                    else:
                        survivor = selected
                        break
                if survivor is not None:
                    selection, failure = survivor, survivor["name"]
                    record.update(failing_test=failure, test_selector=selection["selector"])
                    if status_publication(api, "red", sha) is None:
                        api.atomic_write(api.state_directory() / "main-status", f"red {sha} {failure}\n")
                    if not green:
                        record["diagnostic_error"] = "no_last_green"
                        break
                    if diagnostic_exit:
                        record["diagnostic_exit"] = diagnostic_exit
                    remedy = None
                    if diagnostic_exit == 0 and re.fullmatch(r"[0-9a-f]{40}", answer.get("culprit") or ""):
                        remedy = record["culprit"] = answer["culprit"]
                    elif answer.get("verdict") == "unresolved" and answer.get("revert_range") == green + ".." + sha:
                        remedy = record["revert_range"] = answer["revert_range"]
                    if remedy:
                        reverted = api.command(["sh", "scripts/revert.sh", remedy], cwd=tree, check=False)
                        api.atomic_write(output / "revert.log", reverted.stdout + reverted.stderr)
                        record["revert_exit"] = reverted.returncode
                    break
                if resume == 3:
                    # All remaining failures were classified as flakes. Keep
                    # admission open, without claiming a fresh native truth.
                    verdict = "green"
                    record.update(full_checks_passed=False,
                                  incomplete_reason="full_checks_not_green_after_three_resumes")
                    break
                label = "after-flake-" + str(resume + 1)
                code, text = run_ci(api, tree, output, environment, sha, label, after_tests=True)
                record.setdefault("resumes", []).append({"exit": code, "log": str(output / (label + ".log"))})
                if not code:
                    try:
                        retain_parity(tree, target, sha, api)
                    except api.Refusal as error:
                        code, failure = 1, "parity_store_missing"
                        selection = {"name": failure, "selector": "unknown:" + failure, "source": "unavailable"}
                        api.atomic_write(output / "store-error.log", str(error) + "\n")
                    else:
                        verdict = "green"
                        record["full_checks_passed"] = True
                        break
                else:
                    selection = test_selector(text, target / "parity/report.json")
                    failure = selection["name"]
            record.update(verdict=verdict, initial_exit=record["exit"], exit=code,
                          ended=api.now(), failing_test=failure)
        record["native_retained"] = record["full_checks_passed"]
        return publish_verdict(root, api, record)
    finally:
        # Only our newly-created detached checkout. Keep its target for the next
        # batch and all command output in the machine-wide evidence directory.
        api.git("worktree", "remove", "--force", str(tree), cwd=root)


def run(arguments, api):
    parser = argparse.ArgumentParser()
    parser.add_argument("--once", action="store_true")
    manual = parser.add_mutually_exclusive_group()
    manual.add_argument("--manual-green", metavar="SHA", help="log the Oracle's evidenced hand call")
    manual.add_argument("--manual-revert", metavar="SHA", help="log the landed corrective Revert commit")
    parser.add_argument("--evidence")
    args = parser.parse_args(arguments)
    def terminate(signum, frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, terminate)
    root = api.repository()
    if args.manual_green or args.manual_revert:
        if not args.evidence or not args.evidence.strip():
            raise api.Refusal("manual_postsubmit_requires_evidence")
        api.git("fetch", "origin", "main", cwd=root)
        sha = api.git("rev-parse", "--verify", "--end-of-options",
                      (args.manual_green or args.manual_revert) + "^{commit}", cwd=root)
        if api.command(["git", "merge-base", "--is-ancestor", sha, "origin/main"],
                       cwd=root, check=False).returncode:
            raise api.Refusal("manual_postsubmit_sha_not_on_main")
        if args.manual_revert and not api.git("show", "-s", "--format=%s", sha, cwd=root).startswith("Revert"):
            raise api.Refusal("manual_revert_requires_a_landed_revert")
        row = {"sha": sha, "verdict": "manual-green" if args.manual_green else "manual-revert",
               "manual": True, "evidence": args.evidence, "ended": api.now(), "full_checks_passed": False}
        return publish_verdict(root, api, row)
    with Lease(api):
        while True:
            code = one(root, api)
            if args.once:
                return code
            time.sleep(60)
