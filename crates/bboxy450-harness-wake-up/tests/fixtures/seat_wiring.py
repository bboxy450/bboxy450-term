#!/usr/bin/env python3
"""Local fixture client/shim: inspect launch wiring without credentials or services."""

import json
import os
from pathlib import Path
import re
import shlex
import socket
import stat
import subprocess
import sys
import select
import tomllib


def identity():
    return {
        "seat": os.environ["BBOXY_SEAT_ID"],
        "generation": os.environ["BBOXY_GENERATION"],
        "socket": os.environ["BBOXY_SOCK"],
        "role": os.environ["BBOXY_ROLE"],
        "capability_present": bool(re.fullmatch(r"[0-9a-f]{64}", os.environ["BBOXY_CAPABILITY"])),
    }


def wait_for_file(path, publish=lambda: None):
    # Watch the owned directory before publishing the report. The caller keeps
    # writing an ordinary file, so a dead receiver can never block its writer.
    if path.exists():
        publish()
        return
    if hasattr(select, "kqueue"):
        descriptor = os.open(path.parent, os.O_RDONLY)
        queue = select.kqueue()
        try:
            queue.control([select.kevent(
                descriptor, filter=select.KQ_FILTER_VNODE,
                flags=select.KQ_EV_ADD | select.KQ_EV_CLEAR,
                fflags=select.KQ_NOTE_WRITE,
            )], 0, 0)
            publish()
            while not path.exists():
                if not queue.control(None, 1, 30):
                    raise TimeoutError("private fixture notification hung")
        finally:
            queue.close()
            os.close(descriptor)
    else:
        # Linux's directory notification equivalent; no extra Python package.
        import ctypes
        native = ctypes.CDLL(None, use_errno=True)
        native.inotify_init1.argtypes = [ctypes.c_int]
        native.inotify_add_watch.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_uint32]
        descriptor = native.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK)
        if descriptor < 0:
            raise OSError(ctypes.get_errno(), "fixture inotify init failed")
        try:
            # IN_CREATE | IN_MOVED_TO | IN_CLOSE_WRITE.
            if native.inotify_add_watch(descriptor, os.fsencode(path.parent), 0x100 | 0x80 | 0x8) < 0:
                raise OSError(ctypes.get_errno(), "fixture directory watch failed")
            poller = select.poll()
            poller.register(descriptor, select.POLLIN)
            publish()
            while not path.exists():
                if not poller.poll(30_000):
                    raise TimeoutError("private fixture notification hung")
                os.read(descriptor, 65536)
        finally:
            os.close(descriptor)


if sys.argv[1] == "harness-exec":
    endpoint, program = sys.argv[2:4]
    wait_for_file(Path(endpoint + ".ready"))
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.connect(endpoint)
    os.execv(program, sys.argv[3:])

if sys.argv[1] in ("mcp", "hook"):
    # Never echo the inherited capability, including in a failure report.
    print(json.dumps({"argv": sys.argv[1:], "identity": identity()}))
    sys.exit(0)

kind, output, executable, role_prompt = sys.argv[1:5]
args = sys.argv[5:]
if kind == "codex" and args[:2] == ["app-server", "--listen"]:
    assert args[2].startswith("unix://")
    endpoint = args[2][len("unix://"):]
    assert "--remote" not in args and "resume" not in args
    assert 'mcp_servers.bboxy.default_tools_approval_mode="approve"' in args
    ready = Path(endpoint + ".ready")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.bind(endpoint)
        listener.listen()
        Path(output).with_suffix(".runtime.json").write_text(json.dumps({
            "argv": args, "identity": identity(), "pid": os.getpid(), "endpoint": endpoint,
        }))
        # The readiness marker is published after bind/listen and never waits
        # for a consumer, including one that exited during startup.
        ready.write_bytes(b"ready")
        while True:
            stream, _ = listener.accept()
            stream.close()

assert not any(x in args for x in ("--strict-mcp-config", "--bare"))
assert not any(
    "bypass" in x or "trusted_hash" in x
    for x in args if not x.startswith("developer_instructions=")
)


def option(name):
    index = args.index(name)
    assert index + 1 < len(args)
    return args[index + 1]


def run(command):
    result = subprocess.run(command, check=True, capture_output=True, text=True, timeout=30)
    return json.loads(result.stdout)


if kind == "claude":
    settings_path = Path(option("--settings"))
    overlay = settings_path.parent
    settings = json.loads(settings_path.read_text())
    assert settings["crossSessionInbound"] == "accept"
    mcp_path = Path(option("--mcp-config"))
    # --mcp-config is variadic: a following option ends its one-file value.
    assert args[args.index("--mcp-config") + 2] == "--plugin-dir"
    plugin = Path(option("--plugin-dir"))
    assert mcp_path == overlay / "mcp.json"
    assert plugin == overlay / "bboxy450"
    manifest = json.loads((plugin / ".claude-plugin/plugin.json").read_text())
    assert manifest["name"] == "bboxy450"
    skill = (plugin / "skills/bboxy450/SKILL.md").read_text()
    assert option("--append-system-prompt") == role_prompt
    mcp = json.loads(mcp_path.read_text())["mcpServers"]["bboxy"]
    hooks = settings["hooks"]
    assert set(hooks) == {
        "SessionStart", "SessionEnd", "UserPromptSubmit", "PreToolUse", "PostToolUse",
        "PermissionRequest", "Notification", "Stop", "StopFailure", "SubagentStart", "SubagentStop",
    }
    hook_commands = [entry[0]["hooks"][0]["command"] for entry in hooks.values()]
    hook_agent = "claude"
    overrides = None
else:
    assert kind == "codex"
    # Parse the real TOML values independently of Rust's JSON string encoder.
    overrides = {}
    for index, value in enumerate(args):
        if value == "-c":
            key, encoded = args[index + 1].split("=", 1)
            assert key not in overrides
            overrides[key] = tomllib.loads("value=" + encoded)["value"]
    skill = overrides["developer_instructions"]
    assert skill.startswith(role_prompt + "\n\n")
    mcp = {"command": overrides["mcp_servers.bboxy.command"], "args": overrides["mcp_servers.bboxy.args"]}
    assert overrides["mcp_servers.bboxy.env_vars"] == [
        "BBOXY_SOCK", "BBOXY_CAPABILITY", "BBOXY_SEAT_ID", "BBOXY_GENERATION", "BBOXY_RUN_DIR",
    ]
    assert overrides["notify"] == [executable, "hook", "--agent", "codex-notify"]
    hooks = overrides["hooks"]
    assert set(hooks) == {
        "SessionStart", "SessionEnd", "UserPromptSubmit", "PreToolUse", "PostToolUse",
        "Stop", "SubagentStart", "SubagentStop", "PreCompact", "PostCompact",
    }
    hook_commands = [entry[0]["hooks"][0]["command"] for entry in hooks.values()]
    hook_agent = "codex"
    overlay = None

assert mcp == {"command": executable, "args": ["mcp"]}
for command in hook_commands:
    assert shlex.split(command) == [executable, "hook", "--agent", hook_agent]
assert "{{ROLE_" not in skill
assert json.dumps(os.environ["BBOXY_ROLE"], ensure_ascii=False) in skill
assert skill.index("1. **Orient.") < skill.index("2. **Tasks.") < skill.index("3. **Messages.") < skill.index("4. **Artifacts.")
assert "Storage does not prove model receipt" in skill
assert os.environ["BBOXY_CAPABILITY"] not in skill

mcp_result = run([mcp["command"], *mcp["args"]])
hook_result = run(shlex.split(hook_commands[0]))
assert mcp_result["identity"] == identity() == hook_result["identity"]
if kind == "codex":
    assert run(overrides["notify"])["argv"] == ["hook", "--agent", "codex-notify"]

modes = {}
if overlay:
    for path in [overlay, *sorted(overlay.rglob("*"))]:
        mode = stat.S_IMODE(path.stat().st_mode)
        assert mode == (0o700 if path.is_dir() else 0o600), (path, oct(mode))
        modes[str(path.relative_to(overlay))] = oct(mode)

report = {
    "kind": kind, "argv": args, "identity": identity(), "mcp": mcp_result,
    "hook": hook_result, "skill_present": True, "role_prompt_preserved": True,
    "modes": modes, "overrides": overrides,
}
path = Path(output)
release = path.with_suffix(".release")


def publish_report():
    temporary = path.with_suffix(".writing")
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.rename(path)
    if os.environ.get("BBT_FIXTURE_REPORT_READY") == "1":
        print("fixture-report-ready", flush=True)


wait_for_file(release, publish_report)
