"""Installed clients consume L3 wiring; model responses are scripted on loopback.

These assertions prove client integration, not a remote model's judgement.
The provider stores schema names/digests only, never headers or message bodies.
"""
import hashlib
import http.server
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import threading
import time
import tomllib

from task_checks import ctl, seats


WORK = Path(os.environ["E2E_WORK"])
HOME = WORK.parent / "home"
MARKER = "L3_NATIVE_OK"
SKILL = (Path(__file__).resolve().parents[2] /
         "crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md").read_text().replace(
             "{{ROLE_NAME}}", json.dumps("executor")).replace(
             "{{ROLE_CLASS}}", json.dumps("worker"))


def snapshot():
    result = {}
    for path in HOME.rglob("*"):
        if not path.is_file():
            continue
        try:
            data = path.read_bytes()
        except FileNotFoundError:
            # Native caches use temporary files. Name that observation instead
            # of mistaking a removed cache for a changed configuration byte.
            result[str(path.relative_to(HOME))] = "vanished-during-snapshot"
        else:
            result[str(path.relative_to(HOME))] = hashlib.sha256(data).hexdigest()
    return result


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)


class Provider(http.server.BaseHTTPRequestHandler):
    requests = []

    def log_message(self, *_):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path.endswith("/count_tokens"):
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"input_tokens":64}')
            return
        kind = "codex" if self.path.endswith("/responses") else "claude"
        assert self.path.endswith("/responses") or "/messages" in self.path
        tools = body.get("tools", [])
        self.requests.append(dict(
            kind=kind, path=self.path, model=body.get("model"),
            tools=tools, skill_in_request=any(SKILL in s for s in strings(body)),
            request_sha256=hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()))
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()

        def event(name, **data):
            self.wfile.write(("event: " + name + "\ndata: " +
                             json.dumps(dict(type=name, **data)) + "\n\n").encode())
            self.wfile.flush()

        if kind == "claude":
            event("message_start", message=dict(
                id="msg_l3", type="message", role="assistant", model=body["model"],
                content=[], stop_reason=None, stop_sequence=None,
                usage=dict(input_tokens=64, output_tokens=0)))
            event("content_block_start", index=0, content_block=dict(type="text", text=""))
            event("content_block_delta", index=0,
                  delta=dict(type="text_delta", text=MARKER))
            event("content_block_stop", index=0)
            event("message_delta", delta=dict(stop_reason="end_turn", stop_sequence=None),
                  usage=dict(output_tokens=4))
            event("message_stop")
        else:
            part = dict(type="output_text", text=MARKER, annotations=[])
            item = dict(id="msg_l3", type="message", status="completed",
                        role="assistant", content=[part])
            response = dict(id="resp_l3", object="response", status="completed",
                            created_at=int(time.time()), model=body["model"], output=[item],
                            usage=dict(input_tokens=64, output_tokens=4, total_tokens=68))
            event("response.created", response=dict(response, status="in_progress", output=[]))
            event("response.output_item.added", output_index=0,
                  item=dict(item, status="in_progress", content=[]))
            event("response.content_part.added", item_id=item["id"], output_index=0,
                  content_index=0, part=dict(part, text=""))
            event("response.output_text.delta", item_id=item["id"], output_index=0,
                  content_index=0, delta=MARKER)
            event("response.output_text.done", item_id=item["id"], output_index=0,
                  content_index=0, text=MARKER)
            event("response.content_part.done", item_id=item["id"], output_index=0,
                  content_index=0, part=part)
            event("response.output_item.done", output_index=0, item=item)
            event("response.completed", response=response)


def until(read, ready, description):
    deadline = time.monotonic() + 30
    while True:
        value = read()
        if ready(value):
            return value
        assert time.monotonic() < deadline, (description, value)
        time.sleep(.05)


def native_claude():
    seat = ctl("seat", "spawn", "claude", "--role", "executor", "--place", "tab",
               "--cwd", str(WORK), "--name", "L3 native Claude")['data']['seat']
    path = WORK / (seat + ".native.jsonl")

    def rows():
        if not path.exists():
            return []
        return [json.loads(line) for line in path.read_text().splitlines()
                if line.startswith("{") and line.endswith("}")]

    result = until(rows, lambda xs: any(x.get("type") == "result" for x in xs),
                   "native Claude result")
    init = next(x for x in result if x.get("subtype") == "init")
    assert {"name": "bboxy", "status": "connected", "source": "dynamic"} in init["mcp_servers"], init
    plugin = next(x for x in init["plugins"] if x["name"] == "bboxy450")
    assert Path(plugin["path"]).is_relative_to(WORK.parent / "run"), plugin
    assert "bboxy450:bboxy450" in init["skills"], init
    hook = next(x for x in result if x.get("subtype") == "hook_response")
    assert hook.get("exit_code") == 0 and hook.get("outcome") == "success", hook
    done = next(x for x in result if x.get("type") == "result")
    assert not done["is_error"] and done["result"] == MARKER and done["num_turns"] == 1, done
    return dict(seat=seat, mcp=init["mcp_servers"], skill="bboxy450:bboxy450",
                plugin=plugin, hook=hook, turns=done["num_turns"])


def approve_codex_hooks(seat, marker=MARKER, prefix="l3"):
    ctl("seat", "wait", seat, "--text", "Hooks need review|Ask Codex to do anything|" + marker,
        "--timeout", "30000")
    screen = ctl("seat", "screen", seat)["data"]
    approved = False
    if "Hooks need review" in json.dumps(screen):
        expected = subprocess.run([os.environ["BBT"], "hook", "--list", "codex"],
                                  text=True, capture_output=True, check=True).stdout.splitlines()
        plain = subprocess.run([os.environ["BBT"], "ctl", "seat", "screen", seat, "--human"],
                               text=True, capture_output=True, check=True).stdout
        count = re.search(r"(\d+) hooks? are new or changed", plain)
        assert count and int(count[1]) == len(expected) == 10, (plain, expected)
        assert "2. Trust all and continue" in plain, plain
        assert not (WORK / ".codex").exists(), "unexpected repository hook source"
        (WORK / (prefix + "-codex-hook-review.txt")).write_text(plain)
        (WORK / (prefix + "-codex-expected-hooks.jsonl")).write_text("\n".join(expected) + "\n")
        ctl("seat", "key", seat, "down", "enter")
        approved = True
    return approved


def native_codex():
    seat = ctl("seat", "spawn", "codex", "--role", "executor", "--place", "tab",
               "--cwd", str(WORK), "--name", "L3 native Codex")['data']['seat']
    approved = approve_codex_hooks(seat)
    screen = until(lambda: ctl("seat", "screen", seat)["data"],
                   lambda x: json.dumps(x).count(MARKER) >= 2, "native Codex result")
    # The native runtime's Reported observation outranks a Shim observation;
    # a completed real turn must not wait for that stronger evidence to decay.
    state = until(lambda: seats()[seat]["state"],
                  lambda x: x["grade"] in ("shim", "reported"),
                  "native Codex observation")
    return dict(seat=seat, screen_sequence=screen["sequence"],
                published_grade=state["grade"], approved_app_hooks=approved)


def native_writes(before, after, config_before, codex):
    """Keep every seeded byte except the native client's named, approved writes."""
    config_path = HOME / ".codex/config.toml"
    config = tomllib.loads(config_path.read_text())
    hooks = config.pop("hooks", {})
    assert set(hooks) == {"state"}, "native Codex added unrelated hook configuration"
    states = hooks["state"]
    names = [json.loads(line)["event"] for line in
             (WORK / "l3-codex-expected-hooks.jsonl").read_text().splitlines()]
    expected = {"/<session-flags>/config.toml:" +
                re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower() + ":0:0" for name in names}
    assert codex["approved_app_hooks"] and len(expected) == 10 and set(states) == expected, states
    assert all(set(value) == {"trusted_hash"} and
               re.fullmatch(r"sha256:[a-f0-9]{64}", value["trusted_hash"])
               for value in states.values()), states
    tui = config.get("tui", {})
    if "screen_reader_detection_done" in tui:
        assert tui.pop("screen_reader_detection_done") is True
        if not tui:
            config.pop("tui")
    assert config == config_before, "native Codex changed unrelated configuration"
    delta = {key: dict(before=before.get(key), after=after.get(key))
             for key in sorted(set(before) | set(after)) if before.get(key) != after.get(key)}
    for key in before:
        if key != ".codex/config.toml":
            assert after.get(key) == before[key], ("seeded config changed", key)
    causes = {}
    for key in delta:
        if key == ".codex/config.toml":
            cause = "Codex native approved app-hook trust (#270) and TUI initialization"
        elif key.startswith(".codex/skills/.system/"):
            cause = "Codex bundled system-skill installation"
        elif key.startswith((".codex/.tmp/", ".codex/tmp/")):
            cause = "Codex native plugin/helper cache and locks"
        elif key.startswith((".codex/sessions/", ".codex/shell_snapshots/")) or key == ".codex/history.jsonl":
            cause = "Codex native session history"
        elif key.startswith((".codex/thread-writer-locks/", ".codex/tui-thread-reference-capabilities/",
                             ".codex/app-server-control/")):
            cause = "Codex native runtime locks and handles"
        elif key.startswith((".codex/packages/app-server-daemon/", ".codex/app-server-daemon/")):
            cause = "Codex's own managed shared daemon package, lifecycle and logs"
        elif key.startswith(".codex/") and (".sqlite" in key or
                key in (".codex/installation_id", ".codex/.sandbox_migration")):
            cause = "Codex native runtime database and initialization metadata"
        elif key == ".claude/.claude.json" or key.startswith(".claude/backups/"):
            cause = "Claude native configuration/session metadata and backup"
        elif key.startswith(".claude/projects/"):
            cause = "Claude native session history"
        elif key.startswith("Library/Caches/claude-cli-nodejs/"):
            cause = "Claude native MCP cache log"
        else:
            raise AssertionError(("unattributed native write", key))
        delta[key]["cause"] = cause
        causes.setdefault(cause, []).append(key)
    return delta, causes


def shell():
    user = WORK.parent / "zsh-config"
    user_rc = user / ".zshrc"
    user_rc.write_text("export L3_ZSH_USER_CONFIG=preserved\n")
    before = hashlib.sha256(user_rc.read_bytes()).hexdigest()
    script = WORK / "l3-shell-probe.sh"
    output = WORK / "l3-shell.txt"
    script.write_text("""[[ ${preexec_functions[(Ie)_bbt_preexec]} -gt 0 ]] || return 1
[[ ${precmd_functions[(Ie)_bbt_precmd]} -gt 0 ]] || return 1
printf '%s\\n' "$ZDOTDIR" "$BBOXY_USER_ZDOTDIR" "$BBOXY_SEAT_ID" "$L3_ZSH_USER_CONFIG" > """ +
                      shlex.quote(str(output)) + "\nprintf 'L3_SHELL_READY\\n'\n")
    seat = ctl("seat", "spawn", "zsh", "--place", "tab", "--cwd", str(WORK),
               "--name", "L3 shell integration")["data"]["seat"]
    ctl("seat", "type", seat, "source " + shlex.quote(str(script)))
    ctl("seat", "key", seat, "enter")
    ctl("seat", "wait", seat, "--text", "(?m)^L3_SHELL_READY$", "--timeout", "20000")
    overlay, actual_user, actual_seat, loaded = output.read_text().splitlines()
    overlay = Path(overlay)
    assert overlay.is_relative_to(WORK.parent / "run"), overlay
    assert Path(actual_user) == user and actual_seat == seat and loaded == "preserved"
    source = (Path(__file__).resolve().parents[2] /
              "crates/bboxy450-harness-wake-up/src/launch.rs").read_text()
    expected = source.split('const ZSH_RC: &str = r#"', 1)[1].split('"#;', 1)[0]
    assert (overlay / ".zshrc").read_text() == expected, "shell hook template changed"
    assert hashlib.sha256(user_rc.read_bytes()).hexdigest() == before, "user zshrc changed"
    proof = dict(seat=seat, overlay=str(overlay), user_zdotdir=str(user),
                 user_rc_sha256=before, shell_hooks=["_bbt_preexec", "_bbt_precmd"])
    (WORK / "l3-shell-evidence.json").write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(dict(kind="L3-shell", **proof)), flush=True)


def configure_codex_provider(port):
    # Test preparation, before the native-write baseline. A shared daemon reads
    # its provider from this throwaway home, independently of the TUI's flags.
    assert (HOME / ".l3-private-home").is_file()
    path = HOME / ".codex/config.toml"
    text = path.read_text()
    assert 'http://127.0.0.1:9/v1' in text
    path.write_text(text.replace('http://127.0.0.1:9/v1', 'http://127.0.0.1:' + str(port) + '/v1'))


def run():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Provider)
    configure_codex_provider(server.server_port)
    before = snapshot()
    config_before = tomllib.loads((HOME / ".codex/config.toml").read_text())
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    config = WORK / "l3-native-provider.json"
    config.write_text(json.dumps(dict(port=server.server_port)))
    try:
        claude = native_claude()
        codex = native_codex()
        requests = Provider.requests
        for kind in ("claude", "codex"):
            request = next(x for x in requests if x["kind"] == kind)
            encoded = json.dumps(request["tools"])
            assert "bboxy" in encoded and "orient" in encoded, request
            if kind == "codex":
                assert request["skill_in_request"], "Codex did not receive pinned skill text"
        after = snapshot()
        delta, causes = native_writes(before, after, config_before, codex)
        evidence = dict(harness="installed clients", model="scripted localhost",
                        claude=claude, codex=codex, requests=requests, config_delta=delta,
                        app_home_writes=[], native_writes_by_cause=causes)
        (WORK / "l3-native-evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
        print(json.dumps(dict(kind="L3-native", harness="installed clients",
                             model="scripted localhost", claude=claude, codex=codex,
                             changed_files=len(delta),
                             config_delta_sha256=hashlib.sha256(
                                 json.dumps(delta, sort_keys=True).encode()).hexdigest())), flush=True)
    finally:
        config.unlink(missing_ok=True)
        server.shutdown()
        server.server_close()
        worker.join(timeout=3)
    shell()
    print(json.dumps(dict(kind="L3", verdict="pass", harness="installed clients",
                          model="scripted localhost", app_home_writes=[],
                          native_writes_by_cause=causes)), flush=True)


if __name__ == "__main__":
    run()
