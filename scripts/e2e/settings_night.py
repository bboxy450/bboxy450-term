"""Run exactly R2/G1/G2 with fresh private apps; individual scripts decide verdicts.

Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
Private environment/readiness contract adapted from bboxy450-term src/e2e_night.rs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(binary, script, out):
    root = Path(tempfile.mkdtemp(prefix="bset-", dir="/tmp"))
    for name in ("home", "work", "repository", "zsh-config", "bin"):
        (root / name).mkdir(mode=0o700)
    python = root / "bin/python3"
    python.write_text("#!/bin/sh\nexec " + shlex.quote(sys.executable) + ' "$@"\n')
    python.chmod(0o700)
    env = {k: v for k, v in os.environ.items() if k in
           ("TERM", "LANG", "LC_ALL", "LC_CTYPE", "TMPDIR")}
    env.update(HOME=str(root / "home"), CODEX_HOME=str(root / "home/.codex"),
               CLAUDE_CONFIG_DIR=str(root / "home/.claude"), ZDOTDIR=str(root / "zsh-config"),
               BBOXY_RUN_DIR=str(root / "run"),
               PATH=str(root / "bin") + ":/usr/bin:/bin:/usr/sbin:/sbin")
    (root / "zsh-config/.zprofile").write_text("export PATH=" + shlex.quote(env["PATH"]) + "\n")
    calls = []

    def ctl(*args, check=True):
        result = subprocess.run([str(binary), "ctl", "--run-dir", str(root / "run"), *map(str, args)],
                                env=env, cwd=root / "repository", stdin=subprocess.DEVNULL,
                                capture_output=True, text=True, timeout=45)
        rows = [json.loads(s) for s in result.stdout.splitlines() if s.strip()]
        assert len(rows) == 1, (args, result.stdout, result.stderr)
        reply = rows[0]
        calls.append({"args": list(map(str, args)), "exit_code": result.returncode, "reply": reply})
        if check:
            assert result.returncode == 0 and reply.get("ok") is True, (args, reply, result.stderr)
        return result.returncode, reply

    verdict = {"promise": script.stem, "coverage": "full", "root": str(root),
               "script_sha256": digest(script), "exit_code": None, "status": "fail"}
    start = time.monotonic()
    started = False
    try:
        assert script.read_text().splitlines()[0] == "# coverage: full", "script is not full coverage"
        _, reply = ctl("app", "start", "--headless", "--ctl", "--unattended",
                       "--notifications", "record", "--test-account", root / "account")
        started = True
        status = reply["data"]
        assert status["windows"] == 0 and status["mode"] == "headless" and status["custody_backend"] == "file", status
        # Read the actual admitted identity rendered by Account, never invent a label.
        _, account = ctl("ui", "open", "settings")
        def texts(node):
            if isinstance(node, dict):
                if node.get("kind") == "text":
                    yield node.get("text", "")
                for child in node.get("children", []):
                    yield from texts(child)
        labels = [s for tree in account["data"]["tree"] for s in texts(tree)
                  if re.fullmatch(r"test-account:[0-9A-Z]{26}", s)]
        assert len(labels) == 1, "Account did not expose a verified test-account identity"
        env.update(BBT=str(binary), E2E_WORK=str(root / "work"), E2E_APP=str(status["pid"]),
                   BBOXY_CTL_ISSUER=labels[0])
        ctl("ui", "key", "esc")
        with (root / "work/output.log").open("w") as log:
            result = subprocess.run(["/bin/sh", str(script)], env=env, cwd=root / "repository",
                                    stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                                    timeout=180)
        verdict["exit_code"] = result.returncode
        negative_file = root / "work/negative-controls.jsonl"
        negatives = [json.loads(s) for s in negative_file.read_text().splitlines()] if negative_file.exists() else []
        assert result.returncode == 0, (root / "work/output.log").read_text()[-3000:]
        assert negatives and all(n["rejected"] for n in negatives), "missing negative witnesses"
        verdict.update(status="pass", negatives=negatives, issued_by=labels[0], readiness=status)
    except Exception as error:
        verdict["error"] = str(error)
    finally:
        if started:
            try:
                ctl("app", "stop")
            except Exception as error:
                verdict.update(status="fail", cleanup_error=str(error))
        verdict["elapsed_seconds"] = round(time.monotonic() - start, 3)
        (root / "runner-calls.json").write_text(json.dumps(calls, indent=2) + "\n")
        (out / (script.stem + ".json")).write_text(json.dumps(verdict, indent=2) + "\n")
        print(json.dumps(verdict), flush=True)
    return verdict


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--only", nargs="+", choices=("R2", "G1", "G2"), default=["R2", "G1", "G2"])
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    scripts = Path(__file__).resolve().parent
    args.out.mkdir(parents=True, exist_ok=True)
    inputs = [scripts / (name + ".sh") for name in args.only]
    inputs += [scripts / name for name in ("settings.py", "settings_night.py", "rim_more.py", "ui.py", "rim_clock.py")]
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=scripts, text=True).strip()
    diff = subprocess.check_output(["git", "diff", "HEAD", "--"], cwd=scripts)
    manifest = {"binary": str(binary), "binary_sha256": digest(binary), "source_head": revision,
                "tracked_diff_sha256": hashlib.sha256(diff).hexdigest(),
                "inputs": {str(p): digest(p) for p in inputs}, "promises": args.only}
    (args.out / "invocation.json").write_text(json.dumps(manifest, indent=2) + "\n")
    verdicts = [run(binary, scripts / (name + ".sh"), args.out) for name in args.only]
    passed = all(v["status"] == "pass" and v["coverage"] == "full" and v["exit_code"] == 0 for v in verdicts)
    (args.out / "verdict.json").write_text(json.dumps({"pass": passed, "verdicts": verdicts}, indent=2) + "\n")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
