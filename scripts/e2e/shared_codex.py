"""Real Codex 0.164+ shared-daemon startup/wake, private HOME, local model only.

Every app action uses ctl. Native hook review is scripted only in this throwaway
identity. No operator daemon, credentials, GUI input or external model is used.
Success retires the private root; failure retains only bounded diagnostics once
private owners have stopped. Unmarked legacy roots are never inferred to be ours.
"""
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import argparse
import http.server
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import signal
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import time
import traceback

_scan_spec = importlib.util.spec_from_file_location('capability_scan', Path(__file__).with_name('capability_scan.py'))
_scan_module = importlib.util.module_from_spec(_scan_spec)
_scan_spec.loader.exec_module(_scan_module)
NativeScanner = _scan_module.NativeScanner


OWNER_TAG = "bboxy450-term/scripts/e2e/shared_codex.py:1"
DIAGNOSTICS = {
    "app.log", "fixture-error.log", "requests.json", "seat-state.json",
    "evidence.json", "message-evidence.jsonl", "identity-observation.json",
    "owned-daemon-updater.json", "owned-logic-fault.json", "render-evidence.json",
    "stream-evidence.json", "color-evidence.json", "cleanup-recovery.json",
    "capability-evidence.json",
}
DIAGNOSTIC_BYTES = 64 * 1024


def pid_alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        pass
    return True


def owner_pids(root):
    """Read only the known private producer records; malformed custody fails closed."""
    paths = [root / "home/.codex/app-server-daemon" / name
             for name in ("daemon.pid", "daemon-updater.pid")]
    paths += sorted((root / "run").glob("*.owner.json"))
    result = set()
    for path in paths:
        if not path.exists():
            continue
        if path.is_symlink() or path.stat().st_size > 1024 * 1024:
            raise ValueError("invalid private owner record: " + str(path))
        record = json.loads(path.read_text())
        if not isinstance(record, dict):
            raise ValueError("invalid private owner record: " + str(path))
        process = record.get("process", {})
        pid = record.get("pid") or (process.get("pid") if isinstance(process, dict) else None)
        if type(pid) is not int or pid <= 1:
            raise ValueError("invalid private owner PID: " + str(path))
        result.add(pid)
    return result


def reclaim_roots(parent):
    """Never infer ownership from a /tmp name alone, and never signal old owners."""
    for root in parent.glob("bbt-shared-codex-*"):
        try:
            if root.is_symlink() or not root.is_dir():
                continue
            info = root.stat()
            marker = root / "cleanup-owner.json"
            if marker.is_symlink() or marker.stat().st_size > 1024 * 1024:
                continue
            record = json.loads(marker.read_text())
            if not isinstance(record, dict):
                continue
            pids = record["owner_pids"]
            if (record.get("script") != OWNER_TAG or record.get("root") != str(root.resolve())
                    or record.get("inode") != info.st_ino or info.st_uid != os.getuid()
                    or not isinstance(pids, list) or not pids
                    or any(type(pid) is not int or pid <= 1 for pid in pids)):
                continue
            if any(pid_alive(pid) for pid in set(pids) | owner_pids(root)):
                continue
            # Concurrent reapers may remove the same dead root; no shared lock.
            if root.stat().st_ino == info.st_ino and not root.is_symlink():
                shutil.rmtree(root)
        except (OSError, ValueError, KeyError, TypeError):
            continue


class FixtureRoot:
    """The runner owns the root from birth through shutdown and bounded retention."""

    def __init__(self, parent=Path("/tmp")):
        reclaim_roots(parent)
        self.root = Path(tempfile.mkdtemp(prefix="bbt-shared-codex-", dir=parent)).resolve()
        self.pids = {os.getpid()}
        self.children = []
        self.closers = []
        self.write_owner()

    def write_owner(self):
        record = dict(script=OWNER_TAG, root=str(self.root), inode=self.root.stat().st_ino,
                      owner_pids=sorted(self.pids))
        pending = self.root / "cleanup-owner.json.new"
        pending.write_text(json.dumps(record) + "\n")
        pending.replace(self.root / "cleanup-owner.json")

    def record(self, pid):
        self.pids.add(pid)
        self.write_owner()

    def track(self, child):
        self.children.append(child)
        self.record(child.pid)
        return child

    def retire(self, child):
        assert child.poll() is not None
        self.children.remove(child)
        self.pids.discard(child.pid)
        self.write_owner()

    def finish(self, succeeded):
        # Popen identity and wait/reap prevent signalling an unrelated reused PID.
        for child in reversed(self.children):
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait(timeout=5)
            self.pids.discard(child.pid)
        for close in self.closers:
            close()
        self.pids.update(owner_pids(self.root))
        self.write_owner()
        live = [pid for pid in self.pids if pid != os.getpid() and pid_alive(pid)]
        if live:
            raise RuntimeError(f"private owners still alive; root retained: {live}: {self.root}")
        if succeeded:
            shutil.rmtree(self.root)
            return
        for entry in self.root.iterdir():
            if entry.name == "cleanup-owner.json":
                continue
            if entry.name in DIAGNOSTICS and entry.is_file() and not entry.is_symlink():
                with entry.open("rb") as log:
                    log.seek(max(0, entry.stat().st_size - DIAGNOSTIC_BYTES))
                    tail = log.read(DIAGNOSTIC_BYTES)
                entry.write_bytes(tail)
            elif entry.is_dir() and not entry.is_symlink():
                shutil.rmtree(entry)
            else:
                entry.unlink()


def wait(read, accept, label, seconds=35):
    deadline = time.monotonic() + seconds
    while True:
        value = read()
        if accept(value):
            return value
        assert time.monotonic() < deadline, (label, value)
        time.sleep(.1)


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)


def objects(value):
    if isinstance(value, dict):
        yield value
        for item in value.values():
            yield from objects(item)
    elif isinstance(value, list):
        for item in value:
            yield from objects(item)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--cold", action="store_true")
    parser.add_argument("--render-check", action="store_true")
    parser.add_argument("--stream-check", action="store_true",
                        help="stream synthetic local text while resizing and sample actual Codex source grids")
    parser.add_argument("--native", action="store_true",
                        help="use the real window/display link and retain this private Codex byte stream")
    parser.add_argument("--color-check", action="store_true",
                        help="prove real Codex foreground colors despite inherited automation settings")
    parser.add_argument("--codex", help="Codex executable (defaults to PATH)")
    parser.add_argument("--shim", action="store_true", help="exercise a private forwarding shim")
    parser.add_argument("--artifacts", type=Path,
                        help="copy frames and stream bytes to an empty caller-owned directory")
    options = parser.parse_args()
    assert not options.native or options.stream_check, "--native requires --stream-check"
    binary = options.binary.resolve()
    codex = shutil.which(options.codex or "codex")
    assert binary.is_file() and codex, "build the app and install Codex first"
    fixture = FixtureRoot()
    try:
        run_fixture(options, binary, codex, fixture)
        if options.artifacts is not None:
            export_artifacts(fixture.root, options.artifacts)
        fixture.finish(True)
    except BaseException:
        (fixture.root / "fixture-error.log").write_text(traceback.format_exc())
        fixture.finish(False)
        raise


def export_artifacts(root, destination):
    if destination.is_symlink():
        raise ValueError("artifact destination must not be a symlink")
    destination = destination.resolve()
    if destination.is_relative_to(root):
        raise ValueError("artifact destination must be outside the disposable fixture")
    if not destination.is_dir() or any(destination.iterdir()):
        raise ValueError("artifact destination must be an existing empty directory")
    for name in ("frames", "codex-stream.bin"):
        if (root / name).is_symlink():
            raise ValueError("unexpected artifact symlink: " + name)
    # The caller owns the destination's lifetime, including partial-copy failure.
    shutil.copytree(root / "frames", destination / "frames")
    shutil.copyfile(root / "codex-stream.bin", destination / "codex-stream.bin")
    print("Shared-daemon artifacts: " + str(destination), flush=True)


def run_fixture(options, binary, codex, fixture):
    root = fixture.root
    capability_scanner = NativeScanner(root, binary)
    cold, render_check = options.cold, options.render_check
    print("Private shared-daemon fixture: " + str(root), flush=True)
    home, work, run = (root / name for name in ("home", "work", "run"))
    for directory in [home / ".codex", work, root / "repository", root / "tmp"]:
        directory.mkdir(parents=True)
    (root / "tmp").chmod(0o700)
    env = dict(HOME=str(home), CODEX_HOME=str(home / ".codex"),
               CLAUDE_CONFIG_DIR=str(home / ".claude"),
               XDG_CONFIG_HOME=str(home / ".config"), XDG_CACHE_HOME=str(home / ".cache"),
               XDG_DATA_HOME=str(home / ".local/share"), TMPDIR=str(root / "tmp") + "/",
               PATH=os.path.dirname(codex) + ":/usr/bin:/bin:/usr/sbin:/sbin",
               TERM="xterm-256color", LANG="en_US.UTF-8", BBOXY_RUN_DIR=str(run))
    env["BBOXY_CODEX_PREFLIGHT_TRACE"] = "1"
    probe = root / "frames"
    if options.native:
        probe.mkdir()
    if options.color_check:
        # Contaminate the app/supervisor parent, never the explicit launch map.
        # The warm private daemon also inherits this, testing TUI-local colors.
        env.update(NO_COLOR="1", COLORTERM="monochrome", CLICOLOR="0")
    shim_log = root / "shim-calls.jsonl"
    if options.shim:
        # Resolve through the login shell's PATH, preserving argv, environment
        # and the harness PID with exec. No third-party shim is required.
        shim = root / "forwarding shim"
        shim.mkdir(mode=0o700)
        driver = shim / "forward.py"
        driver.write_text("import json, os, sys\n"
                          "with open(" + repr(str(shim_log)) + ", 'a') as log:\n"
                          "    log.write(json.dumps(dict(args=sys.argv[1:], home=os.environ['HOME'])) + '\\n')\n"
                          "real = " + repr(codex) + "\n"
                          "os.execv(real, [real, *sys.argv[1:]])\n")
        wrapper = shim / "codex"
        wrapper.write_text("#!/bin/sh\nexec " + shlex.quote(sys.executable) + " -I -B " +
                           shlex.quote(str(driver)) + ' "$@"\n')
        wrapper.chmod(0o700)
        env["PATH"] = str(shim) + ":" + env["PATH"]
        codex = str(wrapper)
    os.environ.update(BBT=str(binary), E2E_WORK=str(work))
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import first_turn

    stream_started, stream_finished = threading.Event(), threading.Event()
    stream_stats = {"deltas": 0}

    class Provider(http.server.BaseHTTPRequestHandler):
        requests = []

        def log_message(self, *_):
            pass

        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            assert self.path.endswith("/responses"), self.path
            text = "\n".join(strings(body))
            labels = [label for label in ("A", "B") if 'NAME: "Shared Codex ' + label + '"' in text]
            if len(labels) != 1:
                observation = dict(name_fragments=re.findall(r'.{0,20}NAME.{0,100}', text),
                                   tool_names=[item.get("name") for item in objects(body.get("tools", [])) if "name" in item])
                (root / "identity-observation.json").write_text(json.dumps(observation, indent=2) + "\n")
            assert len(labels) == 1, "seat identity missing or mixed in native model request"
            label = labels[0]
            assert "ROLE: EXECUTOR" in text
            assert "First call the bboxy MCP `orient` tool, before other work" in text
            if label == "B":
                assert "WAKE_ONLY_A" not in text, "cross-thread wake leak"
            stem = "call_shared_" + label + "_"
            oriented = any(item.get("type") == "function_call_output" and
                           item.get("call_id") == stem + "orient" for item in objects(body))
            wake = "WAKE_ONLY_" + label in text
            stage = "done" if oriented else "orient"
            self.requests.append(dict(seat=label, stage=stage, wake=wake))
            first_turn.MARKER = ("WAKE_ACK_" if wake else "INTRO_OK_") + label
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.end_headers()

            def event(name, **data):
                self.wfile.write(("event: " + name + "\ndata: " +
                                 json.dumps(dict(type=name, **data)) + "\n\n").encode())
                self.wfile.flush()

            streaming = options.stream_check and label == "A" and wake and stage == "done"
            if streaming:
                marker = first_turn.MARKER
                first_turn.MARKER = "\n\n".join(
                    f"STREAM {index:03} " + "W" * 384 for index in range(64)
                ) + "\n\n" + marker

            def stream_event(name, **data):
                if name == "response.output_text.delta":
                    stream_started.set()
                    for fragment in data["delta"].splitlines(keepends=True):
                        event(name, **dict(data, delta=fragment))
                        stream_stats["deltas"] += 1
                        time.sleep(.05)
                else:
                    event(name, **data)
                if name == "response.completed":
                    stream_finished.set()

            first_turn.Provider.send_responses(stream_event if streaming else event,
                                               body, stage, stem, {})

    server = http.server.HTTPServer(("127.0.0.1", 0), Provider)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    def close_server():
        server.shutdown()
        server.server_close()
        worker.join(timeout=3)
        assert not worker.is_alive(), "private model server did not stop"
    fixture.closers.append(close_server)
    config = home / ".codex/config.toml"
    config.write_text('model="fixture-model"\nmodel_provider="fixture"\n'
                      'check_for_update_on_startup=false\napproval_policy="never"\n'
                      '[model_providers.fixture]\nname="Local startup fixture"\n'
                      'base_url="http://127.0.0.1:' + str(server.server_port) + '/v1"\n'
                      'wire_api="responses"\nrequires_openai_auth=false\n'
                      'supports_websockets=false\n[projects.' + json.dumps(str(work)) + ']\n'
                      'trust_level="trusted"\n')
    (home / ".zprofile").write_text("export PATH=" + shlex.quote(env["PATH"]) + "\n")
    (home / ".zshrc").write_text("PROMPT='%# '\n")
    app = None
    daemon_started = False
    updater = None
    log = None

    def command(args, timeout=35, check=True):
        child = fixture.track(subprocess.Popen(args, env=env, cwd=work, text=True,
                                              stdout=subprocess.PIPE, stderr=subprocess.PIPE))
        try:
            stdout, stderr = child.communicate(timeout=timeout)
        except BaseException:
            child.kill()
            child.wait()
            raise
        finally:
            fixture.retire(child)
        result = subprocess.CompletedProcess(args, child.returncode, stdout, stderr)
        assert not check or result.returncode == 0, (args[1:4], result.returncode,
                                                    result.stdout[-2000:], result.stderr[-2000:])
        return result

    def ctl(*args, check=True):
        result = command([str(binary), "ctl", *args], check=check)
        reply = json.loads(result.stdout) if result.stdout.strip() else {}
        if args[:2] in (("message", "send"), ("message", "receipts")):
            with (root / "message-evidence.jsonl").open("a") as log:
                log.write(json.dumps(reply) + "\n")
        return reply

    def screen(seat):
        return command([str(binary), "ctl", "seat", "screen", seat, "--human"]).stdout

    color_observations = {}

    def colored_screen(seat):
        snapshot = ctl("seat", "screen", seat, "--cells")["data"]
        runs = [run for row in snapshot["cells"] for run in row]
        # Default foreground brightness is not proof of chromatic SGR output.
        defaults = [{"Named": name} for name in
                    ("Foreground", "BrightForeground", "DimForeground")]
        colored = [run for run in runs if run["fg"] not in defaults and run["text"].strip()]
        observation = dict(generation=snapshot["generation"], sequence=snapshot["sequence"],
                           styled_runs=len(runs), nondefault_foreground_runs=len(colored),
                           nonblank_foreground_chars=sum(sum(not char.isspace() for char in run["text"])
                                                         for run in colored),
                           foregrounds=sorted({json.dumps(run["fg"], sort_keys=True) for run in colored}))
        color_observations[seat] = observation
        (root / "color-evidence.json").write_text(json.dumps(dict(
            parent={key: env[key] for key in ("NO_COLOR", "COLORTERM", "CLICOLOR")},
            seats=color_observations), indent=2) + "\n")
        return observation

    def seats():
        return {row["record"]["id"]: row for row in ctl("seat", "ls")["data"]}

    def ready(seat):
        deadline = time.monotonic() + 35

        def observe(accept, label):
            return wait(lambda: screen(seat), accept, label,
                        seconds=max(0, deadline - time.monotonic()))

        # Both client startup and the per-thread overlay can request review.
        # Approve only this throwaway HOME's hooks, through the native chooser.
        while True:
            plain = observe(lambda text: "Hooks need review" in text or
                            ("Ask Codex to do anything" in text and
                             "fixture-model" in text and "loading" not in text) or
                            "Lifecycle hooks from config" in text, "Codex startup")
            if "Hooks need review" in plain:
                count = re.search(r"(\d+) hooks? are new or changed", plain)
                assert count and int(count[1]) > 0, plain
                selected = re.search(r"› ([123])\.", plain)
                assert selected and "2. Trust all and continue" in plain, plain
                if selected[1] != "2":
                    ctl("seat", "key", seat, "down" if selected[1] == "1" else "up")
                    observe(lambda text: "› 2. Trust all and continue" in text,
                            "private hook review selects trust")
                ctl("seat", "key", seat, "enter")
                observe(lambda text: "Hooks need review" not in text, "private hook chooser closed")
                continue
            if "Lifecycle hooks from config" in plain:
                break  # A deferred /hooks command can open after the chooser.
            ctl("seat", "key", seat, "ctrl-u")
            ctl("seat", "type", seat, "/hooks")
            plain = observe(lambda text: "› /hooks" in text or "Hooks need review" in text,
                            "private hook command is rendered")
            if "Hooks need review" in plain:
                continue
            ctl("seat", "key", seat, "enter")
            plain = observe(lambda text: "Lifecycle hooks from config" in text or "Hooks need review" in text,
                            "private hook drawer")
            if "Hooks need review" not in plain:
                break
        if re.search(r"\d+ hooks need review", plain):
            assert "t trust all" in plain, plain
            ctl("seat", "type", seat, "t")
            observe(lambda text: not re.search(r"\d+ hooks need review", text), "private hooks trusted")
        ctl("seat", "key", seat, "escape")
        observe(lambda text: "Ask Codex to do anything" in text and "fixture-model" in text and
                "Hooks need review" not in text and "Lifecycle hooks from config" not in text,
                "private hook drawer closed")
        ctl("seat", "key", seat, "ctrl-u")

    def spawn(label):
        seat = ctl("seat", "spawn", "codex", "--place", "tab", "--cwd", str(work),
                   "--name", "Shared Codex " + label)["data"]["seat"]
        ready(seat)
        ctl("seat", "type", seat, "Introduce yourself and orient first.")
        wait(lambda: screen(seat), lambda text: "Introduce yourself and orient first." in text,
             "private first prompt is rendered")
        ctl("seat", "key", seat, "enter")
        wait(lambda: screen(seat), lambda text: "INTRO_OK_" + label in text, "orient first turn")
        return seat

    def record_updater():
        nonlocal updater
        path = home / ".codex/app-server-daemon/daemon-updater.pid"
        if path.exists():
            updater = json.loads(path.read_text())
            (root / "owned-daemon-updater.json").write_text(json.dumps(updater) + "\n")
            fixture.record(updater["pid"])

    def delivered(sent, seat):
        message = sent["data"]["message"]["envelope"]["id"]
        def queued(reply):
            return any(row.get("receipt", row).get("recipient") == seat and
                       row.get("receipt", row).get("evidence") == "native-queued"
                       for row in reply["data"])
        return wait(lambda: ctl("message", "receipts", message), queued, "native queue admission")

    try:
        daemon_started = True
        if not cold:
            started = json.loads(command([codex, "app-server", "daemon", "start"]).stdout)
            assert started["status"] == "started", started
            daemon_pid = started["pid"]
            fixture.record(daemon_pid)
            record_updater()
        log = (root / "app.log").open("w")
        app = fixture.track(subprocess.Popen([str(binary), *(["--native-frame-probe", str(probe)] if options.native else ["--headless"]),
                                "--settings-config", str(root / "settings.toml"), "--ctl", "--unattended", "--test-account",
                                str(root / "account"), "--run-dir", str(run), "--repository",
                                str(root / "repository")], env=env, cwd=work,
                               stdin=subprocess.DEVNULL, stdout=log, stderr=log))
        wait(lambda: ctl("app", "status", check=False),
             lambda value: value.get("data", {}).get("backend_ready") is True and
             value.get("data", {}).get("gui_ready") is True, "headless app ready")
        a = spawn("A")
        if cold:
            started = json.loads(command([codex, "app-server", "daemon", "start"]).stdout)
            assert started["status"] != "started", "the launch failed to start its daemon"
            daemon_pid = json.loads((home / ".codex/app-server-daemon/daemon.pid").read_text())["pid"]
            fixture.record(daemon_pid)
            record_updater()
        b = spawn("B")
        if options.color_check:
            for seat in (a, b):
                wait(lambda: colored_screen(seat),
                     lambda row: row["nonblank_foreground_chars"] > 0,
                     "real Codex emits nondefault foreground colors: " + seat, seconds=10)
            print("Color observations: " + json.dumps(color_observations), flush=True)
        rows = seats()
        threads = [rows[seat]["record"]["harness_session_id"] for seat in (a, b)]
        assert all(threads) and len(set(threads)) == 2, rows
        wait(lambda: seats(), lambda rows: all(rows[seat]["state"]["grade"] == "reported"
                                              for seat in (a, b)), "shared native binding")
        before = seats()
        capability_scanner.scan(daemon_pid, app.pid, 'two-live-seats')
        ctl("seat", "restart", a)
        wait(lambda: seats(), lambda rows: rows[a]["record"]["generation"] == before[a]["record"]["generation"] + 1,
             "seat restarts its generation")
        ready(a)
        after = seats()
        assert after[a]["record"]["harness_session_id"] == threads[0], after[a]
        assert after[b]["state"]["process"] == before[b]["state"]["process"], after[b]
        assert after[b]["record"]["generation"] == before[b]["record"]["generation"]
        os.kill(daemon_pid, 0)
        capability_scanner.scan(daemon_pid, app.pid, 'after-generation-replacement')
        (root / 'capability-evidence.json').write_text(json.dumps(capability_scanner.observations) + '\n')
        print('Native capability environments: ' + json.dumps(capability_scanner.observations), flush=True)
        def captured_output(seat):
            chunks = []
            generation = after[seat]["record"]["generation"]
            for path in sorted((run / ("seat-" + seat) / ("ring-" + str(generation))).glob("data-*.log")):
                data, offset = path.read_bytes(), 0
                while offset + 56 <= len(data) and data[offset:offset + 4] == b"BBR1":
                    kind, length = struct.unpack_from("<HI", data, offset + 6)
                    if offset + 56 + length > len(data):
                        break
                    if kind == 1:
                        chunks.append(data[offset + 56:offset + 56 + length])
                    offset += 56 + length
            return b"".join(chunks)
        if render_check:
            # Let the final first-turn presentation settle before measuring idle.
            time.sleep(1)
            baseline = {seat: len(captured_output(seat)) for seat in (a, b)}
            time.sleep(5)
            observations = {}
            for seat in (a, b):
                output = captured_output(seat)[baseline[seat]:]
                observations[seat] = dict(reconnects=output.count(b"Reconnecting to server"),
                                          clears=output.count(b"\x1b[1;1H\x1b[J"))
            (root / "render-evidence.json").write_text(json.dumps(observations, indent=2) + "\n")
            print("Rendering observations: " + json.dumps(observations), flush=True)
            assert all(row["reconnects"] == 0 and row["clears"] == 0
                       for row in observations.values()), observations
            for size in ("159x58", "159x59", "159x27", "159x58"):
                ctl("app", "resize", size)
            before = seats()
            owner = run / "logic.owner.json"
            ownership = json.loads(owner.read_text())
            assert Path(ownership["run"]).resolve() == run.resolve()
            assert Path(ownership["account"]).resolve() == (root / "account").resolve()
            logic_pid = ownership["process"]["pid"]
            fingerprint = subprocess.check_output(
                ["/bin/ps", "-ww", "-p", str(logic_pid), "-o", "lstart=,args="], text=True)
            parent = int(subprocess.check_output(
                ["/bin/ps", "-p", str(logic_pid), "-o", "ppid="], text=True))
            assert str(binary) in fingerprint and parent == app.pid, \
                "fault target is not this fixture's logic"
            (root / "owned-logic-fault.json").write_text(json.dumps(
                dict(pid=logic_pid, fingerprint=fingerprint)) + "\n")
            assert json.loads(owner.read_text()) == ownership
            assert subprocess.check_output(
                ["/bin/ps", "-ww", "-p", str(logic_pid), "-o", "lstart=,args="], text=True) == fingerprint
            os.kill(logic_pid, signal.SIGKILL)
            def recovered():
                try:
                    pid = json.loads(owner.read_text())["process"]["pid"]
                    status = ctl("app", "status", check=False)
                    return pid != logic_pid and status.get("data", {}).get("backend_ready") is True
                except (OSError, ValueError):
                    return False
            wait(recovered, bool, "logic recovers resized checkpoints")
            after = seats()
            for seat in (a, b):
                assert before[seat]["state"]["process"] == after[seat]["state"]["process"]
                assert before[seat]["record"]["generation"] == after[seat]["record"]["generation"]
        if options.stream_check:
            focused = ctl("ui", "focus", a)["data"]
            assert focused["focused"] == a, focused
        sent = ctl("message", "send", a, "WAKE_ONLY_A")
        delivered(sent, a)
        if options.stream_check:
            assert stream_started.wait(20), "local Codex text stream never started"
            samples = []
            sizes = ("159x58", "91x31", "180x43", "80x24", "137x39", "159x58")
            for index in range(24):
                if stream_finished.is_set():
                    break
                if options.native and index % 6 == 0:
                    ctl("ui", "key", "cmd-equal" if index % 12 == 0 else "cmd-minus")
                ctl("app", "resize", sizes[index % len(sizes)])
                snapshot = ctl("seat", "screen", a)["data"]
                ink = [row for row in snapshot["rows"] if "W" * 8 in row]
                samples.append(dict(cols=snapshot["cols"], rows=len(snapshot["rows"]),
                                    sequence=snapshot["sequence"], ink_rows=len(ink),
                                    # Character offsets are diagnostic, not a glyph-width oracle.
                                    max_w_end=max((row.rfind("W") + 1 for row in ink), default=None)))
                time.sleep(.05)
            assert stream_finished.wait(20), "local Codex text stream did not finish"
            stream_stats["samples"] = samples
            stream_stats["basis"] = ("real private Codex local SSE; native frame bands await independent native_strip.py analysis; no physical scanout claim"
                                     if options.native else "real private Codex 0.164 local SSE; source grids only, no forced capture or per-present GPU/scanout claim")
            (root / "stream-evidence.json").write_text(json.dumps(stream_stats, indent=2) + "\n")
            print("Streaming observations: " + json.dumps(stream_stats), flush=True)
            assert stream_stats["deltas"] >= 64, stream_stats
            assert len(samples) >= 4 and sum(row["ink_rows"] > 0 for row in samples) >= 4, stream_stats
            assert len({(row["cols"], row["rows"]) for row in samples}) >= 3, stream_stats
        wait(lambda: screen(a), lambda text: "WAKE_ACK_A" in text, "native shared-daemon wake")
        if options.native:
            # Capture persistence can lag the live replica; never accept a
            # startup-query prefix as a streaming fixture. Read only complete
            # records and wait for discriminating real output with closed sync
            # batches. The source screen above proves the final ack; raw delta
            # paints may spell it as a reused W plus a new "AKE_ACK_A" suffix.
            recorded = wait(lambda: captured_output(a),
                            lambda data: data.count(b"W") >= 8192 and
                            data.count(b"\x1b[?2026h") >= 20 and
                            data.count(b"\x1b[?2026h") == data.count(b"\x1b[?2026l"),
                            "real Codex bytes persisted for replay")
            (root / "codex-stream.bin").write_bytes(recorded)
            ctl("seat", "capture", a, str(root / "native.png"))
        assert "WAKE_ONLY_A" not in screen(b)
        assert not any(request["seat"] == "B" and request["wake"] for request in Provider.requests)
        ctl("seat", "close", a)
        os.kill(daemon_pid, 0)
        delivered(ctl("message", "send", b, "WAKE_ONLY_B"), b)
        wait(lambda: screen(b), lambda text: "WAKE_ACK_B" in text, "peer survives seat close")
        ctl("app", "stop")
        app.wait(timeout=15)
        os.kill(daemon_pid, 0)
        if options.shim:
            calls = [json.loads(line) for line in shim_log.read_text().splitlines()]
            assert all(call["home"] == str(home) for call in calls), calls
            assert any("--remote" in call["args"] for call in calls), calls
            assert any("resume" in call["args"] for call in calls), calls
        proof = dict(cold_start=cold, daemon_pid=daemon_pid, seats=[a, b], threads=threads,
                     forwarding_shim=options.shim,
                     seat_restart_resumed=True,
                     requests=Provider.requests, wake_operation=sent["op"],
                     daemon_survived_seat_close_and_app_stop=True,
                     external_model_calls=0, operator_input=0)
        if options.stream_check:
            proof["stream_observations"] = stream_stats
        if options.color_check:
            proof["color_observations"] = color_observations
        (root / "evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
        print(json.dumps(proof), flush=True)
    finally:
        def stop_app():
            if app is not None and app.poll() is None:
                (root / "requests.json").write_text(json.dumps(Provider.requests, indent=2) + "\n")
                (root / "seat-state.json").write_text(json.dumps(ctl("seat", "ls", check=False), indent=2) + "\n")
                ctl("app", "stop", check=False)
                try:
                    app.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    app.terminate()
                    app.wait(timeout=5)
            if app is not None and (run / "logic.owner.json").exists():
                recovery = ctl("app", "start", "--headless", "--ctl", "--unattended",
                               "--test-account", str(root / "account"))
                (root / "cleanup-recovery.json").write_text(json.dumps(recovery) + "\n")
                ctl("app", "stop")

        def stop_daemon():
            if daemon_started:
                if updater is None:
                    record_updater()
                command([codex, "app-server", "daemon", "stop"])

        def stop_updater():
            if updater is not None:
                # This PID was recorded from the manager started in this private HOME.
                current = subprocess.run(["/bin/ps", "-ww", "-p", str(updater["pid"]), "-o", "lstart=", "-o", "command="],
                                         capture_output=True, text=True).stdout.strip()
                if current:
                    assert current.startswith(updater["processStartTime"]), current
                    assert str(home / ".codex/packages/app-server-daemon/") in current and "pid-update-loop" in current
                    os.kill(updater["pid"], signal.SIGTERM)
                    wait(lambda: pid_alive(updater["pid"]), lambda live: not live,
                         "private updater exit", seconds=5)

        failures = []
        for stop in (stop_app, stop_daemon, stop_updater):
            try:
                stop()
            except Exception:
                failures.append(traceback.format_exc())
        if log is not None:
            log.close()
        if failures:
            raise RuntimeError("private shutdown failed:\n" + "\n".join(failures))


if __name__ == "__main__":
    main()
