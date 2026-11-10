# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Landing contracts exercised only on private throwaway repositories (#323 §12)."""
import importlib.util
import contextlib
import io
import atexit
import hashlib
import json
import os
import platform
from pathlib import Path
import subprocess
import threading
import shutil
import tempfile
import shlex
import sys
import time
import unittest
from unittest.mock import Mock, patch
from concurrent.futures import ThreadPoolExecutor

sys.dont_write_bytecode = True

SOURCE = Path(__file__).with_name("landing.py")
# The copy loop and production trigger both consume this tuple. Keep it literal
# so landing.py can read the declaration without importing this fixture runner.
FIXTURE_SCRIPTS = ("landing.py", "formal.py", "formal-stage.py", "harness-submodule.py", "presubmit.sh", "land.sh",
                   "quarantine.py", "quarantine.sh", "quarantine-test.sh", "main_watch.py", "main-watch.sh",
                   "landing_recovery.py", "main-red-bounded.sh", "release-inputs.py", "verify-signatures.py",
                   "verify-signatures-core.py", "test-landing-budget-controls.py", "qos.sh", "qos-policy.py")
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
                            ("commit.gpgsign", "true"),
                            # A developer's global excludes (e.g. a private
                            # `.bboxy/`) must not hide fixture records.
                            ("core.excludesFile", os.devnull)):
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
                                RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="",
                                BBOXY_TEST_RUNNER="cargo",  # fixtures fake libtest output
                                BBOXY_GATE_SLOTS="3", BBOXY_GATE_WAIT="0")
        self.environment.pop("BBOXY_CARGO_TARGET_DIR", None)
        # A parent make test may pass TARGET_DIR on its command line. GNU Make
        # exports those overrides through MAKEFLAGS; no private repository may
        # inherit them and write another fixture's formal evidence.
        for name in ("MAKEFLAGS", "MFLAGS", "GNUMAKEFLAGS", "MAKEOVERRIDES", "MAKELEVEL",
                     "TARGET_DIR", "CI_LOG", "CI_ARGS", "CARGO_BUILD_BUILD_DIR"):
            self.environment.pop(name, None)
        for name in FIXTURE_SCRIPTS:
            target = self.root / "scripts" / name
            target.parent.mkdir(exist_ok=True)
            source = "verify-signatures.py" if name == "verify-signatures-core.py" else name
            shutil.copy2(SOURCE.with_name(source), target)
        self.write("scripts/cache-env.sh", "# isolated fixture; never the production cache policy\nexport CARGO_BUILD_JOBS=2\n")
        shell = '#!/bin/sh\nprintf "%s\\n" "$0 $*" >> "$BBOXY_LANDING_STATE_DIR/shell.log"\n'
        for name in ("invariants", "layering", "pending-flags", "kill-own-pids", "generated-sources", "licenses",
                     "brand", "images", "cross-os", "never-block", "lab-check"):
            self.write(f"scripts/{name}.sh", shell)
        self.write("scripts/test-landing.py", f'FIXTURE_SCRIPTS = {FIXTURE_SCRIPTS!r}\nprint("nested fixture checks")\n')
        self.write("scripts/test-landing-budget-controls.py", 'print("nested budget controls")\n')
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
        self.write("Makefile", 'test:\n\t@sh scripts/ci-local.sh > "$(CI_LOG)" 2>&1\n\ntest-harness-native:\n\t@true\ntest-formal:\n\t@python3 -I scripts/formal-stage.py export -- python3 -c "pass"\n\t@python3 -I scripts/formal-stage.py self-tests -- python3 -c "pass"\n\t@python3 -I scripts/formal-stage.py verify -- python3 -c "pass"\n\t@python3 -I scripts/formal-stage.py parity -- python3 -I scripts/fixture-formal.py "$(TARGET_DIR)"\n')
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
if args[0]=="test" and cfg.get("coverage_real"):
    actual=list(args)
    if cfg.get("coverage_filtered") and "--exact" not in actual:
        if "--" not in actual:actual.append("--")
        actual += ["--skip","ctl_once"]
    child=subprocess.Popen([REAL,*actual],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    for line in child.stdout:
        if "Running tests/ctl.rs (" in line:
            with (state/"ctl-runs").open("a") as stream:stream.write("ctl\\n")
        print(line,end="",flush=True)
    child.stdout.close()
    raise SystemExit(child.wait())
if cfg.get("block")==args[0] and (not cfg.get("block_once") or not (state/"blocked-once").exists()):
    if cfg.get("block_once"):
        target=pathlib.Path(os.environ["BBOXY_CARGO_TARGET_DIR"])
        target.mkdir(parents=True,exist_ok=True)
        (target/"warm-build-artifact").write_text("first attempt")
    (state/"blocked-once").write_text("blocked")
    pid_file=state/"blocked.pid"
    pid_part=state/"blocked.pid.writing"
    pid_part.write_text(str(os.getpid()))
    pid_part.replace(pid_file)
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


class MemberOnlyManifestTests(unittest.TestCase):
    old_toml = '[workspace]\nmembers=["crates/a","crates/c"]\n[workspace.dependencies]\nshared="1"\n'
    new_toml = old_toml.replace('"crates/c"', '"crates/leaf","crates/c"')
    old_lock = ('version=4\n[[package]]\nname="a"\nversion="0.1.0"\ndependencies=["registry"]\n'
                '[[package]]\nname="registry"\nversion="1.0.0"\nsource="registry+fixture"\nchecksum="abc"\n')
    leaf = '[[package]]\nname="new-package"\nversion="0.1.0"\n'
    new_lock = old_lock + leaf
    packages = {"crates/a": "a", "crates/c": "c", "crates/leaf": "new-package"}

    def delta(self, **changes):
        values = dict(old_toml=self.old_toml, new_toml=self.new_toml,
                      old_lock=self.old_lock, new_lock=self.new_lock, packages=self.packages)
        values.update(changes)
        return landing.member_only_manifest_delta(**values)

    def test_appended_inserted_and_multiple_members_use_package_names(self):
        self.assertTrue(self.delta())
        self.assertTrue(self.delta(new_toml=self.old_toml.replace('"crates/c"]', '"crates/c","crates/leaf"]')))
        self.assertTrue(self.delta(new_toml=self.new_toml.replace('"crates/leaf"', '"crates/second","crates/leaf"'),
                                   new_lock=self.new_lock + self.leaf.replace('new-package', 'second'),
                                   packages={**self.packages, "crates/second": "second"}))

    def test_members_cannot_be_removed_changed_reordered_or_duplicated(self):
        for members in ('["crates/leaf","crates/c"]', '["crates/a","crates/leaf"]',
                        '["crates/c","crates/a","crates/leaf"]',
                        '["crates/a","crates/leaf","crates/c","crates/leaf"]',
                        '["crates/a","crates/c"]', '"crates/leaf"', '[1]'):
            with self.subTest(members=members):
                self.assertFalse(self.delta(new_toml='[workspace]\nmembers=' + members + '\n[workspace.dependencies]\nshared="1"\n'))

    def test_all_other_manifest_fields_must_be_equal(self):
        for text in (self.new_toml.replace('shared="1"', 'shared="2"'),
                     self.new_toml + 'other="1"\n',
                     self.new_toml + '[workspace.package]\nversion="2"\n',
                     self.new_toml + '[profile.dev]\nopt-level=1\n',
                     self.new_toml + '[features]\nextra=[]\n'):
            with self.subTest(text=text):
                self.assertFalse(self.delta(new_toml=text))

    def test_every_existing_lock_table_and_its_dependency_order_are_preserved(self):
        self.assertTrue(self.delta(new_lock='version=4\n' + self.leaf + self.old_lock.removeprefix('version=4\n')))
        for text in (self.new_lock.replace('version="0.1.0"', 'version="0.2.0"', 1),
                     self.new_lock.replace('name="a"', 'name="renamed"'),
                     self.new_lock.replace('source="registry+fixture"', 'source="registry+other"'),
                     self.new_lock.replace('checksum="abc"', 'checksum="def"'),
                     self.new_lock.replace('["registry"]', '["registry","new-package"]')):
            with self.subTest(text=text):
                self.assertFalse(self.delta(new_lock=text))
        removed = self.new_lock.replace('[[package]]\nname="a"\nversion="0.1.0"\ndependencies=["registry"]\n', '')
        landing.tomllib.loads(removed)
        self.assertFalse(self.delta(new_lock=removed))
        self.assertFalse(self.delta(old_lock=self.old_lock
                                   + '[[package]]\nname="a"\nversion="0.1.0"\ndependencies=["registry"]\n'))
        self.assertFalse(self.delta(old_lock=self.old_lock.replace('["registry"]', '["registry","other"]'),
                                   new_lock=self.new_lock.replace('["registry"]', '["other","registry"]')))

    def test_extra_lock_tables_are_only_added_path_members(self):
        for extra in (self.leaf + 'source=""\n', self.leaf.replace('new-package', 'unrelated'),
                      self.leaf + 'source="registry+fixture"\nchecksum="abc"\n'):
            with self.subTest(extra=extra):
                self.assertFalse(self.delta(new_lock=self.new_lock + extra))
        self.assertFalse(self.delta(packages={}))
        self.assertFalse(self.delta(new_toml=self.new_toml.replace('crates/leaf', 'crates/*')))

    def test_lock_metadata_and_table_shape_must_match(self):
        for text in (self.new_lock.replace('version=4', 'version=3'),
                     self.new_lock + '[metadata]\nextra=true\n', 'version=4\npackage=[1]\n'):
            with self.subTest(text=text):
                self.assertFalse(self.delta(new_lock=text))

    def test_unchanged_lock_is_allowed_but_lock_only_or_missing_endpoint_is_broad(self):
        self.assertTrue(self.delta(old_lock=None, new_lock=None))
        self.assertTrue(self.delta(new_lock=self.old_lock))
        for changes in (dict(old_toml=None, new_toml=None), dict(old_toml=None),
                        dict(new_toml=None), dict(old_lock=None), dict(new_lock=None),
                        dict(old_toml=None, new_toml=None, new_lock=self.old_lock + '# comment\n')):
            with self.subTest(changes=changes):
                self.assertFalse(self.delta(**changes))

    def test_parse_failures_and_missing_members_are_broad(self):
        for field in ('old_toml', 'new_toml', 'old_lock', 'new_lock'):
            with self.subTest(field=field):
                self.assertFalse(self.delta(**{field: '['}))
        self.assertFalse(self.delta(old_toml='[workspace]\n'))
        self.assertFalse(self.delta(new_toml='workspace=1\n'))


class AffectedTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)

    def changed(self, path):
        self.repo.write(path, "// fixture change\n")
        self.repo.commit("Fixture change")
        return landing.affected(self.repo.base + "..HEAD", root=self.repo.root)

    def add_leaf(self):
        manifest = self.repo.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('"crates/c"', '"crates/leaf","crates/c"'))
        self.repo.write("crates/leaf/Cargo.toml", '[package]\nname="new-package"\nversion="0.1.0"\nedition="2024"\n')
        self.repo.write("crates/leaf/src/lib.rs", "pub fn leaf() {}\n")
        run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)

    def manifest_result(self):
        self.repo.commit("Fixture manifest delta")
        return landing.affected(self.repo.base + "..HEAD", root=self.repo.root)

    def assert_broad_manifest(self):
        result = self.manifest_result()
        self.assertEqual(result["touched"], sorted(landing.metadata(self.repo.root)))
        self.assertEqual(result["affected"], result["touched"])
        self.assertEqual(result["tier"], "wide")

    def test_member_only_leaf_addition_tests_exactly_the_new_crate(self):
        self.add_leaf()
        result = self.manifest_result()
        self.assertEqual(result["touched"], ["new-package"])
        self.assertEqual(result["affected"], ["new-package"])
        self.assertNotEqual(result["tier"], "wide")
        self.assertEqual(result["shared"], ["Cargo.lock", "Cargo.toml"])
        self.assertEqual(landing.lean(self.repo.base + "..HEAD", root=self.repo.root), ['test new-package --lib'])

    def test_member_addition_with_root_dependency_version_change_is_broad(self):
        manifest = self.repo.root / "Cargo.toml"
        manifest.write_text(manifest.read_text() + '[workspace.dependencies]\nunused="1"\n')
        self.repo.base = self.repo.commit("Fixture shared dependency")
        self.add_leaf()
        manifest.write_text(manifest.read_text().replace('unused="1"', 'unused="2"'))
        self.assert_broad_manifest()

    def test_member_addition_with_registry_lock_addition_is_broad(self):
        self.add_leaf()
        lock = self.repo.root / "Cargo.lock"
        lock.write_text(lock.read_text() + '[[package]]\nname="third-party"\nversion="1.0.0"\nsource="registry+fixture"\nchecksum="abc"\n')
        self.assert_broad_manifest()

    def test_member_addition_with_changed_existing_lock_dependencies_is_broad(self):
        self.add_leaf()
        lock = self.repo.root / "Cargo.lock"
        lock.write_text(lock.read_text().replace('name = "a"', 'dependencies = ["new-package"]\nname = "a"'))
        self.assert_broad_manifest()

    def test_member_removal_is_broad(self):
        manifest = self.repo.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace(',"crates/c"', ''))
        run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)
        self.assert_broad_manifest()

    def test_member_addition_never_narrows_toolchain_or_cargo_config_changes(self):
        self.add_leaf()
        toolchain = run("rustup", "show", "active-toolchain", cwd=self.repo.root).split()[0]
        self.repo.write("rust-toolchain.toml", f'[toolchain]\nchannel="{toolchain}"\n')
        self.assert_broad_manifest()
        (self.repo.root / "rust-toolchain.toml").unlink()
        self.repo.write(".cargo/config.toml", '[build]\njobs=2\n')
        self.assert_broad_manifest()

    def test_lock_only_change_is_broad(self):
        lock = self.repo.root / "Cargo.lock"
        lock.write_text(lock.read_text() + '# lock-only edit\n')
        self.assert_broad_manifest()

    def test_member_addition_with_existing_consumer_and_regenerated_lock_is_broad(self):
        self.add_leaf()
        manifest = self.repo.root / "crates/b/Cargo.toml"
        manifest.write_text(manifest.read_text() + 'new-package={path="../leaf"}\n')
        run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)
        lock = landing.tomllib.loads((self.repo.root / "Cargo.lock").read_text())
        self.assertIn('new-package', next(p for p in lock['package'] if p['name'] == 'b')['dependencies'])
        self.assert_broad_manifest()

    def test_missing_old_lock_is_broad(self):
        (self.repo.root / "Cargo.lock").unlink()
        self.repo.base = self.repo.commit("Fixture missing old lock")
        self.add_leaf()
        self.assert_broad_manifest()

    def test_historical_ranges_read_their_own_new_manifest_endpoint(self):
        self.add_leaf()
        member_revision = self.repo.commit("Fixture historical member-only endpoint")
        manifest = self.repo.root / "Cargo.toml"
        member_text = manifest.read_text()
        manifest.write_text(member_text + '[profile.dev]\nopt-level=1\n')
        broad_revision = self.repo.commit("Fixture later broad endpoint")
        result = landing.affected(self.repo.base + ".." + member_revision, root=self.repo.root)
        self.assertEqual(result["touched"], ["new-package"])
        manifest.write_text(member_text)
        result = landing.affected(self.repo.base + ".." + broad_revision, root=self.repo.root)
        self.assertEqual(result["touched"], ["a", "b", "c", "new-package"])
        result = landing.affected(self.repo.base + "..", root=self.repo.root)
        self.assertEqual(result["touched"], ["a", "b", "c", "new-package"])
        result = landing.affected(self.repo.base, root=self.repo.root)
        self.assertEqual(result["touched"], ["new-package"])

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
        for name in ("bboxy450-term-proto", "bboxy450-term-core"):
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


class ConsolidatedIntegrationTests(unittest.TestCase):
    def test_repository_entrypoints_include_every_integration_file(self):
        for crate in ("bbt-mux", "bboxy450-term"):
            with self.subTest(crate=crate):
                base = SOURCE.parents[1] / "crates" / crate
                manifest = landing.tomllib.loads((base / "Cargo.toml").read_text())
                self.assertIs(manifest["package"]["autotests"], False)
                separate = ["scrollback"] if crate == "bboxy450-term" else []
                self.assertEqual(manifest["test"], [{"name": name, "path": f"tests/{name}.rs"}
                                                   for name in ["it", *separate]])
                entry = (base / "tests/it.rs").read_text()
                declared = landing.re.findall(r'#\[path = "([^/\"]+)\.rs"\]\s*mod (\w+);', entry)
                # These two known Q1 support files have explicit existing
                # owners. They cannot compile as independent `it` modules.
                # Verify their exact ownership before excluding only them.
                support = {"dream_read"}
                self.assertTrue((base / "tests/dream_read.rs").is_file())
                if crate == "bbt-mux":
                    source = (base / "src/logic.rs").read_text()
                    mask, _ = landing.rust_paths(source)
                    owners = list(landing.re.finditer(r'\bmod\s+dream_public_api_tests\s*\{', mask))
                    self.assertEqual(len(owners), 1)
                    first = mask.index("{", owners[0].start()) + 1
                    last = landing.balanced_end(mask, first, "{", "}")
                    self.assertRegex(source[first:last],
                        r'include!\(\s*concat!\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*"/tests/dream_read\.rs"\s*\)\s*\)\s*;')
                    self.assertEqual(source.count('"/tests/dream_read.rs"'), 1)
                else:
                    owner = base / "tests/formal_contracts/models.rs"
                    models = owner.read_text()
                    self.assertEqual(models.count('#[path = "../dream_read.rs"]'), 1)
                    self.assertRegex(models,
                        r'#\[path = "\.\./dream_read\.rs"\]\s*mod dream_read_process;')
                    self.assertEqual((owner.parent / "../dream_read.rs").resolve(),
                                     (base / "tests/dream_read.rs").resolve())
                    self.assertRegex((base / "tests/formal_contracts.rs").read_text(),
                        r'#\[path = "formal_contracts/models\.rs"\]\s*mod models;')
                expected = sorted(p.stem for p in (base / "tests").glob("*.rs")
                                  if p.stem not in {"it", *separate, *support})
                self.assertEqual(sorted(name for name, _ in declared), expected)
                self.assertTrue(all(name == module for name, module in declared))
                for name in [*expected, *support]:
                    self.assertNotIn("#[global_allocator]", (base / f"tests/{name}.rs").read_text())

    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        manifest = self.repo.root / "crates/c/Cargo.toml"
        manifest.write_text(manifest.read_text().replace('[package]', '[package]\nautotests=false')
                            + '\n[[test]]\nname="it"\npath="tests/it.rs"\n')
        self.repo.write("crates/c/tests/it.rs", '#[path="alpha.rs"] mod alpha;\n#[path="beta.rs"] mod beta;\n')
        self.repo.write("crates/c/tests/alpha.rs", '#[test] fn selected() {}\n')
        self.repo.write("crates/c/tests/beta.rs", '#[test] fn unselected() { panic!("unselected module ran"); }\n')
        self.repo.base = self.repo.commit("Private consolidated integration target")

    def plan(self, *paths):
        for path in paths:
            file = self.repo.root / path
            self.repo.write(path, (file.read_text() if file.exists() else "") + "// fixture change\n")
        self.repo.commit("Private selected files")
        return landing.lean(self.repo.base + "..HEAD", root=self.repo.root)

    def test_one_changed_file_selects_only_its_module(self):
        self.assertEqual(self.plan("crates/c/tests/alpha.rs"),
                         ["test c --test it -E '(binary(=it)&test(/^(alpha)::/))'"])

    def test_separate_allocator_binary_does_not_select_shared_modules(self):
        manifest = self.repo.root / "crates/c/Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[[test]]\nname="allocator"\npath="tests/allocator.rs"\n')
        self.repo.write("crates/c/tests/allocator.rs", '#[test] fn private_allocator() {}\n')
        self.repo.base = self.repo.commit("Private standalone allocator target")
        self.assertEqual(self.plan("crates/c/tests/allocator.rs"), ["test c --test allocator"])

    def test_nested_module_and_source_preserve_unit_tests(self):
        self.assertEqual(self.plan("crates/c/tests/alpha/child.rs", "crates/c/src/lib.rs"),
                         ["test c --lib --test it -E '(binary(=it)&test(/^(alpha)::/))|(kind(lib))'"])

    def test_quoted_filter_is_one_argument_without_shell_evaluation(self):
        expression = "binary(it) & test(/^alpha::/)"
        with patch.object(landing.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)) as run:
            self.assertEqual(landing.lean_test("c", shlex.join(["--test", "it", "-E", expression])), 0)
        self.assertEqual(run.call_args.args[0], ["sh", "scripts/quarantine-test.sh", "--allow-ignored-only",
                                               "--locked", "-p", "c", "--test", "it", "-E", expression])

    def test_invariant_selection_and_coverage_use_the_qualified_test(self):
        source = SOURCE.with_name("invariants.sh").read_text().split("<<'PY_INVARIANTS'\n", 1)[1]
        functions = [node for node in landing.ast.parse(source).body
                     if isinstance(node, landing.ast.FunctionDef) and node.name in {"test_check", "covered"}]
        namespace = {"re": landing.re, "shlex": shlex, "tomllib": landing.tomllib,
                     "workspace_packages": lambda: {"c": self.repo.root / "crates/c/Cargo.toml"}}
        exec(compile(landing.ast.Module(body=functions, type_ignores=[]), "invariant module selection", "exec"), namespace)
        check = namespace["test_check"]("cargo test -p c --locked --test alpha selected -- --exact")
        self.assertEqual(check["targets"], [("--test", "it")])
        self.assertEqual(check["test"], "alpha::selected")
        row = {"package": "c", "kind": "test", "target": "it", "test": "alpha::selected", "result": "ok"}
        self.assertTrue(namespace["covered"](check, [row]))
        self.assertFalse(namespace["covered"](check, [dict(row, result="ignored")]))
        manifest = self.repo.root / "crates/c/Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[[test]]\nname="allocator"\npath="tests/allocator.rs"\n')
        self.repo.write("crates/c/tests/allocator.rs", '#[test] fn private_allocator() {}\n')
        separate = namespace["test_check"]("cargo test -p c --locked --test allocator private_allocator -- --exact")
        self.assertEqual(separate["targets"], [("--test", "allocator")])
        self.assertEqual(separate["test"], "private_allocator")
        self.assertTrue(namespace["covered"](separate, [dict(row, target="allocator", test="private_allocator")]))

    def test_shared_helpers_select_the_whole_integration_binary(self):
        for path in ("crates/c/tests/common/mod.rs", "crates/c/tests/support/events.rs", "crates/c/tests/it.rs"):
            with self.subTest(path=path):
                self.assertEqual(self.plan(path), ["test c --test it"])

    def test_make_land_runs_only_the_changed_module(self):
        self.repo.tooling()
        for name in ("land-fast.sh", "affected.sh"):
            shutil.copy2(SOURCE.with_name(name), self.repo.root / "scripts" / name)
        # Use real Cargo/nextest, a private signer, and a local bare origin.
        # The unselected module panics if the actual land selection is widened.
        (self.repo.bin / "cargo").unlink()
        environment = dict(self.repo.environment, BBOXY_TEST_RUNNER="nextest",
                           CARGO_BUILD_BUILD_DIR=str(self.repo.state / "target"))
        makefile = self.repo.root / "Makefile"
        makefile.write_text(makefile.read_text() + '\nland:\n\t@sh scripts/land-fast.sh\ninit-submodules:\n\t@true\n')
        self.repo.commit("Private lean landing workflow")
        self.repo.remote()
        self.repo.write("crates/c/tests/alpha.rs", '#[test] fn selected() {} // changed\n')
        candidate = self.repo.commit("Private one-module cut")
        result = subprocess.run(["make", "land"], cwd=self.repo.root, env=environment,
                                capture_output=True, text=True, timeout=45)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("1 test run: 1 passed", result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), candidate)


class HarnessBumpTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.harness = self.repo.root / landing.HARNESS_PATH
        self.repo.write(landing.HARNESS_PATH + "/Cargo.toml", '''[package]
name="bboxy450-harness"
version="0.1.0"
edition="2024"
[features]
default=["standalone"]
standalone=[]
embedded=[]
''')
        self.repo.write(landing.HARNESS_PATH + "/src/lib.rs", '''pub fn value() {}
#[cfg(test)] mod tests {
 #[test] fn term_features() {
  assert!(cfg!(feature="embedded"));
  assert!(!cfg!(feature="standalone"));
 }
}
''')
        # Compiling this integration target is an error: only the harness's
        # own Make workflow supplies its standalone fixture environment.
        self.repo.write(landing.HARNESS_PATH + "/tests/standalone.rs",
                        'compile_error!("harness Make owns this integration fixture");\n')
        run("git", "init", "-q", "--initial-branch=main", cwd=self.harness)
        key, wrapper = fixture_identity()
        for name, value in (("user.name", "Landing Fixture"),
                            ("user.email", "fixture@bboxy450.invalid"),
                            ("user.signingkey", key), ("gpg.program", str(wrapper)),
                            ("commit.gpgsign", "true")):
            run("git", "config", name, value, cwd=self.harness)
        self.harness_commit()
        manifest = self.repo.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('"crates/c"',
                            '"crates/c","' + landing.HARNESS_PATH + '"'))
        consumer = self.repo.root / "crates/b/Cargo.toml"
        consumer.write_text(consumer.read_text() +
            'renamed_harness={package="bboxy450-harness",path="../bboxy450-harness",default-features=false,features=["embedded"]}\n')
        for name in ("b", "c"):
            self.repo.write(f"crates/{name}/tests/consumer.rs",
                            '#[test] fn embedded_dependency_contract() {}\n')
        run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)
        self.repo.base = self.repo.commit("Private embedded harness gitlink")

    def harness_commit(self):
        run("git", "add", ".", cwd=self.harness)
        run("git", "commit", "-q", "-m", "Private harness source", cwd=self.harness)

    def bump(self, lock=False):
        source = self.harness / "src/lib.rs"
        source.write_text(source.read_text() + "// private pointer bump\n")
        self.harness_commit()
        if lock:
            path = self.repo.root / "Cargo.lock"
            path.write_text(path.read_text() + "# pointer-bump lock update\n")
        return self.repo.commit("Private harness pointer update")

    def test_gitlink_bump_checks_unit_only_with_term_features_and_full_transitive_tests(self):
        self.bump()
        selected = landing.lean(self.repo.base + "..HEAD", root=self.repo.root)
        self.assertEqual(selected, [
            "harness-check bboxy450-harness --lib --no-default-features --features embedded",
            "harness-test bboxy450-harness --lib --no-default-features --features embedded",
            "full b", "full c",
        ])
        self.assertNotIn("full a", selected, "a is a dependency, not a harness dependent")

    def test_lock_bump_keeps_ordinary_workspace_coverage_and_harness_owned_fixtures_out(self):
        self.bump(lock=True)
        selected = landing.lean(self.repo.base + "..HEAD", root=self.repo.root)
        self.assertIn("test a --lib", selected)
        self.assertIn("full b", selected)
        self.assertIn("full c", selected)
        self.assertFalse(any("standalone" in line or "--test" in line for line in selected))
        wide = landing.lean(self.repo.base + "..HEAD", root=self.repo.root, wide=True)
        self.assertIn("full a", wide)
        self.assertNotIn("test a --lib", wide)

    def test_native_protocol_owners_and_their_dependents_run_full_without_cargo_producer_edge(self):
        self.assertIn("bboxy450-term-proto", landing.HARNESS_CONSUMER_OWNERS,
                      "the environment-selected signed producer corpus has no Cargo edge")
        members = []
        for name in sorted(landing.HARNESS_CONSUMER_OWNERS):
            directory = "crates/" + name
            members.append(directory)
            self.repo.write(directory + "/Cargo.toml",
                f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2024"\n')
            self.repo.write(directory + "/src/lib.rs", 'pub fn value() {}\n')
            self.repo.write(directory + "/tests/native.rs", '#[test] fn native_wire() {}\n')
        members.append("crates/transport-dependent")
        self.repo.write("crates/transport-dependent/Cargo.toml", '''[package]
name="transport-dependent"
version="0.1.0"
edition="2024"
[dependencies]
wire={package="bboxy450-harness-wake-up",path="../bboxy450-harness-wake-up"}
''')
        self.repo.write("crates/transport-dependent/src/lib.rs", 'pub fn value() {}\n')
        manifest = self.repo.root / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('"crates/c"',
                            '"crates/c",' + ",".join(json.dumps(p) for p in members)))
        run("cargo", "generate-lockfile", "--offline", cwd=self.repo.root)
        self.repo.base = self.repo.commit("Private protocol owners without producer dependency")
        dependencies = landing.package_dependencies(landing.metadata(self.repo.root))
        for owner in landing.HARNESS_CONSUMER_OWNERS:
            self.assertNotIn(landing.HARNESS_PACKAGE, dependencies[owner])
        self.bump()
        plan = landing.lean(self.repo.base + "..HEAD", root=self.repo.root)
        for owner in landing.HARNESS_CONSUMER_OWNERS | {"transport-dependent", "b", "c"}:
            self.assertIn("full " + owner, plan)
        self.assertNotIn("full a", plan)

    def test_ordinary_lock_change_keeps_existing_lean_plan(self):
        path = self.repo.root / "Cargo.lock"
        path.write_text(path.read_text() + "# ordinary lock change\n")
        self.repo.commit("Private ordinary lock update")
        selected = landing.lean(self.repo.base + "..HEAD", root=self.repo.root)
        self.assertEqual(selected, ["test a --lib", "test b --lib",
                                    "test bboxy450-harness --lib", "test c --lib"])

    def test_source_tree_or_non_gitlink_cannot_receive_pointer_policy(self):
        with patch.object(landing, "changed_paths", return_value=[landing.HARNESS_PATH,
                            landing.HARNESS_PATH + "/src/lib.rs"]):
            self.assertFalse(landing.harness_pointer_bump("base..HEAD", root=self.repo.root))
        with patch.object(landing, "changed_paths", return_value=[landing.HARNESS_PATH]), \
             patch.object(landing, "git", return_value=":100644 100644 old new M\0" + landing.HARNESS_PATH + "\0"):
            self.assertFalse(landing.harness_pointer_bump("base..HEAD", root=self.repo.root))

    def gate(self, mode):
        self.repo.tooling()
        for name in ("land-fast.sh", "affected.sh"):
            shutil.copy2(SOURCE.with_name(name), self.repo.root / "scripts" / name)
        path = self.repo.root / "Makefile"
        path.write_text(path.read_text() + "\ninit-submodules:\n\t@true\n")
        self.repo.commit("Private pointer-gate tooling")
        self.repo.remote()
        candidate = self.bump(lock=True)
        environment = dict(self.repo.environment)
        if mode is not None:
            environment["LAND_MODE"] = mode
        else:
            environment.pop("LAND_MODE", None)
        result = subprocess.run(["sh", "scripts/land-fast.sh"], cwd=self.repo.root,
            env=environment, capture_output=True, text=True, timeout=45)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), candidate)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        harness = [args for args in calls if any(args[i:i+2] == ["-p", landing.HARNESS_PACKAGE]
                                                for i in range(len(args) - 1))]
        self.assertEqual({args[0] for args in harness}, {"check", "test"})
        for args in harness:
            self.assertIn("--lib", args)
            self.assertIn("--no-default-features", args)
            self.assertEqual(args[args.index("--features") + 1], "embedded")
            self.assertFalse(set(args) & {"--all-targets", "--tests", "--test", "--bins", "--doc"})
        for name in ("b", "c"):
            tests = [args for args in calls if args[0] == "test"
                     and any(args[i:i+2] == ["-p", name] for i in range(len(args) - 1))]
            self.assertTrue(tests, f"missing full term dependent: {name}")
            self.assertTrue(any(not set(args) & {"--lib", "--test", "--bins"} for args in tests))

    def test_default_lean_gate_runs_pointer_policy_without_harness_integration(self):
        self.gate(None)

    def test_explicit_wide_gate_retains_term_coverage_without_harness_integration(self):
        self.gate("wide")


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
        self.assertIn(["test", "--locked", "-p", "bboxy450-term-proto", "--test", "gui_wire_golden"], calls)
        self.assertFalse(any("golden" in call for call in calls))

    def test_dispatch_cut_still_refuses_a_zero_test_gui_wire_target(self):
        (self.repo.state / "behavior.json").write_text('{"golden_names":[]}')
        self.cut("crates/c/src/logic.rs")
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("golden_dispatch_tests_missing", result.stdout + result.stderr)

    def test_landing_script_cut_runs_its_fixtures_and_only_non_cargo_invariants(self):
        self.cut("scripts/main_watch.py")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("nested fixture checks", result.stdout)
        self.assertIn("invariants.sh --affected  --covered-tests ", (self.repo.state / "shell.log").read_text())
        self.assertFalse((self.repo.state / "cargo.jsonl").exists())

    def test_script_only_report_trailer_selects_the_committed_tree_gate(self):
        self.repo.write("scripts/fixture-gate.sh", "// fixture cut\n")
        self.repo.commit("Fixture report candidate\n\nReport: [[reports/2026-10-31-work]]")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        self.assertEqual(len(calls), 1)
        self.assertIn("reports::committed_report_trailers_resolve_in_their_own_trees", calls[0])

    def test_script_only_report_gate_failure_refuses_presubmit(self):
        self.repo.write("scripts/fixture-gate.sh", "// fixture cut\n")
        self.repo.commit("Fixture report candidate\n\nReport: [[missing]]")
        (self.repo.state / "behavior.json").write_text('{"fail":"test"}')
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("reports::committed_report_trailers_resolve_in_their_own_trees", result.stdout + result.stderr)

    def test_formal_failure_refuses_a_scoped_presubmit_before_cargo(self):
        self.repo.write("Makefile", "test-harness-native:\n\t@true\ntest-formal:\n\t@exit 23\n")
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("test-formal", result.stdout + result.stderr)
        self.assertFalse((self.repo.state / "cargo.jsonl").exists())

    def test_code_cut_passes_the_affected_crates_to_invariants(self):
        self.cut("crates/c/src/lib.rs")
        result = self.repo.presubmit()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertRegex((self.repo.state / "shell.log").read_text(),
                         r"invariants\.sh --affected c --covered-tests [^\n]+/quarantine-coverage\.json\n")

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
        self.assertEqual(json.loads(trace.removeprefix('ci-command: ')),
                         ['sh', 'scripts/qos.sh', 'utility', 'sh', 'scripts/layering.sh'])
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

    def deadline_fixture(self, *, parent_only=False):
        # Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        # keeps: #323 §3 and kill-only-own-pids. The fake build supplies its
        # elapsed compile time after its real child is paused. Host scheduling
        # cannot choose which clock expires. The wall clock only bounds a
        # broken fixture that never reaches that observation.
        self.cut("crates/c/src/lib.rs")
        (self.repo.state / "behavior.json").write_text('{"block":"test"}')
        marker = self.repo.state / "blocked.pid"
        owned_pids = set()
        ceiling = time.monotonic() + 45
        original_clock = landing.TwoClocks
        class ReportedBuild(original_clock):
            def __init__(clock, started):
                super().__init__(started)
                clock.previous = marker.read_text() if marker.exists() else None
            def expired(clock, ended, build_cap, run_bound):
                observed = marker.read_text() if marker.exists() else None
                if observed is not None and observed != clock.previous:
                    owned_pids.add(int(observed))
                    clock.event({"at": max(clock.at, ended), "phase": "compile"})
                    clock.event({"at": clock.at + build_cap + 1, "phase": "other"})
                    return super().expired(clock.at, build_cap, run_bound)
                if ended > ceiling:
                    raise AssertionError("fake build never entered its paused child")
                return None
        def alive(pid):
            row = subprocess.run(["ps", "-ww", "-p", str(pid), "-o", "stat=", "-o", "command="],
                                 text=True, capture_output=True, check=False).stdout.strip()
            return bool(row) and not row.startswith("Z") and str(self.repo.bin / "cargo") in row
        def parent_stop(child):
            child.terminate()
            child.wait(timeout=5)
        control = subprocess.Popen([sys.executable, "-c", "import signal; signal.pause()"])
        try:
            with patch.dict(os.environ, self.repo.environment, clear=True), \
                 patch.object(landing, "TwoClocks", ReportedBuild), \
                 patch.object(landing, "repository", return_value=self.repo.root), \
                 patch.object(landing, "stop_worker", parent_stop if parent_only else landing.stop_worker), \
                 patch("sys.stdout", io.StringIO()):
                code = landing.presubmit(self.repo.base)
            rows = [json.loads(line) for line in (self.repo.state / "presubmit.log").read_text().splitlines()]
            self.assertEqual(code, 124)
            self.assertEqual(len(rows), 2)
            self.assertEqual([row["attempt"] for row in rows], [0, 1])
            self.assertEqual(rows[-1]["finding"], "recurring_build_cap")
            record = rows[-1]
            self.assertEqual(record["exit"], 124)
            self.assertEqual(record["error"], "build_cap")
            self.assertGreaterEqual(record["compile_seconds"], 1800)
            self.assertIsNone(control.poll(), "unrelated process was killed")
            self.assertEqual(len(owned_pids), 2, "both real build children must enter")
            until = time.monotonic() + 3
            while any(alive(pid) for pid in owned_pids) and time.monotonic() < until:
                time.sleep(0.01)
            self.assertFalse(any(alive(pid) for pid in owned_pids), "owned build child survived the deadline")
        finally:
            control.terminate()
            control.wait()
            for owned in owned_pids:
                if alive(owned):
                    os.kill(owned, 15)  # exact PID created and recorded by this fixture

    def test_deadline_is_enforced_and_does_not_kill_an_unrelated_process(self):
        self.deadline_fixture()

    def test_deadline_broken_control_detects_a_surviving_owned_child(self):
        # Deliberately stop only the worker, leaving its paused build child.
        with self.assertRaisesRegex(AssertionError, "owned build child survived"):
            self.deadline_fixture(parent_only=True)

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
        self.assertTrue(landing.cargo_backed("R5G_SEATS=4 cargo test -p bboxy450-term-mux"))
        self.assertTrue(landing.cargo_backed("sh scripts/ui-lint.sh"))
        self.assertFalse(landing.cargo_backed("sh scripts/brand.sh"))


class FormalRetentionTests(unittest.TestCase):
    def setUp(self):
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        spec = importlib.util.spec_from_file_location("retention_formal", SOURCE.with_name("formal.py"))
        self.formal = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.formal)
        self.store = self.formal.evidence_store(self.repo.root)
        target = Path(self.repo.environment["CARGO_TARGET_DIR"]) / "formal"
        models = target / "models"
        models.mkdir(parents=True, exist_ok=True)
        self.graph = b"1 -> 2; graph state edge label and repeated metadata\n" * 20000
        (models / "Fixture.states").write_bytes(self.graph)
        transcript = run(sys.executable, "-I", "scripts/fixture-formal.py", cwd=self.repo.root,
                         env=self.repo.environment)
        self.receipt = landing.formal_receipt(transcript, run("git", "rev-parse", "HEAD", cwd=self.repo.root), self.repo.root)
        self.live = Path(self.receipt["evidence"]).parent

    def cold(self, token, *, source=None):
        data = json.loads((self.live / "evidence.json").read_text())
        data.update(source=source or "f" * 40, retention_fixture=token)
        raw = json.dumps(data).encode()
        path = self.store / hashlib.sha256(raw).hexdigest()
        shutil.copytree(self.live, path, copy_function=os.link)
        # Never overwrite a hardlinked live evidence inode.
        (path / "evidence.json").unlink()
        (path / "evidence.json").write_bytes(raw)
        return path

    def test_live_heads_pending_reports_and_newest_are_kept_cold_is_archived(self):
        older = run("git", "rev-parse", "HEAD~1", cwd=self.repo.root)
        worktree = self.repo.root / ".retention-live-tree"
        run("git", "worktree", "add", "--detach", "--no-checkout", str(worktree), older, cwd=self.repo.root)
        pinned_tree = self.cold("worktree", source=older)
        run("git", "update-ref", "refs/remotes/origin/main", "HEAD", cwd=self.repo.root)
        pinned_main = self.cold("remote", source=run("git", "rev-parse", "HEAD", cwd=self.repo.root))
        pending = self.cold("pending")
        (self.repo.root / ".bboxy/tasks").mkdir(parents=True, exist_ok=True)
        (self.repo.root / ".bboxy/reports").mkdir(parents=True, exist_ok=True)
        (self.repo.root / ".bboxy/tasks/pending.md").write_text('---\nstatus: reported\n---\n')
        (self.repo.root / ".bboxy/reports/pending.md").write_text(
            '---\ntask: "[[tasks/pending]]"\n---\nEvidence ' + pending.name)
        closed = self.cold("closed-without-acceptance")
        (self.repo.root / ".bboxy/tasks/closed.md").write_text('---\nstatus: closed\n---\n')
        (self.repo.root / ".bboxy/reports/closed.md").write_text(
            '---\ntask: "[[tasks/closed]]"\n---\nEvidence ' + closed.name)
        cold = self.cold("cold")
        (self.repo.root / ".bboxy/tasks/accepted.md").write_text('---\nstatus: accepted\n---\n')
        (self.repo.root / ".bboxy/reports/accepted.md").write_text(
            '---\ntask: "[[tasks/accepted]]"\n---\nEvidence ' + cold.name)
        newest = self.cold("newest")
        os.utime(newest, (time.time() + 10, time.time() + 10))
        self.formal.prune_evidence(self.repo.root, keep=1)
        self.assertFalse(cold.exists())
        for path in (self.live, pinned_tree, pinned_main, pending, closed, newest):
            self.assertTrue(path.exists(), path.name + " lost its live/newest pin")
        archive = next(self.store.with_name(self.store.name + "-archive").glob("*.tar.zst"))
        manifest = json.loads(archive.with_suffix(".sha256.json").read_text())
        self.formal.verify_archive(archive, manifest)
        self.assertTrue(any(name.startswith(cold.name + "/") for name in manifest["files"]))
        # Canonical checker still accepts the same source-bound receipt after prune.
        landing.verify_formal_receipt(self.receipt, self.receipt["source"], self.repo.root)
        result = subprocess.run([sys.executable, "-I", "scripts/formal.py", "--target", str(self.live),
                                 "--verify-evidence"], cwd=self.repo.root, env=self.repo.environment,
                                text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_monthly_append_retains_previous_members(self):
        first = self.cold("first")
        self.formal.prune_evidence(self.repo.root, keep=0)
        second = self.cold("second")
        self.formal.prune_evidence(self.repo.root, keep=0)
        archive = next(self.store.with_name(self.store.name + "-archive").glob("*.tar.zst"))
        manifest = json.loads(archive.with_suffix(".sha256.json").read_text())
        self.formal.verify_archive(archive, manifest)
        for path in (first, second):
            self.assertFalse(path.exists())
            self.assertTrue(any(name.startswith(path.name + "/") for name in manifest["files"]))
        raw = subprocess.check_output(["zstd", "-q", "-dc", str(archive)])
        with self.formal.tarfile.open(fileobj=io.BytesIO(raw)) as tar:
            self.assertTrue(any(member.islnk() for member in tar.getmembers()),
                            "monthly appends duplicated an identical compressed graph")
            for path in (first, second):
                self.assertEqual(tar.extractfile(path.name + "/models/Fixture.states.zst").read(),
                                 (self.live / "models/Fixture.states.zst").read_bytes())

    def test_corrupt_archive_and_failed_verification_keep_originals(self):
        self.cold("first")
        self.formal.prune_evidence(self.repo.root, keep=0)
        cold = self.cold("second")
        archive = next(self.store.with_name(self.store.name + "-archive").glob("*.tar.zst"))
        archive.write_bytes(b"corrupt archive")
        with self.assertRaises((ValueError, subprocess.SubprocessError)):
            self.formal.prune_evidence(self.repo.root, keep=0)
        self.assertTrue(cold.exists())
        self.assertTrue(self.live.exists())

    def test_failed_new_archive_verification_never_removes_a_receipt(self):
        cold = self.cold("verify-failure")
        with patch.object(self.formal, "verify_archive", side_effect=ValueError("injected integrity failure")):
            with self.assertRaisesRegex(ValueError, "integrity failure"):
                self.formal.prune_evidence(self.repo.root, keep=0)
        self.assertTrue(cold.exists())
        self.assertTrue(self.live.exists())

    def test_dry_run_changes_nothing(self):
        self.cold("dry")
        def snapshot():
            return {str(p.relative_to(self.store)): (p.read_bytes(), p.stat().st_mtime_ns)
                    for p in self.store.rglob("*") if p.is_file()}
        before = snapshot()
        self.formal.prune_evidence(self.repo.root, keep=0, dry_run=True)
        self.assertEqual(snapshot(), before)
        self.assertFalse(self.store.with_name(self.store.name + "-archive").exists())

    def test_graphs_round_trip_shrink_and_dedupe_across_receipts(self):
        compressed = self.live / "models/Fixture.states.zst"
        self.assertFalse((self.live / "models/Fixture.states").exists())
        with self.formal.model_stream(compressed.with_suffix("")) as stream:
            self.assertEqual(stream.read(), self.graph)
        self.assertLess(compressed.stat().st_size * 10, len(self.graph))
        second = Path(self.repo.environment["CARGO_TARGET_DIR"]) / "second"
        shutil.copytree(self.live, second)
        (second / "evidence.json").write_bytes((second / "evidence.json").read_bytes() + b"\n")
        published = self.formal.archive_evidence(second, self.repo.root)
        other = published / "models/Fixture.states.zst"
        self.assertEqual(compressed.stat().st_ino, other.stat().st_ino)
        self.assertEqual(len(list((self.store / "by-model/graphs").glob("*.zst"))), 1)
        # CRC and decoded hashes must be checked, even when a compressed object exists.
        other.write_bytes(b"broken zstd")
        with self.assertRaises(ValueError):
            self.formal.model_digest(other.with_suffix(""))

    def test_orphan_sweep_keeps_staging_files_and_live_graph_links(self):
        objects = self.store / "by-model/graphs"
        staging = objects / "staging-in-progress.zst"
        staging.write_bytes(b"producer still owns this file")
        orphan = objects / ("0" * 64 + ".zst")
        orphan.write_bytes(b"unreferenced object")
        self.formal.prune_evidence(self.repo.root, keep=0)
        self.assertTrue(staging.exists())
        self.assertFalse(orphan.exists())
        with self.formal.model_stream(self.live / "models/Fixture.states") as stream:
            self.assertEqual(stream.read(), self.graph)

    def test_background_budget_and_qos_are_nonblocking(self):
        with patch.dict(os.environ, {"FORMAL_EVIDENCE_BUDGET_BYTES": "9999999999"}):
            self.assertFalse(self.formal.schedule_prune(self.repo.root))
        original = self.formal.subprocess.Popen
        def dispatch(argv, **kwargs):
            return Mock() if "--prune-evidence" in argv else original(argv, **kwargs)
        with patch.dict(os.environ, {"FORMAL_EVIDENCE_BUDGET_BYTES": "0"}), \
                patch.object(self.formal.subprocess, "Popen", side_effect=dispatch) as spawn:
            self.assertTrue(self.formal.schedule_prune(self.repo.root))
        argv = spawn.call_args.args[0]
        self.assertIn("19", argv)
        if platform.system() == "Darwin":
            self.assertEqual(argv[:2], ["taskpolicy", "-b"])
        self.assertTrue(spawn.call_args.kwargs["start_new_session"])
        makefile = SOURCE.parents[1] / "Makefile"
        self.assertIn('scripts/land-fast.sh\n\t@python3 -I scripts/formal.py --prune-background', makefile.read_text())


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
        control_marker = repo.state / "capability-control-ran"
        control_target = '\ntest-capability-control:\n\t@touch ' + shlex.quote(str(control_marker)) + '\n'
        scan_target = '\ntest-capability-scan:\n\t@true\n'
        normal_makefile = (repo.root / 'Makefile').read_text() + scan_target
        repo.write('Makefile', normal_makefile + control_target)
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
        self.assertTrue(control_marker.is_file(), 'canonical CI must execute the capability control')
        self.assertIn('"make", "test-capability-control"', result.stdout)
        log = repo.state / "actual-ci.log"
        log.write_text(result.stdout + result.stderr)
        self.assertEqual(landing.gate_proof(log, sha, repo.root)["tested_sha"], sha)
        repo.write('Makefile', normal_makefile + '\ntest-capability-control:\n\t@exit 29\n')
        control_failed = repo.commit('Fixture failing capability control')
        result = subprocess.run(['sh', 'scripts/ci-local.sh'], cwd=repo.root,
                                env=environment, text=True, capture_output=True, timeout=15)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('"make", "test-capability-control"', result.stdout)
        self.assertNotIn('"make", "test-formal"', result.stdout)
        log.write_text(result.stdout + result.stderr)
        with self.assertRaisesRegex(landing.Refusal, 'gate_log_not_green'):
            landing.gate_proof(log, control_failed)
        repo.write("Makefile", "test-harness-native:\n\t@true\ntest-formal:\n\t@exit 23\n" + scan_target + control_target)
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

    def test_real_ci_runs_budget_controls_and_propagates_failure(self):
        repo = Repository()
        self.addCleanup(repo.close)
        repo.tooling()
        shutil.copy2(SOURCE.with_name("ci-local.sh"), repo.root / "scripts/ci-local.sh")
        repo.write("scripts/ui-lint.sh", "#!/bin/sh\nexit 0\n")
        binary = Path(repo.environment["CARGO_TARGET_DIR"]) / "debug/bboxy450-term"
        binary.parent.mkdir(parents=True)
        binary.write_text("#!/bin/sh\nexit 0\n")
        binary.chmod(0o700)
        # Dynamic ps priority can be temporarily boosted by IPC. Inspect the
        # actual calling thread's requested class, which taskpolicy inherits.
        policy = ('import ctypes\nlibc=ctypes.CDLL(None)\nlibc.pthread_self.restype=ctypes.c_void_p\nqos=ctypes.c_uint()\nrelative=ctypes.c_int()\nassert libc.pthread_get_qos_class_np(ctypes.c_void_p(libc.pthread_self()),ctypes.byref(qos),ctypes.byref(relative))==0\nassert qos.value==0x11,qos.value\n'
                  if sys.platform == "darwin" else '')
        repo.write("scripts/test-landing-budget-controls.py", policy + 'print("budget-control-sentinel")\nraise SystemExit(19)\n')
        repo.commit("Fixture failing budget controls")
        result = subprocess.run(["sh", "scripts/ci-local.sh"], cwd=repo.root,
                                env=dict(repo.environment, BBOXY_GATE_SLOT="none"), capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 19, "budget controls did not block full CI: " + result.stdout + result.stderr)
        self.assertIn("budget-control-sentinel", result.stdout)


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
previous=target/'candidate-artifact'
row={'tree':str(pathlib.Path.cwd().resolve()),'target':str(target.resolve()),
     'sha':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
     'warm':previous.read_text() if previous.exists() else None}
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

    def test_two_commits_reuse_warm_slot_paths_and_keep_the_caller(self):
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
        self.assertEqual(second["warm"], first["sha"])  # the second candidate builds on the first's target
        self.assertEqual(unrelated.read_text(), "retained")
        # Release drops ownership only; the checkout and target stay warm for the next gate.
        self.assertFalse((slot / "pid").exists())
        self.assertTrue((slot / "target").is_dir())
        self.assertIn(str(slot / "tree"), run("git", "worktree", "list", "--porcelain", cwd=self.repo.root))

    def test_dead_owner_slot_is_reused_warm(self):
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
        self.assertEqual((slot / "target/old-artifact").read_text(), "retired")
        self.assertFalse((slot / "pid").exists())

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

    def test_scoped_exact_full_gate_supersedes_a_timed_out_presubmit(self):
        sha = self.cut()
        (self.repo.state / "presubmit.log").write_text(json.dumps({
            "sha": sha, "error": "presubmit_timeout", "exit": 124,
            "compile_seconds": 900., "tests_started": False}) + '\n')
        proof = self.gate_log(sha)
        result = self.repo.land("--gate-log", proof)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), sha)
        self.assertEqual(self.cargo_test_count(), 0)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["tested_sha"], sha)
        self.assertEqual(record["gate_log_sha256"], hashlib.sha256(proof.read_bytes()).hexdigest())
        self.assertEqual(record["tier"], "code")

    def test_full_code_cut_runs_normal_make_after_initial_rebase(self):
        old = self.cut()
        self.repo.movement(".bboxy/other.md")
        hook = json.loads((self.repo.state / "behavior.json").read_text())["hook"]
        run("sh", hook, cwd=self.repo.root)
        (self.repo.state / "behavior.json").unlink()
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":0.0001}')
        result = self.repo.land("--full")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        sha = self.repo.remote_head()
        self.assertNotEqual(sha, old)
        self.assertEqual(self.cargo_test_count(), 1)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["tier"], "code")
        self.assertEqual(record["tested_sha"], sha)
        self.assertEqual(record["formal"]["source"], sha)
        proof = Path(record["gate_log"]).read_text()
        self.assertIn(f"ci-local: source {sha} normal", proof)
        self.assertEqual(proof.splitlines()[-1], f"ci-local: result {sha} normal 0")

    def test_private_make_overrides_stay_private_with_a_shared_path_control(self):
        parent = self.repo.state / "inherited-parent-target"
        overrides = " -- TARGET_DIR=" + str(parent)
        with patch.dict(os.environ, {"MAKEFLAGS": overrides, "MAKELEVEL": "7",
                                    "TARGET_DIR": str(parent), "CI_LOG": str(parent / "ci.log"),
                                    "CI_ARGS": "--slow"}):
            other = Repository()
            self.addCleanup(other.close)
            other.tooling()
        other.remote()
        other.write("crates/c/src/lib.rs", "// isolated inherited Make flags\n")
        other.commit("Fixture private Make overrides")
        result = other.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(parent.exists(), "private formal check wrote a parent target")
        # Deliberately restore the inheritance: the same real Make path now
        # writes evidence into the shared parent. This is the collision witness.
        other.environment["MAKEFLAGS"] = overrides
        other.environment["TARGET_DIR"] = str(parent)
        other.write("crates/c/src/lib.rs", "// deliberately shared Make target\n")
        other.commit("Fixture broken Make isolation")
        result = other.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((parent / "formal/evidence.json").is_file())

    def test_full_failed_make_cannot_push(self):
        self.cut()
        (self.repo.state / "behavior.json").write_text('{"fail":"test"}')
        result = self.repo.land("--full")
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertFalse((self.repo.state / "landings.log").exists())

    def test_full_disjoint_movement_reuses_rust_and_refreshes_formal(self):
        self.cut()
        self.repo.movement(".bboxy/other.md")
        result = self.repo.land("--full")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["formal"]["source"], self.repo.remote_head())
        self.assertNotEqual(record["tested_sha"], self.repo.remote_head())

    def test_scoped_stale_full_gate_cannot_supersede_a_run_timeout(self):
        sha = self.cut()
        (self.repo.state / "behavior.json").write_text('{"block":"test","test_started":true}')
        (self.repo.state / "fixture.json").write_text('{"timeout_seconds":3}')
        result = self.repo.land()
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        result = self.repo.land("--gate-log", self.gate_log(self.repo.base))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_scoped_source_binding_has_a_broken_control(self):
        sha = self.cut()
        proof = self.gate_log(sha)
        proof.write_text(proof.read_text().replace(f"ci-local: source {sha}", "ci-local: source " + "0" * 40))
        result = self.repo.land("--gate-log", proof)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        # The result and formal receipt still match HEAD. Removing just the
        # source binding must make this real private landing accept the bad log.
        tool = self.repo.root / "scripts/landing.py"
        original = tool.read_text()
        guard = 'if sources != [[sha, "normal"]]:'
        self.assertEqual(original.count(guard), 1)
        tool.write_text(original.replace(guard, 'if False: # deliberately broken source binding'))
        broken = self.repo.commit("Fixture broken source binding")
        proof = self.gate_log(broken)
        proof.write_text(proof.read_text().replace(f"ci-local: source {broken}", "ci-local: source " + "0" * 40))
        result = self.repo.land("--gate-log", proof)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.repo.remote_head(), broken)

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

    def test_records_rebase_accepts_gate_and_runs_only_records_checks(self):
        self.repo.write("scripts/generated-sources.sh", (self.repo.root / "scripts/generated-sources.sh").read_text()
                        + "test ! -e .bboxy/untracked-invalid.md\n")
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        # Checks must inspect committed C', not unrelated untracked records in
        # the author's worktree. The generated-sources witness above enforces it.
        self.repo.write(".bboxy/untracked-invalid.md", "# Not a committed record\n")
        name = ".bboxy/decisions/unreferenced-record.md"
        self.repo.movement(name, contents="---\ntype: decision\nnumber: 1\n---\n# Fixture record\n")
        hook = json.loads((self.repo.state / "behavior.json").read_text())["hook"]
        run("sh", hook, cwd=self.repo.root)
        (self.repo.state / "behavior.json").unlink()
        result = self.repo.land("--gate-log", proof)
        self.assertEqual(result.returncode, 0, "records-only gate was refused: " + result.stdout + result.stderr)
        self.assertNotEqual(self.repo.remote_head(), sha)
        record = json.loads((self.repo.state / "landings.log").read_text())
        self.assertEqual(record["gate_log_source"], sha)
        self.assertEqual(record["tested_sha"], sha)
        self.assertEqual(record["records_delta"], [name])
        self.assertEqual(record["records_checks_exit"], 0)
        self.assertEqual(record["formal"]["source"], self.repo.remote_head())
        # The old evidence stays strict; only the guarded R5 route produces a
        # new current-source receipt, without rerunning its checked models.
        with self.assertRaises(landing.Refusal):
            landing.formal_receipt(proof.read_text(), sha, self.repo.root)
        landing.verify_formal_receipt(record["formal"], self.repo.remote_head(), self.repo.root)
        calls = [json.loads(s) for s in (self.repo.state / "cargo.jsonl").read_text().splitlines()]
        for argv in calls:
            self.assertEqual(argv[1:3], ["--target-dir", str((self.repo.state / "slots/bboxy450-gate-slot-1/target").resolve())])
        self.assertEqual([argv[:1] + argv[3:] for argv in calls], [["test", "-p", "bboxy450-doc", "--locked", "--test", "corpus", "-q"],
                                 ["test", "-p", "bboxy450-mermaid", "--locked", "-q"]])
        scripts = (self.repo.state / "shell.log").read_text().splitlines()
        self.assertEqual(scripts, ["scripts/" + name + ".sh " for name in (
            "layering", "pending-flags", "kill-own-pids", "generated-sources", "licenses", "brand",
            "images", "cross-os", "never-block")])
        self.assertNotIn("formal-phase:", result.stdout, "records rebase reran formal export")
        self.assertNotIn("nested fixture checks", result.stdout)

    def test_records_gate_refuses_a_byte_outside_records(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        # Outside the formal source inventory: this must be stopped by R5's
        # tree predicate itself, rather than the separate evidence hash check.
        self.repo.write("crates/c/uninventoried.txt", "x")
        self.repo.commit("Fixture one-byte non-record delta")
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0, "non-records delta was accepted")
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_records_gate_refuses_a_changed_rust_source(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        self.cut("crates/c/src/lib.rs")
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_records_gate_refuses_a_record_named_by_code(self):
        name = ".bboxy/specs/e2e-acceptance.md"
        self.repo.write("crates/a/src/lib.rs", f'const RECORD: &str = "{name}";\n')
        self.repo.write(name, "original fixture")
        sha = self.repo.commit("Fixture named record")
        proof = self.gate_log(sha)
        self.repo.write(name, "changed fixture")
        self.repo.commit("Fixture changed named record")
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0, "code-named record was accepted")
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_records_gate_refuses_a_new_executable_invariant(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        self.repo.write(".bboxy/invariants/new-gate.md",
                        "---\ntype: invariant\nnumber: 1\ncheck: cargo test -p a fixture\n---\n# New gate\n")
        self.repo.commit("Fixture new executable invariant after full gate")
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0, "executable invariant delta was accepted")
        self.assertIn("gate_log_source_mismatch: executable gate record", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)
        self.assertEqual(self.cargo_test_count(), 0)

    def test_records_gate_refuses_submodule_delta_despite_ignore_config(self):
        first = self.cut("crates/a/src/lib.rs")
        run("git", "update-index", "--add", "--cacheinfo", "160000", self.repo.base,
            "vendor/fixture", cwd=self.repo.root)
        run("git", "commit", "-q", "-m", "Fixture first gitlink", cwd=self.repo.root)
        source = run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        run("git", "update-index", "--cacheinfo", "160000", first,
            "vendor/fixture", cwd=self.repo.root)
        run("git", "commit", "-q", "-m", "Fixture changed gitlink", cwd=self.repo.root)
        candidate = run("git", "rev-parse", "HEAD", cwd=self.repo.root)
        run("git", "config", "diff.ignoreSubmodules", "all", cwd=self.repo.root)
        with self.assertRaisesRegex(landing.Refusal, "non-records tree delta",
                                    msg="ignored submodule delta was accepted"):
            landing.records_gate_delta(self.repo.root, source, candidate)

    def test_records_gate_refuses_failed_records_checks(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        name = ".bboxy/decisions/bad-record.md"
        self.repo.write(name, "# Missing frontmatter")
        self.repo.commit("Fixture invalid records delta")
        (self.repo.state / "behavior.json").write_text(json.dumps({"corpus_frontmatter": name}))
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0, "failed records checks were accepted")
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertEqual(self.repo.remote_head(), self.repo.base)

    def test_records_gate_refuses_secret_like_changed_record(self):
        sha = self.cut("crates/a/src/lib.rs")
        proof = self.gate_log(sha)
        self.repo.write(".bboxy/decisions/secret-fixture.md", "---\ntype: decision\nnumber: 1\n---\ntoken: " + "a" * 40)
        self.repo.commit("Fixture synthetic secret-like record")
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0, "secret-like record was accepted")
        self.assertIn("records secret scan failed", result.stderr)
        self.assertNotIn("a" * 40, result.stdout + result.stderr)

    def test_records_gate_refuses_non_records_rename_and_mode(self):
        self.cut("crates/a/src/lib.rs")
        self.repo.write("crates/c/original.txt", "x")
        sha = self.repo.commit("Fixture rename and mode baseline")
        proof = self.gate_log(sha)
        run("git", "mv", "crates/c/original.txt", "crates/c/renamed.txt", cwd=self.repo.root)
        self.repo.commit("Fixture non-record rename")
        result = self.repo.land("--gate-log", proof)
        self.assertIn("gate_log_source_mismatch", result.stderr)
        self.assertNotEqual(result.returncode, 0)
        run("git", "mv", "crates/c/renamed.txt", "crates/c/original.txt", cwd=self.repo.root)
        path = self.repo.root / "crates/c/original.txt"
        path.chmod(0o755)
        self.repo.commit("Fixture non-record mode delta")
        result = self.repo.land("--gate-log", proof)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("gate_log_source_mismatch", result.stderr)

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
        self.assertRegex((self.repo.state / "shell.log").read_text(),
                         r"invariants\.sh --affected c --covered-tests [^\n]+/quarantine-coverage\.json\n")

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
        self.repo.write('scripts/main_watch.py', (self.repo.root / 'scripts/main_watch.py').read_text() + '\n# fixture copied script cut\n')
        self.repo.commit('Fixture copied script cut')
        self.repo.movement('scripts/main-watch.sh', contents=(self.repo.root / 'scripts/main-watch.sh').read_text() + '\n# incoming copied script\n')
        result = self.repo.land()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.cargo_test_count(), 1)
        self.assertEqual(result.stdout.count('nested fixture checks'), 2)
        checks = [json.loads(line) for line in (self.repo.state / 'presubmit.log').read_text().splitlines()]
        self.assertEqual(checks[-1]['recheck'], {'affected': [], 'commands': [
            ['python3', '-I', 'scripts/test-landing.py', '--all']]})

    def test_a_record_only_move_reuses_a_script_cuts_fixture_result(self):
        self.repo.write('scripts/main_watch.py', (self.repo.root / 'scripts/main_watch.py').read_text() + '\n# fixture copied script cut\n')
        self.repo.commit('Fixture copied script cut')
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
        self.assertEqual(checks[-1]['recheck']['affected'], [])
        commands = checks[-1]['recheck']['commands']
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][:5], ['sh', 'scripts/invariants.sh', '--affected', 'c', '--covered-tests'])
        self.assertEqual(Path(commands[0][5]).name, 'quarantine-coverage.json')
        self.assertEqual(commands[0][6:], ['fixture'])

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
        self.assertFalse(list((self.repo.state / "slots").glob("bboxy450-gate-slot-*/pid")))
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

    def test_one_default_slot_queues_until_its_owner_exits(self):
        self.cut()
        slot = self.repo.state / "slots" / "bboxy450-gate-slot-1"
        slot.mkdir(parents=True)
        holder = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(4)"])
        (slot / "pid").write_text(str(holder.pid))
        reaper = threading.Thread(target=holder.wait)  # an unreaped zombie still answers kill(pid, 0)
        reaper.start()
        environment = dict(self.repo.environment)
        for name in ("BBOXY_GATE_SLOTS", "BBOXY_GATE_WAIT"):
            environment.pop(name)
        result = subprocess.run(["sh", "scripts/land.sh"], cwd=self.repo.root, env=environment,
                                capture_output=True, text=True, timeout=50)
        reaper.join(timeout=5)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("queued, waiting for a gate slot", result.stdout)
        self.assertFalse((self.repo.state / "slots" / "bboxy450-gate-slot-2").exists())

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


class IgnoredIntegrationTests(unittest.TestCase):
    def invoke(self, cases, *, extra=(), prefix=(), allow=True, run_code=4, listing_code=0, malformed=False):
        spec = importlib.util.spec_from_file_location("ignored_quarantine", SOURCE.with_name("quarantine.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        api = Mock()
        api.metadata.return_value = {}
        payload = {"rust-suites": {"fixture::native": {"testcases": cases}}}
        api.command.return_value = subprocess.CompletedProcess(
            [], listing_code, "{" if malformed else json.dumps(payload), "")
        with tempfile.TemporaryDirectory(prefix="ignored-integration-") as directory:
            api.repository.return_value = Path(directory)
            arguments = ["--locked", "-p", "fixture", "--test", "native", *extra]
            if allow:
                arguments.insert(0, "--allow-ignored-only")
            arguments = [*prefix, *arguments]
            with patch.object(module, "nextest", return_value=True), \
                    patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess([], run_code)) as run, \
                    patch.dict(os.environ, {"BBOXY_PRESUBMIT_COVERAGE": "", "BBOXY_PRESUBMIT_ATTEMPT": ""}), \
                    contextlib.redirect_stdout(io.StringIO()) as output:
                code = module.test(arguments, api)
        return code, api.command, run.call_args[0][0], output.getvalue()

    @staticmethod
    def ignored():
        return {"native_case": {"kind": "test", "ignored": True,
                "filter-match": {"status": "mismatch", "reason": "ignored"}}}

    def test_explicit_lean_opt_in_accepts_verified_ignored_target_without_forwarding_flag(self):
        code, listing, argv, output = self.invoke(self.ignored())
        self.assertEqual(code, 0)
        self.assertNotIn("--allow-ignored-only", argv)
        self.assertNotIn("--no-fail-fast", listing.call_args[0][0])
        self.assertIn("all 1 selected tests are ignored; compiled only", output)

    def test_normal_runner_keeps_no_tests_failure(self):
        code, listing, _, _ = self.invoke(self.ignored(), allow=False)
        self.assertEqual(code, 4)
        listing.assert_not_called()

    def test_target_override_can_precede_the_lean_opt_in(self):
        code, _, argv, _ = self.invoke(self.ignored(), prefix=("--target-dir", "private-target"))
        self.assertEqual(code, 0)
        self.assertNotIn("--allow-ignored-only", argv)

    def test_explicit_filters_cannot_be_mistaken_for_ignored_tests(self):
        for extra in [("--", "missing_case"), ("missing_case",)]:
            with self.subTest(extra=extra):
                code, listing, _, _ = self.invoke(self.ignored(), extra=extra)
                self.assertEqual(code, 4)
                listing.assert_not_called()

    def test_module_filter_counts_only_matching_ignored_tests(self):
        expression = "binary(=it)&test(/^native::/)"
        cases = {"native::case": {"kind": "test", "ignored": True, "filter-match": {"status": "matches"}},
                 "other::ignored": {"kind": "test", "ignored": True,
                                    "filter-match": {"status": "mismatch", "reason": "expression"}},
                 "other::ordinary": {"kind": "test", "ignored": False,
                                     "filter-match": {"status": "mismatch", "reason": "expression"}}}
        code, listing, argv, output = self.invoke(cases, extra=("-E", expression))
        self.assertEqual(code, 0)
        self.assertIn(expression, argv)
        self.assertIn("--run-ignored", listing.call_args[0][0])
        self.assertIn("all", listing.call_args[0][0])
        self.assertIn("all 1 selected tests are ignored", output)

    def test_module_filter_empty_nonmatching_and_nonignored_selections_fail(self):
        expression = "binary(=it)&test(/^missing::/)"
        for cases in [{}, self.ignored(),
                      {"other::ignored": {"kind": "test", "ignored": True,
                                          "filter-match": {"status": "mismatch", "reason": "expression"}}},
                      {"native::case": {"kind": "test", "ignored": False, "filter-match": {"status": "matches"}}},
                      {"native::case": {"kind": "bench", "ignored": True, "filter-match": {"status": "matches"}}},
                      {"native::case": {"kind": "test", "ignored": True,
                                        "filter-match": {"status": "mismatch", "reason": "unknown"}}}]:
            with self.subTest(cases=cases):
                self.assertEqual(self.invoke(cases, extra=("-E", expression))[0], 4)

    def test_module_filter_never_accepts_failed_run_or_listing(self):
        cases = {"native::case": {"kind": "test", "ignored": True, "filter-match": {"status": "matches"}}}
        extra = ("-E", "binary(=it)&test(/^native::/)")
        for code in (100, 101):
            self.assertEqual(self.invoke(cases, extra=extra, run_code=code)[0], code)
        self.assertEqual(self.invoke(cases, extra=extra, listing_code=101)[0], 4)

    def test_empty_or_filtered_metadata_keeps_no_tests_failure(self):
        for cases in [{}, {"case": {"kind": "test", "ignored": False}},
                      {"case": {"kind": "test", "ignored": True, "filter-match": {"reason": "expression"}}},
                      {"case": {"kind": "bench", "ignored": True, "filter-match": {"reason": "ignored"}}}]:
            with self.subTest(cases=cases):
                self.assertEqual(self.invoke(cases)[0], 4)

    def test_missing_or_malformed_listing_fails_closed(self):
        self.assertEqual(self.invoke(self.ignored(), listing_code=101)[0], 4)
        self.assertEqual(self.invoke(self.ignored(), malformed=True)[0], 4)

    def test_build_and_test_failures_are_never_accepted_as_ignored(self):
        for run_code in [100, 101]:
            with self.subTest(run_code=run_code):
                code, listing, _, _ = self.invoke(self.ignored(), run_code=run_code)
                self.assertEqual(code, run_code)
                listing.assert_not_called()

    def test_cargo_profile_is_preserved_for_ignored_listing_not_execution_selection(self):
        for profile in [("--profile", "dream-c1-fixture"), ("--profile=dream-c1-fixture",)]:
            with self.subTest(profile=profile):
                code, listing, argv, _ = self.invoke(self.ignored(), prefix=profile)
                self.assertEqual(code, 0)
                wanted = (["--cargo-profile", "dream-c1-fixture"] if len(profile) == 2 else
                          ["--cargo-profile=dream-c1-fixture"])
                self.assertEqual(argv[3:3 + len(wanted)], wanted)
                self.assertEqual(listing.call_args[0][0][3:3 + len(wanted)], wanted)
                self.assertNotIn("--profile", argv)
                self.assertNotIn("--profile", listing.call_args[0][0])
                self.assertEqual(self.invoke({}, prefix=profile)[0], 4)


class CargoProfileForwardingTests(unittest.TestCase):
    def test_exact_retry_preserves_ignored_mode_and_drops_nextest_threads(self):
        module = self.module()
        for option in [["--test-threads", "1"], ["--test-threads=1"]]:
            actual = module.retry_args(["--test", "it", *option, "--run-ignored", "only"],
                                       "bboxy450-term-mux", "mcp::case", Mock())
            self.assertEqual(actual, ["cargo", "test", "--test", "it", "-p", "bboxy450-term-mux",
                                      "--", "--exact", "mcp::case", "--ignored"])
        for tail, expected in [(["--run-ignored=all"], ["--include-ignored"]),
                               (["--", "--ignored"], ["--ignored"]),
                               (["--", "--include-ignored"], ["--include-ignored"]),
                               (["--", "--skip", "--ignored"], [])]:
            self.assertEqual(module.retry_args(["--test", "it", *tail], "fixture", "mcp::case", Mock()),
                             ["cargo", "test", "--test", "it", "-p", "fixture",
                              "--", "--exact", "mcp::case", *expected])

    def test_qualified_quarantine_requires_observed_crate_and_original_identity(self):
        module = self.module()
        entry = {"test": "case", "crate": "bboxy450-term-mux", "expired": False}
        observed = {"test": "mcp::case", "package": "bboxy450-term-mux", "kind": "test",
                    "target": "it", "result": "FAILED"}
        def invoke(entries, observations, name="mcp::case"):
            api = Mock()
            api.command.return_value = subprocess.CompletedProcess([], 0, f"test {name} ... ok\n", "")
            coverage = Mock(tests=observations)
            arguments = ["--lib", "--bins", "--test", "it", "-E", "binary(it)&test(/^mcp::/)"]
            with patch.object(module, "rows", return_value=entries), contextlib.redirect_stdout(io.StringIO()):
                code = module.retry_quarantined([name], arguments, coverage, Path("."), api, 100)
            return code, api, coverage
        code, api, coverage = invoke([entry], [observed])
        self.assertEqual(code, 0)
        self.assertEqual(api.command.call_args.args[0], ["cargo", "test", "--test", "it", "-p",
                                                        "bboxy450-term-mux", "--", "--exact", "mcp::case"])
        self.assertEqual(api.append_record.call_args.args[1]["listed_test"], "case")
        coverage.passed.assert_called_once()
        negatives = [([entry], []), ([dict(entry, expired=True)], [observed]),
                     ([entry, entry], [observed]), ([entry], [observed, observed]),
                     ([entry], [dict(observed, kind="lib")]),
                     ([entry], [dict(observed, target="other")]),
                     ([entry], [dict(observed, package="other")]),
                     ([entry], [dict(observed, test="mcp::nested::case")])]
        for entries, observations in negatives:
            with self.subTest(entries=entries, observations=observations):
                name = observations[0]["test"] if observations else "mcp::case"
                code, api, coverage = invoke(entries, observations, name)
                self.assertEqual(code, 100)
                api.command.assert_not_called()
                coverage.passed.assert_not_called()
        library = dict(observed, test="logic::case", kind="lib", target="bbt_mux")
        self.assertEqual(invoke([dict(entry, test="logic::case")], [library], "logic::case")[0], 0)
        nested = dict(observed, test="mcp::nested::case")
        self.assertEqual(invoke([dict(entry, test="nested::case")], [nested], nested["test"])[0], 0)

    def test_exact_retry_replaces_module_filter_with_qualified_test(self):
        module = self.module()
        for selection in [["-E", "binary(=it)&test(/^native::/)"],
                          ["--filterset", "binary(=it)&test(/^native::/)"],
                          ["--filterset=binary(=it)&test(/^native::/)"],
                          ["-Ebinary(=it)&test(/^native::/)"]]:
            arguments = ["--profile", "fixture", "-p", "old", "--test", "it", *selection]
            self.assertEqual(module.retry_args(arguments, "fixture", "native::case", Mock()),
                             ["cargo", "test", "--profile", "fixture", "--test", "it",
                              "-p", "fixture", "--", "--exact", "native::case"])

    def module(self):
        spec = importlib.util.spec_from_file_location("profile_quarantine", SOURCE.with_name("quarantine.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def invoke(self, arguments):
        module = self.module()
        api = Mock()
        api.metadata.return_value = {"fixture": {"targets": [
            {"name": "fixture", "kind": ["lib"], "doctest": True}]}}
        calls = []
        def run(argv, **kwargs):
            calls.append(argv)
            return subprocess.CompletedProcess(argv, 0, "", "")
        with tempfile.TemporaryDirectory(prefix="cargo-profile-") as directory, \
                patch.object(module.subprocess, "run", side_effect=run), \
                contextlib.redirect_stdout(io.StringIO()):
            result = module.nextest_run(Path(directory), arguments, Path(directory) / "target", api)
        return result, calls

    def test_nextest_build_profile_preserves_features_targets_packages_and_filters(self):
        for profile in [["--profile", "dream-c1-fixture"], ["--profile=dream-c1-fixture"]]:
            with self.subTest(profile=profile):
                arguments = [*profile, "--locked", "--features", "fixture-a,fixture-b",
                    "--target", "aarch64-apple-darwin", "--target-dir", "private-target",
                    "-p", "fixture", "--lib", "-E", "test(c1)", "--", "--nocapture"]
                original = arguments.copy()
                result, calls = self.invoke(arguments)
                wanted = (["--cargo-profile", "dream-c1-fixture"] if len(profile) == 2 else
                          ["--cargo-profile=dream-c1-fixture"])
                self.assertEqual(result[0], 0)
                self.assertEqual(calls, [["cargo", "nextest", "run", *wanted, *arguments[len(profile):]]])
                self.assertEqual(arguments, original, "translation cannot mutate Cargo retry/doc arguments")

    def test_cargo_doctests_keep_the_original_cargo_profile(self):
        for profile in [["--profile", "dream-c1-fixture"], ["--profile=dream-c1-fixture"]]:
            with self.subTest(profile=profile):
                options = ["--features", "fixture-a,fixture-b", "--target", "aarch64-apple-darwin",
                           "--target-dir", "private-target", "--locked"]
                _, calls = self.invoke([*profile, "-p", "fixture", *options, "--", "filter"])
                self.assertEqual(len(calls), 2)
                self.assertEqual(calls[1], ["cargo", "test", "--doc", *profile, *options,
                                           "-p", "fixture", "--", "filter"])
                self.assertFalse(any(arg.startswith("--cargo-profile") for arg in calls[1]))
                self.assertFalse(any(arg == "--profile" or arg.startswith("--profile=") for arg in calls[0]))

    def test_exact_cargo_retry_keeps_the_original_profile(self):
        module = self.module()
        for profile in [["--profile", "dream-c1-fixture"], ["--profile=dream-c1-fixture"]]:
            with self.subTest(profile=profile):
                options = ["--features", "a,b", "--target", "aarch64-apple-darwin",
                           "--target-dir", "private-target", "--locked", "--test", "native"]
                retry = module.retry_args([*profile, "-p", "old", *options, "old_filter", "--", "--ignored"],
                                          "fixture", "native::case", Mock())
                self.assertEqual(retry, ["cargo", "test", *profile, *options,
                                         "-p", "fixture", "--", "--exact", "native::case", "--ignored"])

    def test_filter_tail_profile_text_is_never_translated(self):
        _, calls = self.invoke(["--lib", "-p", "fixture", "--", "--profile=literal_filter"])
        self.assertEqual(calls, [["cargo", "nextest", "run", "--lib", "-p", "fixture",
                                  "--", "--profile=literal_filter"]])


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

    def test_nextest_junit_maps_binary_ids_to_coverage_rows(self):
        spec = importlib.util.spec_from_file_location("fixture_quarantine", SOURCE.with_name("quarantine.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        junit = self.repo.state / "junit.xml"
        junit.write_text("""<testsuites>
<testsuite name="c"><testcase name="tests::unit"/></testsuite>
<testsuite name="c::ctl"><testcase name="once"><failure type="test failure"/></testcase>
<testcase name="slow"><skipped/></testcase></testsuite>
<testsuite name="c::bin/tool"><testcase name="main_test"/></testsuite>
<testsuite name="c::example/demo"><testcase name="ignored_kind"/></testsuite>
</testsuites>""")
        packages = {"c": {"targets": [{"name": "c_lib", "kind": ["lib"]}]}}
        rows = module.junit_tests(junit, packages)
        self.assertEqual([(r["kind"], r["target"], r["test"], r["result"]) for r in rows],
                         [("lib", "c_lib", "tests::unit", "ok"), ("test", "ctl", "once", "FAILED"),
                          ("test", "ctl", "slow", "ignored"), ("bin", "tool", "main_test", "ok")])

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
 (target/'debug/bboxy450-book').write_bytes(b'fixture binary')
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
        (target / "debug/bboxy450-book").unlink()
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
        (store / "bboxy450-book").write_bytes(b"last green binary")
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
        self.assertTrue(any(c[:2] == [str(store.resolve() / "bboxy450-book"), "compare"] for c in self.calls))

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
        self.assertEqual(self.calls,[["make", "test-formal", "FORMAL_REUSE=1"]])

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


class PublishingIdentityTests(unittest.TestCase):
    def publish(self, actor, origin="https://github.com/bboxy450/bboxy450-term.git"):
        result = subprocess.CompletedProcess([], 0, stdout=actor + "\n", stderr="")
        with patch.dict(os.environ, {}, clear=True), \
                patch.object(landing, "local_origin", return_value=False), \
                patch.object(landing, "git", return_value=origin), \
                patch.object(landing, "command", return_value=result):
            landing.publishing_identity(Path("/unused-private-fixture"))

    def test_operator_approved_source_publishers_are_accepted(self):
        for actor in ("bboxy450", "Gonzih"):
            with self.subTest(actor=actor):
                self.publish(actor)

    def test_other_publishers_and_origins_remain_refused(self):
        with self.assertRaisesRegex(landing.Refusal, "unapproved_github_identity"):
            self.publish("unapproved-fixture")
        with self.assertRaisesRegex(landing.Refusal, "unapproved_origin"):
            self.publish("Gonzih", "https://github.com/unapproved-fixture/project.git")



class BudgetClockTests(unittest.TestCase):
    def phases(self, missing=False):
        counter = [0.]
        class Stream:
            def __iter__(self):
                for step in ("export", "self-tests", "verify", "parity"):
                    if not (missing and step == "verify"):
                        yield landing.FORMAL_PHASE_PREFIX + json.dumps({"event":"start","step":step}) + "\n"
                    counter[0] += 20.
                    yield "fixture work\n"
                    if not (missing and step == "verify"):
                        yield landing.FORMAL_PHASE_PREFIX + json.dumps({"event":"end","step":step}) + "\n"
            def close(self): pass
        class Child:
            stdout = Stream()
            returncode = 0
            def wait(self): return 0
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "phases.jsonl"
            with patch.object(landing, "ledger_module", return_value=None), \
                 patch.object(landing.subprocess, "Popen", return_value=Child()), \
                 patch.object(landing.time, "monotonic", side_effect=lambda:counter[0]):
                self.assertEqual(landing.measured_command(["make","test-formal"], Path(directory), path,
                    stream=io.StringIO(), stage="formal"), 0)
            clocks = landing.TwoClocks(0.)
            for line in path.read_text().splitlines(): clocks.event(json.loads(line))
            return clocks, clocks.totals(counter[0])

    def test_four_formal_substeps_sum_to_the_formal_stage(self):
        _, totals = self.phases()
        self.assertEqual(totals["stage_seconds"]["formal"],80.)
        self.assertEqual(sum(totals["formal_step_seconds"].values()),80.)
        self.assertLess(totals["unclassified_seconds"],1.)

    def test_missing_substep_is_unclassified_and_advisory(self):
        _, totals = self.phases(missing=True)
        self.assertEqual(totals["unclassified_seconds"],20.,"missing phase was silently classified")
        findings=landing.budget_findings(totals,4.,5.)
        self.assertTrue(any(row["kind"]=="unclassified" for row in findings),
                        "missing phase had no finding")
        self.assertEqual(findings[0]["load_average_start"],4.)

    def test_stage_allocation_is_advisory_but_900_seconds_still_blocks(self):
        clocks=landing.TwoClocks(0.)
        clocks.event({"at":0.,"phase":"other","stage":"formal","substep":"parity"})
        totals=clocks.totals(291.)
        self.assertTrue(landing.budget_findings(totals,4.,5.))
        self.assertIsNone(clocks.expired(291.,1800.,900.))
        self.assertEqual(clocks.expired(901.,1800.,900.),"presubmit_timeout","900 second run bound was bypassed")

    def test_formal_stage_wrapper_emits_both_boundaries(self):
        result=subprocess.run([sys.executable,"-I",str(SOURCE.with_name("formal-stage.py")),
            "verify","--",sys.executable,"-c","print('fixture')"],capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        events=[json.loads(line[len(landing.FORMAL_PHASE_PREFIX):]) for line in result.stdout.splitlines()
                if line.startswith(landing.FORMAL_PHASE_PREFIX)]
        self.assertEqual([e["event"] for e in events],["start","end"])


class BudgetPresubmitTests(unittest.TestCase):
    def setUp(self):
        self.repo=Repository()
        self.addCleanup(self.repo.close)
        self.repo.tooling()
        self.repo.environment["BBOXY_CARGO_TARGET_DIR"]=str(self.repo.state/"target")
        self.repo.environment.pop("CARGO_TARGET_DIR",None)
        self.repo.environment.update(RUSTC_WRAPPER="",RUSTC_WORKSPACE_WRAPPER="")

    def cut(self,path,text):
        self.repo.write(path,text)
        return self.repo.commit("Fixture budget code cut")

    def coverage_fixture(self,*,filtered=False,ignored=False):
        shutil.copyfile(SOURCE.with_name("invariants.sh"),self.repo.root/"scripts/invariants.sh")
        self.repo.write("crates/c/tests/ctl.rs",("#[ignore]\n" if ignored else "")+"#[test] fn ctl_once() {}\n")
        suffix=" --ignored" if ignored else ""
        self.repo.write(".bboxy/invariants/ctl-once.md",
            "---\ntype: invariant\ncheck: cargo test -p c --locked --test ctl ctl_once -- --exact"+suffix+"\n---\n")
        self.repo.base=self.repo.commit("Fixture coverage and invariant")
        (self.repo.state/"behavior.json").write_text(json.dumps({
            "coverage_real":True,"coverage_filtered":filtered,"coverage_ignored":ignored}))
        self.cut("crates/c/src/lib.rs","pub fn value()->u8 {2}\n")

    def dry_run(self):
        result=subprocess.run(["sh","scripts/presubmit.sh",self.repo.base],cwd=self.repo.root,
            env=self.repo.environment,capture_output=True,text=True,timeout=45)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        record=json.loads((self.repo.state/"presubmit.log").read_text().splitlines()[-1])
        self.assertEqual(record,json.loads(Path(record["result"]).read_text()))
        return record,result

    def test_observed_ctl_runs_once(self):
        self.coverage_fixture()
        record,result=self.dry_run()
        self.assertEqual((self.repo.state/"ctl-runs").read_text().splitlines(),["ctl"],
                         "ctl binary ran twice")
        self.assertIn("covered in this presubmit",result.stdout)
        self.assertIn("stage_seconds",record)
        self.assertIn("load_average_start",record)
        self.assertIn("load_average_end",record)
        self.assertEqual(record["stage_seconds"]["formal"],sum(record["formal_step_seconds"].values()))

    def test_filtered_invariant_still_runs(self):
        self.coverage_fixture(filtered=True)
        self.dry_run()
        self.assertEqual(len((self.repo.state/"ctl-runs").read_text().splitlines()),2,
                         "filtered invariant was skipped")

    def test_ignored_invariant_still_runs(self):
        # Exercise the exact product discriminator as well as the process
        # witness (whose explicit --ignored command stays outside batching).
        source = SOURCE.with_name("invariants.sh").read_text().split("<<'PY_INVARIANTS'\n", 1)[1]
        module = landing.ast.parse(source)
        function = next(node for node in module.body if isinstance(node, landing.ast.FunctionDef) and node.name == "covered")
        namespace = {}
        exec(compile(landing.ast.Module(body=[function], type_ignores=[]), "invariant coverage discriminator", "exec"), namespace)
        check = {"env": {}, "packages": ["c"], "targets": [("--test", "ctl")], "test": "ctl_once"}
        observed = [{"package": "c", "kind": "test", "target": "ctl", "test": "ctl_once", "result": "ignored"}]
        self.assertFalse(namespace["covered"](check, observed), "ignored result suppressed the invariant")
        self.coverage_fixture(ignored=True)
        self.dry_run()
        self.assertEqual(len((self.repo.state/"ctl-runs").read_text().splitlines()),2,
                         "ignored invariant was skipped")


    def stale_coverage(self,field):
        self.coverage_fixture()
        record,_=self.dry_run()
        path=Path(record["result"]).parent/"quarantine-coverage.json"
        data=json.loads(path.read_text())
        environment=dict(self.repo.environment)
        environment.update(BBOXY_PRESUBMIT_ATTEMPT=data["attempt"],BBOXY_CARGO_TARGET_DIR=data["target"])
        data[field]+="-stale";path.write_text(json.dumps(data))
        result=subprocess.run(["sh","scripts/invariants.sh","--affected","c","--covered-tests",str(path)],
            cwd=self.repo.root,env=environment,capture_output=True,text=True,timeout=45)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertEqual(len((self.repo.state/"ctl-runs").read_text().splitlines()),2,
                         "coverage from another "+field+" skipped the invariant")

    def test_another_attempt_cannot_discharge_the_invariant(self):
        self.stale_coverage("attempt")

    def test_another_target_cannot_discharge_the_invariant(self):
        self.stale_coverage("target")

    def test_brand_cut_skips_landing_fixtures(self):
        file=self.repo.root/"scripts/brand.sh"
        self.cut("scripts/brand.sh",file.read_text()+"\n# ordinary helper cut\n")
        _,result=self.dry_run()
        self.assertNotIn("nested fixture checks",result.stdout,"unrelated script ran landing fixtures")

    def test_harness_helper_cut_runs_landing_fixtures(self):
        file=self.repo.root/"scripts/harness-submodule.py"
        self.cut("scripts/harness-submodule.py",file.read_text()+"\n# copied helper cut\n")
        _,result=self.dry_run()
        self.assertIn("nested fixture checks",result.stdout,"copied harness helper skipped fixtures")

    def test_main_watch_cut_runs_landing_fixtures(self):
        file=self.repo.root/"scripts/main_watch.py"
        self.cut("scripts/main_watch.py",file.read_text()+"\n# copied watcher cut\n")
        _,result=self.dry_run()
        self.assertIn("nested fixture checks",result.stdout,"copied watcher skipped fixtures")

    def test_added_fixture_copy_automatically_triggers(self):
        name = "new-copied-fixture.py"
        self.assertFalse(landing.landing_machinery(["scripts/" + name], self.repo.root))
        path = self.repo.root / "scripts/test-landing.py"
        text = path.read_text().replace("FIXTURE_SCRIPTS = (", 'FIXTURE_SCRIPTS = ("' + name + '", ', 1)
        path.write_text(text)
        self.assertTrue(landing.landing_machinery(["scripts/" + name], self.repo.root),
                        "added fixture copy did not trigger")

    def test_landing_machinery_cut_runs_fixtures(self):
        self.cut("scripts/landing.py",(self.repo.root/"scripts/landing.py").read_text()+"\n# landing machinery fixture\n")
        _,result=self.dry_run()
        self.assertIn("nested fixture checks",result.stdout)

    def test_allocation_overrun_is_advisory(self):
        sha=self.cut("crates/c/src/lib.rs","pub fn value()->u8 {2}\n")
        data=landing.affected(self.repo.base+"..HEAD",root=self.repo.root)
        with patch.dict(os.environ,self.repo.environment), \
             patch.dict(landing.STAGE_ALLOCATIONS,{"formal":0.}), \
             contextlib.redirect_stdout(io.StringIO()):
            record=landing.presubmit_attempt(self.repo.root,sha,self.repo.base,data,1800.,900.,0,"budget-fixture",
                                             environment=self.repo.environment)
        self.assertTrue(any(row["kind"]=="stage_allocation" for row in record["findings"]))
        self.assertEqual(record["exit"],0,"stage advisory blocked presubmit")
        self.assertEqual(json.loads(Path(record["result"]).read_text())["exit"],0)

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


class CostTests(unittest.TestCase):
    def test_consolidated_modules_preserve_timing_policy(self):
        policy = landing.qos_policy()
        for target in policy.TIMED_TARGETS:
            with self.subTest(target=target):
                self.assertTrue(policy.timed_test("fixture::" + target, "ordinary_case"))
                self.assertTrue(policy.timed_test("fixture::it", target + "::ordinary_case"))
                self.assertFalse(policy.timed_test("fixture::other", target + "::ordinary_case"))
        self.assertFalse(policy.timed_test("fixture::it", "ordinary::case"))
        self.assertFalse(policy.timed_test("fixture::it", "gates_seats_extra::case"))

    def test_qos_probe_accepts_catalog_notifications_before_rpc_reply(self):
        spec = importlib.util.spec_from_file_location("qos_probe", SOURCE.with_name("qos-probe.py"))
        probe = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(probe)
        reader, writer = os.pipe()
        os.write(writer, b'{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}\n{"jsonrpc":"2.0","id":7,"result":{}}\n')
        os.close(writer)
        with os.fdopen(reader, "rb", buffering=0) as stream:
            server = Mock(stdout=stream)
            self.assertEqual(probe.rpc_response(server, 7)["id"], 7)

    def test_fast_land_initializes_rebased_source_before_cargo(self):
        repo = Repository()
        self.addCleanup(repo.close)
        repo.tooling()
        for name in ("land-fast.sh", "affected.sh"):
            shutil.copy2(SOURCE.with_name(name), repo.root / "scripts" / name)
        makefile = repo.root / "Makefile"
        makefile.write_text(makefile.read_text() + '\ninit-submodules:\n\t@git rev-parse HEAD > "$(BBOXY_LANDING_STATE_DIR)/initialized-source"\n')
        cargo = repo.bin / "cargo"
        cargo.write_text(cargo.read_text().replace('if args[0]=="metadata":',
            'assert (state/"initialized-source").read_text().strip()==subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(), "source was not initialized before Cargo"\nif args[0]=="metadata":'))
        repo.commit("Private lean gate tooling")
        repo.remote()
        repo.write("README.md", "Private documentation cut\n")
        candidate = repo.commit("Private lean candidate")
        (repo.state / "initialized-source").write_text(repo.base + "\n")
        result = subprocess.run(["sh", "scripts/land-fast.sh"], cwd=repo.root,
                                env=repo.environment, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(repo.remote_head(), candidate)
        self.assertIn("land-cost: make", result.stdout)
        self.assertIn("land-cost: total", result.stdout)

    def test_prebuild_keeps_harness_arguments_outside_cargo(self):
        self.assertEqual(landing.cargo_prebuild(["+stable", "test", "--lib", "--", "--ignored"]),
                         ["+stable", "test", "--lib", "--no-run", "--", "--ignored"])
        self.assertEqual(landing.cargo_prebuild(["nextest", "run", "--test", "one"]),
                         ["nextest", "run", "--test", "one", "--no-run"])
        arguments = ["nextest", "run", "--no-fail-fast", "--lib", "--", "filter"]
        self.assertEqual(landing.cargo_prebuild(arguments), ["nextest", "run", "--lib", "--no-run", "--", "filter"])
        self.assertIn("--no-fail-fast", arguments)
        self.assertEqual(landing.cargo_prebuild(["+stable", "run", "--bin", "app", "--", "--no-run"]),
                         ["+stable", "build", "--bin", "app"])
        self.assertEqual(landing.cargo_prebuild(["bench"]), ["bench", "--no-run"])
        self.assertIsNone(landing.cargo_prebuild(["test", "--no-run"]))
        self.assertEqual(landing.cargo_prebuild(["test", "--doc"]), ["test", "--lib", "--no-run"])
    def test_cargo_policy_parses_only_cargo_arguments(self):
        cases = [(["+stable", "--color", "never", "test"], "default"),
                 (["test", "--test=gates_seats"], "default"),
                 (["test", "--test", "formal_contracts"], "default"),
                 (["test", "--test", "gates_seats", "--no-run"], "background"),
                 (["clippy", "--test", "gates_seats"], "background"),
                 (["nextest", "list"], "background"),
                 (["nextest", "run", "--test", "input_latency"], "default"),
                 (["run", "--", "--no-run"], "default"),
                 (["run", "--example", "verify"], "utility"),
                 (["test", "--", "gates_seats", "--no-run"], "default"),
                 (["test", "--bin", "app", "lazy_projection_tests::input_frame_cpu_profile"], "default"),
                 (["test", "--lib", "repeated_text_frame_cpu_profile"], "default"),
                 (["test", "--bin", "app", "rich_faces::budget_verification::rich_four_view_aggregate_budget"], "default"),
                 (["test", "-p", "bboxy450-harness-host", "--lib", "client::tests"], "default"),
                 (["test", "-p", "other", "--lib", "client::tests"], "utility"),
                 (["test", "-p", "bboxy450-term-mux", "--lib"], "default"),
                 (["test", "-p", "bboxy450-term-mux", "--lib", "ordinary"], "utility"),
                 (["test", "--workspace", "--lib"], "default"),
                 (["test", "-p", "bboxy450-term-mux", "--lib", "--test", "mcp"], "default"),
                 (["test", "-p", "bboxy450-term-mux", "--test", "it"], "default"),
                 (["test", "-p", "bboxy450-term", "--test", "it", "formal_contracts::selected"], "default"),
                 (["test", "-p", "bboxy450-term", "--test", "it", "ordinary::selected"], "utility"),
                 (["test", "--lib", "logic::dream_c1::tests::dream_c1_source_error_capacity_is_atomic_across_real_sqlite_owners"], "default"),
                 (["test", "-p", "bboxy450-term-mux", "--test", "logic", "--", "--ignored"], "default"),
                 (["test", "-p", "bboxy450-term-mux", "--test", "logic", "ordinary", "--", "--include-ignored"], "utility"),
                 (["test", "-p", "bboxy450-rt", "--test", "runtime", "million_messages_per_second_never_wait_for_a_consumer", "--", "--ignored"], "default"),
                 (["test", "-p", "bboxy450-term-mux", "--lib", "cpu_profile"], "default"),
                 (["bench"], "default")]
        for arguments, expected in cases:
            self.assertEqual(landing.cargo_qos(arguments), expected, arguments)

    def test_nextest_runner_covers_both_darwin_architectures(self):
        key = "CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER"
        environment = landing.nextest_environment(["--target", "x86_64-apple-darwin"], {key: "custom --flag"})
        self.assertEqual(environment[key], "bboxy-qos-test --target-triple x86_64-apple-darwin")
        self.assertEqual(json.loads(environment["BBOXY_QOS_PREVIOUS_TEST_RUNNERS"])[key], "custom --flag")
        self.assertEqual(json.loads(environment["BBOXY_QOS_INNER_TEST_RUNNERS"])["x86_64-apple-darwin"], ["custom", "--flag"])

    def test_cost_counts_descendants_disk_and_preserves_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            code = "from pathlib import Path; import sys; Path(sys.argv[1]).write_bytes(b'x'*8192); raise SystemExit(7)"
            result = subprocess.run([sys.executable, "-I", str(SOURCE), "cost", directory,
                                     "fixture", sys.executable, "-c", code,
                                     str(Path(directory) / "growth")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 7, result.stderr)
            self.assertRegex(result.stdout, r"land-cost: fixture wall=\d+\.\d+s cpu=\d+\.\d+s disk=\+[1-9]\d*B exit=7")

    def test_invariant_leaves_separate_timing_from_ordinary_checks(self):
        for command, role in [("python3 -I scripts/release-inputs.py", "utility"),
                              ("sh scripts/ui-lint.sh", "utility"),
                              ("make test-formal", "default"),
                              ("cargo test --test formal_contracts", "default"),
                              ("GATE_BUDGET=8.33 sh scripts/gates.sh frame", "default")]:
            self.assertEqual(landing.invariant_qos(command), role)


@unittest.skipUnless(sys.platform == "darwin", "Darwin effective-priority witness")
class QosTests(unittest.TestCase):
    SNAPSHOT = ('import ctypes,json,os,subprocess; '
                'print(json.dumps(dict(background=ctypes.CDLL(None).getpriority(4,os.getpid()),'
                'priority=int(subprocess.check_output(["ps","-o","pri=","-p",str(os.getpid())],text=True)))))')

    def assert_role(self, snapshot, role):
        # ps reports a dynamic scheduling priority: background may read 3,
        # default 30, etc. Query the native Darwin background flag separately.
        self.assertEqual(snapshot["background"], int(role == "background"), snapshot)
        if role == "utility":
            self.assertGreater(snapshot["priority"], 4)
            self.assertLessEqual(snapshot["priority"], 20)
        elif role == "default":
            self.assertGreater(snapshot["priority"], 20)

    def test_mixed_nextest_tests_select_their_own_effective_policy(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory(prefix="qos-nextest-") as directory:
            own = Path(directory)
            (own / "src").mkdir()
            (own / "Cargo.toml").write_text('[package]\nname="qos-fixture"\nversion="0.0.0"\nedition="2024"\nautotests=false\n[workspace]\n[[test]]\nname="it"\npath="tests/it.rs"\n')
            witness = '''unsafe extern "C" { fn getpriority(which: i32, who: u32) -> i32; }
fn priority() -> i32 {
    let wanted = std::env::var("NEXTEST_DYLD_FALLBACK_LIBRARY_PATH").unwrap();
    assert_eq!(std::env::var("DYLD_FALLBACK_LIBRARY_PATH").unwrap(), wanted);
    let runner_key = std::env::var("QOS_RUNNER_KEY").unwrap();
    assert_eq!(std::env::var(runner_key).unwrap(), std::env::var("QOS_EXPECTED_RUNNER").unwrap());
    assert_eq!(unsafe { getpriority(4, std::process::id()) }, 0);
    let output = std::process::Command::new("ps").args(["-o", "pri=", "-p", &std::process::id().to_string()]).output().unwrap();
    String::from_utf8(output.stdout).unwrap().trim().parse().unwrap()
}
#[test] fn ordinary() { let p=priority(); assert!(p>4 && p<=20, "{p}"); }
#[test] fn actual_private_control_invalid_ulids_and_unsafe_revision_emit_no_new_frame() { let p=priority(); assert!(p>20, "{p}"); }
'''
            (own / "src/lib.rs").write_text(witness)
            (own / "tests").mkdir()
            (own / "tests/it.rs").write_text(witness.split("#[test]", 1)[0] + '''
mod gates_seats {
    #[test] fn ordinary_case() { let p=super::priority(); assert!(p>20, "{p}"); }
}
mod ordinary {
    #[test] fn ordinary_case() { let p=super::priority(); assert!(p>4 && p<=20, "{p}"); }
}
''')
            inner = own / "inner-runner"
            inner.write_text("#!" + sys.executable + "\nimport os,sys\nwith open(os.environ['QOS_INNER_TRACE'],'a') as f: f.write(repr(sys.argv[1:])+'\\n')\nos.execv(sys.argv[1],sys.argv[1:])\n")
            inner.chmod(0o700)
            triple = "AARCH64_APPLE_DARWIN" if os.uname().machine == "arm64" else "X86_64_APPLE_DARWIN"
            environment = dict(os.environ, BBOXY_CACHE_MODE="off", BBOXY_CARGO_TARGET_DIR=str(own / "target"),
                               QOS_INNER_TRACE=str(own / "trace"), QOS_RUNNER_KEY="CARGO_TARGET_" + triple + "_RUNNER",
                               QOS_EXPECTED_RUNNER=str(inner), **{"CARGO_TARGET_" + triple + "_RUNNER": str(inner)})
            result = subprocess.run(["sh", "-c", '. scripts/cache-env.sh; cd "$1"; cargo nextest run --offline --lib --test it --test-threads 1', "fixture", directory],
                                    cwd=root, env=environment, capture_output=True, text=True, timeout=90)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("4 tests run: 4 passed", result.stdout + result.stderr)
            self.assertIn("ordinary", (own / "trace").read_text())
            self.assertIn("actual_private_control", (own / "trace").read_text())
            self.assertIn("gates_seats::ordinary_case", (own / "trace").read_text())
            self.assertIn("ordinary::ordinary_case", (own / "trace").read_text())

    def test_cargo_shim_survives_private_environment_filtering(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            own = Path(directory)
            real = own / "real"
            real.mkdir()
            fake = real / "cargo"
            fake.write_text("#!" + sys.executable + "\n" + self.SNAPSHOT + "\n")
            fake.chmod(0o700)
            launcher = 'import os,subprocess; e={k:v for k,v in os.environ.items() if not k.startswith("BBOXY_")}; r=subprocess.run(["cargo","build"],env=e); raise SystemExit(r.returncode)'
            environment = dict(os.environ, BBOXY_CACHE_MODE="off", BBOXY_CARGO_TARGET_DIR=str(own / "target"),
                               PATH=str(real) + os.pathsep + os.environ["PATH"])
            result = subprocess.run(["sh", "-c", '. scripts/cache-env.sh; exec "$1" -I -c "$2"', "fixture", sys.executable, launcher],
                                    cwd=root, env=environment, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assert_role(json.loads(result.stdout), "background")

    def test_cost_retains_the_cpu_of_the_background_prebuild(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "cargo"
            fake.write_text("#!" + sys.executable + "\nimport sys,time\n"
                            "if '--no-run' in sys.argv:\n"
                            "    until=time.process_time()+0.4\n"
                            "    while time.process_time()<until: pass\n")
            fake.chmod(0o700)
            environment = dict(os.environ, BBOXY_REAL_CARGO=str(fake), BBOXY_CACHE_MODE="off",
                               BBOXY_CARGO_TARGET_DIR=str(Path(directory) / "target"))
            result = subprocess.run([sys.executable, "-I", str(SOURCE), "cost", directory,
                                     "native-qos", "sh", "-c", ". scripts/cache-env.sh; cargo test"],
                                    cwd=root, env=environment, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            cost = next(line for line in result.stdout.splitlines() if line.startswith("land-cost:"))
            fields = dict(field.split("=", 1) for field in cost.split()[2:])
            self.assertGreaterEqual(float(fields["cpu"].removesuffix("s")), .35, cost)

    def test_main_watcher_does_not_nice_its_timing_descendants(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            own = Path(directory)
            for name in ("main-watch.sh", "qos.sh"):
                shutil.copy2(root / "scripts" / name, own / name)
            (own / "cache-env.sh").write_text("# private launcher fixture\n")
            (own / "landing.py").write_text(self.SNAPSHOT + "\n")
            result = subprocess.run(["sh", str(own / "main-watch.sh")],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assert_role(json.loads(result.stdout), "default")

    def test_rustdoc_compiles_background_and_executes_utility(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "doc.rs"
            source.write_text('/// ```\n/// unsafe extern "C" { fn getpriority(which: i32, who: u32) -> i32; }\n/// assert_eq!(unsafe { getpriority(4, std::process::id()) }, 0);\n/// let output = std::process::Command::new("ps").args(["-o", "pri=", "-p", &std::process::id().to_string()]).output().unwrap();\n/// let priority: i32 = String::from_utf8(output.stdout).unwrap().trim().parse().unwrap();\n/// assert!(priority > 4 && priority <= 20);\n/// ```\npub fn fixture() {}\n')
            environment = dict(os.environ, BBOXY_CACHE_MODE="off", BBOXY_CARGO_TARGET_DIR=str(Path(directory) / "target"))
            result = subprocess.run(["sh", "-c", '. scripts/cache-env.sh; "$RUSTDOC" --test "$1"', "fixture", str(source)],
                                    cwd=root, env=environment, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("1 passed", result.stdout)

    def test_policy_is_inherited_by_a_grandchild(self):
        root = SOURCE.parents[1]
        parent = 'import subprocess,sys;subprocess.run([sys.executable,"-I","-c",sys.argv[1]],check=True)'
        for role in ("background", "utility", "default"):
            result = subprocess.run(["sh", str(root / "scripts/qos.sh"), role,
                                     sys.executable, "-I", "-c", parent, self.SNAPSHOT],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assert_role(json.loads(result.stdout), role)
        for role in ("utility", "default"):
            result = subprocess.run(["sh", str(root / "scripts/qos.sh"), "background", "sh",
                                     str(root / "scripts/qos.sh"), role, sys.executable, "-I", "-c", self.SNAPSHOT],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assert_role(json.loads(result.stdout), role)

    def test_make_interpreted_tests_use_utility(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            script = Path(directory) / "priority.py"
            script.write_text(self.SNAPSHOT + "\n")
            makefile = Path(directory) / "Makefile"
            makefile.write_text("test-qos-fixture:\n\t@" + shlex.join([sys.executable, str(script)]) + "\n")
            result = subprocess.run(["make", "-s", "-f", "Makefile", "-f", str(makefile), "test-qos-fixture"],
                                    cwd=root, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assert_role(json.loads(result.stdout), "utility")

    def test_real_invariant_dispatcher_clamps_only_ordinary_leaves(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            own = Path(directory)
            (own / "scripts").mkdir()
            (own / ".bboxy/invariants").mkdir(parents=True)
            for name in ("invariants.sh", "landing.py", "qos.sh"):
                shutil.copy2(root / "scripts" / name, own / "scripts" / name)
            (own / "scripts/cache-env.sh").write_text("# private fixture; no cache or operator paths\n")
            for name, role in [("ordinary", "utility"), ("timed", "default")]:
                script = "gates.sh" if role == "default" else "ordinary.sh"
                (own / "scripts" / script).write_text("#!/bin/sh\n" + shlex.join([sys.executable, "-I", "-c", self.SNAPSHOT]) + "\n")
                (own / ".bboxy/invariants" / (name + ".md")).write_text(
                    "---\ntype: invariant\ncheck: sh scripts/" + script + "\n---\n")
            result = subprocess.run(["sh", "scripts/invariants.sh"], cwd=own,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            snapshots = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
            self.assertEqual(len(snapshots), 2, result.stdout)
            self.assert_role(snapshots[0], "utility")
            self.assert_role(snapshots[1], "default")

    def test_real_make_timing_entries_leave_the_measured_process_unclamped(self):
        root = SOURCE.parents[1]
        # A utility ancestor cannot be undone by clearing Darwin background.
        # Keep timing orchestration default, with utility only on sibling leaves.
        inherited = subprocess.run(["sh", str(root / "scripts/qos.sh"), "utility", "sh",
                                    str(root / "scripts/qos.sh"), "default", sys.executable,
                                    "-I", "-c", self.SNAPSHOT], capture_output=True, text=True)
        self.assertEqual(inherited.returncode, 0, inherited.stderr)
        self.assert_role(json.loads(inherited.stdout), "utility")
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "cargo"
            fake.write_text("#!" + sys.executable + "\n" + self.SNAPSHOT + "\n")
            fake.chmod(0o700)
            environment = dict(os.environ, BBOXY_REAL_CARGO=str(fake), BBOXY_CACHE_MODE="off")
            for target, expected in [("test-input-latency", "default"),
                                     ("profile-input-ui", "default"),
                                     ("profile-render-input", "default"),
                                     ("bench-budgets", "default"),
                                     ("test-input-layout", "utility")]:
                result = subprocess.run(["make", "-s", "-o", "tools", "-o", "init-submodules",
                                         target, "TARGET_DIR=" + str(Path(directory) / "target")],
                                        cwd=root, env=environment, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, target + result.stdout + result.stderr)
                snapshots = [json.loads(line) for line in result.stdout.splitlines()]
                self.assertTrue(snapshots, target)
                for snapshot in snapshots:
                    self.assert_role(snapshot, expected)
            nested = Path(directory) / "nested.mk"
            nested.write_text("test-harness-native:\n\t@$(MAKE) -s -o init-submodules bench-budgets\n")
            result = subprocess.run(["make", "-s", "-f", "Makefile", "-f", str(nested),
                                     "-o", "init-submodules", "test-harness-native",
                                     "TARGET_DIR=" + str(Path(directory) / "target")],
                                    cwd=root, env=environment, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            snapshots = [json.loads(line) for line in result.stdout.splitlines()]
            self.assertTrue(snapshots)
            for snapshot in snapshots:
                self.assert_role(snapshot, "default")

    def test_timed_make_parents_do_not_inherit_utility(self):
        root = SOURCE.parents[1]
        bodies = {}
        current = []
        for line in (root / "Makefile").read_text().splitlines():
            if line.startswith("\t"):
                for target in current:
                    bodies[target] = bodies.get(target, "") + "\n" + line
            elif __import__("re").match(r"^[\w-]+(?: [\w-]+)*:", line):
                current = line.split(":", 1)[0].split()
            else:
                current = []
        # Inventory actual timed leaves and every literal recursive Make parent;
        # this fails when a new parent accidentally clamps a timed descendant.
        timed = {target for target, body in bodies.items()
                 if ("cargo test " in body or "cargo nextest run " in body)
                 and ("--test formal_contracts" in body
                      or ("--test it" in body and "formal_contracts::" in body)
                      or ("--test it" in body and "(formal_contracts)" in body))}
        timed.update({"bench-budgets", "test-input-latency", "profile-input-ui", "profile-render-input", "test-harmony-host", "test-harness-startup"})
        while True:
            parents = {target for target, body in bodies.items() if target != "help" and
                       any("$(MAKE)" in line and any(child in line.split() for child in timed)
                           for line in body.splitlines())}
            if parents <= timed:
                break
            timed.update(parents)
        for target in timed:
            command = shlex.join([sys.executable, "-I", "-c", self.SNAPSHOT])
            result = subprocess.run(["sh", str(root / "scripts/qos.sh"), "make-" + target,
                                     "-c", command], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assert_role(json.loads(result.stdout), "default")

    def test_cache_env_routes_cargo_and_compiler_independently(self):
        root = SOURCE.parents[1]
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "fake-cargo"
            fake.write_text("#!" + sys.executable + "\nimport json,os,subprocess,sys\nSNAPSHOT=" + repr(self.SNAPSHOT) + "\n"
                'own=json.loads(subprocess.check_output([sys.executable,"-I","-c",SNAPSHOT],text=True))\n'
                'with open(os.environ["QOS_TRACE"],"a") as f:f.write(json.dumps(own)+"\\n")\n'
                'print(json.dumps(own),flush=True)\n'
                'subprocess.run([os.environ["RUSTC_WRAPPER"],sys.executable,"-I","-c",SNAPSHOT],check=True)\n')
            fake.chmod(0o700)
            environment = dict(os.environ, BBOXY_REAL_CARGO=str(fake), BBOXY_CACHE_MODE="off",
                               BBOXY_CARGO_TARGET_DIR=str(Path(directory) / "target"), QOS_TRACE=str(Path(directory) / "trace"))
            for arguments, role in [("build", "background"), ("test --lib ordinary", "utility"), ("test --test gates_seats", "default")]:
                Path(environment["QOS_TRACE"]).write_text("")
                result = subprocess.run(["sh", "-c", '. scripts/cache-env.sh; cargo ' + arguments],
                                        cwd=root, env=environment, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                snapshots = [json.loads(line) for line in result.stdout.splitlines()]
                self.assertEqual(len(snapshots), 2)
                self.assert_role(snapshots[0], role)
                self.assert_role(snapshots[1], "background")
                trace = [json.loads(line)["background"] for line in Path(environment["QOS_TRACE"]).read_text().splitlines()]
                self.assertEqual(trace, [1] if arguments == "build" else [1, 0])


if __name__ == "__main__":
    if len(sys.argv) == 1 or sys.argv[1:] == ["--all"]:
        raise SystemExit(fixture_suite(extended=sys.argv[1:] == ["--all"]))
    unittest.main()
