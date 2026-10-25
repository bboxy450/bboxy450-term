"""# keeps: spec landing.md §§6,8,12; invariant main-red-bounded.

Real Git repositories and per-run signing keys, never the production remote or
landing state. Predicate executables are deterministic fixture processes.
"""
import importlib.util
import json
import os
from pathlib import Path
import shlex
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
        for name in ("landing_recovery.py", "culprit.sh", "revert.sh", "main-red-bounded.sh", "scale-night.sh", "e2e-night.sh"):
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
        self.assertEqual(json.loads(result.stdout)["revert_range"], self.base + ".." + red)
        self.options.pop("recovery_test")
        self.configure()
        selector = self.repo.state / "selector.json"
        selector.write_text(json.dumps({"name": "exact_test", "cargo_args": ["-p", "c", "--lib"], "source": "cargo_rerun"}))
        env = dict(self.repo.environment, BBOXY_FAILED_TEST_SELECTOR=str(selector))
        result = self.command("culprit", self.base, red, "exact_test", environment=env)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("test_not_observed", json.loads(result.stdout)["reason"])
        self.assertEqual(json.loads(result.stdout)["revert_range"], self.base + ".." + red)
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
        result = self.command("culprit", green, red, "test:cargo:c:lib:regression", environment=env)
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

    # keeps: spec landing.md §8 — build errors and missing reports keep evidence.
    def test_scale_records_build_failure_and_cannot_pass_without_report(self):
        night = self.repo.state / "night.jsonl"
        night.write_text('{"kind":"summary","group":"terminal","unwritten":[]}\n')
        for build_exit in (2, 0):
            with self.subTest(build_exit=build_exit):
                self.options.update(scale_build=[sys.executable, "-c", f"raise SystemExit({build_exit})"],
                                    scale_run=[sys.executable, "-c", "print('no report')"])
                self.configure()
                result = self.command("scale-night", "--night-log", str(night))
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                answer = json.loads(result.stdout)
                self.assertFalse(answer["report_valid"])
                self.assertEqual(answer["build_exit"], build_exit)
                self.assertEqual(fixtures.run("git", "show", "-s", "--format=%G?", answer["evidence_commit"], cwd=self.repo.root), "G")

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

    # keeps: spec landing.md §8 — scale runs once, last, despite a failed group.
    def test_all_night_pins_source_and_calls_scale_once_after_every_group(self):
        self.repo.write(".bboxy/specs/e2e-acceptance.md", "## 1. Promises\n### first group\n- **T1** fixture\n### last group\n- **T2** fixture\n")
        self.repo.write("scripts/night-test-boundaries.sh", '''#!/bin/sh
python3 -I - <<'PY'
import json,os,pathlib
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
with (state/'order.jsonl').open('a') as f:f.write(json.dumps('test-boundaries')+'\\n')
raise SystemExit(int((state/'fail-boundary').exists()))
PY
''')
        self.repo.write("scripts/scale-night.sh", '''#!/bin/sh
python3 -I - "$@" <<'PY'
import json,os,pathlib,sys
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
args=sys.argv[1:]; log=pathlib.Path(args[args.index('--night-log')+1])
sha=args[args.index('--sha')+1];rows=[json.loads(x) for x in log.read_text().splitlines()]
assert rows[0]['sha']==sha and rows[-1]['kind']=='night-summary'
assert [r['group'] for r in rows[-1]['groups']]==['first group','last group']
with (state/'order.jsonl').open('a') as f:f.write(json.dumps('scale')+'\\n')
print(json.dumps({'exit':0}))
PY
''')
        pinned = self.repo.commit("Fixture whole-night runner")
        self.publish()
        binary = self.repo.state / "night-app"
        binary.write_text('''#!/usr/bin/env python3
import json,os,pathlib,sys
s=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
group=sys.argv[sys.argv.index('--group')+1]
with (s/'order.jsonl').open('a') as f:f.write(json.dumps(group)+'\\n')
print(json.dumps({'kind':'summary','group':group,'unwritten':[]}))
raise SystemExit(int(group=='first group' and (s/'fail-group').exists()))
''')
        binary.chmod(0o700)
        env = dict(self.repo.environment, BBT=str(binary))
        for failed, boundary_failed in ((False, False), (True, False), (False, True)):
            with self.subTest(failed=failed, boundary_failed=boundary_failed):
                (self.repo.state / "order.jsonl").unlink(missing_ok=True)
                for name, active in (("fail-group", failed), ("fail-boundary", boundary_failed)):
                    flag = self.repo.state / name
                    flag.touch() if active else flag.unlink(missing_ok=True)
                result = self.command("e2e-night", "--all", environment=env)
                self.assertEqual(result.returncode, int(failed or boundary_failed), result.stdout + result.stderr)
                answer = json.loads(result.stdout)
                self.assertEqual(answer["sha"], pinned)
                self.assertEqual(answer["scale_exit"], 0)
                self.assertEqual(answer["boundary_exit"], int(boundary_failed))
                order = [json.loads(x) for x in (self.repo.state / "order.jsonl").read_text().splitlines()]
                self.assertEqual(order, ["first group", "last group", "test-boundaries", "scale"])


    # keeps: invariant main-red-bounded — hand calls close red intervals by their named verdict.
    def test_manual_green_and_legacy_correction_close_the_same_sha_interval(self):
        red = self.change("first red")
        self.publish()
        self.logs([{"sha": red, "verdict": "red"},
                   {"sha": red, "verdict": "manual-green", "evidence": "three isolated passes"},
                   {"sha": red, "correction": "tests only; full watcher still required"}])
        result = self.command("main-red-bounded")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "held")

    def test_manual_revert_is_a_revert_boundary_not_an_ordinary_code_exception(self):
        red = self.change("first red")
        fixtures.run("git", "revert", "--no-edit", red, cwd=self.repo.root)
        reverted = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        self.publish()
        self.logs([{"sha": red, "verdict": "red", "culprit": red},
                   {"sha": reverted, "verdict": "manual-revert", "evidence": "landed corrective commit"}])
        result = self.command("main-red-bounded")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["status"], "held")


class TypedPredicateTests(unittest.TestCase):
    setUp = RecoveryTests.setUp
    configure = RecoveryTests.configure
    command = RecoveryTests.command
    change = RecoveryTests.change

    def install(self, kind):
        self.options.clear()
        self.configure()
        self.repo.write("scripts/invariants.sh", (SOURCE / "invariants.sh").read_text())
        self.repo.write("scripts/parity.sh", (SOURCE / "parity.sh").read_text())
        self.kind = kind
        self.selector = {"test": "test:cargo:c:lib:regression", "invariant": "invariant:contract",
                         "parity": 'parity:["fixture/one-compact-dark.png"]'}[kind]
        if kind == "invariant":
            check = shlex.join([sys.executable, str(self.predicate)]) + ' "$(git rev-parse HEAD)"'
            self.repo.write(".bboxy/invariants/contract.md",
                            "---\ntype: invariant\nslow: true\ncheck: " + check + "\n---\n")
            self.repo.write(".bboxy/invariants/unrelated.md",
                            "---\ntype: invariant\ncheck: exit 1\n---\n")
        runner = '''import os,pathlib,subprocess,sys
predicate=PREDICATE
def outcome():
 sha=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
 return subprocess.run([sys.executable,predicate,sha]).returncode
'''.replace("PREDICATE", repr(str(self.predicate)))
        if kind == "test":
            program = runner + '''code=outcome()
if code in (0,1):
 print('test regression ... '+('ok' if code==0 else 'FAILED'))
raise SystemExit(101 if code==1 else code)
'''
            path = self.repo.bin / "cargo"
            path.write_text("#!/usr/bin/env python3\n" + program)
            path.chmod(0o700)
        if kind == "parity":
            book = self.repo.state / "bboxybook"
            book.write_text("#!/usr/bin/env python3\nimport json,shutil\n" + runner + '''
args=sys.argv[1:]
cases=[{'component':'fixture','slug':slug,'density':'compact','theme':'dark'} for slug in ('one','two')]
def key(row):return f"{row['component']}/{row['slug']}-{row['density']}-{row['theme']}.png"
if args[0]=='manifest':
 print(json.dumps(cases));raise SystemExit(0)
if args[0]=='render':raise SystemExit(0)
assert args[0]=='compare' and '--strict-geometry' in args
selected=json.loads(pathlib.Path(args[args.index('--manifest')+1]).read_text())
assert list(map(key,selected))==['fixture/one-compact-dark.png'], selected
code=outcome()
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
if code not in (0,1):raise SystemExit(code)
if (state/'missing-report').exists():raise SystemExit(1)
rows=[{'key':key(row),'policy':{'historical':False},'status':'review geometry' if code else 'within policy'} for row in selected]
pathlib.Path(args[args.index('--out')+1],'report.json').write_text(json.dumps(rows))
raise SystemExit(code)
''')
            book.chmod(0o700)
            cargo = self.repo.bin / "cargo"
            cargo.write_text('''#!/usr/bin/env python3
import os,pathlib,shutil,sys
assert sys.argv[1:]==['build','-p','bboxybook','--locked'], sys.argv
out=pathlib.Path(os.environ['CARGO_TARGET_DIR'])/'debug';out.mkdir(parents=True,exist_ok=True)
shutil.copy2(BOOK,out/'bboxybook')
'''.replace("BOOK", repr(str(book))))
            cargo.chmod(0o700)
            node = self.repo.bin / "node"
            node.write_text('''#!/usr/bin/env python3
import pathlib,shutil,sys
shutil.copy2(sys.argv[3],pathlib.Path(sys.argv[2])/'cases.json')
''')
            node.chmod(0o700)
        self.base = self.repo.commit("Fixture typed " + kind + " driver")

    def red(self, kind):
        self.install(kind)
        self.change("good landing")
        culprit = self.change("selected regression", bad=True)
        for n in range(3):
            self.change(f"unrelated follow-up {n}")
        tip = fixtures.run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        result = self.command("culprit", self.base, tip, self.selector)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual((answer["predicate_kind"], answer["culprit"]), (kind, culprit))
        counts = {}
        for trial in answer["trials"]:
            counts[trial["sha"]] = counts.get(trial["sha"], 0) + 1
        self.assertEqual(set(counts.values()), {3})

    def flake(self, kind):
        self.install(kind)
        red = self.change("originally red landing", bad=True)
        (self.repo.state / "mode").write_text("pass")
        result = self.command("culprit", self.base, red, self.selector)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual((answer["predicate_kind"], answer["verdict"]), (kind, "flake"))
        self.assertEqual([r["sha"] for r in answer["trials"]], [red] * 3)

    # keeps: landing.md §6 — each typed predicate bisects only its selected failure.
    def test_cargo_red(self): self.red("test")
    def test_invariant_red(self): self.red("invariant")
    def test_parity_red(self): self.red("parity")

    # keeps: landing.md §6 — each kind needs three passing reruns to propose quarantine.
    def test_cargo_flake(self): self.flake("test")
    def test_invariant_flake(self): self.flake("invariant")
    def test_parity_flake(self): self.flake("parity")

    # keeps: landing.md §6 — missing observations cannot become a culprit or flake.
    def test_parity_missing_report_is_unresolved_with_batch_fallback(self):
        self.install("parity")
        red = self.change("red landing", bad=True)
        (self.repo.state / "missing-report").touch()
        result = self.command("culprit", self.base, red, self.selector)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(answer["verdict"], "unresolved")
        self.assertEqual(answer["revert_range"], self.base + ".." + red)

    def test_missing_gate_or_case_is_unresolved(self):
        self.install("parity")
        red = self.change("red landing", bad=True)
        for selector in ("invariant:absent", 'parity:["fixture/absent-compact-dark.png"]'):
            with self.subTest(selector=selector):
                result = self.command("culprit", self.base, red, selector)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertEqual(json.loads(result.stdout)["verdict"], "unresolved")

    def test_unknown_kind_is_named_and_keeps_validated_fallback(self):
        self.options.clear()
        self.configure()
        red = self.change("red landing", bad=True)
        result = self.command("culprit", self.base, red, "unknown:fixture")
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertIn("unknown_predicate_kind: unknown", answer["reason"])
        self.assertEqual(answer["revert_range"], self.base + ".." + red)

    # keeps: landing.md §6 — first-run diagnosis still executes all three red reruns.
    def test_no_baseline_can_classify_three_passing_reruns(self):
        self.install("test")
        red = self.change("first red landing", bad=True)
        (self.repo.state / "mode").write_text("pass")
        result = self.command("culprit", "-", red, self.selector)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(answer["verdict"], "flake")
        self.assertIsNone(answer["green"])
        self.assertEqual([r["sha"] for r in answer["trials"]], [red] * 3)

    def test_no_baseline_does_not_invent_a_culprit_or_revert_range(self):
        self.install("test")
        red = self.change("first red landing", bad=True)
        result = self.command("culprit", "-", red, self.selector)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        answer = json.loads(result.stdout)
        self.assertEqual(answer["reason"], "no_last_green")
        self.assertEqual([r["sha"] for r in answer["trials"]], [red] * 3)
        self.assertNotIn("revert_range", answer)
        self.assertIsNone(answer["culprit"])

    # keeps: landing.md §6 — red admission follows the three reruns and precedes bisect.
    def test_postsubmit_closes_main_only_after_the_isolated_reruns_survive(self):
        self.install("test")
        red = self.change("red landing", bad=True)
        self.predicate.write_text('''import json,os,pathlib,sys
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR']); status=state/'main-status'
with (state/'status-observations').open('a') as f:
 f.write(json.dumps({'sha':sys.argv[1],'status':status.read_text() if status.exists() else None})+'\\n')
raise SystemExit(int(pathlib.Path('bad').exists()))
''')
        env = dict(self.repo.environment, BBOXY_POSTSUBMIT_ISOLATE="1")
        result = self.command("culprit", self.base, red, self.selector, environment=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        rows = [json.loads(line) for line in (self.repo.state / "status-observations").read_text().splitlines()]
        self.assertEqual([r["status"] for r in rows[:3]], [None] * 3)
        self.assertTrue(all(r["status"].startswith("red " + red + " ") for r in rows[3:]))

    def test_postsubmit_flake_does_not_close_main(self):
        self.install("test")
        red = self.change("red landing", bad=True)
        (self.repo.state / "mode").write_text("pass")
        status = self.repo.state / "main-status"
        status.write_text("green " + self.base + "\n")
        env = dict(self.repo.environment, BBOXY_POSTSUBMIT_ISOLATE="1")
        result = self.command("culprit", self.base, red, self.selector, environment=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(status.read_text(), "green " + self.base + "\n")


class CiPhaseTests(unittest.TestCase):
    setUp = RecoveryTests.setUp
    configure = RecoveryTests.configure
    command = RecoveryTests.command

    def phase(self, after_tests):
        self.repo.write("scripts/ci-local.sh", (SOURCE / "ci-local.sh").read_text())
        self.repo.write("scripts/ui-lint.sh", '#!/bin/sh\nprintf "ui-lint\\n" >> "$BBOXY_LANDING_STATE_DIR/phase-calls"\n')
        app = self.repo.state / "app"
        app.write_text('''#!/usr/bin/env python3
import os,pathlib,sys
assert os.environ['BBOXY_TEST_CUSTODY']=='memory'
assert sys.argv[1:3]==['--headless','--smoke']
with (pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])/'phase-calls').open('a') as f:f.write('smoke\\n')
''')
        app.chmod(0o700)
        cargo = self.repo.bin / "cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json,os,pathlib,shutil,sys
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR']);args=sys.argv[1:]
assert 'CARGO_TARGET_DIR' not in os.environ
index=args.index('--target-dir')
assert args[index+1]==os.environ['BBOXY_CARGO_TARGET_DIR']
args=args[:index]+args[index+2:]
with (state/'phase-cargo').open('a') as f:f.write(json.dumps(args)+'\\n')
if args==['build','-p','bboxy450-term','--locked']:
 out=pathlib.Path(os.environ['BBOXY_CARGO_TARGET_DIR'])/'debug';out.mkdir(parents=True,exist_ok=True)
 shutil.copy2(state/'app',out/'bboxy450-term')
''')
        cargo.chmod(0o700)
        env = dict(self.repo.environment, BBOXY_GATE_SLOT="none")
        args = ["--after-tests", "--parity"] if after_tests else ["--parity"]
        self.repo.commit("Fixture exact CI phase inputs")
        result = self.command("ci-local", *args, environment=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "phase-cargo").read_text().splitlines()]
        self.assertEqual((self.repo.state / "phase-calls").read_text().splitlines(), ["ui-lint", "smoke"])
        self.assertIn("scripts/invariants.sh --parity", (self.repo.state / "shell.log").read_text())
        return result.stdout, calls

    # keeps: landing.md §6 — continuation runs real CI tail and claims only tail completion.
    def test_after_tests_runs_the_remaining_gates_without_repeating_workspace_tests(self):
        output, calls = self.phase(True)
        self.assertEqual(calls, [["build", "-p", "bboxy450-term", "--locked"]])
        self.assertIn("post-test checks passed", output)
        self.assertNotIn("all gates passed", output)
        self.assertIn(" after-tests 0", output)

    def test_normal_ci_keeps_workspace_build_clippy_and_tests(self):
        output, calls = self.phase(False)
        self.assertEqual([c[0] for c in calls], ["build", "clippy", "test", "build"])
        self.assertIn("--workspace", calls[0])
        self.assertIn("--workspace", calls[2])
        self.assertIn("all gates passed", output)


if __name__ == "__main__":
    unittest.main()
