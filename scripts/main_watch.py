"""Batched postsubmit, shared main verdict, and retained native parity truth."""
import argparse
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
    return any(p in ("Cargo.lock", "Cargo.toml") or p.startswith(
        ("crates/", "scripts/", "lab/", "design/", ".cargo/")) for p in paths)


def retain_parity(tree, target, sha, api):
    native = target / "parity/native"
    cases = target / "parity/cases.json"
    binary = target / "debug/bboxybook"
    if not native.is_dir() or not cases.is_file() or not binary.is_file():
        raise api.Refusal("postsubmit_green_without_native_parity_store")
    destination = api.parity_directory() / sha
    if not destination.exists():
        temporary = destination.with_name(sha + "." + uuid.uuid4().hex + ".tmp")
        temporary.mkdir(parents=True, mode=0o700)
        try:
            shutil.copytree(native, temporary / "native")
            shutil.copy2(cases, temporary / "cases.json")
            shutil.copy2(binary, temporary / "bboxybook")
            api.atomic_write(temporary / "source.json", json.dumps({"sha": sha, "stored": api.now()}) + "\n")
            temporary.rename(destination)
        finally:
            if temporary.exists():
                shutil.rmtree(temporary)  # this invocation's unique, recorded directory
    api.atomic_write(api.state_directory() / "parity-last-green", str(destination.resolve()) + "\n")


def fail_name(text):
    match = re.search(r"(?m)^test (.+) \.\.\. FAILED$", text)
    return match.group(1) if match else "exit_or_parity"


def test_selector(text):
    name = fail_name(text)
    failure = text.find("test " + name + " ... FAILED")
    rerun = re.search(r"error: test failed, to rerun pass [`']([^`']+)[`']",
                      text[max(0, failure):])
    args = shlex.split(rerun.group(1)) if rerun else None
    return {"name": name, "cargo_args": args,
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


def one(root, api):
    api.git("fetch", "origin", "main", cwd=root)
    sha = api.git("rev-parse", "origin/main", cwd=root)
    history = api.records("postsubmit.log")
    last = history[-1]["sha"] if history else None
    if last == sha:
        return 0 if history[-1]["verdict"] == "green" else 1
    paths = api.changed_paths(last + ".." + sha, cwd=root) if last else ["crates/"]
    if last and not relevant(paths):
        previous = history[-1]
        record = dict(previous, sha=sha, started=api.now(), ended=api.now(),
                      carried_from=last, skipped="records_only")
        api.append_record("postsubmit.log", record)
        status = f'{record["verdict"]} {sha}'
        if record["verdict"] == "red":
            status += " " + record["failing_test"]
        api.atomic_write(api.state_directory() / "main-status", status + "\n")
        return 0 if record["verdict"] == "green" else 1
    work = root / "scratchpad/landing-watch"
    work.mkdir(parents=True, exist_ok=True)
    tree = work / ("tree-" + uuid.uuid4().hex)
    target = work / "target"
    output = api.state_directory() / "postsubmits" / (sha + "-" + uuid.uuid4().hex[:8])
    output.mkdir(parents=True, mode=0o700)
    api.atomic_write(output / "paths.json", json.dumps({"tree": str(tree), "target": str(target)}) + "\n")
    api.git("worktree", "add", "--detach", str(tree), sha, cwd=root)
    started = api.now()
    try:
        environment = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS="2",
                           BBOXY_GATE_SLOT="none", PARITY_OUT=str(target / "parity"))
        with (output / "ci.log").open("w") as stream:
            child = subprocess.Popen(["sh", "scripts/ci-local.sh", "--parity"], cwd=tree,
                                     env=environment, stdout=stream, stderr=subprocess.STDOUT,
                                     start_new_session=True)
            api.atomic_write(output / "process.json", json.dumps({"pid": child.pid, "pgid": child.pid, "sha": sha}) + "\n")
            try:
                code = child.wait()
            except BaseException:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGTERM)  # our recorded child group only
                    try:
                        child.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                raise
        text = (output / "ci.log").read_text()
        failure = fail_name(text) if code else "-"
        culprit = "-"
        verdict = "red" if code else "green"
        if not code:
            try:
                retain_parity(tree, target, sha, api)
            except api.Refusal as error:
                code, verdict, failure = 1, "red", "parity_store_missing"
                api.atomic_write(output / "store-error.log", str(error) + "\n")
        record = {"sha": sha, "verdict": verdict, "started": started, "ended": api.now(),
                  "failing_test": failure, "culprit": culprit, "exit": code,
                  "log": str(output / "ci.log")}
        # Close the tree before diagnosis; no new code lands on a known red main.
        api.atomic_write(api.state_directory() / "main-status",
                         f"{verdict} {sha}" + (" " + failure if code else "") + "\n")
        if code:
            green = next((r["sha"] for r in reversed(history) if r["verdict"] == "green"), None)
            if green:
                selector = output / "test-selector.json"
                api.atomic_write(selector, json.dumps(test_selector(text)) + "\n")
                diagnosis = api.command(["sh", "scripts/culprit.sh", green, sha, failure],
                                        cwd=tree, check=False,
                                        env=dict(os.environ, BBOXY_FAILED_TEST_SELECTOR=str(selector)))
                api.atomic_write(output / "culprit.log", diagnosis.stdout + diagnosis.stderr)
                if diagnosis.returncode == 0:
                    try:
                        answer = json.loads(diagnosis.stdout)
                    except ValueError:
                        answer = {}
                        record["diagnostic_error"] = "invalid_culprit_json"
                    if answer.get("verdict") == "flake":
                        verdict = record["verdict"] = "green"
                        record["flake"] = True
                        api.atomic_write(api.state_directory() / "main-status", f"green {sha}\n")
                    elif answer.get("culprit"):
                        culprit = record["culprit"] = answer["culprit"]
                        reverted = api.command(["sh", "scripts/revert.sh", culprit], cwd=tree, check=False)
                        api.atomic_write(output / "revert.log", reverted.stdout + reverted.stderr)
                        record["revert_exit"] = reverted.returncode
                else:
                    record["diagnostic_exit"] = diagnosis.returncode
            else:
                record["diagnostic_error"] = "no_last_green"
        api.append_record("postsubmit.log", record)
        print(json.dumps(record, sort_keys=True), flush=True)
        return 0 if verdict == "green" else 1
    finally:
        # Only our newly-created detached checkout. Keep its target for the next
        # batch and all command output in the machine-wide evidence directory.
        api.git("worktree", "remove", "--force", str(tree), cwd=root)


def run(arguments, api):
    parser = argparse.ArgumentParser()
    parser.add_argument("--once", action="store_true")
    args = parser.parse_args(arguments)
    def terminate(signum, frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, terminate)
    root = api.repository()
    with Lease(api):
        while True:
            code = one(root, api)
            if args.once:
                return code
            time.sleep(60)
