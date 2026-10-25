"""Landing contracts exercised only on private throwaway repositories (#323 §12)."""
import importlib.util
import atexit
import json
import os
from pathlib import Path
import subprocess
import shutil
import tempfile
import shlex
import sys
import time
import unittest
from unittest.mock import patch
from concurrent.futures import ThreadPoolExecutor


SOURCE = Path(__file__).with_name("landing.py")
spec = importlib.util.spec_from_file_location("landing", SOURCE)
landing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(landing)


def run(*args, cwd, **kwargs):
    return subprocess.run(args, cwd=cwd, capture_output=True, text=True,
                          check=True, **kwargs).stdout.strip()


IDENTITY = None


def fixture_identity():
    global IDENTITY
    if IDENTITY is not None:
        return IDENTITY
    inherited = os.environ.get("BBOXY_LANDING_FIXTURE_IDENTITY")
    if inherited:
        data = json.loads(Path(inherited).read_text())
        IDENTITY = (data["key"], Path(data["wrapper"]))
        return IDENTITY
    directory = tempfile.TemporaryDirectory(prefix="landing-test-identity-")
    home = Path(directory.name)
    home.chmod(0o700)
    gpg = shutil.which("gpg")
    subprocess.run([gpg, "--homedir", str(home), "--batch", "--pinentry-mode", "loopback",
                    "--passphrase", "", "--quick-generate-key",
                    "Landing Fixture <fixture@bboxy450.invalid>", "ed25519", "sign", "0"],
                   check=True, capture_output=True)
    data = subprocess.check_output([gpg, "--homedir", str(home), "--with-colons", "--list-keys"],
                                   text=True, stderr=subprocess.DEVNULL)
    key = next(line.split(":")[9] for line in data.splitlines() if line.startswith("fpr:"))
    wrapper = home / "gpg"
    wrapper.write_text("#!/bin/sh\nexec " + shlex.quote(gpg) + " --homedir "
                       + shlex.quote(str(home)) + ' "$@"\n')
    wrapper.chmod(0o700)
    connection = subprocess.check_output(["gpg-connect-agent", "--homedir", str(home),
                                          "GETINFO pid", "/bye"], text=True)
    pid = next(int(line[2:]) for line in connection.splitlines() if line.startswith("D "))
    (home / "owned-agent.pid").write_text(str(pid) + "\n")
    def close():
        subprocess.run(["gpg-connect-agent", "--homedir", str(home), "KILLAGENT", "/bye"],
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        directory.cleanup()
    atexit.register(close)
    IDENTITY = (key, wrapper)
    return IDENTITY


class Repository:
    def __init__(self):
        self.temp = tempfile.TemporaryDirectory(prefix="landing-fixture-")
        self.root = Path(self.temp.name)
        run("git", "init", "-q", "--initial-branch=main", cwd=self.root)
        key, program = fixture_identity()
        for name, value in (("user.name", "Landing Fixture"),
                            ("user.email", "fixture@bboxy450.invalid"),
                            ("user.signingkey", key), ("gpg.program", str(program)),
                            ("commit.gpgsign", "true")):
            run("git", "config", name, value, cwd=self.root)
        self.write("Cargo.toml", '[workspace]\nresolver="3"\nmembers=["crates/a","crates/b","crates/c"]\n')
        for name, deps in (("a", ""), ("b", 'alias={package="a",path="../a"}\n'),
                           ("c", 'b={path="../b"}\n')):
            self.write(f"crates/{name}/Cargo.toml", f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2024"\n[dependencies]\n{deps}')
            self.write(f"crates/{name}/src/lib.rs", "pub fn value()->u8 { 1 }\n")
        run("cargo", "generate-lockfile", "--offline", cwd=self.root)
        self.base = self.commit("Fixture base")

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def commit(self, message):
        run("git", "add", ".", cwd=self.root)
        # A fresh per-run identity; never the project's or operator's key.
        run("git", "commit", "-q", "-m", message, cwd=self.root)
        return run("git", "rev-parse", "HEAD", cwd=self.root)

    def close(self):
        self.temp.cleanup()

    def tooling(self):
        self.state = self.root / ".git/fixture-state"
        self.state.mkdir()
        self.environment = dict(os.environ, BBOXY_LANDING_STATE_DIR=str(self.state),
                                CARGO_TARGET_DIR=str(self.state / "target"))
        for name in ("landing.py", "presubmit.sh", "land.sh", "quarantine.py", "quarantine.sh",
                     "main_watch.py", "main-watch.sh"):
            target = self.root / "scripts" / name
            target.parent.mkdir(exist_ok=True)
            shutil.copy2(SOURCE.with_name(name), target)
        self.write("scripts/cache-env.sh", "# isolated fixture; never the production cache policy\nexport CARGO_BUILD_JOBS=2\n")
        shell = '#!/bin/sh\nprintf "%s\\n" "$0 $*" >> "$BBOXY_LANDING_STATE_DIR/shell.log"\n'
        for name in ("invariants", "layering", "pending-flags", "kill-own-pids", "licenses",
                     "brand", "images", "cross-os", "never-block", "lab-check"):
            self.write(f"scripts/{name}.sh", shell)
        self.write("scripts/quarantine-test.sh", '#!/bin/sh\nexec cargo test "$@"\n')
        self.write("scripts/test-landing.py", 'print("nested fixture checks")\n')
        self.bin = self.state / "bin"
        self.bin.mkdir()
        real_cargo = shutil.which("cargo")
        cargo = '''#!/usr/bin/env python3
import json,os,pathlib,signal,subprocess,sys
state=pathlib.Path(os.environ["BBOXY_LANDING_STATE_DIR"])
args=sys.argv[1:]
if args[0]=="metadata":
    raise SystemExit(subprocess.run([REAL,*args]).returncode)
with (state/"cargo.jsonl").open("a") as f:f.write(json.dumps(args)+"\\n")
cfg=json.loads((state/"behavior.json").read_text()) if (state/"behavior.json").exists() else {}
if args[0]=="test" and cfg.get("quarantine"):
    name=cfg["quarantine"]
    if "--exact" in args:
        if cfg.get("retry_zero"):
            print("test result: ok. 0 passed; 0 failed; 0 ignored")
        elif cfg.get("retry_fail"):
            print("test "+name+" ... FAILED\\ntest result: FAILED. 0 passed; 1 failed; 0 ignored")
            raise SystemExit(101)
        else:print("test "+name+" ... ok\\ntest result: ok. 1 passed; 0 failed; 0 ignored")
        raise SystemExit(0)
    print("test "+name+" ... FAILED\\ntest result: FAILED. 0 passed; 1 failed; 0 ignored")
    raise SystemExit(101)
if cfg.get("block")==args[0]:
    (state/"blocked.pid").write_text(str(os.getpid()))
    signal.pause()
if cfg.get("fail")==args[0]:raise SystemExit(1)
if args[0]=="test" and cfg.get("hook"):
    done=state/"hook.done"
    if cfg.get("repeat_hook") or not done.exists():
        subprocess.run(["sh",cfg["hook"]],check=True)
        done.write_text("done")
print("test result: ok. 1 passed; 0 failed; 0 ignored")
'''.replace("REAL", repr(real_cargo))
        (self.bin / "cargo").write_text(cargo)
        (self.bin / "cargo").chmod(0o755)
        (self.bin / "icq").write_text('''#!/usr/bin/env python3
import json,os,pathlib,sys
state=pathlib.Path(os.environ["BBOXY_LANDING_STATE_DIR"])
args=sys.argv[1:]
with (state/"icq.jsonl").open("a") as f:f.write(json.dumps(args)+"\\n")
if args[0]=="--dry-run":
 print(json.dumps({"operation":"send","arguments":{"name":args[2],"text":args[3]}}))
else:
 cfg=json.loads((state/"behavior.json").read_text()) if (state/"behavior.json").exists() else {}
 if cfg.get("fail_announce"):raise SystemExit(1)
 print(json.dumps({"id":"fixture-receipt","state":"stored"}))
''')
        (self.bin / "icq").chmod(0o755)
        self.environment["PATH"] = str(self.bin) + os.pathsep + os.environ["PATH"]
        self.base = self.commit("Fixture tooling")

    def presubmit(self):
        return subprocess.run(["sh", "scripts/presubmit.sh", self.base], cwd=self.root,
                              env=self.environment, capture_output=True, text=True, timeout=15)

    def remote(self):
        self.origin = self.root / ".git/origin.git"
        run("git", "init", "--bare", "-q", "--initial-branch=main", str(self.origin), cwd=self.root)
        run("git", "remote", "add", "origin", str(self.origin), cwd=self.root)
        run("git", "push", "-q", "origin", "HEAD:main", cwd=self.root)
        run("git", "switch", "-q", "-c", "fixture-cut", cwd=self.root)

    def land(self):
        return subprocess.run(["sh", "scripts/land.sh"], cwd=self.root, env=self.environment,
                              capture_output=True, text=True, timeout=30)

    def remote_head(self):
        return run("git", "--git-dir", str(self.origin), "rev-parse", "main", cwd=self.root)

    def movement(self, path, repeat=False):
        other = self.root / ".git/other-worktree"
        run("git", "worktree", "add", "-q", "-b", "fixture-other", str(other), "origin/main", cwd=self.root)
        hook = self.state / "move-main.sh"
        hook.write_text("#!/bin/sh\nset -eu\ncd " + shlex.quote(str(other)) + "\n"
                        + 'count_file=' + shlex.quote(str(self.state / 'moves')) + '\n'
                        + 'n=$(cat "$count_file" 2>/dev/null || echo 0)\nn=$((n+1))\necho "$n" > "$count_file"\n'
                        + "mkdir -p " + shlex.quote(str(Path(path).parent)) + "\n"
                        + "printf '// movement %s\\n' \"$n\" > " + shlex.quote(path) + "\n"
                        + "git add -- " + shlex.quote(path) + "\n"
                        + 'git commit -qm "Fixture movement $n"\ngit push -q origin HEAD:main\n')
        (self.state / "behavior.json").write_text(json.dumps({"hook": str(hook), "repeat_hook": repeat}))

    def cli(self, *arguments):
        return subprocess.run(["python3", "-I", "scripts/landing.py", *arguments], cwd=self.root,
                              env=self.environment, capture_output=True, text=True, timeout=30)


class AffectedTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)

    def changed(self, path):
        self.repo.write(path, "// fixture change\n")
        self.repo.commit("Fixture change")
        return landing.affected(self.repo.base + "..HEAD", root=self.repo.root)

    def test_reverse_closure_includes_renamed_and_transitive_dependencies(self):
        result = self.changed("crates/a/src/lib.rs")
        self.assertEqual(result["touched"], ["a"])
        self.assertEqual(result["affected"], ["a", "b", "c"])

    def test_leaf_does_not_retest_its_dependencies(self):
        self.assertEqual(self.changed("crates/c/src/lib.rs")["affected"], ["c"])

    def test_shared_gate_input_affects_all_crates(self):
        result = self.changed("scripts/fixture-gate.sh")
        self.assertEqual(result["affected"], ["a", "b", "c"])
        self.assertEqual(result["shared"], ["scripts/fixture-gate.sh"])

    def test_lab_tier_has_no_product_target_and_shared_parity_is_marked(self):
        result = self.changed("lab/web/components.js")
        self.assertEqual(result["tier"], "lab")
        self.assertEqual(result["affected"], [])
        self.assertEqual(result["parity"], "yes")
        self.assertEqual(result["shared"], ["lab/web/components.js"])

    def test_records_do_not_request_native_parity(self):
        result = self.changed(".bboxy/fixture with spaces.md")
        self.assertEqual(result["tier"], "lab")
        self.assertEqual(result["parity"], "no")
        self.assertEqual(result["affected"], [])
        self.assertEqual(result["paths"], [".bboxy/fixture with spaces.md"])


class StateTests(unittest.TestCase):
    def test_canonical_state_is_shared_across_worktrees(self):
        with patch.dict(os.environ, {"XDG_STATE_HOME": "/tmp/landing-state-fixture"}, clear=True):
            self.assertEqual(str(landing.state_directory()),
                             "/tmp/landing-state-fixture/bboxy450-term/landing")

    def test_the_single_fixture_override_is_absolute_and_isolates_cache(self):
        with patch.dict(os.environ, {"BBOXY_LANDING_STATE_DIR": "/tmp/fixture-state"}, clear=True):
            self.assertEqual(landing.parity_directory(), Path("/tmp/fixture-state").resolve() / "parity")
        with patch.dict(os.environ, {"BBOXY_LANDING_STATE_DIR": "relative"}, clear=True):
            with self.assertRaises(landing.Refusal):
                landing.state_directory()


class PresubmitTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()

    def cut(self, path):
        self.repo.write(path, "// fixture cut\n")
        self.repo.commit("Fixture candidate")

    def test_code_targets_the_affected_leaf_and_records_one_verdict(self):
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        self.assertEqual(calls[0], ["test", "--locked", "-p", "c"])
        self.assertEqual(calls[1][:4], ["clippy", "--locked", "-p", "c"])
        rows = (self.repo.state / "presubmit.log").read_text().splitlines()
        self.assertEqual(len(rows), 1)
        self.assertEqual(json.loads(rows[0])["verdict"], "green")

    def test_absent_cache_adapter_uses_empty_wrapper_fallback(self):
        (self.repo.root / "scripts/cache-env.sh").unlink()
        self.repo.commit("Fixture without optional cache adapter")
        self.repo.environment["RUSTC_WRAPPER"] = "/unavailable-fixture-wrapper"
        self.repo.environment["RUSTC_WORKSPACE_WRAPPER"] = "/unavailable-fixture-wrapper"
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_failure_never_records_green(self):
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "behavior.json").write_text('{"fail":"test"}')
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0)
        record = json.loads((self.repo.state / "presubmit.log").read_text().splitlines()[-1])
        self.assertEqual(record["verdict"], "red")
        self.assertNotIn('"clippy"', (self.repo.state / "cargo.jsonl").read_text())

    def test_deadline_is_enforced_and_does_not_kill_an_unrelated_process(self):
        # keeps: #323 §3 deadline; kill-only-own-pids. A real blocked child,
        # not a wall-clock performance assertion or an arbitrary test sleep.
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "behavior.json").write_text('{"block":"test"}')
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":0.5}')
        control = subprocess.Popen(["python3", "-c", "import signal; signal.pause()"])
        try:
            result = self.repo.presubmit()
            self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
            self.assertIn("presubmit_timeout", result.stderr)
            self.assertIsNone(control.poll())
        finally:
            control.terminate()
            control.wait()
        record = json.loads((self.repo.state / "presubmit.log").read_text().splitlines()[-1])
        self.assertEqual(record["error"], "presubmit_timeout")
        self.assertEqual(record["exit"], 124)

    def test_fixture_cannot_raise_the_fifteen_minute_limit(self):
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":901}')
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("maximum is 900", result.stderr)
        self.assertFalse((self.repo.state / "cargo.jsonl").exists())

    def test_no_cargo_classifier_handles_environment_prefix_and_ui_lint(self):
        self.assertTrue(landing.cargo_backed("R5G_SEATS=4 cargo test -p bbt-mux"))
        self.assertTrue(landing.cargo_backed("sh scripts/ui-lint.sh"))
        self.assertFalse(landing.cargo_backed("sh scripts/brand.sh"))


class LandTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        self.repo.remote()

    def cut(self, path="crates/c/src/lib.rs", subject="Fixture candidate"):
        self.repo.write(path, "// fixture candidate\n")
        return self.repo.commit(subject)

    def cargo_test_count(self):
        path = self.repo.state / "cargo.jsonl"
        return sum(json.loads(line)[0] == "test" for line in path.read_text().splitlines()) if path.exists() else 0

    def test_signed_cut_pushes_once_logs_and_announces_after_slot_release(self):
        # keeps: #323 §5 fast-forward/signature/log/communication contract.
        sha = self.cut()
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), sha)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["sha"], sha)
        self.assertEqual(record["affected"], ["c"])
        self.assertFalse(list((self.repo.state / "slots").glob("bboxy450-gate-slot-*")))
        self.assertTrue((self.repo.state / "announced" / (sha + ".json")).exists())

    def test_red_main_refuses_before_any_check_or_push(self):
        self.cut()
        (self.repo.state / "main-status").write_text("red " + self.repo.base + " fixture\n")
        result = self.repo.land()
        self.assertEqual(result.returncode, 76, result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_one_non_revert_commit_prevents_the_red_exception(self):
        self.cut(subject='Revert "fixture"')
        self.cut("crates/b/src/lib.rs", "Ordinary fixture")
        (self.repo.state / "main-status").write_text("red " + self.repo.base + " fixture\n")
        self.assertEqual(self.repo.land().returncode, 76)

    def test_presubmit_failure_is_77_and_does_not_push(self):
        self.cut()
        (self.repo.state / "behavior.json").write_text('{"fail":"test"}')
        result = self.repo.land()
        self.assertEqual(result.returncode, 77, result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_conflict_is_78_and_left_for_the_author_to_resolve(self):
        self.cut()
        self.repo.movement("crates/c/src/lib.rs")
        hook = json.loads((self.repo.state / "behavior.json").read_text())["hook"]
        run("sh", hook, cwd=self.repo.root)
        result = self.repo.land()
        self.assertEqual(result.returncode, 78, result.stdout + result.stderr)
        self.assertTrue((self.repo.root / ".git/rebase-merge").exists())

    def test_disjoint_main_movement_rebases_but_does_not_repeat_checks(self):
        self.cut()
        self.repo.movement(".bboxy/other.md")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        self.assertIn("reusing presubmit", result.stdout)
        self.assertEqual(run("git", "show", self.repo.remote_head()+":.bboxy/other.md", cwd=self.repo.root), "// movement 1")

    def test_reverse_dependency_overlap_rechecks(self):
        self.cut()
        self.repo.movement("crates/a/src/lib.rs")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 2)

    def test_three_moving_attempts_are_79_with_no_candidate_push(self):
        sha = self.cut()
        self.repo.movement("crates/a/src/lib.rs", repeat=True)
        result = self.repo.land()
        self.assertEqual(result.returncode, 79, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 3)
        self.assertNotEqual(self.repo.remote_head(), sha)

    def test_busy_slots_are_75_without_a_check(self):
        self.cut()
        for n in (1, 2, 3):
            slot = self.repo.state / "slots" / f"bboxy450-gate-slot-{n}"
            slot.mkdir(parents=True)
            (slot / "pid").write_text(str(os.getpid()))
        result = self.repo.land()
        self.assertEqual(result.returncode, 75, result.stderr)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_notification_failure_can_resume_without_rechecking_or_repush(self):
        sha = self.cut()
        (self.repo.state / "behavior.json").write_text('{"fail_announce":true}')
        self.assertNotEqual(self.repo.land().returncode, 0)
        self.assertEqual(self.repo.remote_head(), sha)
        (self.repo.state / "behavior.json").write_text('{}')
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        self.assertEqual(len((self.repo.state / "landings.log").read_text().splitlines()), 1)

    def test_unsigned_input_is_refused_before_rebase_can_sign_it(self):
        self.repo.write("crates/c/src/lib.rs", "// unsigned fixture\n")
        run("git", "add", ".", cwd=self.repo.root)
        run("git", "-c", "commit.gpgsign=false", "commit", "-qm", "Unsigned fixture", cwd=self.repo.root)
        result = self.repo.land()
        self.assertIn("signature_or_author_refused", result.stderr)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_all_revert_cut_can_land_while_main_is_red(self):
        sha = self.cut(subject='Revert "fixture"')
        (self.repo.state / "main-status").write_text("red " + self.repo.base + " fixture\n")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), sha)


class QuarantineTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        self.repo.write(".bboxy/quarantine.md", "| Test | Crate | Owner | Task | Since | Expires | Evidence |\n|---|---|---|---|---|---|---|\n")
        self.repo.write(".bboxy/tasks/fixture.md", "---\nnumber: 123\n---\n# Fixture task\n")
        self.repo.write("crates/c/src/lib.rs", "#[test] fn fixture_test() {}\n")
        shutil.copy2(SOURCE.with_name("quarantine-test.sh"), self.repo.root / "scripts/quarantine-test.sh")
        (self.repo.state / "behavior.json").write_text('{"quarantine":"fixture_test"}')
        self.repo.commit("Fixture quarantine")

    def listed(self, expires="2999-01-01"):
        with (self.repo.root / ".bboxy/quarantine.md").open("a") as stream:
            stream.write(f"| `fixture_test` | c | Fixture | [[#123]] | 2026-01-01 | {expires} | fixture proof |\n")

    def test_add_runs_exact_test_links_task_and_expires_in_seven_days(self):
        result = self.repo.cli("quarantine", "add", "fixture_test", "--owner", "Fixture", "--task", "123")
        self.assertEqual(result.returncode, 0, result.stderr)
        row = json.loads(self.repo.cli("quarantine", "list").stdout)[0]
        from datetime import date
        self.assertEqual((date.fromisoformat(row["expires"]) - date.fromisoformat(row["since"])).days, 7)
        self.assertIn("Quarantine evidence", (self.repo.root / ".bboxy/tasks/fixture.md").read_text())
        self.assertTrue(Path(json.loads(result.stdout)["evidence"]).is_file())

    def test_no_owner_or_task_or_no_actual_test_refuses_admission(self):
        self.assertNotEqual(self.repo.cli("quarantine", "add", "fixture_test").returncode, 0)
        (self.repo.state / "behavior.json").write_text('{"quarantine":"fixture_test","retry_zero":true}')
        result = self.repo.cli("quarantine", "add", "fixture_test", "--owner", "Fixture", "--task", "123")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(json.loads(self.repo.cli("quarantine", "list").stdout), [])

    def test_expired_row_blocks_then_expire_removes_it(self):
        self.listed("2000-01-01")
        result = self.repo.cli("quarantine-test", "--workspace", "--locked")
        self.assertEqual(result.returncode, 101, result.stderr)
        self.assertIn("expired", result.stderr)
        self.assertEqual(self.repo.cli("quarantine", "expire").returncode, 0)
        self.assertEqual(json.loads(self.repo.cli("quarantine", "list").stdout), [])

    def test_unexpired_failure_passes_only_after_actual_exact_retry(self):
        self.listed()
        result = self.repo.cli("quarantine-test", "--workspace", "--locked", "old_filter")
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        self.assertIn("--no-fail-fast", calls[0])
        self.assertEqual(calls[1], ["test", "--locked", "-p", "c", "--", "--exact", "fixture_test"])

    def test_failure_alone_and_zero_test_retry_both_block(self):
        self.listed()
        for flag in ("retry_fail", "retry_zero"):
            with self.subTest(flag=flag):
                (self.repo.state / "behavior.json").write_text(json.dumps({"quarantine": "fixture_test", flag: True}))
                self.assertNotEqual(self.repo.cli("quarantine-test", "--workspace").returncode, 0)


class WatchTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        self.repo.write(".gitignore", "scratchpad/\n")
        self.repo.write("scripts/ci-local.sh", '''#!/bin/sh
set -eu
[ "$BBOXY_GATE_SLOT" = none ]
[ "$CARGO_BUILD_JOBS" = 2 ]
python3 - <<'PY'
import json,os,pathlib,sys
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
with (state/'ci-calls').open('a') as f:f.write('ci --parity\\n')
cfg=json.loads((state/'watch.json').read_text()) if (state/'watch.json').exists() else {}
if cfg.get('fail'):
 print('test fixture_failed ... FAILED')
 print("error: test failed, to rerun pass '-p c --test fixture'")
 raise SystemExit(101)
if not cfg.get('missing_store'):
 target=pathlib.Path(os.environ['CARGO_TARGET_DIR'])
 (target/'parity/native').mkdir(parents=True,exist_ok=True)
 (target/'parity/native/fixture.png').write_bytes(b'fixture native pixels')
 (target/'parity/cases.json').write_text('[]')
 (target/'debug').mkdir(exist_ok=True)
 (target/'debug/bboxybook').write_bytes(b'fixture binary')
print('ci-local: all gates passed')
PY
''')
        self.repo.write("scripts/culprit.sh", '''#!/bin/sh
printf '%s\\n' "$3" > "$BBOXY_LANDING_STATE_DIR/culprit-selector"
python3 - "$1" "$2" <<'PY'
import json,os,pathlib,sys
s=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
c=json.loads((s/'watch.json').read_text())
if c.get('unresolved'):
 print(json.dumps({'verdict':'unresolved','revert_range':sys.argv[1]+'..'+sys.argv[2]}))
 raise SystemExit(2)
print(json.dumps(c.get('diagnosis',{'verdict':'culprit','culprit':'f'*40})))
PY
''')
        self.repo.write("scripts/revert.sh", '#!/bin/sh\nprintf "%s\\n" "$1" > "$BBOXY_LANDING_STATE_DIR/reverted"\n')
        self.repo.base = self.repo.commit("Fixture postsubmit")
        self.repo.remote()

    def watch(self):
        return self.repo.cli("main-watch", "--once")

    def seed_green(self):
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stderr)

    def publish(self, path):
        self.repo.write(path, "fixture change\n")
        sha = self.repo.commit("Fixture main movement")
        run("git", "push", "-q", "origin", "HEAD:main", cwd=self.repo.root)
        return sha

    def test_green_keeps_native_set_and_same_main_runs_once(self):
        self.seed_green()
        pointer = Path((self.repo.state / "parity-last-green").read_text().strip())
        self.assertEqual((pointer / "native/fixture.png").read_bytes(), b"fixture native pixels")
        self.assertEqual((self.repo.state / "main-status").read_text(), "green " + self.repo.base + "\n")
        self.assertEqual(self.watch().returncode, 0)
        self.assertEqual(len((self.repo.state / "ci-calls").read_text().splitlines()), 1)
        self.assertFalse((self.repo.state / "main-watch.lock").exists())

    def test_records_only_carries_verdict_but_lab_changes_run_full_postsubmit(self):
        self.seed_green()
        self.publish(".bboxy/fixture.md")
        self.assertEqual(self.watch().returncode, 0)
        self.assertEqual(len((self.repo.state / "ci-calls").read_text().splitlines()), 1)
        self.publish("lab/web/fixture.html")
        self.assertEqual(self.watch().returncode, 0)
        self.assertEqual(len((self.repo.state / "ci-calls").read_text().splitlines()), 2)

    def test_red_closes_main_and_invokes_culprit_then_revert(self):
        self.seed_green()
        sha = self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text('{"fail":true}')
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual((self.repo.state / "main-status").read_text(), "red " + sha + " fixture_failed\n")
        self.assertEqual((self.repo.state / "reverted").read_text().strip(), "f" * 40)
        self.assertEqual((self.repo.state / "culprit-selector").read_text().strip(), "cargo:c:test/fixture:fixture_failed")

    def test_three_pass_flake_verdict_reopens_without_revert(self):
        self.seed_green()
        sha = self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text('{"fail":true,"diagnosis":{"verdict":"flake"}}')
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.repo.state / "main-status").read_text(), "green " + sha + "\n")
        self.assertFalse((self.repo.state / "reverted").exists())

    def test_unresolved_failure_reverts_only_the_exact_failed_batch(self):
        self.seed_green()
        sha = self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text('{"fail":true,"unresolved":true}')
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual((self.repo.state / "reverted").read_text().strip(), self.repo.base + ".." + sha)

    def test_green_without_native_store_cannot_be_published(self):
        (self.repo.state / "watch.json").write_text('{"missing_store":true}')
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("red ", (self.repo.state / "main-status").read_text())


class ParityTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="landing-parity-fixture-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.state = self.root / "state"
        self.state.mkdir()
        self.cache = self.root / "parity"
        self.cases = [{"page": "launcher.html", "component": "launcher", "slug": "a"},
                      {"page": "launcher.html", "component": "launcher", "slug": "b"},
                      {"page": "settings.html", "component": "settings", "slug": "a"}]
        self.calls = []
        self.addCleanup(patch.stopall)
        patch.object(landing, "state_directory", return_value=self.state).start()
        patch.object(landing, "parity_directory", return_value=self.cache).start()
        patch.object(landing, "checked", side_effect=self.checked).start()
        patch.object(landing, "command", return_value=subprocess.CompletedProcess([], 0, json.dumps(self.cases))).start()

    def checked(self, arguments, root):
        arguments = list(map(str, arguments))
        self.calls.append(arguments)
        if arguments[0] == "node":
            out = Path(arguments[2])
            (out / "cases.json").write_text(Path(arguments[3]).read_text())
        elif len(arguments) > 1 and arguments[1] == "render":
            native = Path(arguments[arguments.index("--out") + 1])
            native.mkdir()
            (native / "fixture.png").write_bytes(b"rendered native")

    def stored(self):
        store = self.cache / "fixture-green"
        (store / "native").mkdir(parents=True)
        (store / "native/fixture.png").write_bytes(b"last green native")
        (store / "bboxybook").write_bytes(b"last green binary")
        (store / "cases.json").write_text(json.dumps(self.cases))
        (self.state / "parity-last-green").write_text(str(store))
        return store

    def data(self, paths, shared=()):
        return {"tier": "lab", "paths": paths, "shared": list(shared)}

    def test_page_edit_compares_stored_native_without_any_product_build(self):
        store = self.stored()
        output = self.root / "out"
        landing.scoped_parity(self.root, self.data(["lab/web/launcher.html"]), output)
        self.assertEqual(json.loads((output / "requested.json").read_text()), self.cases[:2])
        self.assertFalse(any(c[0] == "cargo" for c in self.calls))
        self.assertEqual((output / "native/fixture.png").read_bytes(), b"last green native")
        self.assertTrue(any(c[:2] == [str(store.resolve() / "bboxybook"), "compare"] for c in self.calls))

    def test_shared_lab_input_selects_one_fixed_case_per_page(self):
        self.stored()
        output = self.root / "out"
        path = "lab/web/components.js"
        landing.scoped_parity(self.root, self.data([path], [path]), output)
        selected = json.loads((output / "requested.json").read_text())
        self.assertEqual(len(selected), 2)
        self.assertEqual(selected, landing.parity_cases(list(reversed(self.cases)), self.data([path], [path])))

    def test_missing_store_builds_bboxybook_alone(self):
        output = self.root / "out"
        landing.scoped_parity(self.root, self.data(["lab/web/launcher.html"]), output)
        cargo = [c for c in self.calls if c[0] == "cargo"]
        self.assertEqual(cargo, [["cargo", "build", "-p", "bboxybook", "--locked"]])
        self.assertEqual((output / "native/fixture.png").read_bytes(), b"rendered native")

    def test_uncatalogued_page_needs_lab_check_but_no_capture(self):
        self.stored()
        landing.scoped_parity(self.root, self.data(["lab/web/showcase.html"]), self.root / "out")
        self.assertEqual(self.calls, [["sh", "scripts/lab-check.sh"]])

    def test_per_binary_duration_over_sixty_is_reported(self):
        result = landing.binary_durations("   Running tests/fixture.rs (target/debug/deps/fixture-abc)\n"
                                          "test result: ok. 1 passed; 0 failed; finished in 61.25s\n"
                                          "   Doc-tests fixture\n"
                                          "test result: ok. 0 passed; 0 failed; finished in 0.00s\n")
        self.assertEqual(result, [{"binary": "target/debug/deps/fixture-abc", "seconds": 61.25, "over_60s": True},
                                  {"binary": "doc-tests:fixture", "seconds": 0., "over_60s": False}])


def fixture_suite(extended=False):
    # Every test has an independent mktemp repo and local bare origin. Isolated
    # subprocesses keep mocks apart; one fresh test-only signer avoids 39 agents.
    key, wrapper = fixture_identity()
    identity = wrapper.parent / "fixture-identity.json"
    identity.write_text(json.dumps({"key": key, "wrapper": str(wrapper)}))
    environment = dict(os.environ, BBOXY_LANDING_FIXTURE_IDENTITY=str(identity))
    def flatten(suite):
        for item in suite:
            if isinstance(item, unittest.TestSuite):
                yield from flatten(item)
            else:
                yield item.id().removeprefix("__main__.")
    names = list(flatten(unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])))
    extended_names = {"LandTests." + name for name in (
        "test_three_moving_attempts_are_79_with_no_candidate_push",
        "test_disjoint_main_movement_rebases_but_does_not_repeat_checks",
        "test_reverse_dependency_overlap_rechecks",
        "test_notification_failure_can_resume_without_rechecking_or_repush")}
    extended_names.add("WatchTests.test_unresolved_failure_reverts_only_the_exact_failed_batch")
    if not extended:
        names = [name for name in names if name not in extended_names]
        print("Fast CI fixtures; 5 extended moving-main/recovery cases run with --all (presubmit).", flush=True)
    # Put multi-rebase fixtures first, so their serial Git steps do not form a
    # long tail after the many small state/parity checks.
    names.sort(key=lambda name: (0 if "three_moving_attempts" in name else
                                 1 if name.startswith("LandTests.") else 2, name))
    def execute(name):
        began = time.monotonic()
        result = subprocess.run([sys.executable, "-I", __file__, name], env=environment,
                                text=True, capture_output=True, timeout=60)
        return name, result, time.monotonic() - began
    started = time.monotonic()
    failures = 0
    with ThreadPoolExecutor(max_workers=min(8, os.cpu_count() or 1)) as pool:
        for name, result, seconds in pool.map(execute, names):
            if seconds > 5:
                print(f"fixture duration: {name} {seconds:.3f}s", flush=True)
            if result.returncode:
                failures += 1
                print(name + " FAILED\n" + result.stdout + result.stderr, flush=True)
    print(f"Ran {len(names)} fixture tests in {time.monotonic() - started:.3f}s; failures={failures}", flush=True)
    return bool(failures)


if __name__ == "__main__":
    if len(sys.argv) == 1 or sys.argv[1:] == ["--all"]:
        raise SystemExit(fixture_suite(extended=sys.argv[1:] == ["--all"]))
    unittest.main()
