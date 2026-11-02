"""Seven-day quarantine administration and exact, logged retries (#323 §7)."""
import argparse
from datetime import date, datetime, timedelta, timezone
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import uuid
import os
import xml.etree.ElementTree as ElementTree


class TestCoverage:
    """Only observed libtest outcomes, tied to this attempt, source and target."""
    def __init__(self, root, arguments, api):
        self.root, self.api = root, api
        self.path = os.environ.get("BBOXY_PRESUBMIT_COVERAGE")
        self.attempt = os.environ.get("BBOXY_PRESUBMIT_ATTEMPT")
        self.packages = api.metadata(root) if self.path and self.attempt else {}
        self.current = None
        self.tests = []
        target = os.environ.get("BBOXY_CARGO_TARGET_DIR", "target")
        for i, arg in enumerate(arguments):
            if arg == "--target-dir" and i + 1 < len(arguments):
                target = arguments[i + 1]
            elif arg.startswith("--target-dir="):
                target = arg.split("=", 1)[1]
        self.target = str((root / target).resolve())
        self.sha = api.git("rev-parse", "HEAD", cwd=root) if self.packages else None

    def observe(self, line):
        if not self.packages:
            return
        line = re.sub(r"\x1b\[[0-9;]*m", "", line).strip()
        heading = re.fullmatch(r"Running (.+) \((.+)\)", line)
        if heading:
            source, binary = heading.groups()
            source = source.removeprefix("unittests ")
            stem = re.sub(r"-[0-9a-f]{8,}$", "", Path(binary).name)
            candidates = []
            for package, info in self.packages.items():
                for target in info["targets"]:
                    kinds = target["kind"]
                    kind = ("lib" if set(kinds) & {"lib", "proc-macro"} else
                            "test" if "test" in kinds else "bin" if "bin" in kinds else None)
                    if (kind and target["name"].replace("-", "_") == stem.replace("-", "_")
                            and str(target["src_path"]).endswith("/" + source)):
                        candidates.append((package, kind, target["name"], Path(binary).name))
            self.current = candidates[0] if len(candidates) == 1 else None
        elif line.startswith(("Doc-tests ", "test result:")):
            self.current = None
        elif self.current:
            result = re.fullmatch(r"test (\S+) \.\.\. (ok|FAILED|ignored)(?:,.*| .*)?", line)
            if result:
                package, kind, target, binary = self.current
                self.tests.append({"package": package, "kind": kind, "target": target,
                                   "binary": binary, "test": result[1], "result": result[2]})

    def passed(self):
        if self.packages and self.sha == self.api.git("rev-parse", "HEAD", cwd=self.root):
            self.api.atomic_write(Path(self.path), json.dumps({"sha": self.sha, "target": self.target,
                "attempt": self.attempt, "tests": self.tests}) + "\n")


def rows(root):
    path = root / ".bboxy/quarantine.md"
    result = []
    for line in path.read_text().splitlines() if path.exists() else []:
        fields = [f.strip().strip("`") for f in line.strip().strip("|").split("|")]
        if line.startswith("| `") and len(fields) == 7:
            test, crate, owner, task, since, expires, evidence = fields
            result.append(dict(test=test, crate=crate, owner=owner, task=task,
                               since=since, expires=expires, evidence=evidence,
                               expired=date.fromisoformat(expires) < datetime.now(timezone.utc).date()))
    return result


def administer(arguments, api):
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("add", "expire", "list"))
    parser.add_argument("test", nargs="?")
    parser.add_argument("--owner")
    parser.add_argument("--task", type=int)
    parser.add_argument("--crate")
    args = parser.parse_args(arguments)
    root = api.repository()
    path = root / ".bboxy/quarantine.md"
    entries = rows(root)
    if args.action == "list":
        print(json.dumps(entries, sort_keys=True))
        return 0
    if args.action == "expire":
        expired = {r["test"] for r in entries if r["expired"]}
        text = "\n".join(line for line in path.read_text().splitlines()
                         if not any(line.startswith(f"| `{test}` |") for test in expired)) + "\n"
        api.atomic_write(path, text)
        print(json.dumps({"expired": sorted(expired)}))
        return 0
    if not args.test or not args.owner or not args.task or args.task < 1:
        raise api.Refusal("quarantine_add_requires_test_owner_task")
    if any(c in value for value in (args.test, args.owner) for c in "|\n\r`"):
        raise api.Refusal("quarantine_invalid_table_value")
    if any(r["test"] == args.test for r in entries):
        raise api.Refusal("quarantine_already_listed: " + args.test)
    tasks = [p for p in (root / ".bboxy/tasks").glob("*.md")
             if re.search(r"(?m)^number:\s*" + str(args.task) + r"\s*$", p.read_text())]
    if len(tasks) != 1:
        raise api.Refusal("quarantine_task_not_unique: " + str(args.task))
    packages = api.metadata(root)
    crate = args.crate
    if crate is None:
        leaf = args.test.split("::")[-1]
        matches = {name for name, package in packages.items()
                   if any(re.search(r"\bfn\s+" + re.escape(leaf) + r"\b", p.read_text())
                          for p in Path(package["manifest_path"]).parent.rglob("*.rs"))}
        if len(matches) != 1:
            raise api.Refusal("quarantine_crate_ambiguous: supply --crate")
        crate = matches.pop()
    if crate not in packages:
        raise api.Refusal("quarantine_unknown_crate: " + crate)
    result = api.command(["cargo", "test", "--locked", "-p", crate, args.test,
                          "--", "--exact", args.test], cwd=root, check=False)
    passed = re.search(r"(?m)^test " + re.escape(args.test) + r" \.\.\. ok$", result.stdout)
    evidence = api.state_directory() / "quarantine" / (uuid.uuid4().hex + ".log")
    api.atomic_write(evidence, result.stdout + result.stderr)
    if result.returncode or not passed:
        raise api.Refusal("quarantine_isolated_retry_failed_or_zero_tests: " + str(evidence), 1)
    today = datetime.now(timezone.utc).date()
    sha = api.git("rev-parse", "HEAD", cwd=root)
    row = f"| `{args.test}` | {crate} | {args.owner} | [[#{args.task}]] | {today} | {today + timedelta(days=7)} | isolated retry at {sha}; {evidence} |\n"
    api.atomic_write(path, path.read_text().rstrip() + "\n" + row)
    with tasks[0].open("a") as stream:
        stream.write(f"\nQuarantine evidence ({today}): `{args.test}` at `{sha}` passed an isolated exact retry; raw `{evidence}`. Expires {today + timedelta(days=7)}.\n")
    print(json.dumps({"test": args.test, "crate": crate, "task": args.task,
                      "expires": str(today + timedelta(days=7)), "evidence": str(evidence),
                      "review_required": len(entries) + 1 > 5}))
    return 0


def retry_args(arguments, crate, test, api):
    # Remove old package/name filters. Preserve build/feature/target arguments as
    # arrays, so a retry cannot silently select zero tests or split quoted args.
    kept = []
    valued = {"--features", "-F", "--target", "--target-dir", "--manifest-path",
              "--profile", "--test", "--bin", "--example", "--color", "-j", "--jobs", "--config"}
    i = 0
    while i < len(arguments) and arguments[i] != "--":
        arg = arguments[i]
        i += 1
        if arg in ("-p", "--package", "--exclude"):
            i += 1
        elif arg in ("--workspace", "--all", "--no-fail-fast") or arg.startswith(("--package=", "--exclude=")) or (arg.startswith("-p") and not arg.startswith("--")):
            continue
        elif arg in valued:
            if i == len(arguments):
                raise api.Refusal("quarantine_invalid_cargo_arguments")
            kept.extend((arg, arguments[i]))
            i += 1
        elif arg.startswith("-"):
            kept.append(arg)
    return ["cargo", "test", *kept, "-p", crate, "--", "--exact", test]


def nextest(api):
    """cargo-nextest is the default runner; BBOXY_TEST_RUNNER=cargo selects libtest."""
    runner = os.environ.get("BBOXY_TEST_RUNNER", "auto")
    if runner not in ("auto", "nextest", "cargo"):
        raise api.Refusal("BBOXY_TEST_RUNNER must be auto, nextest or cargo")
    found = shutil.which("cargo-nextest") is not None
    if runner == "nextest" and not found:
        raise api.Refusal("cargo-nextest missing: run make tools")
    return runner != "cargo" and found


def junit_tests(path, packages):
    """Coverage rows from nextest JUnit: binary ids are pkg, pkg::test, pkg::bin/name."""
    rows = []
    for suite in ElementTree.parse(path).getroot().iter("testsuite"):
        package, _, rest = suite.get("name", "").partition("::")
        kind, name = ("lib", None) if not rest else ("bin", rest[4:]) if rest.startswith("bin/") \
            else (None, None) if "/" in rest else ("test", rest)
        info = packages.get(package)
        if kind is None or info is None:
            continue
        if name is None:
            name = next((t["name"] for t in info["targets"] if set(t["kind"]) & {"lib", "proc-macro"}), package)
        for case in suite.iter("testcase"):
            failed = case.find("failure") is not None or case.find("error") is not None
            skipped = case.find("skipped") is not None
            rows.append({"package": package, "kind": kind, "target": name, "binary": suite.get("name"),
                         "test": case.get("name"), "result": "FAILED" if failed else "ignored" if skipped else "ok"})
    return rows


def nextest_run(root, cargo_args, target, api):
    """Every test binary of the selection runs concurrently; doctests run after, by cargo."""
    split = cargo_args.index("--") if "--" in cargo_args else len(cargo_args)
    build, filters = cargo_args[:split], cargo_args[split + 1:]
    junit = Path(target) / "nextest" / "default" / "junit.xml"
    junit.unlink(missing_ok=True)
    argv = ["cargo", "nextest", "run", *build, *(["--", *filters] if filters else [])]
    print("quarantine: " + " ".join(argv), flush=True)
    code = subprocess.run(argv, cwd=root).returncode
    packages = api.metadata(root)
    tests = junit_tests(junit, packages) if junit.exists() else []
    output = ""
    selective = {"--test", "--lib", "--bin", "--bins", "--tests", "--example", "--examples"}
    if not selective & set(build) and code in (0, 100):
        chosen = [build[i + 1] for i, a in enumerate(build[:-1]) if a in ("-p", "--package")]
        if "--workspace" in build or "--all" in build:
            chosen = list(packages)
        libraries = [n for n in chosen if n in packages and any(
            set(t["kind"]) & {"lib", "proc-macro"} and t.get("doctest", True) for t in packages[n]["targets"])]
        if libraries:
            kept = [a for i, a in enumerate(build) if a not in ("-p", "--package", "--workspace", "--all")
                    and (i == 0 or build[i - 1] not in ("-p", "--package"))]
            doc = ["cargo", "test", "--doc", *kept, *[x for n in libraries for x in ("-p", n)],
                   *(["--", *filters] if filters else [])]
            print("quarantine: " + " ".join(doc), flush=True)
            result = subprocess.run(doc, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            output = result.stdout
            print(output, end="", flush=True)
            if result.returncode:
                return result.returncode, tests, output + "\ndoctest failed\n"
    return code, tests, output


def test(arguments, api):
    root = api.repository()
    if arguments and arguments[0] == "--":
        arguments = arguments[1:]
    cargo_args = list(arguments)
    split = cargo_args.index("--") if "--" in cargo_args else len(cargo_args)
    if "--no-fail-fast" not in cargo_args[:split]:
        cargo_args.insert(split, "--no-fail-fast")
    coverage = TestCoverage(root, cargo_args, api)
    if nextest(api):
        code, coverage.tests, output = nextest_run(root, cargo_args, coverage.target, api)
        if code == 0:
            coverage.passed()
            return 0
        failed = [row["test"] for row in coverage.tests if row["result"] == "FAILED"]
        if code != 100 or not failed or "doctest failed" in output:
            return code or 1
        return retry_quarantined(failed, arguments, coverage, root, api, code)
    # One ordered stream preserves Cargo's binary headings alongside libtest's
    # per-binary durations; --no-fail-fast guarantees later suites still run.
    child = subprocess.Popen(["cargo", "test", *cargo_args], cwd=root, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    lines = []
    for line in child.stdout:
        lines.append(line)
        coverage.observe(line)
        print(line, end="", flush=True)
    child.stdout.close()
    result = subprocess.CompletedProcess(child.args, child.wait(), "".join(lines), "")
    if not result.returncode:
        coverage.passed()
        return 0
    failed = re.findall(r"(?m)^test (.+) \.\.\. FAILED$", result.stdout)
    counts = sum(map(int, re.findall(r"test result: FAILED\..*? (\d+) failed", result.stdout)))
    if not failed or len(failed) != len(set(failed)) or counts != len(failed):
        return result.returncode
    if re.search(r"could not compile|could not execute|doctest failed", result.stdout):
        return result.returncode
    return retry_quarantined(failed, arguments, coverage, root, api, result.returncode)


def retry_quarantined(failed, arguments, coverage, root, api, code=1):
    """Only listed, unexpired tests are retried, once each and alone; anything else fails."""
    entries = {row["test"]: row for row in rows(root) if not row["expired"]}
    if len(failed) != len(set(failed)):
        return code
    for name in failed:
        if name not in entries:
            print("quarantine: unlisted or expired: " + name, file=sys.stderr)
            return code
    for name in failed:
        retry = api.command(retry_args(arguments, entries[name]["crate"], name, api), cwd=root, check=False)
        print(retry.stdout + retry.stderr, end="", flush=True)
        passed = re.search(r"(?m)^test " + re.escape(name) + r" \.\.\. ok$", retry.stdout)
        record = {"timestamp": api.now(), "sha": api.git("rev-parse", "HEAD", cwd=root),
                  "test": name, "crate": entries[name]["crate"], "exit": retry.returncode,
                  "verdict": "flake" if retry.returncode == 0 and passed else "failed"}
        api.append_record("quarantine-retries.log", record)
        if record["verdict"] != "flake":
            return retry.returncode or 1
        print("quarantine: passed alone, still tracked: " + name, file=sys.stderr)
    coverage.passed()
    return 0
