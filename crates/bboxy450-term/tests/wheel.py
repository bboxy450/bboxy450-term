#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Private native-wheel/process witness; no OS input or operator account access."""
import argparse
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile
import time

TUI = r'''
import os,re,sys,tty
for i in range(1,501): print(f"line {i:05}\r")
tty.setraw(0)
sys.stdout.write("\x1b[?1049h" + ("\x1b[?1000h\x1b[?1006h" if sys.argv[1]=="mouse" else ""))
offset=100
pending=b""
def render():
    sys.stdout.write(f"\x1b[H\x1b[2JTUI OFFSET {offset}\r\n")
    sys.stdout.flush()
render()
while True:
    chunk=os.read(0,4096)
    if not chunk: break
    pending+=chunk
    matches=list(re.finditer(rb"\x1b\[<(64|65);[0-9]+;[0-9]+M|\x1b[\[O]([AB])",pending))
    for m in matches:
        offset+= -1 if m[1]==b"64" or m[2]==b"A" else 1
    if matches:
        pending=pending[matches[-1].end():]
        render()
    assert len(pending)<4096, "unrecognized or unbounded wheel input"
'''


def await_value(read, label, timeout=35):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        last = read()
        if last:
            return last
        time.sleep(0.05)
    raise AssertionError(f"{label} did not converge; last={last}")


def case(binary, mode, expect_dead):
    root = Path(tempfile.mkdtemp(prefix=f"bboxy-wheel-{mode}-", dir="/tmp"))
    print(f"wheel fixture mode={mode} root={root}", flush=True)
    account, run = root / "account", root / "account/run"
    shell, repository = root / "shell", root / "repository"
    shell.mkdir()
    repository.mkdir()
    account.mkdir(mode=0o700)
    run.mkdir(mode=0o700)
    if mode == "plain":
        command = "for ((i=1;i<=500;i++)); do printf 'line %05d\\r\\n' \"$i\"; done\nexec /bin/cat\n"
    else:
        (root / "tui.py").write_text(TUI)
        command = f"exec {shlex.quote(sys.executable)} {shlex.quote(str(root / 'tui.py'))} {mode}\n"
    (shell / ".zshrc").write_text(command)
    env = {"HOME": str(root), "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "ZDOTDIR": str(shell),
           "BBOXY_RUN_DIR": str(run), "BBOXY_TEST_SCRIPTED_HOSTS": "off"}
    log_path = root / "frontend.log"
    log = log_path.open("a")
    process = None
    owned = []

    def start(spawn):
        args = [str(binary), "--headless", "--ctl", "--unattended", "--test-account", str(account),
                "--run-dir", str(run), "--repository", str(repository), "--wheel-after", "3"]
        if spawn:
            args += ["--spawn-zsh", "1"]
        p = subprocess.Popen(args, env=env, cwd=repository, stdin=subprocess.DEVNULL, stdout=log, stderr=log)
        owned.append(p.pid)
        (root / "owned-frontend-pids.json").write_text(json.dumps(owned))
        return p

    def ctl(*args, optional=False):
        result = subprocess.run([str(binary), "ctl", *args, "--run-dir", str(run)], env=env,
                                cwd=repository, capture_output=True, text=True, timeout=35)
        if optional and result.returncode:
            return None
        assert result.returncode == 0, (args, result.returncode, result.stdout, result.stderr, str(root))
        return json.loads(result.stdout)["data"]

    def pane():
        assert process is not None and process.poll() is None, log_path.read_text()[-4000:]
        layout = ctl("ui", "layout", optional=True)
        return layout["panes"][0] if layout and layout.get("panes") else None

    def backend_pids():
        return {name: json.loads((run / f"{name}.owner.json").read_text())["process"]["pid"]
                for name in ["supervisor", "logic"]}

    def offset(seat):
        screen = ctl("seat", "screen", seat)
        match = re.search(r"TUI OFFSET (\d+)", "\n".join(screen["rows"]))
        return (int(match[1]) if match else None, screen["generation"])

    def saw_wheel(count):
        return log_path.read_text().count("wheel route=") >= count

    try:
        process = start(True)
        before = await_value(pane, "initial pane")
        seat = before["seat"]
        pids = backend_pids()
        (root / "owned-backend-pids.json").write_text(json.dumps(pids))
        assert before["scroll"]["live"], before
        await_value(lambda: saw_wheel(1), "scripted native wheel")
        if mode == "plain":
            after = await_value(lambda: p if (p := pane()) and not p["scroll"]["live"] else None,
                                "plain scrollback offset")
            assert after["scroll"]["top_row"] is not None, after
            print(json.dumps(dict(mode=mode, root=str(root), pids=pids, scroll=after["scroll"], verdict="pass")), flush=True)
            return
        expected = 100 if expect_dead else 97
        position, generation = await_value(lambda: p if (p := offset(seat))[0] == expected else None,
                                           f"{mode} application offset {expected}")
        # The frontend alone dies. The same real PTY, process generation and
        # mode bits must still route the next wheel after a new GUI attachment.
        process.kill()
        assert process.wait(timeout=15) == -9
        process = start(False)
        await_value(pane, "reattached pane")
        await_value(lambda: saw_wheel(2), "reattached scripted native wheel")
        expected_again = 100 if expect_dead else 94
        again, same_generation = await_value(lambda: p if (p := offset(seat))[0] == expected_again else None,
                                             f"reattached {mode} offset {expected_again}")
        assert same_generation == generation and backend_pids() == pids
        current = pane()
        assert current["seat"] == seat and current["scroll"]["live"], current
        print(json.dumps(dict(mode=mode, root=str(root), pids=pids, generation=generation,
                              application_offsets=[100, position, again], scroll=current["scroll"],
                              expected_dead=expect_dead, verdict="pass")), flush=True)
    finally:
        if process is not None:
            try:
                ctl("app", "stop")
            except Exception:
                if process.poll() is None:
                    process.terminate()  # Only the exact frontend this case started.
            process.wait(timeout=35)
        log.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", type=Path)
    parser.add_argument("--expect-dead", action="store_true")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    for mode in ["plain", "mouse", "cursor"]:
        case(binary, mode, args.expect_dead)


if __name__ == "__main__":
    main()
