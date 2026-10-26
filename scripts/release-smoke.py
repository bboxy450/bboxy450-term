"""One real headless pane on an isolated file account; no desktop or login custody."""
import json
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import time

binary = Path(sys.argv[1]).resolve()
binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
root = Path(sys.argv[2]).resolve()
root.mkdir(mode=0o700)
for name in ("home", "repository", "zsh-config"):
    (root / name).mkdir(mode=0o700)
environment = {key: os.environ[key] for key in (
    "PATH", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "TMPDIR"
) if key in os.environ}
environment.update(HOME=str(root / "home"),
                   CLAUDE_CONFIG_DIR=str(root / "home/.claude"),
                   CODEX_HOME=str(root / "home/.codex"),
                   ZDOTDIR=str(root / "zsh-config"))
run = root / "account/run"
calls, pids, failure = [], set(), None
observations = {}
started = False


def ctl(*args):
    argv = [str(binary), "ctl", "--run-dir", str(run), *map(str, args)]
    result = subprocess.run(argv, cwd=root / "repository", env=environment,
                            stdin=subprocess.DEVNULL, capture_output=True,
                            text=True, timeout=60)
    calls.append({"argv": argv, "exit": result.returncode})
    (root / f"ctl-{len(calls):02}.jsonl").write_text(result.stdout)
    (root / f"ctl-{len(calls):02}.stderr").write_text(result.stderr)
    rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    assert result.returncode == 0 and len(rows) == 1 and rows[0].get("ok"), (
        args, result.returncode, f"see ctl-{len(calls):02} logs")
    return rows[0]["data"]


try:
    status = ctl("app", "start", "--headless", "--ctl", "--unattended",
                 "--test-account", root / "account")
    started = True
    pids.add(status["pid"])
    assert status["mode"] == "headless" and status["windows"] == 0
    assert status["custody_backend"] == "file"
    observations.update(mode=status["mode"], windows=status["windows"], custody="file")
    seat = ctl("seat", "spawn", "zsh", "--place", "tab", "--cwd",
               root / "repository", "--name", "Release smoke")["seat"]
    roster = ctl("seat", "ls")
    assert len(roster) == 1 and roster[0]["record"]["id"] == seat
    process = roster[0]["state"]["process"]
    assert process["kind"] == "running"
    pids.add(process["pid"])
    ctl("seat", "type", seat, "printf 'RELEASE_SMOKE_OK\\n'")
    ctl("seat", "key", seat, "enter")
    ctl("seat", "wait", seat, "--text", "(?m)^RELEASE_SMOKE_OK$", "--timeout", "30000")
    for _ in range(100):
        layout = ctl("ui", "layout")
        assert len(layout["panes"]) == 1 and layout["focused"] == seat
        pane = layout["panes"][0]
        shown = pane.get("rendered")
        if shown and "RELEASE_SMOKE_OK" in (shown.get("lines") or []):
            assert layout["frame_seq"] > 0
            observations.update(panes=1, drawn_nonce=True, frame_seq=layout["frame_seq"])
            break
        time.sleep(0.05)
    else:
        raise AssertionError("the pane never drew the shell output")
    processes = ctl("debug", "ps")
    assert not processes["truncated"] and processes["omitted"] == 0
    pids.update(row["pid"] for row in processes["processes"])
    status = ctl("app", "status")
    assert status["mode"] == "headless" and status["windows"] == 0
    assert status["custody_backend"] == "file"
except Exception as error:
    failure = repr(error)
finally:
    if started:
        try:
            ctl("app", "stop")
        except Exception as error:
            failure = failure or f"orderly stop failed: {error!r}"
    for _ in range(100):
        result = subprocess.run(["ps", "-p", ",".join(map(str, sorted(pids))),
                                 "-o", "pid=,lstart=,comm="],
                                capture_output=True, text=True) if pids else None
        if result is None or (result.returncode == 1 and not result.stdout):
            break
        time.sleep(0.1)
    else:
        failure = failure or "recorded app or terminal process survived orderly stop"
    if hashlib.sha256(binary.read_bytes()).hexdigest() != binary_hash:
        failure = failure or "binary changed during smoke"
    report = {"binary": str(binary), "root": str(root), "calls": calls,
              "binary_sha256": binary_hash, "observations": observations,
              "recorded_app_and_pty_pids": sorted(pids), "failure": failure,
              "pid_check": None if result is None else {
                  "argv": result.args, "exit": result.returncode, "stdout": result.stdout},
              "limits": ["No OS presentation, notification or performance claim."]}
    (root / "report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({"root": str(root), "verdict": "fail" if failure else "pass",
                  "failure": failure}))
raise SystemExit(1 if failure else 0)
