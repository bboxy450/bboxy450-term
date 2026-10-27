"""Real Codex 0.164+ shared-daemon startup/wake, private HOME, local model only.

Every app action uses ctl. Native hook review is scripted only in this throwaway
identity. No operator daemon, credentials, GUI input or external model is used.
"""
import http.server
import json
import os
from pathlib import Path
import re
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
    binary = Path(sys.argv[1]).resolve()
    cold = "--cold" in sys.argv[2:]
    render_check = "--render-check" in sys.argv[2:]
    codex = shutil.which("codex")
    assert binary.is_file() and codex, "build the app and install Codex first"
    root = Path(tempfile.mkdtemp(prefix="bbt-shared-codex-", dir="/tmp")).resolve()
    print("Private shared-daemon fixture: " + str(root), flush=True)
    home, work, run = (root / name for name in ("home", "work", "run"))
    for directory in [home / ".codex", work, root / "repository"]:
        directory.mkdir(parents=True)
    env = dict(HOME=str(home), CODEX_HOME=str(home / ".codex"),
               CLAUDE_CONFIG_DIR=str(home / ".claude"),
               XDG_CONFIG_HOME=str(home / ".config"), XDG_CACHE_HOME=str(home / ".cache"),
               PATH=os.path.dirname(codex) + ":/usr/bin:/bin:/usr/sbin:/sbin",
               TERM="xterm-256color", LANG="en_US.UTF-8", BBOXY_RUN_DIR=str(run))
    env["BBOXY_CODEX_PREFLIGHT_TRACE"] = "1"
    os.environ.update(BBT=str(binary), E2E_WORK=str(work))
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import first_turn

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

            first_turn.Provider.send_responses(event, body, stage, stem, {})

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
    (home / ".zprofile").write_text("export PATH=" + env["PATH"] + "\n")
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

    def seats():
        return {row["record"]["id"]: row for row in ctl("seat", "ls")["data"]}

    def spawn(label):
        seat = ctl("seat", "spawn", "codex", "--place", "tab", "--cwd", str(work),
                   "--name", "Shared Codex " + label)["data"]["seat"]
        plain = wait(lambda: screen(seat), lambda text: "Hooks need review" in text or
                     "Ask Codex to do anything" in text, "Codex startup")
        if "Hooks need review" in plain:
            assert re.search(r"10 hooks? are new or changed", plain), plain
            assert "2. Trust all and continue" in plain, plain
            ctl("seat", "key", seat, "down", "enter")
        wait(lambda: screen(seat), lambda text: "Ask Codex to do anything" in text,
             "Codex ready after private hook review")
        # Review this fixture's ten private hooks explicitly. A second thread on
        # the same daemon can defer changed-hook review until its first input.
        ctl("seat", "type", seat, "/hooks")
        ctl("seat", "key", seat, "enter")
        plain = wait(lambda: screen(seat), lambda text: "Lifecycle hooks from config" in text,
                     "private hook drawer")
        if "10 hooks need review" in plain:
            assert "t trust all" in plain, plain
            ctl("seat", "type", seat, "t")
            wait(lambda: screen(seat), lambda text: "10 hooks need review" not in text,
                 "private hooks trusted")
        ctl("seat", "key", seat, "escape")
        ctl("seat", "key", seat, "ctrl-u")
        wait(lambda: screen(seat), lambda text: "Ask Codex to do anything" in text,
             "private hook drawer closed")
        ctl("seat", "type", seat, "Introduce yourself and orient first.")
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
        rows = seats()
        threads = [rows[seat]["record"]["harness_session_id"] for seat in (a, b)]
        assert all(threads) and len(set(threads)) == 2, rows
        wait(lambda: seats(), lambda rows: all(rows[seat]["state"]["grade"] == "reported"
                                              for seat in (a, b)), "shared native binding")
        def captured_output(seat):
            chunks = []
            for path in sorted((run / ("seat-" + seat) / "ring-1").glob("data-*.log")):
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
        sent = ctl("message", "send", a, "WAKE_ONLY_A")
        delivered(sent, a)
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
        proof = dict(cold_start=cold, daemon_pid=daemon_pid, seats=[a, b], threads=threads,
                     requests=Provider.requests, wake_operation=sent["op"],
                     daemon_survived_seat_close_and_app_stop=True,
                     external_model_calls=0, operator_input=0)
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
