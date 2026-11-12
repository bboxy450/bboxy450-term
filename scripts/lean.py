#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Check exported Lean theorems with the pinned Lean 4 kernel. No operator, no global install.

    lean.py --check DIR     verify every row of DIR/lean.tsv, write DIR/lean-evidence.json
    lean.py --where         print the pinned `lean` executable (installing it if absent)

A row is `MODULE<TAB>pass|fail<TAB>THEOREM,...`. DIR holds MODULE.lean (generated
definitions and statements), MODULEProof.lean (written by an agent) and
MODULECheck.lean (generated: each proof has exactly its generated statement).
`pass` needs all three to compile and every theorem to rest only on Lean's three
standard axioms, so `sorry`, `native_decide` and a declared axiom are refused.
`fail` is a broken control: the generated definitions must compile and the proof
must be refused. A tool failure or timeout is an error, never either verdict.
"""
import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STANDARD_AXIOMS = {"propext", "Classical.choice", "Quot.sound"}
TIMEOUT = 300


def cache_root():
    if os.environ.get("BBOXY_LEAN_CACHE"):
        return Path(os.environ["BBOXY_LEAN_CACHE"])
    if platform.system() == "Darwin":
        return Path.home() / "Library/Caches/bboxy450-lean"
    return Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "bboxy450-lean"


def wanted(member):
    """The kernel, its shared libraries and the core library: about a fifth of the archive."""
    parts = Path(member).parts[1:]
    if parts == ("bin", "lean"):
        return True
    name = parts[-1] if parts else ""
    shared = name.endswith(".dylib") or ".so" in name
    if parts[:2] == ("lib", "lean") and len(parts) == 3:
        return shared or name.startswith("Init.olean")
    if parts[:3] == ("lib", "lean", "Init"):
        return ".olean" in name
    return parts[:2] == ("lib", "libc") and shared


class Hashing:
    """Hashes every byte the archive reader consumes."""

    def __init__(self, raw):
        self.raw, self.digest = raw, hashlib.sha256()

    def read(self, size=-1):
        data = self.raw.read(size)
        self.digest.update(data)
        return data


def lean():
    pins = json.loads((ROOT / "verification/tools.lock.json").read_text())["lean"]
    host = f"{platform.system()}-{platform.machine()}"
    if host not in pins:
        raise ValueError(f"no pinned Lean toolchain for {host}; add a reviewed tools.lock.json entry")
    pin = pins[host]
    home = cache_root() / f"lean-{pin['version']}-{pin['sha256'][:16]}"
    binary = home / "bin/lean"
    if binary.is_file():
        return binary, pin
    if sys.version_info < (3, 14):
        raise ValueError("installing Lean needs Python 3.14+ (stdlib zstd)")
    home.parent.mkdir(parents=True, exist_ok=True)
    # Stream: the 570 MB archive is hashed and filtered without touching the disk.
    with tempfile.TemporaryDirectory(prefix="install-", dir=home.parent) as own:
        curl = subprocess.Popen(["curl", "-fsSL", "--retry", "2", "--max-time", "1500", pin["url"]],
                                stdout=subprocess.PIPE)
        stream = Hashing(curl.stdout)
        with tarfile.open(fileobj=stream, mode="r|zst") as archive:
            for member in archive:
                if (member.isfile() or member.issym()) and wanted(member.name):
                    archive.extract(member, own, filter="data")
        while stream.read(1 << 20):
            pass
        if curl.wait(timeout=60) != 0:
            raise ValueError("Lean download failed")
        if stream.digest.hexdigest() != pin["sha256"]:
            raise ValueError(f"Lean archive checksum mismatch: {stream.digest.hexdigest()} != {pin['sha256']}")
        (entry,) = Path(own).iterdir()
        if not home.exists():  # A concurrent installer may have won; both are verified.
            entry.replace(home)
    if not binary.is_file():
        raise ValueError(f"Lean toolchain missing executable: {binary}")
    return binary, pin


def compile_file(binary, directory, name):
    """(compiled, output). A timeout or a missing tool raises."""
    result = subprocess.run(
        [str(binary), "-o", f"{name}.olean", f"{name}.lean"], cwd=directory, text=True,
        capture_output=True, timeout=TIMEOUT, env={"LEAN_PATH": str(directory), "PATH": os.defpath})
    if result.returncode not in (0, 1):
        raise ValueError(f"Lean tool failure on {name}.lean: exit {result.returncode}: {result.stderr}")
    return result.returncode == 0, result.stdout + result.stderr


def axioms(output):
    """`#print axioms` results by theorem: {'den_check_ID': {'propext', ...}}."""
    found, lines = {}, output.splitlines()
    for i, line in enumerate(lines):
        if "depends on axioms:" in line:
            name = line.split("'")[1]
            text = line.split("depends on axioms:", 1)[1]
            j = i + 1
            while "]" not in text and j < len(lines):
                text += lines[j]
                j += 1
            found[name] = {a.strip() for a in text.strip(" []").split(",") if a.strip()}
        elif "does not depend on any axioms" in line:
            found[line.split("'")[1]] = set()
    return found


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(directory):
    binary, pin = lean()
    rows = [line.split("\t") for line in (directory / "lean.tsv").read_text().splitlines() if line]
    if not rows:
        raise ValueError("no Lean theorems to check")
    evidence, failed = [], []
    for module, expected, theorems in rows:
        theorems = theorems.split(",")
        if expected not in ("pass", "fail"):
            raise ValueError(f"unknown expectation for {module}: {expected}")
        ok, output = compile_file(binary, directory, module)
        if not ok:  # Generated text: a refusal here is our defect, never a verdict.
            raise ValueError(f"generated {module}.lean does not compile:\n{output}")
        ok, output = compile_file(binary, directory, module + "Proof")
        reason = None if ok else "the proof does not compile"
        used = {}
        if ok:
            ok, checked = compile_file(binary, directory, module + "Check")
            output += checked
            used = axioms(checked)
            if not ok:
                reason = "a proof does not have its generated statement"
            elif set(used) != {f"den_check_{t}" for t in theorems}:
                raise ValueError(f"{module}: axiom report does not cover every theorem:\n{checked}")
            else:
                extra = sorted(set().union(*used.values()) - STANDARD_AXIOMS)
                if extra:
                    reason = f"a proof rests on {', '.join(extra)}"
        actual = "fail" if reason else "pass"
        print(f"lean: {module}: {actual}" + (f" ({reason})" if reason else f", {len(theorems)} theorems"))
        if actual != expected:
            failed.append(f"{module}: expected {expected}, got {actual}\n{output}")
        evidence.append({
            "module": module, "expected": expected, "verdict": actual, "reason": reason,
            "theorems": theorems, "evidence": "unbounded" if actual == "pass" else None,
            "axioms": {k: sorted(v) for k, v in sorted(used.items())},
            "statements_sha256": sha(directory / f"{module}.lean"),
            "proof_sha256": sha(directory / f"{module}Proof.lean"),
        })
    if failed:
        raise ValueError("Lean gate refused:\n" + "\n".join(failed))
    receipt = {"lean": {"version": pin["version"], "archive_sha256": pin["sha256"]}, "modules": evidence}
    (directory / "lean-evidence.json").write_text(json.dumps(receipt, indent=2) + "\n")
    for stale in directory.glob("*.olean*"):
        stale.unlink()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", type=Path)
    parser.add_argument("--where", action="store_true")
    args = parser.parse_args()
    if args.where:
        print(lean()[0])
    elif args.check:
        check(args.check.resolve())
    else:
        parser.error("expected --check DIR or --where")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.TimeoutExpired, OSError) as error:
        print(f"lean: {error}", file=sys.stderr)
        raise SystemExit(1)
