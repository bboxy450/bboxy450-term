# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Exercise the real node, operator socket and supervised PTY with private custody."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def smoke(binary):
    with tempfile.TemporaryDirectory(prefix="bb-node-", dir="/tmp") as temporary:
        root = Path(temporary).resolve()
        run = root / "run"
        env = {
            "HOME": temporary,
            "PATH": os.pathsep.join((str(binary.parent), "/usr/bin", "/bin")),
            "SHELL": "/bin/zsh",
            "ZDOTDIR": temporary,
            "BBOXY_RUN_DIR": str(run),
        }
        subprocess.run(["git", "init", "-q", temporary], env=env, check=True)

        def ctl(*args):
            return subprocess.run(
                [binary, "ctl", *args], cwd=root, env=env,
                capture_output=True, text=True, timeout=35,
            )

        def data(*args):
            result = ctl(*args)
            assert result.returncode == 0, (args, result.returncode, result.stdout, result.stderr)
            return json.loads(result.stdout)["data"]

        log = root / "node.log"
        with log.open("w") as output:
            # Linux uses its real file-custody selector in a throwaway run.
            custody_args = (
                [] if sys.platform == "linux"
                else ["--unattended", "--test-account", root / "account"]
            )
            node = subprocess.Popen(
                [binary, "node", "--run-dir", run, "--repository", root,
                 *custody_args],
                cwd=root, env=env, stdin=subprocess.DEVNULL, stdout=output, stderr=output,
            )
        endpoint = run / "gui.endpoint"
        try:
            deadline = time.monotonic() + 60
            status = None
            while node.poll() is None and time.monotonic() < deadline:
                if endpoint.is_file():
                    result = ctl("app", "status")
                    if result.returncode == 0:
                        status = json.loads(result.stdout)["data"]
                        break
                time.sleep(0.05)
            assert status is not None, "node did not become ready through ctl"
            assert status["running"] and status["backend_ready"] and status["generation"] > 0, status
            assert status["mode"] == "node" and status["windows"] == 0 and not status["gui_ready"], status
            assert status["unattended"] == bool(custody_args), status
            assert status["custody_backend"] == "file", status
            assert data("seat", "ls") == []
            spawned = data("seat", "spawn", "zsh", "--name", "node-smoke", "--cwd", str(root))
            seat = spawned["seat"]
            rows = data("seat", "ls")
            assert len(rows) == 1 and rows[0]["record"]["id"] == seat, rows
            typed = ctl("seat", "type", seat, "print -r -- node-smoke-''ready")
            assert typed.returncode == 0, (typed.stdout, typed.stderr)
            entered = ctl("seat", "key", seat, "enter")
            assert entered.returncode == 0, (entered.stdout, entered.stderr)
            ready = ctl("seat", "wait", seat, "--text", "node-smoke-ready", "--timeout", "15000")
            assert ready.returncode == 0, (ready.stdout, ready.stderr)
            stopped = data("app", "stop")
            assert not stopped["running"] and not endpoint.exists(), stopped
            assert node.wait(timeout=15) == 0
            for name in ["logic", "supervisor"]:
                assert not (run / f"{name}.owner.json").exists(), f"{name} failed to retire"
            print("node smoke passed: ctl status, zero windows, zsh PTY output, ctl stop, both owners retired")
        except Exception:
            for path in [log, run / "supervisor.log", run / "logic.log"]:
                if path.is_file():
                    print(f"{path.name}:\n{path.read_text(errors='replace')[-8192:]}", file=sys.stderr)
            raise
        finally:
            if node.poll() is None:
                node.terminate()
                try:
                    node.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    node.kill()
                    node.wait()


if __name__ == "__main__":
    smoke(Path(sys.argv[1]).resolve(strict=True))
