"""L3 stand-in: inspect our own launch artifacts and call the real inherited bridge.

No model, login, capability extraction or fixture-generated bridge responses.
The night script independently asserts this evidence and the published hook state.
"""
import hashlib
import json
import os
from pathlib import Path
import shlex
import socket
import subprocess
import sys
import time
import tomllib
import uuid


def digest(data):
    return hashlib.sha256(data).hexdigest()


def snapshot(home):
    paths = [home / ".claude.json"] + [path for name in (".claude", ".codex")
            for path in sorted((home / name).rglob("*")) if path.is_file()]
    return {str(path.relative_to(home)): digest(path.read_bytes()) for path in paths}


def write(path, value):
    temporary = path.with_suffix(".writing")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def option(args, flag):
    return args[args.index(flag) + 1]


def overrides(args):
    values = {}
    for index, value in enumerate(args):
        if value == "-c":
            key, encoded = args[index + 1].split("=", 1)
            assert key not in values, key
            values[key] = tomllib.loads("value=" + encoded)["value"]
    return values


kind, *args = sys.argv[1:]
home = Path(os.environ["HOME"]).resolve()
assert (home / ".l3-private-home").read_text() == "L3 throwaway harness home\n"
assert Path(os.environ["CLAUDE_CONFIG_DIR"]) == home / ".claude"
assert Path(os.environ["CODEX_HOME"]) == home / ".codex"
work = home.parent / "work"
seat = os.environ["BBOXY_SEAT_ID"]
generation = int(os.environ["BBOXY_GENERATION"])
executable = os.environ["BBT"]
template = (Path(__file__).resolve().parents[2] /
            "crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md").read_text()
expected_skill = template.replace("{{ROLE_NAME}}", json.dumps("executor")).replace(
    "{{ROLE_CLASS}}", json.dumps("worker"))
before = snapshot(home)

if kind == "codex" and args[:2] == ["app-server", "--listen"]:
    assert args[2].startswith("unix://")
    endpoint = args[2].removeprefix("unix://")
    assert Path(endpoint).resolve().parent.name.startswith("launch-")
    assert os.environ["BBOXY_HOOK_SOCKET_ONLY"] == "1"
    values = overrides(args)
    assert values["mcp_servers.bboxy.command"] == executable
    assert values["mcp_servers.bboxy.args"] == ["mcp"]
    assert values["mcp_servers.bboxy.default_tools_approval_mode"] == "approve"
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.bind(endpoint)
        write(work / (seat + ".runtime.json"), dict(
            harness="stand-in", pid=os.getpid(), seat=seat, generation=generation,
            endpoint=endpoint, socket_only_hooks=True))
        # The readiness connect may release the TUI immediately after listen().
        # Publish its evidence before that boundary, not after.
        listener.listen()
        while True:
            stream, _ = listener.accept()
            stream.close()

if kind == "claude":
    settings_path = Path(option(args, "--settings"))
    overlay = settings_path.parent
    settings = json.loads(settings_path.read_text())
    assert settings["crossSessionInbound"] == "accept"
    hooks = settings["hooks"]
    expected_events = {"SessionStart", "SessionEnd", "UserPromptSubmit", "PreToolUse",
                       "PostToolUse", "PermissionRequest", "Notification", "Stop",
                       "StopFailure", "SubagentStart", "SubagentStop"}
    mcp_path = Path(option(args, "--mcp-config"))
    assert mcp_path == overlay / "mcp.json"
    mcp = json.loads(mcp_path.read_text())["mcpServers"]["bboxy"]
    plugin = Path(option(args, "--plugin-dir"))
    assert plugin == overlay / "bboxy450"
    assert json.loads((plugin / ".claude-plugin/plugin.json").read_text())["name"] == "bboxy450"
    skill = (plugin / "skills/bboxy450/SKILL.md").read_text()
    session = option(args, "--session-id")
    uuid.UUID(session)
else:
    assert kind == "codex", kind
    values = overrides(args)
    wrapper = values["mcp_servers.bboxy.command"]
    assert Path(wrapper).name == "bboxy-seat"
    overlay = Path(wrapper).parent
    assert Path(option(args, "--remote").removeprefix("unix://")).resolve().parent == overlay.resolve()
    assert "--no-daemon" not in args
    hooks = values["hooks"]
    expected_events = {"SessionStart", "SessionEnd", "UserPromptSubmit", "PreToolUse",
                       "PostToolUse", "Stop", "SubagentStart", "SubagentStop",
                       "PreCompact", "PostCompact"}
    assert values["notify"] == [wrapper, "hook", "--agent", "codex-notify"]
    assert values["mcp_servers.bboxy.env_vars"] == [
        "BBOXY_SOCK", "BBOXY_CAPABILITY", "BBOXY_SEAT_ID", "BBOXY_GENERATION", "BBOXY_RUN_DIR"]
    assert values["mcp_servers.bboxy.default_tools_approval_mode"] == "approve"
    assert values["check_for_update_on_startup"] is False
    mcp = {"command": values["mcp_servers.bboxy.command"],
           "args": values["mcp_servers.bboxy.args"]}
    instructions = values["developer_instructions"]
    assert instructions.endswith("\n\n" + expected_skill), "Codex instructions lost the pinned skill"
    skill = instructions[-len(expected_skill):]
    session = str(uuid.uuid4())

assert overlay.is_dir() and overlay.name.startswith("launch-")
assert overlay.stat().st_mode & 0o777 == 0o700
expected_command = executable if kind == "claude" else wrapper
assert mcp == {"command": expected_command, "args": ["mcp"]}, mcp
assert set(hooks) == expected_events, hooks.keys()
for event, entries in hooks.items():
    assert len(entries) == 1 and len(entries[0]["hooks"]) == 1, event
    hook = entries[0]["hooks"][0]
    assert hook["type"] == "command", event
    assert shlex.split(hook["command"]) == [expected_command, "hook", "--agent", kind], event
assert skill == expected_skill, "installed skill differs from pinned template"

# Use the MCP command from the launch artifact, with its original inherited env.
# Do not inspect or rewrite any capability, socket or authentication value.
frames = [
    {"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "L3-launch-stand-in", "version": "1"}}},
    {"jsonrpc": "2.0", "method": "notifications/initialized"},
    {"jsonrpc": "2.0", "id": 1, "method": "tools/list"},
    {"jsonrpc": "2.0", "id": 2, "method": "tools/call",
     "params": {"name": "orient", "arguments": {}}},
]
called = subprocess.run([mcp["command"], *mcp["args"]],
                        input="".join(json.dumps(row) + "\n" for row in frames),
                        text=True, capture_output=True, timeout=30)
assert called.returncode == 0, (called.returncode, called.stderr)
responses = {row["id"]: row for row in map(json.loads, called.stdout.splitlines()) if "id" in row}
assert set(responses) == {0, 1, 2}, responses
assert "error" not in responses[0] and "serverInfo" in responses[0]["result"], responses[0]
assert {"orient", "task_list", "task_report", "message_send"} <= {
    row["name"] for row in responses[1]["result"]["tools"]}, responses[1]
assert responses[2]["result"]["isError"] is False, responses[2]
actor = responses[2]["result"]["structuredContent"]["actor"]
assert actor["seat_id"] == seat and actor["process_generation"] == generation, actor
assert actor["role"] == {"name": "executor", "class": "worker"}, actor

for event in ("SessionStart", "PreToolUse"):
    command = shlex.split(hooks[event][0]["hooks"][0]["command"])
    raw = dict(hook_event_name=event, session_id=session, tool_name="L3-LAUNCH-PROBE")
    shim = subprocess.run(command, input=json.dumps(raw), text=True,
                          capture_output=True, timeout=30)
    assert shim.returncode == 0, (event, shim.returncode, shim.stderr)

# A hook has a deliberate 50 ms best-effort budget and returns success even
# when its transport misses that budget. Exercise the installed observation
# path until the real bridge confirms it, instead of treating process exit or
# PTY READY as an acknowledgement. Repeating this fixture observation has no
# tool/effect side effect; real harness input is never retried here.
deadline = time.monotonic() + 20
attempts = 0
while True:
    attempts += 1
    command = shlex.split(hooks["PreToolUse"][0]["hooks"][0]["command"])
    shim = subprocess.run(command, input=json.dumps(dict(
        hook_event_name="PreToolUse", session_id=session, tool_name="L3-LAUNCH-PROBE")),
        text=True, capture_output=True, timeout=5)
    assert shim.returncode == 0, (shim.returncode, shim.stderr)
    status_frame = {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": {"name": "self_status", "arguments": {}}}
    status = subprocess.run([mcp["command"], *mcp["args"]],
        input="".join(json.dumps(row) + "\n" for row in [*frames[:2], status_frame]),
        text=True, capture_output=True, timeout=5)
    assert status.returncode == 0, (status.returncode, status.stderr)
    current = next(row for row in map(json.loads, status.stdout.splitlines())
                   if row.get("id") == 3)["result"]
    assert not current["isError"], current
    adapter = current["structuredContent"]["seat"]["adapter"]
    if adapter["state"] == "active" and adapter["grade"] == "shim":
        break
    assert time.monotonic() < deadline, ("shim observation not published", attempts, adapter)
    time.sleep(.05)

after = snapshot(home)
assert before == after, "launch probe changed the private harness configs"
write(work / (seat + ".launch.json"), dict(
    harness="stand-in", kind=kind, seat=seat, generation=generation,
    home=str(home), overlay=str(overlay), mcp=mcp, responses=responses,
    hook_events=sorted(hooks), hook_commands_match=True, session=session,
    hook_observation_attempts=attempts,
    skill_sha256=digest(skill.encode()), pinned_skill_sha256=digest(expected_skill.encode()),
    before=before, after=after))
print("L3_LAUNCH_READY " + seat, flush=True)
for line in sys.stdin:
    if line.strip() == "L3_EXIT":
        break
