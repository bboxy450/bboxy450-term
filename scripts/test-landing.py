# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Landing contracts exercised only on private throwaway repositories (#323 §12)."""
import importlib.util
import io
import atexit
import hashlib
import json
import os
import platform
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

sys.dont_write_bytecode = True

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
                                CARGO_TARGET_DIR=str(self.state / "target"),
                                RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="")
        self.environment.pop("BBOXY_CARGO_TARGET_DIR", None)
        for name in ("landing.py", "formal.py", "presubmit.sh", "land.sh", "quarantine.py", "quarantine.sh", "quarantine-test.sh",
                     "main_watch.py", "main-watch.sh", "landing_recovery.py", "main-red-bounded.sh"):
            target = self.root / "scripts" / name
            target.parent.mkdir(exist_ok=True)
            shutil.copy2(SOURCE.with_name(name), target)
        self.write("scripts/cache-env.sh", "# isolated fixture; never the production cache policy\nexport CARGO_BUILD_JOBS=2\n")
        shell = '#!/bin/sh\nprintf "%s\\n" "$0 $*" >> "$BBOXY_LANDING_STATE_DIR/shell.log"\n'
        for name in ("invariants", "layering", "pending-flags", "kill-own-pids", "licenses",
                     "brand", "images", "cross-os", "never-block", "lab-check"):
            self.write(f"scripts/{name}.sh", shell)
        self.write("scripts/test-landing.py", 'print("nested fixture checks")\n')
        self.write("verification/corpus.toml", '[[model]]\nname="Fixture"\nexpected="pass"\n')
        self.write("verification/coverage.toml", "".join(
            f'[[crate]]\nname="{name}"\nstatus="implementation-tests"\nmodels=[]\nscope="private fixture"\nsources=["crates/{name}/src/lib.rs"]\n'
            for name in ("a", "b", "c")))
        self.write("verification/tools.lock.json", json.dumps({"tlc": {"version": "fixture"}, "java": {
            platform.system() + "-" + platform.machine(): {"version": "fixture"}}}))
        # A test double for native/TLC execution, using the real receipt validator.
        self.write("scripts/fixture-formal.py", '''import importlib.util,json,os,subprocess,sys
sys.dont_write_bytecode=True
from pathlib import Path
root=Path.cwd()
spec=importlib.util.spec_from_file_location("fixture_formal",root/"scripts/formal.py")
formal=importlib.util.module_from_spec(spec);spec.loader.exec_module(formal)
target=Path((sys.argv[1] if len(sys.argv)>1 else "") or os.environ.get("BBOXY_CARGO_TARGET_DIR") or os.environ["CARGO_TARGET_DIR"])/"formal"
models=target/"models";models.mkdir(parents=True,exist_ok=True)
tla=models/"Fixture.tla";tla.write_text("private fixture model\\n")
records=formal.coverage(root,{"Fixture"});pins=json.loads((root/"verification/tools.lock.json").read_text())
sha=subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip()
data={"source":sha,"dirty":False,"tlc":pins["tlc"],"java":next(iter(pins["java"].values())),"crates":records,"contracts":[],"complete_tlc_replacement":False,"input_sha256":formal.input_hashes(root,records,[]),"models":[{"name":"Fixture","expected":"pass","graph_equal":True,"states":1,"edges":0,"tlc_exit":0,"contract_metadata":None,"tla_sha256":formal.hashlib.sha256(tla.read_bytes()).hexdigest()}]}
(target/"evidence.json").write_text(json.dumps(data))
archive=formal.archive_evidence(target,root)
print("formal: receipt "+json.dumps(formal.evidence_receipt(archive,root,require_clean=False),sort_keys=True))
''')
        self.write("Makefile", 'test-formal:\n\t@python3 -I scripts/fixture-formal.py "$(TARGET_DIR)"\n')
        self.write("scripts/ci-local.sh", '''#!/bin/sh
set -eu
sha=$(git rev-parse HEAD)
printf 'ci-local: source %s normal\\n' "$sha"
printf '%s\\n' "$BBOXY_GATE_SLOT $BBOXY_CARGO_TARGET_DIR" >> "$BBOXY_LANDING_STATE_DIR/wide.log"
pwd -P > "$BBOXY_LANDING_STATE_DIR/wide-cwd"
cargo test --workspace --locked
make test-formal
echo 'ci-local: all gates passed'
printf 'ci-local: result %s normal 0\\n' "$sha"
''')
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
with (state/"targets.jsonl").open("a") as f:f.write(json.dumps({k:os.environ.get(k) for k in ("CARGO_TARGET_DIR","BBOXY_CARGO_TARGET_DIR")})+"\\n")
cfg=json.loads((state/"behavior.json").read_text()) if (state/"behavior.json").exists() else {}
if args[0]=="test" and "golden_names" in cfg and ("golden" in args or "gui_wire_golden" in args):
    names=cfg["golden_names"]
    if "golden" in args:names=[name for name in names if "golden" in name]
    for name in names:print("test "+name+" ... ok")
    print("test result: ok. "+str(len(names))+" passed; 0 failed; 0 ignored")
    raise SystemExit(0)
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
if cfg.get("block")==args[0] and (not cfg.get("block_once") or not (state/"blocked-once").exists()):
    if cfg.get("block_once"):
        target=pathlib.Path(os.environ["BBOXY_CARGO_TARGET_DIR"])
        target.mkdir(parents=True,exist_ok=True)
        (target/"warm-build-artifact").write_text("first attempt")
    (state/"blocked-once").write_text("blocked")
    (state/"blocked.pid").write_text(str(os.getpid()))
    if cfg.get("test_started"):print("Running tests/fixture.rs (fixture-binary)\\nrunning 1 test",flush=True)
    signal.pause()
if cfg.get("block_once") and cfg.get("block")==args[0]:
    assert (pathlib.Path(os.environ["BBOXY_CARGO_TARGET_DIR"])/"warm-build-artifact").read_text()=="first attempt"
if args[0]=="test" and "corpus" in args and cfg.get("corpus_frontmatter"):
    path=pathlib.Path(cfg["corpus_frontmatter"])
    if not path.read_text().startswith("---" + chr(10)):
        print(str(path)+":1: frontmatter missing",flush=True)
        raise SystemExit(101)
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
        # A sentinel proves no historical messaging path is reached.
        (self.bin / "icq").write_text('#!/bin/sh\necho forbidden-messenger >&2\nexit 97\n')
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

    def land(self, *arguments):
        return subprocess.run(["sh", "scripts/land.sh", *map(str, arguments)], cwd=self.root, env=self.environment,
                              capture_output=True, text=True, timeout=60)

    def remote_head(self):
        return run("git", "--git-dir", str(self.origin), "rev-parse", "main", cwd=self.root)

    def movement(self, path, repeat=False, contents=None):
        other = self.root / ".git/other-worktree"
        run("git", "worktree", "add", "-q", "-b", "fixture-other", str(other), "origin/main", cwd=self.root)
        hook = self.state / "move-main.sh"
        write = ("printf '// movement %s\\n' \"$n\" > " + shlex.quote(path) if contents is None else
                 "printf '%s\\n' " + shlex.quote(contents) + " > " + shlex.quote(path))
        hook.write_text("#!/bin/sh\nset -eu\ncd " + shlex.quote(str(other)) + "\n"
                        + 'count_file=' + shlex.quote(str(self.state / 'moves')) + '\n'
                        + 'n=$(cat "$count_file" 2>/dev/null || echo 0)\nn=$((n+1))\necho "$n" > "$count_file"\n'
                        + "mkdir -p " + shlex.quote(str(Path(path).parent)) + "\n"
                        + write + "\n"
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
        self.assertEqual(result["tier"], "wide")

    def test_leaf_does_not_retest_its_dependencies(self):
        result = self.changed("crates/c/src/lib.rs")
        self.assertEqual(result["affected"], ["c"])
        self.assertEqual(result["tier"], "code")

    def test_contract_names_do_not_override_the_measured_affected_set(self):
        manifest = self.repo.root / "crates/c/Cargo.toml"
        for name in ("bboxy450-proto", "bboxy450-term-core"):
            with self.subTest(name=name):
                manifest.write_text('[package]\nname="' + name + '"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nb={path="../b"}\n')
                run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)
                self.repo.base = self.repo.commit("Fixture contract leaf " + name)
                self.repo.write("crates/c/src/lib.rs", "// changed " + name + "\n")
                self.repo.commit("Fixture contract leaf change")
                result = landing.affected(self.repo.base + "..HEAD", root=self.repo.root)
                self.assertEqual(result["affected"], [name])
                self.assertEqual(result["tier"], "code")

    def test_shared_validation_input_is_checked_without_inventing_rust_dependencies(self):
        result = self.changed("scripts/fixture-gate.sh")
        self.assertEqual(result["affected"], [])
        self.assertEqual(result["parity"], "no")
        self.assertEqual(result["shared"], ["scripts/fixture-gate.sh"])

    def test_root_cargo_input_still_affects_the_entire_workspace(self):
        self.repo.write("Cargo.toml", (self.repo.root / "Cargo.toml").read_text() + "# shared workspace input\n")
        self.repo.commit("Fixture workspace manifest")
        result = landing.affected(self.repo.base + "..HEAD", root=self.repo.root)
        self.assertEqual(result["affected"], ["a", "b", "c"])
        self.assertEqual(result["tier"], "wide")

    def test_exactly_half_the_workspace_remains_narrow(self):
        self.repo.write("crates/d/Cargo.toml", '[package]\nname="d"\nversion="0.1.0"\nedition="2024"\n')
        self.repo.write("crates/d/src/lib.rs", "pub fn unrelated() {}\n")
        manifest = self.repo.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('"crates/c"', '"crates/c","crates/d"'))
        run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)
        self.repo.base = self.repo.commit("Fixture fourth crate")
        result = self.changed("crates/b/src/lib.rs")
        self.assertEqual(result["affected"], ["b", "c"])
        self.assertEqual(result["tier"], "code")

    def test_lab_tier_has_no_product_target_and_reports_referenced_pages(self):
        self.repo.write("lab/web/launcher.html", '<script src="components.js"></script>')
        self.repo.commit("Fixture page includes")
        result = self.changed("lab/web/components.js")
        self.assertEqual(result["tier"], "lab")
        self.assertEqual(result["affected"], [])
        self.assertEqual(result["parity"], "yes")
        self.assertEqual(result["shared"], [])
        self.assertEqual(result["lab_pages"], ["lab/web/launcher.html"])

    def test_records_do_not_request_native_parity(self):
        result = self.changed(".bboxy/fixture with spaces.md")
        self.assertEqual(result["tier"], "lab")
        self.assertEqual(result["parity"], "no")
        self.assertEqual(result["affected"], [])
        self.assertEqual(result["paths"], [".bboxy/fixture with spaces.md"])

    def test_design_input_named_by_build_script_is_code_and_reaches_dependents(self):
        self.repo.write("crates/a/build.rs", 'fn main(){let _=std::fs::read("../../design/tokens.toml");}\n')
        self.repo.base = self.repo.commit("Fixture native input consumer")
        result = self.changed("design/tokens.toml")
        self.assertEqual(result["tier"], "wide")  # native consumer plus all its dependents
        self.assertEqual(result["native_inputs"], ["a"])
        self.assertEqual(result["affected"], ["a", "b", "c"])
        self.assertEqual(result["parity"], "yes")

    def test_packaged_input_copy_keeps_its_canonical_source_native(self):
        self.repo.write("lab/web/fixture.json", "{}\n")
        self.repo.write("crates/b/release-inputs/lab/web/fixture.json", "{}\n")
        self.repo.base = self.repo.commit("Fixture packaged source input")
        check = SOURCE.with_name("release-inputs.py")
        self.assertEqual(subprocess.run([sys.executable, "-I", check, self.repo.root],
                                       capture_output=True).returncode, 0)
        result = self.changed("lab/web/fixture.json")
        self.assertEqual(result["native_inputs"], ["b"])
        self.assertEqual(result["affected"], ["b", "c"])
        failed = subprocess.run([sys.executable, "-I", check, self.repo.root],
                                capture_output=True, text=True)
        self.assertEqual(failed.returncode, 1)
        self.assertIn("crates/b/release-inputs/lab/web/fixture.json differs", failed.stderr)

    def test_rust_include_concat_and_raw_paths_identify_only_real_consumers(self):
        self.repo.write("crates/b/src/lib.rs", 'pub const IMAGE:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../lab/web/bboxy.png"));\n')
        self.repo.write("crates/c/src/lib.rs", 'pub const UI:&str=include_str!(r"../../../lab/web/ui.js");\n')
        self.repo.base = self.repo.commit("Fixture nested include")
        result = self.changed("lab/web/bboxy.png")
        self.assertEqual(result["tier"], "wide")  # two of three fixture crates
        self.assertEqual(result["native_inputs"], ["b"])
        self.assertEqual(result["affected"], ["b", "c"])
        self.repo.base = run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        raw = self.changed("lab/web/ui.js")
        self.assertEqual(raw["native_inputs"], ["c"])
        self.assertEqual(raw["tier"], "code")

    def test_named_unit_test_input_is_native_but_comment_and_literal_examples_are_not(self):
        self.repo.write("crates/a/src/lib.rs", '''
// include_str!("../../lab/web/lab.js");
pub const EXAMPLE:&str=r#"include_str!("../../lab/web/lab.js")"#;
#[cfg(test)] mod tests {
 const INPUT:&str="lab/web/tokens.js";
 #[test] fn fixture(){let _=std::fs::read(INPUT);}
}
''')
        self.repo.base = self.repo.commit("Fixture test-only native consumer")
        native = self.changed("lab/web/tokens.js")
        self.assertEqual(native["tier"], "wide")
        self.assertEqual(native["native_inputs"], ["a"])
        self.repo.base = run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        pure = self.changed("lab/web/lab.js")
        self.assertEqual(pure["tier"], "lab")
        self.assertEqual(pure["native_inputs"], [])


    def test_validation_scripts_stay_out_of_the_native_input_set(self):
        self.repo.write("scripts/fixture.py", "# baseline validation input\n")
        self.repo.write("crates/c/src/lib.rs", 'const FIXTURE: &str = include_str!("../../../scripts/fixture.py");\n')
        self.repo.base = self.repo.commit("Fixture script reader")
        result = self.changed("scripts/fixture.py")
        self.assertEqual(result["affected"], [])
        self.assertEqual(result["native_inputs"], [])


class StateTests(unittest.TestCase):
    def test_phase_accounting_clips_an_active_compile_without_inventing_test_time(self):
        with tempfile.TemporaryDirectory(prefix="landing-timing-") as temp:
            path = Path(temp) / "phases.jsonl"
            path.write_text('\n'.join(json.dumps(e) for e in [
                {"at": 1., "phase": "compile"}, {"at": 4., "phase": "test"},
                {"at": 9., "phase": "other"}, {"at": 10., "phase": "compile"}]) + '\n')
            totals = landing.phase_totals(path, 12.)
        self.assertEqual(totals, {"compile_seconds": 5., "test_seconds": 5., "tests_started": True})

    def test_twenty_minute_cold_build_then_ten_minute_run_passes(self):
        # keeps: #323 §3 two clocks, with no real twenty-minute fixture wait.
        clocks = landing.TwoClocks(0.)
        clocks.event({"at": 20 * 60., "phase": "test"})
        self.assertIsNone(clocks.expired(30 * 60., 30 * 60., 15 * 60.))
        self.assertEqual(clocks.totals(30 * 60.)["compile_seconds"], 20 * 60.)
        self.assertEqual(clocks.totals(30 * 60.)["run_seconds"], 10 * 60.)

    def test_sixteen_minute_run_fails_independently_of_build(self):
        clocks = landing.TwoClocks(0.)
        clocks.event({"at": 0., "phase": "test"})
        self.assertEqual(clocks.expired(16 * 60., 30 * 60., 15 * 60.), "presubmit_timeout")

    def test_thirty_one_minute_build_hits_infrastructure_cap(self):
        clocks = landing.TwoClocks(0.)
        self.assertEqual(clocks.expired(31 * 60., 30 * 60., 15 * 60.), "build_cap")
        self.assertEqual(clocks.totals(31 * 60.)["run_seconds"], 0)

    def test_later_compilation_pauses_the_execution_clock(self):
        clocks = landing.TwoClocks(0.)
        clocks.event({"at": 0., "phase": "test"})
        clocks.event({"at": 10 * 60., "phase": "compile"})
        self.assertIsNone(clocks.expired(20 * 60., 30 * 60., 15 * 60.))
        self.assertEqual(clocks.totals(20 * 60.)["run_seconds"], 10 * 60.)
        clocks.event({"at": 20 * 60., "phase": "test"})
        self.assertEqual(clocks.expired(26 * 60., 30 * 60., 15 * 60.), "presubmit_timeout")

    def test_completed_failure_wins_over_a_simultaneously_observed_cap(self):
        with tempfile.TemporaryDirectory(prefix="landing-failure-cap-") as temp:
            root = Path(temp)
            with patch.object(landing, "state_directory", return_value=root / "state"), \
                 patch.object(landing.subprocess, "Popen") as launch, \
                 patch.object(landing.TwoClocks, "expired", return_value="build_cap"), \
                 patch.object(landing, "stop_worker") as stop:
                launch.return_value.pid = 12345
                launch.return_value.poll.return_value = 101
                result = landing.presubmit_attempt(root, "f" * 40, "fixture-base",
                                                   {"tier": "code", "affected": []}, 1800, 900, 0, "fixture")
                self.assertEqual(result["exit"], 101)
                self.assertIsNone(result["error"])
                stop.assert_not_called()

    def test_target_is_keyed_by_checkout_path_and_stable_at_that_path(self):
        with patch.dict(os.environ, {"CARGO_TARGET_DIR": "/tmp/fixture-warm-target"}, clear=True):
            first = landing.build_environment(Path("/tmp/fixture-checkout-a"))
            again = landing.build_environment(Path("/tmp/fixture-checkout-a"))
            other = landing.build_environment(Path("/tmp/fixture-checkout-b"))
        self.assertNotIn("CARGO_TARGET_DIR", first)
        self.assertEqual(first["BBOXY_CARGO_TARGET_DIR"], again["BBOXY_CARGO_TARGET_DIR"])
        self.assertNotEqual(first["BBOXY_CARGO_TARGET_DIR"], other["BBOXY_CARGO_TARGET_DIR"])
        self.assertEqual(Path(first["BBOXY_CARGO_TARGET_DIR"]).parent, Path("/tmp/fixture-warm-target").resolve())

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

    def signature_invariant(self):
        invariant = SOURCE.parent.parent / ".bboxy/invariants/signed-main.md"
        check = next(line.removeprefix("check: ") for line in invariant.read_text().splitlines()
                     if line.startswith("check: "))
        key, program = fixture_identity()
        # Copy the real verifier unchanged. Only this private test entry point
        # supplies the fixture's own public pin; production has no such override.
        self.repo.write("scripts/verify-signatures-core.py", SOURCE.with_name("verify-signatures.py").read_text())
        public = run(str(program), "--batch", "--armor", "--export", key, cwd=self.repo.root)
        self.repo.write("scripts/fixture-public.asc", public + "\n")
        self.repo.write("scripts/verify-signatures.py", f'''# bboxy450-term private fixture adapter; not a production policy override.
# Origin: https://github.com/bboxy450/bboxy450-term/blob/main/scripts/verify-signatures.py
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import importlib.util
from pathlib import Path
import sys
sys.dont_write_bytecode = True
try:
    source = Path(__file__).with_name("verify-signatures-core.py")
    spec = importlib.util.spec_from_file_location("fixture_signature_core", source)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    policy = module.Policy(module.Identity({key!r}, "Landing Fixture", "fixture@bboxy450.invalid"),
                           (Path(__file__).with_name("fixture-public.asc"),))
    result = module.verify(Path.cwd(), sys.argv[1] if len(sys.argv) == 2 else "HEAD", policy)
    print("fixture signatures:", result)
except Exception as error:
    print("fixture signatures refused:", error, file=sys.stderr)
    sys.exit(1)
''')
        self.repo.write("scripts/invariants.sh", "#!/bin/sh\n" + check + "\n")
        self.repo.base = self.repo.commit("Fixture signature invariant")
        self.repo.environment.pop("SIGNED_RANGE", None)

    # keeps: invariant signed-main — every candidate commit must verify.
    def test_presubmit_rejects_an_unsigned_commit_inside_its_signed_range(self):
        self.signature_invariant()
        self.repo.write("crates/c/src/lib.rs", "// unsigned candidate fixture\n")
        run("git", "add", "crates/c/src/lib.rs", cwd=self.repo.root)
        run("git", "-c", "commit.gpgsign=false", "commit", "-qm", "Unsigned fixture candidate",
            cwd=self.repo.root)
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        record = json.loads((self.repo.state / "presubmit.log").read_text().splitlines()[-1])
        self.assertEqual(record["verdict"], "red")

    # keeps: invariant signed-main — only presubmit narrows the verified history.
    def test_presubmit_excludes_old_unsigned_history_but_unset_check_does_not(self):
        self.signature_invariant()
        self.repo.write("README.md", "unsigned historical fixture\n")
        run("git", "add", "README.md", cwd=self.repo.root)
        run("git", "-c", "commit.gpgsign=false", "commit", "-qm", "Unsigned historical fixture",
            cwd=self.repo.root)
        self.repo.write("README.md", "signed fixture cutoff\n")
        self.repo.base = self.repo.commit("Fixture signed cutoff")
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        whole_history = subprocess.run(["sh", "scripts/invariants.sh"], cwd=self.repo.root,
                                       env=self.repo.environment, capture_output=True, text=True, timeout=15)
        self.assertEqual(whole_history.returncode, 1, whole_history.stdout + whole_history.stderr)

    def test_signature_fixture_refuses_missing_verifier_or_public_pin(self):
        self.signature_invariant()
        for name in ("verify-signatures-core.py", "fixture-public.asc"):
            path = self.repo.root / "scripts" / name
            original = path.read_bytes()
            path.unlink()
            result = subprocess.run(["sh", "scripts/invariants.sh"], cwd=self.repo.root,
                                    env=self.repo.environment, capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
            path.write_bytes(original)

    def test_code_targets_the_affected_leaf_and_records_one_verdict(self):
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        for call in calls[:2]:
            index = call.index("--target-dir")
            self.assertTrue(Path(call[index + 1]).resolve().is_relative_to((self.repo.state / "target").resolve()))
        test_args = calls[0][:1] + calls[0][3:]
        self.assertEqual(test_args, ["test", "--locked", "-p", "c", "--no-fail-fast"])
        self.assertEqual(calls[1][:1] + calls[1][3:6], ["clippy", "--locked", "-p", "c"])
        rows = (self.repo.state / "presubmit.log").read_text().splitlines()
        self.assertEqual(len(rows), 1)
        self.assertEqual(json.loads(rows[0])["verdict"], "green")
        self.assertNotIn("nested fixture checks", result.stdout)

    def test_dispatch_cut_selects_real_gui_wire_target_without_a_name_filter(self):
        source = SOURCE.parent.parent / "crates/bbt-proto/tests/gui_wire_golden.rs"
        import re
        names = re.findall(r"#\[test\]\s*fn\s+(\w+)", source.read_text())
        self.assertGreaterEqual(len(names), 1)
        self.assertFalse(any("golden" in name for name in names))
        (self.repo.state / "behavior.json").write_text(json.dumps({"golden_names": names}))
        self.cut("crates/c/src/logic.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        for call in calls:
            if "--target-dir" in call:
                index = call.index("--target-dir")
                del call[index:index + 2]
        self.assertIn(["test", "--locked", "-p", "bboxy450-proto", "--test", "gui_wire_golden"], calls)
        self.assertFalse(any("golden" in call for call in calls))

    def test_dispatch_cut_still_refuses_a_zero_test_gui_wire_target(self):
        (self.repo.state / "behavior.json").write_text('{"golden_names":[]}')
        self.cut("crates/c/src/logic.rs")
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("golden_dispatch_tests_missing", result.stdout + result.stderr)

    def test_script_only_cut_runs_its_fixtures_and_only_non_cargo_invariants(self):
        self.cut("scripts/fixture-gate.sh")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("nested fixture checks", result.stdout)
        self.assertIn("invariants.sh --affected \n", (self.repo.state / "shell.log").read_text())
        self.assertFalse((self.repo.state / "cargo.jsonl").exists())

    def test_formal_failure_refuses_a_scoped_presubmit_before_cargo(self):
        self.repo.write("Makefile", "test-formal:\n\t@exit 23\n")
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("test-formal", result.stdout + result.stderr)
        self.assertFalse((self.repo.state / "cargo.jsonl").exists())

    def test_code_cut_passes_the_affected_crates_to_invariants(self):
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("invariants.sh --affected c\n", (self.repo.state / "shell.log").read_text())

    def test_ci_command_trace_executes_the_original_check_once_and_keeps_its_exit(self):
        shutil.copy2(SOURCE.with_name('ci-local.sh'), self.repo.root / 'scripts/ci-local.sh')
        self.repo.write('scripts/layering.sh', '#!/bin/sh\nprintf x >> "$BBOXY_LANDING_STATE_DIR/check-count"\nexit 23\n')
        self.repo.commit('Fixture CI trace exit')
        result = subprocess.run(['sh', 'scripts/ci-local.sh'], cwd=self.repo.root,
                                env=dict(self.repo.environment, BBOXY_GATE_SLOT='none'),
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 23, result.stdout + result.stderr)
        self.assertEqual((self.repo.state / 'check-count').read_text(), 'x')
        trace = next(line for line in result.stdout.splitlines() if line.startswith('ci-command: '))
        self.assertEqual(json.loads(trace.removeprefix('ci-command: ')), ['sh', 'scripts/layering.sh'])
        self.assertFalse((self.repo.state / 'cargo.jsonl').exists())

    def test_recheck_refuses_crates_outside_the_cut_without_running_tests(self):
        self.cut('crates/c/src/lib.rs')
        plan = self.repo.state / 'outside-scope.json'
        plan.write_text(json.dumps({'affected': ['outside'], 'commands': []}))
        result = subprocess.run(['sh', 'scripts/presubmit.sh', self.repo.base, '--recheck', str(plan)],
                                cwd=self.repo.root, env=self.repo.environment, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('invalid_recheck_scope', result.stderr)
        self.assertFalse((self.repo.state / 'cargo.jsonl').exists())

    def test_wide_uses_full_normal_ci_without_the_bounded_deadline(self):
        self.cut("crates/a/src/lib.rs")
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":0.0001}')
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        self.assertEqual(calls, [["test", "--workspace", "--locked"]])
        self.assertTrue((self.repo.state / "wide.log").read_text().startswith("none "))
        record = json.loads((self.repo.state / "presubmit.log").read_text())
        self.assertEqual(record["tier"], "wide")
        self.assertEqual(record["exit"], 0)
        self.assertEqual(record["gate_log_sha256"], hashlib.sha256(Path(record["gate_log"]).read_bytes()).hexdigest())

    def test_absent_cache_adapter_uses_empty_wrapper_fallback(self):
        (self.repo.root / "scripts/cache-env.sh").unlink()
        self.repo.write("scripts/invariants.sh", '#!/bin/sh\n[ -z "${RUSTC_WRAPPER:-}" ] && [ -z "${RUSTC_WORKSPACE_WRAPPER:-}" ]\n')
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
            self.assertIn("build_cap", result.stderr)
            self.assertIsNone(control.poll())
        finally:
            control.terminate()
            control.wait()
        record = json.loads((self.repo.state / "presubmit.log").read_text().splitlines()[-1])
        self.assertEqual(record["error"], "build_cap")
        self.assertEqual(record["exit"], 124)
        self.assertEqual(record["finding"], "recurring_build_cap")
        self.assertEqual(len((self.repo.state / "presubmit.log").read_text().splitlines()), 2)

    def test_build_cap_retries_once_warm_with_the_same_target_and_then_passes(self):
        self.repo.remote()
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "behavior.json").write_text('{"block":"test","block_once":true}')
        (self.repo.state / "fixture.json").write_text('{"build_cap_seconds":3,"run_bound_seconds":5}')
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        rows = [json.loads(line) for line in (self.repo.state / "presubmit.log").read_text().splitlines()]
        self.assertEqual([r["error"] for r in rows], ["build_cap", None])
        self.assertEqual([r["attempt"] for r in rows], [0, 1])
        self.assertEqual(len({r["operation"] for r in rows}), 1)
        self.assertIn("retrying once warm", result.stdout)
        targets = [json.loads(line) for line in (self.repo.state / "targets.jsonl").read_text().splitlines()]
        self.assertEqual(len({t["BBOXY_CARGO_TARGET_DIR"] for t in targets}), 1)
        self.assertTrue(all(t["CARGO_TARGET_DIR"] is None for t in targets))
        self.assertTrue(all(Path(t["BBOXY_CARGO_TARGET_DIR"]).is_relative_to(self.repo.state.resolve()) for t in targets))

    def test_run_timeout_during_a_test_does_not_retry(self):
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "behavior.json").write_text('{"block":"test","test_started":true}')
        # Leave process/metadata startup outside the assertion: the contract is
        # expiry of an already-running test, never subsecond fixture startup.
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":3}')
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        record = json.loads((self.repo.state / "presubmit.log").read_text())
        self.assertEqual(record["error"], "presubmit_timeout")
        self.assertTrue(record["tests_started"])
        self.assertGreater(record["test_seconds"], 0)
        self.assertEqual(record["attempt"], 0)

    def test_fixture_cannot_raise_the_fifteen_minute_limit(self):
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":901}')
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("maximum is 900", result.stderr)
        self.assertFalse((self.repo.state / "cargo.jsonl").exists())

    def test_no_cargo_classifier_handles_environment_prefix_and_ui_lint(self):
        self.assertTrue(landing.cargo_backed("R5G_SEATS=4 cargo test -p bboxy450-mux"))
        self.assertTrue(landing.cargo_backed("sh scripts/ui-lint.sh"))
        self.assertFalse(landing.cargo_backed("sh scripts/brand.sh"))


class GateReceiptTests(unittest.TestCase):
    def test_formal_receipt_refuses_missing_stale_incomplete_and_tampered_evidence(self):
        repo = Repository()
        self.addCleanup(repo.close)
        repo.tooling()
        sha = run("git", "rev-parse", "HEAD", cwd=repo.root)
        transcript = run(sys.executable, "-I", "scripts/fixture-formal.py", cwd=repo.root, env=repo.environment)
        receipt = landing.formal_receipt(transcript, sha, repo.root)
        evidence = Path(receipt["evidence"])
        baseline = evidence.read_bytes()
        original = json.loads(baseline)
        mutations = [
            {**original, "source": "0" * 40}, {**original, "dirty": True},
            {**original, "models": []}, {**original, "crates": []},
            {**original, "input_sha256": {}}, {**original, "tlc": {"version": "unreviewed"}},
            {**original, "complete_tlc_replacement": True},
            {**original, "models": [{**original["models"][0], "graph_equal": False}]},
            {**original, "models": [{**original["models"][0], "tlc_exit": 13}]},
            {**original, "models": [{**original["models"][0], "states": 0}]},
            {**original, "unrecorded_change": True},
        ]
        for changed in mutations:
            with self.subTest(changed=changed):
                evidence.write_text(json.dumps(changed))
                with self.assertRaises(landing.Refusal):
                    landing.formal_receipt(transcript, sha, repo.root)
        evidence.write_bytes(baseline)
        (evidence.parent / "models/Fixture.tla").write_text("tampered after checking")
        with self.assertRaises(landing.Refusal):
            landing.formal_receipt(transcript, sha, repo.root)
        for missing in ["", transcript + "\n" + transcript]:
            with self.assertRaises(landing.Refusal):
                landing.formal_receipt(missing, sha, repo.root)

    def test_real_ci_entrypoint_binds_success_and_does_not_certify_failure(self):
        repo = Repository()
        self.addCleanup(repo.close)
        repo.tooling()
        shutil.copy2(SOURCE.with_name("ci-local.sh"), repo.root / "scripts/ci-local.sh")
        repo.write("scripts/ui-lint.sh", "#!/bin/sh\nexit 0\n")
        binary = Path(repo.environment["CARGO_TARGET_DIR"]) / "debug/bboxy450-term"
        binary.parent.mkdir(parents=True)
        binary.write_text("#!/bin/sh\nexit 0\n")
        binary.chmod(0o700)
        sha = repo.commit("Fixture actual CI receipt entrypoint")
        environment = dict(repo.environment, BBOXY_GATE_SLOT="none")
        result = subprocess.run(["sh", "scripts/ci-local.sh"], cwd=repo.root,
                                env=environment, text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        log = repo.state / "actual-ci.log"
        log.write_text(result.stdout + result.stderr)
        self.assertEqual(landing.gate_proof(log, sha, repo.root)["tested_sha"], sha)
        repo.write("Makefile", "test-formal:\n\t@exit 23\n")
        formal_failed = repo.commit("Fixture failing formal gate")
        result = subprocess.run(["sh", "scripts/ci-local.sh"], cwd=repo.root,
                                env=environment, text=True, capture_output=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        log.write_text(result.stdout + result.stderr)
        with self.assertRaisesRegex(landing.Refusal, "gate_log_not_green"):
            landing.gate_proof(log, formal_failed)
        repo.write("scripts/layering.sh", "#!/bin/sh\nexit 17\n")
        failed = repo.commit("Fixture failing actual gate")
        result = subprocess.run(["sh", "scripts/ci-local.sh"], cwd=repo.root,
                                env=environment, text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 17)
        log.write_text(result.stdout + result.stderr)
        with self.assertRaisesRegex(landing.Refusal, "gate_log_not_green"):
            landing.gate_proof(log, failed)
        repo.write("crates/c/src/lib.rs", "// dirty source must not certify HEAD\n")
        result = subprocess.run(["sh", "scripts/ci-local.sh"], cwd=repo.root,
                                env=environment, text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 2)
        self.assertIn("dirty source tree", result.stderr)
        self.assertNotIn("ci-local: source", result.stdout)


class SlotTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        self.repo.write("scripts/ci-local.sh", '''#!/bin/sh
set -eu
sha=$(git rev-parse HEAD)
printf 'ci-local: source %s normal\\n' "$sha"
python3 - <<'PY'
import json,os,pathlib,subprocess
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
target=pathlib.Path(os.environ['BBOXY_CARGO_TARGET_DIR'])
assert 'CARGO_TARGET_DIR' not in os.environ
assert not list(target.iterdir()), 'a candidate inherited an old target'
row={'tree':str(pathlib.Path.cwd().resolve()),'target':str(target.resolve()),
     'sha':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()}
with (state/'slot-calls.jsonl').open('a') as stream:stream.write(json.dumps(row)+'\\n')
(target/'candidate-artifact').write_text(row['sha'])
PY
make test-formal
echo 'ci-local: all gates passed'
printf 'ci-local: result %s normal 0\\n' "$sha"
''')
        self.repo.base = self.repo.commit("Fixture shared slot entrypoint")
        self.slots = (self.repo.state / "slots").resolve()

    def gate(self, sha):
        result = self.repo.cli("ci-local")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        log = self.repo.state / "slot-ci.log"
        log.write_text(result.stdout + result.stderr)
        self.assertEqual(landing.gate_proof(log, sha, self.repo.root)["tested_sha"], sha)
        return json.loads((self.repo.state / "slot-calls.jsonl").read_text().splitlines()[-1])

    def test_two_commits_reuse_slot_paths_with_empty_targets_and_keep_the_caller(self):
        unrelated = self.repo.state / "target/caller-artifact"
        unrelated.parent.mkdir()
        unrelated.write_text("retained")
        first = self.gate(self.repo.base)
        self.repo.write("crates/c/src/lib.rs", "// second signed candidate\n")
        second_sha = self.repo.commit("Fixture second candidate")
        second = self.gate(second_sha)
        slot = self.slots / "bboxy450-gate-slot-1"
        self.assertEqual(first["tree"], str(slot / "tree"))
        self.assertEqual(first["target"], str(slot / "target"))
        self.assertEqual(first["tree"], second["tree"])
        self.assertEqual(first["target"], second["target"])
        self.assertNotEqual(first["sha"], second["sha"])
        self.assertEqual(unrelated.read_text(), "retained")
        self.assertFalse(slot.exists())
        self.assertNotIn(str(slot / "tree"), run("git", "worktree", "list", "--porcelain", cwd=self.repo.root))

    def test_dead_owner_reclaims_its_known_tree_and_target(self):
        dead = subprocess.Popen([sys.executable, "-c", "pass"])
        self.assertEqual(dead.wait(timeout=5), 0)
        slot = self.slots / "bboxy450-gate-slot-1"
        slot.mkdir(parents=True)
        (slot / "pid").write_text(str(dead.pid))
        (slot / "owner.json").write_text(json.dumps({"pid": dead.pid, "token": "dead"}))
        (slot / "target").mkdir()
        (slot / "target/old-artifact").write_text("retired")
        run("git", "worktree", "add", "--detach", str(slot / "tree"), self.repo.base, cwd=self.repo.root)
        row = self.gate(self.repo.base)
        self.assertEqual(row["tree"], str(slot / "tree"))
        self.assertFalse(slot.exists())

    def test_live_owner_and_held_transition_are_skipped_without_deletion(self):
        live = self.slots / "bboxy450-gate-slot-1"
        live.mkdir(parents=True)
        (live / "pid").write_text(str(os.getpid()))
        sentinel = live / "owner-data"
        sentinel.write_text("live")
        self.assertEqual(self.gate(self.repo.base)["tree"], str(self.slots / "bboxy450-gate-slot-2/tree"))
        self.assertEqual(sentinel.read_text(), "live")
        sentinel.unlink()
        (live / "pid").unlink()
        live.rmdir()
        with landing.slot_guard(live) as held:
            self.assertTrue(held)
            row = self.gate(self.repo.base)
            self.assertEqual(row["tree"], str(self.slots / "bboxy450-gate-slot-2/tree"))
            self.assertFalse(live.exists())


class ExecutionToolsTests(unittest.TestCase):
    def test_command_trace_preserves_the_logical_argument_array(self):
        argv = ["sh", "scripts/check.sh", "an argument with spaces", "$literal"]
        stream = io.StringIO()
        with patch.object(sys, "argv", ["landing.py", "trace-command", "--", *argv]), patch.object(sys, "stdout", stream):
            landing.main()
        self.assertEqual(json.loads(stream.getvalue().removeprefix("ci-command: ")), argv)

    def test_actual_commands_and_named_invariants_define_the_tool_footprint(self):
        transcript = ('presubmit: sh scripts/quarantine-test.sh --locked -p c\n'
                      'presubmit: sh scripts/invariants.sh --affected c\n'
                      'invariant fixture: pass\n'
                      'ci-command: ["sh", "scripts/lab-check.sh"]\n'
                      'presubmit: python3 -I scripts/test-landing.py --all\n'
                      'presubmit: echo scripts/not-executed.sh\n')
        source = subprocess.CompletedProcess([], 0, '---\ncheck: "sh scripts/fixture-guard.sh"\n---\n', '')
        with patch.object(landing, "command", return_value=source):
            rows = landing.execution_commands(Path('/fixture'), 'candidate', transcript)
        tools = {tool for row in rows for tool in row['tools']}
        self.assertEqual(tools, {'scripts/quarantine-test.sh', 'scripts/quarantine.py', 'scripts/invariants.sh',
                                 'scripts/fixture-guard.sh', 'scripts/lab-check.sh'})
        selected = next(row for row in rows if 'scripts/fixture-guard.sh' in row['tools'])
        self.assertEqual(selected['argv'], ['sh', 'scripts/invariants.sh', '--affected', 'c', 'fixture'])

    def test_other_script_fixtures_only_intersect_a_script_cut(self):
        transcript = 'ci-command: ["python3", "-I", "scripts/test-fixture.py"]\n'
        self.assertEqual(landing.execution_commands(Path('/fixture'), 'candidate', transcript), [])
        rows = landing.execution_commands(Path('/fixture'), 'candidate', transcript, fixtures=True)
        self.assertEqual(rows[0]['tools'], ['scripts/test-fixture.py'])


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

    def gate_log(self, sha, status=0, mode="normal"):
        path = self.repo.state / "provided-gate.log"
        formal = (run(sys.executable, "-I", "scripts/fixture-formal.py", cwd=self.repo.root,
                      env=self.repo.environment) + "\n") if status == 0 else ""
        path.write_text(f"ci-local: source {sha} {mode}\n" + formal
                        + ("ci-local: all gates passed\n" if status == 0 else "fixture failed\n")
                        + f"ci-local: result {sha} {mode} {status}\n")
        return path

    def test_mixed_script_cut_refuses_frontmatterless_run_record_before_push(self):
        path = ".bboxy/runs/no-frontmatter.md"
        self.repo.write(path, "# Evidence without record metadata\n")
        self.repo.write("scripts/new-check.sh", "#!/bin/sh\nexit 0\n")
        self.repo.commit("Fixture mixed script and malformed record")
        (self.repo.state / "behavior.json").write_text(json.dumps({"corpus_frontmatter": path}))
        result = self.repo.land()
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        self.assertIn(path + ":1: frontmatter missing", result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertFalse((self.repo.state / "landings.log").exists())

    def test_narrow_cold_log_is_no_longer_accepted(self):
        sha = self.cut()
        (self.repo.state / "presubmit.log").write_text(json.dumps({
            "sha": sha, "error": "presubmit_timeout", "exit": 124,
            "compile_seconds": 900., "tests_started": False}) + '\n')
        result = self.repo.land("--gate-log", self.gate_log(sha))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_requires_wide_cut", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_run_timeout_does_not_push_or_accept_a_provided_gate(self):
        sha = self.cut()
        (self.repo.state / "behavior.json").write_text('{"block":"test","test_started":true}')
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":3}')
        result = self.repo.land()
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        result = self.repo.land("--gate-log", self.gate_log(sha))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_requires_wide_cut", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_wide_exact_green_log_is_reused_and_hashed_without_another_gate(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        result = self.repo.land("--gate-log", proof)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), sha)
        self.assertEqual(self.cargo_test_count(), 0)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["tier"], "wide")
        self.assertEqual(record["tested_sha"], sha)
        self.assertEqual(record["base"], self.repo.base)
        self.assertEqual(record["gate_log_sha256"], hashlib.sha256(proof.read_bytes()).hexdigest())
        self.assertEqual(record["formal"]["source"], sha)
        self.assertEqual(record["formal"]["models"], 1)

    def test_green_rust_log_without_formal_receipt_cannot_publish(self):
        sha = self.cut("crates/a/src/lib.rs")
        log = self.gate_log(sha)
        log.write_text("\n".join(line for line in log.read_text().splitlines()
                                 if not line.startswith("formal: receipt ")) + "\n")
        result = self.repo.land("--gate-log", log)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("formal_receipt_missing", result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_wide_wrong_sha_log_is_refused_without_push_or_fallback(self):
        self.cut("crates/a/src/lib.rs")
        result = self.repo.land("--gate-log", self.gate_log(self.repo.base))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_wide_extended_or_concatenated_log_is_not_a_normal_gate_proof(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha, mode="extended")
        result = self.repo.land("--gate-log", proof)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        proof = self.gate_log(sha)
        proof.write_text(proof.read_text() * 2)
        result = self.repo.land("--gate-log", proof)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_wide_failed_or_incomplete_log_is_refused(self):
        sha = self.cut("crates/a/src/lib.rs")
        for status in (101, -15):
            with self.subTest(status=status):
                result = self.repo.land("--gate-log", self.gate_log(sha, status))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("gate_log_not_green", result.stderr)
        incomplete = self.gate_log(sha)
        incomplete.write_text(incomplete.read_text().split("ci-local: result")[0])
        result = self.repo.land("--gate-log", incomplete)
        self.assertIn("gate_log_not_green", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_wide_runs_full_normal_gate_and_records_tier(self):
        sha = self.cut("crates/a/src/lib.rs")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), sha)
        self.assertEqual(self.cargo_test_count(), 1)
        self.assertEqual(Path((self.repo.state / 'wide-cwd').read_text().strip()).resolve(),
                         (self.repo.state / 'slots/bboxy450-gate-slot-1/tree').resolve())
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["tier"], "wide")
        self.assertIn("gate_log_sha256", record)

    def test_wide_failed_full_gate_never_pushes(self):
        self.cut("crates/a/src/lib.rs")
        (self.repo.state / "behavior.json").write_text('{"fail":"test"}')
        result = self.repo.land()
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_wide_rebase_cannot_reuse_a_green_log_from_the_old_candidate(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        self.repo.movement(".bboxy/other.md")
        hook = json.loads((self.repo.state / "behavior.json").read_text())["hook"]
        run("sh", hook, cwd=self.repo.root)
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertNotEqual(self.repo.remote_head(), sha)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_wide_moved_main_reuses_the_full_green_for_disjoint_changes(self):
        self.cut("crates/a/src/lib.rs")
        self.repo.movement(".bboxy/other.md")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertNotEqual(record["tested_sha"], self.repo.remote_head())
        self.assertEqual(record["formal"]["source"], self.repo.remote_head())


    def test_intersecting_move_checks_only_the_crate_intersection(self):
        self.cut("crates/b/src/lib.rs")
        self.repo.movement("crates/c/src/lib.rs")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        tests = [call for call in calls if call[0] == "test"]
        self.assertEqual(sum("--workspace" in call for call in tests), 1)
        self.assertEqual([call[call.index("-p") + 1] for call in tests if "-p" in call], ["c"])
        checks = [json.loads(line) for line in (self.repo.state / "presubmit.log").read_text().splitlines()]
        self.assertEqual(checks[-1]["affected"], ["c"])
        self.assertIn("invariants.sh --affected c\n", (self.repo.state / "shell.log").read_text())

    def test_moved_scope_uses_the_rebased_dependency_graph(self):
        self.cut()
        path = 'crates/c/Cargo.toml'
        manifest = (self.repo.root / path).read_text().replace('name="c"', 'name="c_new"')
        self.repo.movement(path, contents=manifest)
        hook = self.repo.state / 'move-main.sh'
        update_coverage = shlex.join([sys.executable, "-c",
            """from pathlib import Path;p=Path("verification/coverage.toml");p.write_text(p.read_text().replace('name="c"','name="c_new"'))"""])
        text = hook.read_text().replace('git add -- ' + shlex.quote(path),
            update_coverage + "\n" + shlex.quote(shutil.which('cargo')) +
            ' generate-lockfile --offline\ngit add -- verification/coverage.toml Cargo.lock ' + shlex.quote(path))
        hook.write_text(text)
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 2)
        checks = [json.loads(line) for line in (self.repo.state / 'presubmit.log').read_text().splitlines()]
        self.assertEqual(checks[-1]['affected'], ['c_new'])

    def test_a_file_changed_on_both_sides_requires_a_full_recheck(self):
        path = "crates/c/src/lib.rs"
        self.repo.write(path, "// baseline\n" * 40)
        self.repo.base = self.repo.commit("Fixture separated changes")
        run("git", "push", "-q", "origin", "HEAD:main", cwd=self.repo.root)
        source = self.repo.root / path
        source.write_text(source.read_text() + "// candidate tail\n")
        self.repo.commit("Fixture candidate")
        self.repo.movement(path)
        hook = self.repo.state / "move-main.sh"
        text = hook.read_text().replace(
            "printf '// movement %s\\n' \"$n\" > " + shlex.quote(path),
            "printf '// movement %s\\n' \"$n\" | cat - " + shlex.quote(path) + " > changed && mv changed " + shlex.quote(path))
        hook.write_text(text)
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        checks = [json.loads(line) for line in (self.repo.state / "presubmit.log").read_text().splitlines()]
        self.assertEqual(len(checks), 2)
        self.assertTrue(all("recheck" not in check for check in checks))

    def test_moved_driver_does_not_invalidate_a_green_code_cut(self):
        self.cut()
        self.repo.movement("scripts/test-landing.py")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)

    def test_recovery_scripts_and_record_moves_reuse_a_green_rust_cut(self):
        self.cut()
        self.repo.movement('scripts/landing_recovery.py')
        hook = self.repo.state / 'move-main.sh'
        text = hook.read_text().replace('git commit -qm',
            'mkdir -p .bboxy/invariants .bboxy/runs\n'
            'printf "# fixture recovery test\\n" > scripts/test-landing-recovery.py\n'
            'printf "fixture policy\\n" > .bboxy/invariants/main-red-bounded.md\n'
            'printf "{}\\n" > .bboxy/runs/cache-pair.json\n'
            'git add -- scripts/test-landing-recovery.py .bboxy\n'
            'git commit -qm')
        hook.write_text(text)
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        self.assertNotIn('nested fixture checks', result.stdout)
        checks = (self.repo.state / 'presubmit.log').read_text().splitlines()
        self.assertEqual(len(checks), 1)

    def test_a_script_move_replays_script_fixtures_but_keeps_the_green_rust(self):
        self.cut()
        self.cut('scripts/fixture-gate.sh')
        self.repo.movement('scripts/unrelated-helper.sh')
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        self.assertEqual(result.stdout.count('nested fixture checks'), 2)
        checks = [json.loads(line) for line in (self.repo.state / 'presubmit.log').read_text().splitlines()]
        self.assertEqual(checks[-1]['recheck'], {'affected': [], 'commands': [
            ['python3', '-I', 'scripts/test-landing.py', '--all']]})

    def test_a_record_only_move_reuses_a_script_cuts_fixture_result(self):
        self.cut('scripts/fixture-gate.sh')
        self.repo.movement('.bboxy/other.md')
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout.count('nested fixture checks'), 1)

    def test_an_unexecuted_script_move_keeps_the_green_code_check(self):
        self.cut()
        self.repo.movement("scripts/not-executed.sh")
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)

    def test_a_test_wrapper_move_replays_only_its_crate_tests(self):
        original = (self.repo.root / 'scripts/quarantine-test.sh').read_text()
        self.cut()
        self.repo.movement('scripts/quarantine-test.sh', contents=original + '\n# fixture move\n')
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 2)
        calls = [json.loads(line) for line in (self.repo.state / 'cargo.jsonl').read_text().splitlines()]
        self.assertEqual(sum(call[0] == 'clippy' for call in calls), 1)
        checks = [json.loads(line) for line in (self.repo.state / 'presubmit.log').read_text().splitlines()]
        self.assertEqual(checks[-1]['recheck'], {'affected': [], 'commands': [
            ['sh', 'scripts/quarantine-test.sh', '--locked', '-p', 'c']]})

    def test_a_selected_invariant_tool_replays_its_named_check_without_crate_tests(self):
        self.repo.write('.bboxy/invariants/fixture.md', '---\ntype: invariant\ncheck: sh scripts/fixture-guard.sh\n---\n')
        self.repo.write('scripts/fixture-guard.sh', '#!/bin/sh\nexit 0\n')
        self.repo.write('scripts/invariants.sh', '#!/bin/sh\nsh scripts/fixture-guard.sh || exit $?\nprintf "invariant fixture: pass\\n"\n')
        self.repo.base = self.repo.commit('Fixture selected invariant')
        run('git', 'push', '-q', 'origin', 'HEAD:main', cwd=self.repo.root)
        self.cut()
        self.repo.movement('scripts/fixture-guard.sh')
        result = self.repo.land()
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        checks = [json.loads(line) for line in (self.repo.state / 'presubmit.log').read_text().splitlines()]
        self.assertEqual(checks[-1]['recheck'], {'affected': [], 'commands': [
            ['sh', 'scripts/invariants.sh', '--affected', 'c', 'fixture']]})

    def test_signed_cut_pushes_once_logs_and_announces_after_slot_release(self):
        # keeps: #323 §5 fast-forward/signature/log/communication contract.
        sha = self.cut()
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), sha)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["sha"], sha)
        self.assertEqual(record["author_key"], fixture_identity()[0].upper())
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
        self.assertEqual(len(json.loads((self.repo.state / "landings.log").read_text())["check_timings"]), 2)

    def test_five_incremental_rechecks_are_79_with_no_candidate_push(self):
        sha = self.cut()
        self.repo.movement("crates/a/src/lib.rs", repeat=True)
        result = self.repo.land()
        self.assertEqual(result.returncode, 79, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 6)
        self.assertNotEqual(self.repo.remote_head(), sha)

    def test_a_disjoint_move_after_five_rechecks_still_reuses_the_green(self):
        self.cut()
        path = 'crates/a/src/lib.rs'
        self.repo.movement(path, repeat=True)
        hook = self.repo.state / 'move-main.sh'
        text = hook.read_text().replace(shlex.quote(path), '"$move_path"')
        text = text.replace('mkdir -p ' + shlex.quote(str(Path(path).parent)),
                            'mkdir -p "$(dirname "$move_path")"')
        text = text.replace('echo "$n" > "$count_file"\n',
                            'echo "$n" > "$count_file"\nmove_path=crates/a/src/lib.rs\n'
                            'if [ "$n" -ge 6 ]; then move_path=.bboxy/after-fifth.md; fi\n')
        hook.write_text(text)
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 6)
        record = json.loads((self.repo.state / 'landings.log').read_text())
        self.assertEqual(record['incremental_rechecks'], 5)

    def test_busy_slots_are_75_without_a_check(self):
        self.cut()
        for n in (1, 2, 3):
            slot = self.repo.state / "slots" / f"bboxy450-gate-slot-{n}"
            slot.mkdir(parents=True)
            (slot / "pid").write_text(str(os.getpid()))
        result = self.repo.land()
        self.assertEqual(result.returncode, 75, result.stderr)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_local_receipt_failure_can_resume_without_rechecking_or_repush(self):
        sha = self.cut()
        (self.repo.state / "announced").write_text("fixture receipt failure")
        self.assertNotEqual(self.repo.land().returncode, 0)
        self.assertEqual(self.repo.remote_head(), sha)
        (self.repo.state / "announced").unlink()
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

    def _page_move(self,overlap):
        a,b=("a.js","b.js") if overlap else ("components.js","ui.js")
        self.repo.write("lab/web/launcher.html",'<script src="'+a+'"></script>')
        self.repo.write("lab/web/settings.html",('<script src="'+a+'"></script>' if overlap else '')+'<script src="'+b+'"></script>')
        self.repo.write("lab/web/"+a,"// baseline a")
        self.repo.write("lab/web/"+b,"// baseline b")
        self.repo.base=self.repo.commit("Fixture page graph")
        run("git","push","-q","origin","HEAD:main",cwd=self.repo.root)
        self.cut("lab/web/"+a)
        self.repo.movement("lab/web/"+b)
        hook=self.repo.state/"move-main.sh"
        original=landing.command;calls=[];plans=[]
        def check(argv,**kwargs):
            if argv[:2]==["sh","scripts/presubmit.sh"]:
                sha=run("git","rev-parse","HEAD",cwd=self.repo.root)
                record={"sha":sha,"exit":0,"tier":"lab","affected":[]}
                transcript=run(sys.executable,"-I","scripts/fixture-formal.py",cwd=self.repo.root,env=self.repo.environment)
                record["formal"]=landing.formal_receipt(transcript,sha,self.repo.root)
                if "--recheck" in argv:
                    plan=json.loads(Path(argv[argv.index("--recheck")+1]).read_text())
                    record["recheck"]=plan;plans.append(plan)
                landing.append_record("presubmit.log",record);calls.append(argv)
                if len(calls)==1:run("sh",str(hook),cwd=self.repo.root)
                return subprocess.CompletedProcess(argv,0,"","")
            return original(argv,**kwargs)
        with patch.dict(os.environ,self.repo.environment),patch.object(landing,"repository",return_value=self.repo.root),patch.object(landing,"command",side_effect=check),patch.object(landing,"ledger_module",return_value=None):
            self.assertEqual(landing.land(),0)
        return calls,plans

    def test_intersecting_pages_get_only_a_page_recheck(self):
        calls,plans=self._page_move(True)
        self.assertEqual(len(calls),2)
        self.assertEqual(plans,[{"affected":[],"commands":[],"parity_pages":["lab/web/settings.html"]}])

    def test_disjoint_pages_reuse_the_green(self):
        calls,plans=self._page_move(False)
        self.assertEqual(len(calls),1);self.assertEqual(plans,[])


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
        self.assertEqual(calls[1][:2], ["test", "--target-dir"])
        self.assertEqual(Path(calls[1][2]).resolve(), (self.repo.state / "target").resolve())
        self.assertEqual(calls[1][:1] + calls[1][3:], ["test", "--locked", "-p", "c", "--", "--exact", "fixture_test"])

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
python3 - "$@" <<'PY'
import json,os,pathlib,sys
state=pathlib.Path(os.environ['BBOXY_LANDING_STATE_DIR'])
after='--after-tests' in sys.argv
with (state/'ci-calls').open('a') as f:f.write(('tail' if after else 'full')+'\\n')
cfg=json.loads((state/'watch.json').read_text()) if (state/'watch.json').exists() else {}
if after and cfg.get('tail_failure'):
 print('invariant '+cfg['tail_failure']+': fail (exit 1)')
 raise SystemExit(1)
if cfg.get('fail') and not after:
 if cfg.get('invariant'):
  print('invariant '+cfg['invariant']+': fail (exit 1)')
  if cfg.get('parity'):
   report=pathlib.Path(os.environ['BBOXY_CARGO_TARGET_DIR'])/'parity/report.json'
   report.parent.mkdir(parents=True,exist_ok=True)
   report.write_text(json.dumps(cfg['parity']))
  raise SystemExit(1)
 print('test fixture_failed ... FAILED')
 if cfg.get('additional_failure'):print('test '+cfg['additional_failure']+' ... FAILED')
 print("error: test failed, to rerun pass '-p c --test fixture'")
 raise SystemExit(101)
if not cfg.get('missing_store'):
 target=pathlib.Path(os.environ['BBOXY_CARGO_TARGET_DIR'])
 (target/'parity/native').mkdir(parents=True,exist_ok=True)
 (target/'parity/native/fixture.png').write_bytes(b'fixture native pixels'+(b' after tests' if after else b''))
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
selector=(s/'culprit-selector').read_text().strip()
with (s/'culprit-calls').open('a') as f:f.write(selector+'\\n')
if c.get('assert_not_red'):
 status=s/'main-status'
 assert not status.exists() or not status.read_text().startswith('red '), 'first-run main closed before reruns'
if c.get('unresolved'):
 print(json.dumps({'verdict':'unresolved','revert_range':sys.argv[1]+'..'+sys.argv[2]}))
 raise SystemExit(2)
answer=c.get('diagnoses',{}).get(selector,c.get('diagnosis',{'verdict':'culprit','culprit':'f'*40}))
if answer.get('verdict')=='flake':
 answer.setdefault('trials',[{'sha':sys.argv[2],'attempt':n,'exit':0} for n in (1,2,3)])
if c.get('red_publication'):
 status=s/'main-status'
 status.write_text('red '+sys.argv[2]+' fixture_failed\\n')
 os.utime(status,(c['red_publication'],c['red_publication']))
print(json.dumps(answer))
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

    # keeps: invariant main-red-bounded — historical failure never turns green CI into red admission.
    def test_watch_finding_is_separate_from_green_ci_and_never_reverted(self):
        self.repo.write("scripts/main-red-bounded.sh", "#!/bin/sh\nprintf '%s\\n' '{\"status\":\"violated\",\"violations\":[{\"reason\":\"fixture\"}]}'\nexit 1\n")
        sha = self.repo.commit("Fixture historical watch violation")
        run("git", "push", "-q", "origin", "HEAD:main", cwd=self.repo.root)
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        rows = [json.loads(line) for line in (self.repo.state / "postsubmit.log").read_text().splitlines()]
        self.assertEqual(rows[0]["verdict"], "green")
        self.assertTrue(rows[0]["full_checks_passed"])
        self.assertEqual(rows[1]["kind"], "invariant_finding")
        self.assertEqual(rows[1]["exit"], 1)
        self.assertEqual(rows[1]["sha"], sha)
        self.assertEqual(rows[1]["evidence"]["report"]["status"], "violated")
        self.assertEqual((self.repo.state / "main-status").read_text(), "green " + sha + "\n")
        self.assertTrue((self.repo.state / "parity-last-green").is_file())
        self.assertFalse((self.repo.state / "reverted").exists())
        self.assertFalse((self.repo.state / "culprit-calls").exists())
        self.assertEqual(self.watch().returncode, 0)  # a finding is not a verdict or retry trigger

    # keeps: invariant main-red-bounded — every new verdict links a completed watch check.
    def test_full_carried_and_manual_verdicts_each_record_the_watch_check(self):
        self.seed_green()
        self.publish(".bboxy/fixture.md")
        self.assertEqual(self.watch().returncode, 0)
        sha = self.repo.remote_head()
        result = self.repo.cli("main-watch", "--manual-green", sha, "--evidence", "fixture hand call")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        rows = [json.loads(line) for line in (self.repo.state / "postsubmit.log").read_text().splitlines()]
        verdicts = [row for row in rows if "verdict" in row]
        self.assertEqual(len(verdicts), 3)
        for row in verdicts:
            report = json.loads(Path(row["watch_check_log"]).read_text())
            self.assertEqual(report["exit"], 0)
        self.assertEqual(verdicts[-1]["verdict"], "manual-green")

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
        (self.repo.state / "watch.json").write_text('{"fail":true,"red_publication":946684800}')
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual((self.repo.state / "main-status").read_text(), "red " + sha + " fixture_failed\n")
        self.assertEqual((self.repo.state / "reverted").read_text().strip(), "f" * 40)
        self.assertEqual((self.repo.state / "culprit-selector").read_text().strip(), "test:cargo:c:test/fixture:fixture_failed")
        rows = [json.loads(line) for line in (self.repo.state / "postsubmit.log").read_text().splitlines()]
        red = next(row for row in rows if row.get("verdict") == "red")
        self.assertEqual(red["ended"], "2000-01-01T00:00:00+00:00")
        self.assertNotEqual(red["completed_at"], red["ended"])

    # keeps: landing.md §6 — postsubmit retains the failing non-Cargo predicate.
    def test_invariant_failure_selects_the_named_gate(self):
        self.seed_green()
        self.publish("scripts/fixture.sh")
        (self.repo.state / "watch.json").write_text('{"fail":true,"invariant":"fixture-contract"}')
        self.assertEqual(self.watch().returncode, 1)
        self.assertEqual((self.repo.state / "culprit-selector").read_text().strip(), "invariant:fixture-contract")

    def test_geometry_failure_selects_only_failing_current_cases(self):
        self.seed_green()
        self.publish("lab/web/fixture.html")
        rows = [{"key": key, "status": status, "policy": {"historical": historical}}
                for key, status, historical in (("a.png", "review geometry", False),
                    ("b.png", "review pixels", False), ("c.png", "unproven geometry", False),
                    ("old.png", "review geometry", True))]
        (self.repo.state / "watch.json").write_text(json.dumps(
            {"fail": True, "invariant": "parity-geometry", "parity": rows}))
        self.assertEqual(self.watch().returncode, 1)
        selector = (self.repo.state / "culprit-selector").read_text().strip()
        self.assertEqual(json.loads(selector.removeprefix("parity:")), ["a.png", "c.png"])

    def test_three_pass_flake_verdict_reopens_without_revert(self):
        self.seed_green()
        sha = self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text('{"fail":true,"diagnosis":{"verdict":"flake"}}')
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.repo.state / "main-status").read_text(), "green " + sha + "\n")
        self.assertFalse((self.repo.state / "reverted").exists())

    # keeps: landing.md §6 — a first postsubmit can classify a flake without a green baseline.
    def test_first_red_runs_flake_diagnosis_before_publishing_red(self):
        (self.repo.state / "watch.json").write_text(json.dumps(
            {"fail": True, "assert_not_red": True, "diagnosis": {"verdict": "flake"}}))
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.repo.state / "main-status").read_text(), "green " + self.repo.base + "\n")
        self.assertTrue((self.repo.state / "culprit-selector").exists())
        self.assertFalse((self.repo.state / "reverted").exists())

    def test_first_red_without_a_baseline_cannot_revert_a_claimed_culprit(self):
        (self.repo.state / "watch.json").write_text('{"fail":true,"assert_not_red":true}')
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["diagnostic_error"], "no_last_green")
        self.assertTrue((self.repo.state / "culprit-selector").exists())
        self.assertFalse((self.repo.state / "reverted").exists())

    # keeps: landing.md §6 — isolated flake passes never replace the remaining CI/native truth.
    def test_flake_runs_remaining_checks_before_storing_the_new_native_set(self):
        self.seed_green()
        sha = self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text('{"fail":true,"diagnosis":{"verdict":"flake"}}')
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        pointer = Path((self.repo.state / "parity-last-green").read_text().strip())
        self.assertEqual(pointer.name, sha)
        self.assertEqual((pointer / "native/fixture.png").read_bytes(), b"fixture native pixels after tests")
        self.assertEqual((self.repo.state / "ci-calls").read_text().splitlines(), ["full", "full", "tail"])
        self.assertEqual(json.loads(result.stdout)["resumes"][0]["exit"], 0)
        proposals = [json.loads(line) for line in (self.repo.state / "quarantine-proposals.log").read_text().splitlines()]
        self.assertEqual([r["exit"] for r in proposals[0]["reruns"]], [0, 0, 0])
        self.assertTrue(Path(proposals[0]["log"]).is_file())

    def test_a_later_ci_failure_cannot_publish_green_after_an_earlier_flake(self):
        self.seed_green()
        pointer = (self.repo.state / "parity-last-green").read_text()
        self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text(json.dumps({"fail": True, "tail_failure": "late",
            "diagnosis": {"verdict": "flake"}, "diagnoses": {"invariant:late":
                {"verdict": "culprit", "culprit": "f" * 40}}}))
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual((self.repo.state / "parity-last-green").read_text(), pointer)
        self.assertTrue((self.repo.state / "main-status").read_text().endswith(" late\n"))

    def test_a_second_failing_test_is_diagnosed_before_resuming_ci(self):
        self.seed_green()
        self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text(json.dumps({"fail": True, "additional_failure": "second_failed",
            "diagnosis": {"verdict": "flake"}, "diagnoses": {"test:cargo:c:test/fixture:second_failed":
                {"verdict": "culprit", "culprit": "f" * 40}}}))
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual((self.repo.state / "ci-calls").read_text().splitlines(), ["full", "full"])
        calls = (self.repo.state / "culprit-calls").read_text().splitlines()
        self.assertEqual(len(calls), 2)
        self.assertTrue(calls[-1].endswith(":second_failed"))

    def test_repeated_tail_flakes_stay_green_without_claiming_native_truth(self):
        self.seed_green()
        pointer = (self.repo.state / "parity-last-green").read_text()
        self.publish("crates/c/src/lib.rs")
        (self.repo.state / "watch.json").write_text(json.dumps(
            {"fail": True, "tail_failure": "late", "diagnosis": {"verdict": "flake"}}))
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        record = json.loads(result.stdout)
        self.assertFalse(record["full_checks_passed"])
        self.assertFalse(record["native_retained"])
        self.assertTrue((self.repo.state / "main-status").read_text().startswith("green "))
        self.assertEqual((self.repo.state / "parity-last-green").read_text(), pointer)

    # keeps: landing.md §6 — explicit hand calls do not substitute for a full watcher verdict.
    def test_manual_green_is_logged_then_the_watcher_still_runs_full_ci(self):
        result = self.repo.cli("main-watch", "--manual-green", self.repo.base,
                               "--evidence", "fixture isolated reruns")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["verdict"], "manual-green")
        self.assertFalse((self.repo.state / "parity-last-green").exists())
        full = self.watch()
        self.assertEqual(full.returncode, 0, full.stdout + full.stderr)
        self.assertTrue(json.loads(full.stdout)["full_checks_passed"])
        self.assertEqual((self.repo.state / "ci-calls").read_text().splitlines(), ["full"])

    def test_manual_call_requires_evidence_and_a_real_landed_revert(self):
        for args in (("--manual-green", self.repo.base),
                     ("--manual-revert", self.repo.base, "--evidence", "not a revert")):
            with self.subTest(args=args):
                result = self.repo.cli("main-watch", *args)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse((self.repo.state / "main-status").exists())

    def test_manual_revert_records_the_landed_corrective_commit(self):
        red = self.publish("crates/c/src/lib.rs")
        run("git", "revert", "--no-edit", red, cwd=self.repo.root)
        reverted = run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        run("git", "push", "-q", "origin", "HEAD:main", cwd=self.repo.root)
        result = self.repo.cli("main-watch", "--manual-revert", reverted,
                               "--evidence", "fixture corrective revert")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads(result.stdout)["verdict"], "manual-revert")
        self.assertEqual((self.repo.state / "main-status").read_text(), "green " + reverted + "\n")

    def test_quarantine_admission_runs_ci_instead_of_carrying_a_previous_verdict(self):
        self.seed_green()
        self.publish(".bboxy/quarantine.md")
        result = self.watch()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.repo.state / "ci-calls").read_text().splitlines(), ["full", "full"])

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

    # keeps: landing.md §6 — missing artifacts use the unresolved recovery path.
    def test_missing_store_after_green_still_invokes_diagnosis_without_a_test_selector(self):
        self.seed_green()
        self.publish("crates/c/src/lib.rs")
        target = self.repo.root / "scratchpad/landing-watch/target"
        shutil.rmtree(target / "parity")
        (target / "debug/bboxybook").unlink()
        (self.repo.state / "watch.json").write_text('{"missing_store":true,"unresolved":true}')
        result = self.watch()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual((self.repo.state / "culprit-selector").read_text().strip(), "unknown:parity_store_missing")
        self.assertIn("parity_store_missing", (self.repo.state / "main-status").read_text())


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
        self.web = self.root / "lab/web"
        self.web.mkdir(parents=True)
        (self.web / "components.js").write_text("window.C = {};\n")
        for page in ("launcher.html", "settings.html"):
            (self.web / page).write_text('<script src="components.js"></script>')
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
        return {"tier": "lab", "paths": paths, "shared": list(shared),
                "lab_pages": landing.affected_lab_pages(self.root, paths)}

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

    def test_mixed_lab_and_native_edits_keep_both_components(self):
        data = self.data(["lab/web/launcher.html", "crates/bbt-ui/src/modules/settings.rs"])
        data["tier"] = "code"
        self.assertEqual(set(c["component"] for c in landing.parity_cases(self.cases, data)),
                         {"launcher", "settings"})

    def test_shared_lab_sample_does_not_reduce_explicit_native_component_cases(self):
        data = self.data(["lab/web/components.js", "crates/bbt-ui/src/modules/launcher.rs"])
        data["tier"] = "code"
        selected = landing.parity_cases(self.cases, data)
        self.assertEqual(len([c for c in selected if c["component"] == "launcher"]), 2)
        self.assertEqual(len(selected), 3)

    def test_missing_store_builds_bboxybook_alone(self):
        output = self.root / "out"
        landing.scoped_parity(self.root, self.data(["lab/web/launcher.html"]), output)
        cargo = [c for c in self.calls if c[0] == "cargo"]
        self.assertEqual(cargo, [["cargo", "build", "-p", "bboxy450-book", "--locked"]])
        self.assertEqual((output / "native/fixture.png").read_bytes(), b"rendered native")

    def test_native_input_uses_fresh_render_even_when_a_stored_set_exists(self):
        self.stored()
        output = self.root / "out"
        data = self.data(["design/tokens.toml"])
        data["tier"] = "code"
        landing.scoped_parity(self.root, data, output)
        self.assertIn(["cargo", "build", "-p", "bboxy450-book", "--locked"], self.calls)
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

    def test_unlisted_script_follows_transitive_imports_and_samples_affected_pages(self):
        (self.web / "components.js").write_text('import "./ui.js";')
        (self.web / "ui.js").write_text('import "./components.js";')  # a cycle is finite
        data = self.data(["lab/web/ui.js"])
        self.assertEqual(data["lab_pages"], ["lab/web/launcher.html", "lab/web/settings.html"])
        selected = landing.parity_cases(self.cases, data)
        self.assertEqual(len(selected), 2)
        self.assertEqual({c["page"] for c in selected}, {"launcher.html", "settings.html"})

    def test_half_the_catalogue_keeps_all_cases_and_navigation_does_not_add_dependencies(self):
        (self.web / "launcher.html").write_text('<script src="ui.js"></script><a href="settings.html">settings</a>')
        (self.web / "settings.html").write_text('<a href="launcher.html">launcher</a>')
        (self.web / "ui.js").write_text("window.UI={};")
        selected = landing.parity_cases(self.cases, self.data(["lab/web/ui.js"]))
        self.assertEqual(selected, self.cases[:2])
        self.assertEqual(landing.parity_cases(self.cases, self.data(["lab/web/settings.html"])), self.cases[2:])

    def test_generated_header_follows_records_design_generator_and_deleted_glob_source(self):
        design = self.root / "design"
        design.mkdir()
        (design / "colors.toml").write_text("color='blue'\n")
        records = self.root / ".bboxy/specs"
        records.mkdir(parents=True)
        (records / "a.md").write_text("fixture record\n")
        (self.web / "gen.py").write_text("# fixture generator\n")
        (self.web / "tokens.js").write_text('// GENERATED by lab/web/gen.py from design/colors.toml, .bboxy/specs/*.md. Do not edit.\n')
        (self.web / "components.js").write_text('import "./tokens.js";')
        for changed in ("design/colors.toml", ".bboxy/specs/a.md", ".bboxy/specs/deleted.md", "lab/web/gen.py"):
            with self.subTest(changed=changed):
                self.assertEqual(self.data([changed])["lab_pages"], ["lab/web/launcher.html", "lab/web/settings.html"])

    def test_css_resources_inline_scripts_queries_and_component_host_use_physical_pages(self):
        (self.web / "launcher.html").write_text('<style>@import "theme.css";</style><script>fetch("data.json")</script>')
        (self.web / "theme.css").write_text('body{background:url("image.svg?version=1")}')
        for changed in ("lab/web/theme.css", "lab/web/image.svg", "lab/web/data.json"):
            self.assertEqual(self.data([changed])["lab_pages"], ["lab/web/launcher.html"])
        cases = [{"page": "paths.html?scale=1", "component": "paths", "slug": "a"},
                 {"page": "paths.html?scale=2", "component": "paths", "slug": "b"},
                 {"page": "@component", "component": "seat-row", "slug": "c"}]
        self.assertEqual(landing.parity_cases(cases, self.data(["lab/web/paths.html"])), cases[:2])
        self.assertEqual(landing.parity_cases(cases, self.data(["lab/web/seat-row.html"])), cases[2:])

    def test_unreferenced_script_and_external_symlink_select_no_cases(self):
        (self.web / "showcase.html").write_text('<script src="showcase.js"></script>')
        (self.web / "gen.py").write_text('OUTPUT = "showcase.js"\n')
        (self.web / "tokens.js").write_text('// GENERATED by lab/web/gen.py from design/input.json.\n')
        (self.web / "components.js").write_text('import "./tokens.js";')
        self.assertEqual(landing.parity_cases(self.cases, self.data(["lab/web/showcase.js"])), [])
        outside = tempfile.TemporaryDirectory(prefix="landing-outside-resource-")
        self.addCleanup(outside.cleanup)
        secret = Path(outside.name) / "external.js"
        secret.write_text('import "components.js";')
        (self.web / "external.js").symlink_to(secret)
        (self.web / "launcher.html").write_text('<script src="external.js"></script>')
        self.assertEqual(self.data(["lab/web/external.js"])["lab_pages"], [])

    def test_page_recheck_intersects_derived_pages(self):
        cut={"affected":[],"lab_pages":["lab/web/a.html","lab/web/b.html"]}
        move={"affected":[],"lab_pages":["lab/web/b.html","lab/web/c.html"]}
        self.assertEqual(landing.parity_recheck_pages(cut,move),["lab/web/b.html"])
        move["lab_pages"]=["lab/web/c.html"]
        self.assertEqual(landing.parity_recheck_pages(cut,move),[])

    def test_page_recheck_keeps_cross_native_lab_coverage(self):
        cut={"affected":[],"lab_pages":["lab/web/a.html","lab/web/b.html"]}
        move={"affected":["bboxy450-ui"],"lab_pages":[]}
        self.assertEqual(landing.parity_recheck_pages(cut,move),cut["lab_pages"])
        self.assertEqual(landing.parity_recheck_pages(move,cut),cut["lab_pages"])

    def test_lab_page_recheck_uses_stored_native_and_only_selected_page(self):
        self.stored()
        data={**self.data(["lab/web/components.js"]),"affected":[],"parity":"yes"}
        plan=self.root/"recheck.json"
        plan.write_text(json.dumps({"affected":[],"parity_pages":["lab/web/settings.html"]}))
        narrowed=landing.recheck_data(data,plan)
        self.assertEqual(narrowed["tier"],"lab")
        output=self.root/"out"
        landing.scoped_parity(self.root,narrowed,output)
        self.assertEqual(json.loads((output/"requested.json").read_text()),self.cases[2:])
        self.assertFalse(any(c[0]=="cargo" for c in self.calls))

    def test_lab_recheck_keeps_formal_while_skipping_unchanged_records_and_invariants(self):
        data={**self.data(["lab/web/components.js"]),"affected":[],"parity":"yes"}
        plan=self.root/"recheck.json"
        plan.write_text(json.dumps({"affected":[],"parity_pages":["lab/web/settings.html"]}))
        with patch.object(landing,"repository",return_value=self.root),patch.object(landing,"affected",return_value=data),patch.object(landing,"records_checks") as records,patch.object(landing,"scoped_parity") as parity:
            self.assertEqual(landing.presubmit_worker("base",self.root/"result.json",plan),0)
        records.assert_not_called();parity.assert_called_once()
        self.assertEqual(self.calls,[["make", "test-formal"]])

    def test_recheck_rejects_page_paths_outside_lab(self):
        data={**self.data(["lab/web/components.js"]),"affected":[],"parity":"yes"}
        plan=self.root/"recheck.json";plan.write_text(json.dumps({"affected":[],"parity_pages":["../outside.html"]}))
        with self.assertRaises(landing.Refusal): landing.recheck_data(data,plan)


    def test_page_recheck_resamples_after_narrowing_the_page_scope(self):
        self.cases.append({"page": "settings.html", "component": "settings", "slug": "b"})
        data = self.data(["lab/web/components.js"])
        data["parity_pages"] = ["lab/web/settings.html"]
        self.assertEqual(landing.parity_cases(self.cases, data), self.cases[2:])

    def test_native_page_recheck_keeps_all_cases_in_a_small_scope(self):
        self.cases.append({"page": "settings.html", "component": "settings", "slug": "b"})
        data = self.data(["crates/bbt-render/src/lib.rs"])
        data.update(tier="code", parity_pages=["lab/web/settings.html"])
        self.assertEqual(landing.parity_cases(self.cases, data), self.cases[2:])

    def test_native_intersection_keeps_the_original_parity_coverage(self):
        cut = {"affected": ["bboxy450-ui"], "lab_pages": ["lab/web/launcher.html"]}
        moved = {"affected": ["bboxy450-ui"], "lab_pages": ["lab/web/settings.html"]}
        self.assertEqual(landing.parity_recheck_pages(cut, moved), [])

    def test_native_shared_sample_uses_physical_catalogue_pages(self):
        cases = [{"page": "paths.html?scale=1", "component": "paths", "slug": "a"},
                 {"page": "paths.html?scale=2", "component": "paths", "slug": "b"},
                 {"page": "@component", "component": "seat-row", "slug": "c"}]
        data = self.data(["crates/bbt-render/src/lib.rs"])
        data["tier"] = "code"
        selected = landing.parity_cases(cases, data)
        self.assertEqual(len(selected), 2)
        self.assertEqual({landing.catalogue_page(case) for case in selected},
                         {"lab/web/paths.html", "lab/web/seat-row.html"})

    def test_catalogue_pages_keep_their_resource_directory(self):
        cases = [{"page": "nested/paths.html?scale=1", "component": "paths", "slug": "a"},
                 {"page": "paths.html?scale=1", "component": "paths", "slug": "b"}]
        data = self.data(["lab/web/nested/paths.html"])
        self.assertEqual(landing.parity_cases(cases, data), cases[:1])


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
        "test_five_incremental_rechecks_are_79_with_no_candidate_push",
        "test_a_disjoint_move_after_five_rechecks_still_reuses_the_green",
        "test_disjoint_main_movement_rebases_but_does_not_repeat_checks",
        "test_reverse_dependency_overlap_rechecks",
        "test_notification_failure_can_resume_without_rechecking_or_repush")}
    extended_names.add("WatchTests.test_unresolved_failure_reverts_only_the_exact_failed_batch")
    if not extended:
        names = [name for name in names if name not in extended_names]
        print("Fast CI fixtures; 6 extended moving-main/recovery cases run with --all (presubmit).", flush=True)
    # Put multi-rebase fixtures first, so their serial Git steps do not form a
    # long tail after the many small state/parity checks.
    names.sort(key=lambda name: (0 if "five_incremental_rechecks" in name else
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
    recovery = SOURCE.with_name("test-landing-recovery.py")
    if extended and recovery.is_file():
        # Recovery/bisect fixtures are the extended presubmit tier, never the
        # fast ci-local hook. The shared signer is a throwaway fixture identity.
        recovered = subprocess.run([sys.executable, "-I", str(recovery)], env=environment,
                                   timeout=600)
        failures += bool(recovered.returncode)
    return bool(failures)


if __name__ == "__main__":
    if len(sys.argv) == 1 or sys.argv[1:] == ["--all"]:
        raise SystemExit(fixture_suite(extended=sys.argv[1:] == ["--all"]))
    unittest.main()
