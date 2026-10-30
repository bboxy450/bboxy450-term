"""Real Codex 0.164+ shared-daemon startup/wake, private HOME, local model only.

Every app action uses ctl. Native hook review is scripted only in this throwaway
identity. No operator daemon, credentials, GUI input or external model is used.
"""
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import argparse
import http.server
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
    parser.add_argument("--color-check", action="store_true",
                        help="prove real Codex foreground colors despite inherited automation settings")
    parser.add_argument("--codex", help="Codex executable (defaults to PATH)")
    parser.add_argument("--shim", action="store_true", help="exercise a private forwarding shim")
    options = parser.parse_args()
    binary = options.binary.resolve()
    cold, render_check = options.cold, options.render_check
    codex = shutil.which(options.codex or "codex")
    assert binary.is_file() and codex, "build the app and install Codex first"
    root = Path(tempfile.mkdtemp(prefix="bbt-shared-codex-", dir="/tmp")).resolve()
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
    succeeded = False

    def command(args, timeout=35, check=True):
        result = subprocess.run(args, env=env, cwd=work, text=True,
                                capture_output=True, timeout=timeout)
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
            record_updater()
        log = (root / "app.log").open("w")
        app = subprocess.Popen([str(binary), "--headless", "--ctl", "--unattended", "--test-account",
                                str(root / "account"), "--run-dir", str(run), "--repository",
                                str(root / "repository")], env=env, cwd=work,
                               stdin=subprocess.DEVNULL, stdout=log, stderr=log)
        wait(lambda: ctl("app", "status", check=False),
             lambda value: value.get("data", {}).get("backend_ready") is True and
             value.get("data", {}).get("gui_ready") is True, "headless app ready")
        a = spawn("A")
        if cold:
            started = json.loads(command([codex, "app-server", "daemon", "start"]).stdout)
            assert started["status"] != "started", "the launch failed to start its daemon"
            daemon_pid = json.loads((home / ".codex/app-server-daemon/daemon.pid").read_text())["pid"]
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
        ctl("seat", "restart", a)
        wait(lambda: seats(), lambda rows: rows[a]["record"]["generation"] == before[a]["record"]["generation"] + 1,
             "seat restarts its generation")
        ready(a)
        after = seats()
        assert after[a]["record"]["harness_session_id"] == threads[0], after[a]
        assert after[b]["state"]["process"] == before[b]["state"]["process"], after[b]
        assert after[b]["record"]["generation"] == before[b]["record"]["generation"]
        os.kill(daemon_pid, 0)
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
            stream_stats["basis"] = "real private Codex 0.164 local SSE; source grids only, no forced capture or per-present GPU/scanout claim"
            (root / "stream-evidence.json").write_text(json.dumps(stream_stats, indent=2) + "\n")
            print("Streaming observations: " + json.dumps(stream_stats), flush=True)
            assert stream_stats["deltas"] >= 64, stream_stats
            assert len(samples) >= 4 and sum(row["ink_rows"] > 0 for row in samples) >= 4, stream_stats
            assert len({(row["cols"], row["rows"]) for row in samples}) >= 3, stream_stats
        wait(lambda: screen(a), lambda text: "WAKE_ACK_A" in text, "native shared-daemon wake")
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
        succeeded = True
    finally:
        if app is not None and app.poll() is None:
            (root / "requests.json").write_text(json.dumps(Provider.requests, indent=2) + "\n")
            (root / "seat-state.json").write_text(json.dumps(ctl("seat", "ls", check=False), indent=2) + "\n")
            ctl("app", "stop", check=False)
            try:
                app.wait(timeout=10)
            except subprocess.TimeoutExpired:
                app.terminate()
                app.wait(timeout=5)
        if daemon_started:
            if updater is None:
                record_updater()
            command([codex, "app-server", "daemon", "stop"])
        if updater is not None:
            # This PID was recorded from the manager we started in our new HOME.
            current = subprocess.run(["/bin/ps", "-ww", "-p", str(updater["pid"]), "-o", "lstart=", "-o", "command="],
                                     capture_output=True, text=True).stdout.strip()
            if current:
                assert current.startswith(updater["processStartTime"]), current
                assert str(home / ".codex/packages/app-server-daemon/") in current and "pid-update-loop" in current
                os.kill(updater["pid"], signal.SIGTERM)
        server.shutdown()
        server.server_close()
        worker.join(timeout=3)
        # Keep diagnostics/evidence; retire only this fixture's large package copy.
        if succeeded:
            shutil.rmtree(home / ".codex/packages", ignore_errors=True)


if __name__ == "__main__":
    main()
