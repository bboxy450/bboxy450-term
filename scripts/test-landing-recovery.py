"""# keeps: spec landing.md §§6,8,12; invariant main-red-bounded.

Real Git repositories and per-run signing keys, never the production remote or
landing state. Predicate executables are deterministic fixture processes.
"""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("landing_fixtures", SOURCE / "test-landing.py")
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.repo = fixtures.Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        for name in ("landing_recovery.py", "culprit.sh", "revert.sh", "main-red-bounded.sh", "scale-night.sh"):
            self.repo.write("scripts/" + name, (SOURCE / name).read_text())
        self.repo.write(".bboxy/specs/e2e-acceptance.md", "## 1. Promises\n### terminal\n- **T1** fixture\n")
        self.base = self.repo.commit("Fixture recovery tooling")
        self.repo.remote()
        self.predicate = self.repo.state / "predicate.py"
        self.predicate.write_text('''import json,os,pathlib,sys
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
sha=sys.argv[1]
with (state/'observed.jsonl').open('a') as f:f.write(json.dumps({'sha':sha})+'\\n')
mode=(state/'mode').read_text() if (state/'mode').exists() else 'normal'
if mode=='infra':raise SystemExit(2)
if mode=='pass':raise SystemExit(0)
raise SystemExit(int(pathlib.Path('bad').exists()))
''')
        self.options = {"recovery_test": [sys.executable, str(self.predicate), "{sha}"]}
        self.configure()

    def configure(self):
        (self.repo.state / "fixture.json").write_text(json.dumps(self.options))

    def command(self, verb, *args, environment=None):
        return subprocess.run(["sh", str(self.repo.root / "scripts" / (verb + ".sh")), *args],
                              cwd=self.repo.root, env=environment or self.repo.environment,
                              capture_output=True, text=True, timeout=120)

    def change(self, subject, bad=False, path="crates/c/src/lib.rs"):
        self.repo.write(path, "// " + subject + "\n")
        if bad:
            self.repo.write("bad", "failure\n")
        return self.repo.commit(subject)

    def publish(self):
        fixtures.run("git", "push", "-q", "origin", "HEAD:main", cwd=self.repo.root)

    def logs(self, rows):
        (self.repo.state / "postsubmit.log").write_text("".join(json.dumps(r) + "\n" for r in rows))

    # keeps: spec landing.md §6 — three reruns, first-parent bisect, caller unchanged.
    def test_finds_first_bad_landing_and_runs_three_times_per_candidate(self):
        self.change("good one")
        culprit = self.change("regression", bad=True)
        for n in range(5):
            self.change(f"following {n}")
        tip = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        branch = fixtures.run("git", "branch", "--show-current", cwd=self.repo.root)
        result = self.command("culprit", self.base, tip, "exact_test")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual((answer["verdict"], answer["culprit"]), ("culprit", culprit))
        counts = {}
        for trial in answer["trials"]:
            counts[trial["sha"]] = counts.get(trial["sha"], 0) + 1
            self.assertTrue(Path(trial["log"]).is_file())
        self.assertEqual(set(counts.values()), {3})
        self.assertLess(len(counts), 8)
        self.assertEqual(fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root), tip)
        self.assertEqual(fixtures.run("git", "branch", "--show-current", cwd=self.repo.root), branch)
        self.assertEqual(len(fixtures.run("git", "worktree", "list", "--porcelain", cwd=self.repo.root).split("worktree ")) - 1, 1)

    # keeps: spec landing.md §6 — merges are diagnosed as landings, not side commits.
    def test_first_parent_merge_is_the_culprit(self):
        fixtures.run("git", "switch", "-q", "-c", "fixture-side", cwd=self.repo.root)
        side = self.change("side regression", bad=True)
        fixtures.run("git", "switch", "-q", "fixture-cut", cwd=self.repo.root)
        self.change("main movement", path="crates/a/src/lib.rs")
        fixtures.run("git", "merge", "--no-ff", "-qm", "Fixture landing", side, cwd=self.repo.root)
        merge = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        result = self.command("culprit", self.base, merge, "exact_test")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["culprit"], merge)

    # keeps: spec landing.md §6 — a flake requires all three red reruns to pass.
    def test_three_passes_at_red_propose_quarantine_without_bisect(self):
        red = self.change("red fixture", bad=True)
        (self.repo.state / "mode").write_text("pass")
        result = self.command("culprit", self.base, red, "exact_test")
        self.assertEqual(result.returncode, 0, result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(answer["verdict"], "flake")
        self.assertTrue(answer["quarantine_proposed"])
        self.assertEqual([r["sha"] for r in answer["trials"]], [red] * 3)

    # keeps: spec landing.md §6 — infrastructure is unresolved, never a false flake.
    def test_infrastructure_failure_and_zero_tests_are_not_classified(self):
        red = self.change("red fixture", bad=True)
        (self.repo.state / "mode").write_text("infra")
        result = self.command("culprit", self.base, red, "exact_test")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(json.loads(result.stdout)["verdict"], "unresolved")
        self.options.pop("recovery_test")
        self.configure()
        selector = self.repo.state / "selector.json"
        selector.write_text(json.dumps({"name": "exact_test", "cargo_args": ["-p", "c", "--lib"], "source": "cargo_rerun"}))
        env = dict(self.repo.environment, BBOXY_FAILED_TEST_SELECTOR=str(selector))
        result = self.command("culprit", self.base, red, "exact_test", environment=env)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("test_not_observed", json.loads(result.stdout)["reason"])
        calls = [json.loads(x) for x in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        self.assertEqual(calls[-1], ["test", "--locked", "-p", "c", "--lib", "exact_test", "--", "--exact", "exact_test"])

    # keeps: spec landing.md §6 — non-first-parent baselines cannot drive a bisect.
    def test_bad_baseline_is_refused(self):
        red = self.change("red fixture", bad=True)
        result = self.command("culprit", red, red, "exact_test")
        self.assertEqual(result.returncode, 2)
        self.assertIn("last_green_must_precede", json.loads(result.stdout)["reason"])

    # keeps: spec landing.md §6 — actual Cargo executes only the named test.
    def test_exact_cargo_selector_does_not_run_the_other_test(self):
        self.options.pop("recovery_test")
        self.configure()
        self.repo.write("marker", "good")
        self.repo.write("crates/c/src/lib.rs", '''#[test]
fn regression() { assert_eq!(include_str!("../../../marker"), "good"); }
#[test]
fn unrelated() { panic!("must not run"); }
''')
        green = self.repo.commit("Fixture exact test")
        self.repo.write("marker", "bad")
        red = self.repo.commit("Fixture product regression")
        env = dict(self.repo.environment, PATH=os.environ["PATH"])
        result = self.command("culprit", green, red, "cargo:c:lib:regression", environment=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(answer["culprit"], red)
        self.assertEqual(len(answer["trials"]), 6)
        for row in answer["trials"]:
            self.assertNotIn("test unrelated", Path(row["log"]).read_text())
        self.assertFalse((Path(answer["evidence"]) / "target").exists())

    # keeps: spec landing.md §§5,6 — signed revert reaches the local remote via land.sh.
    def test_revert_of_red_commit_uses_landing_red_exception(self):
        culprit = self.change("broken public value")
        self.publish()
        (self.repo.state / "main-status").write_text(f"red {culprit} exact_test\n")
        before = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        result = self.command("revert", culprit)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        reverted = self.repo.remote_head()
        self.assertEqual(answer["sha"], reverted)
        self.assertEqual(fixtures.run("git", "show", "-s", "--format=%G?", reverted, cwd=self.repo.root), "G")
        self.assertIn(culprit, fixtures.run("git", "show", "-s", "--format=%B", reverted, cwd=self.repo.root))
        self.assertTrue(fixtures.run("git", "show", "-s", "--format=%s", reverted, cwd=self.repo.root).startswith("Revert"))
        self.assertEqual(fixtures.run("git", "diff", self.base, reverted, "--", "crates/c/src/lib.rs", cwd=self.repo.root), "")
        self.assertEqual(fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root), before)
        self.assertEqual(len((self.repo.state / "landings.log").read_text().splitlines()), 1)

    # keeps: spec landing.md §6 — an inseparable batch can be reverted newest first.
    def test_whole_batch_reverts_through_the_same_landing_path(self):
        self.change("first regression", path="crates/a/src/lib.rs")
        red = self.change("second regression", path="crates/b/src/lib.rs")
        self.publish()
        (self.repo.state / "main-status").write_text(f"red {red} exact_test\n")
        result = self.command("revert", self.base + ".." + red)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(len(answer["commits"]), 2)
        self.assertEqual(answer["commits"][0], red)
        self.assertEqual(fixtures.run("git", "diff", self.base, self.repo.remote_head(), "--", "crates", cwd=self.repo.root), "")

    # keeps: invariant main-red-bounded — records may land, ordinary code may not.
    def test_red_interval_allows_records_and_revert_then_rejects_code(self):
        red = self.change("regression")
        self.change("record", path=".bboxy/fixture.md")
        fixtures.run("git", "revert", "--no-edit", red, cwd=self.repo.root)
        reverted = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        self.publish()
        self.logs([{"sha": self.base, "verdict": "green"}, {"sha": red, "verdict": "red", "culprit": red}])
        result = self.command("main-red-bounded")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        bad = self.change("illegal code")
        self.publish()
        result = self.command("main-red-bounded")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["violations"][0]["sha"], bad)
        self.logs([{"sha": self.base, "verdict": "green"}, {"sha": red, "verdict": "red", "culprit": red}, {"sha": reverted, "verdict": "green"}])
        self.assertEqual(self.command("main-red-bounded").returncode, 0)

    # keeps: invariant main-red-bounded — a different revert is not the remedy.
    def test_wrong_culprit_is_rejected_and_unreverted_tip_is_named(self):
        red = self.change("regression")
        self.publish()
        self.logs([{"sha": self.base, "verdict": "green"}, {"sha": red, "verdict": "red", "culprit": red}])
        answer = json.loads(self.command("main-red-bounded").stdout)
        self.assertEqual(answer["pending_red"], [red])
        self.assertEqual(answer["status"], "pending")
        wrong = self.change('Revert "wrong change"')
        self.publish()
        result = self.command("main-red-bounded")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["violations"][0]["sha"], wrong)

    # keeps: spec landing.md §8 — red measurements still create signed evidence.
    def test_scale_pins_main_runs_after_night_and_commits_red_evidence(self):
        night = self.repo.state / "night.jsonl"
        night.write_text(json.dumps({"kind": "summary", "group": "terminal", "unwritten": [], "state": "amber"}) + "\n")
        script = self.repo.state / "scale.py"
        script.write_text('''import json,os,pathlib,sys
out=pathlib.Path(sys.argv[1]);out.joinpath('report.json').write_text(json.dumps({'rows':967,'real_ptys':967,'watched':50,'timing_gated':True,'metrics':{'diagnostics':{'enabled':True}},'failures':['fixture_bound']}))
print(json.dumps({'custody':os.environ['BBOXY_TEST_CUSTODY'],'jobs':os.environ['CARGO_BUILD_JOBS']}))
raise SystemExit(1)
''')
        self.options.update(scale_build=[sys.executable, "-c", "print('fixture build')"],
                            scale_run=[sys.executable, str(script), "{output}"])
        self.configure()
        before = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        result = self.command("scale-night", "--night-log", str(night))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(answer["sha"], self.base)
        self.assertTrue(answer["report_valid"])
        evidence = answer["evidence_commit"]
        self.assertEqual(fixtures.run("git", "show", "-s", "--format=%G?", evidence, cwd=self.repo.root), "G")
        files = fixtures.run("git", "diff-tree", "--no-commit-id", "--name-only", "-r", evidence, cwd=self.repo.root).splitlines()
        self.assertTrue(any(p.endswith("report.json") for p in files))
        self.assertTrue(all(p.startswith(".bboxy/runs/landing/") for p in files))
        self.assertEqual(self.repo.remote_head(), self.base)
        self.assertEqual(fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root), before)

    # keeps: spec landing.md §8 — no measurement without completed night evidence.
    def test_scale_refuses_incomplete_night_without_starting_a_build(self):
        path = self.repo.state / "incomplete.jsonl"
        path.write_text('{"kind":"night"}\n')
        result = self.command("scale-night", "--night-log", str(path))
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("completed_e2e_night", json.loads(result.stdout)["reason"])
        self.assertFalse((self.repo.state / "scale").exists())

    # keeps: invariant main-red-bounded — the correcting candidate can check itself.
    def test_corrective_candidate_is_checked_before_push(self):
        red = self.change("regression")
        self.publish()
        self.logs([{"sha": self.base, "verdict": "green"}])
        # main-watch closes main before the diagnosis's final log is appended.
        (self.repo.state / "main-status").write_text(f"red {red} exact_test\n")
        self.assertEqual(json.loads(self.command("main-red-bounded").stdout)["status"], "pending")
        fixtures.run("git", "revert", "--no-edit", red, cwd=self.repo.root)
        candidate = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        result = self.command("main-red-bounded")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["main"], candidate)
        self.assertEqual(json.loads(result.stdout)["status"], "held")
        self.assertEqual(self.repo.remote_head(), red)


if __name__ == "__main__":
    unittest.main()
