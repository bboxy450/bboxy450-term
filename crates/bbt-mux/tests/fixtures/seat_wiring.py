#!/usr/bin/env python3
"""Local fixture client/shim: inspect launch wiring without credentials or services."""

import json
import os
from pathlib import Path
import re
import shlex
import stat
import subprocess
import sys
import time
import tomllib


def identity():
    return {
        "seat": os.environ["BBOXY_SEAT_ID"],
        "generation": os.environ["BBOXY_GENERATION"],
        "socket": os.environ["BBOXY_SOCK"],
        "role": os.environ["BBOXY_ROLE"],
        "capability_present": bool(re.fullmatch(r"[0-9a-f]{64}", os.environ["BBOXY_CAPABILITY"])),
    }


if sys.argv[1] in ("mcp", "hook"):
    # Never echo the inherited capability, including in a failure report.
    print(json.dumps({"argv": sys.argv[1:], "identity": identity()}))
    sys.exit(0)

kind, output, executable, role_prompt = sys.argv[1:5]
args = sys.argv[5:]
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
    result = subprocess.run(command, check=True, capture_output=True, text=True, timeout=3)
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
        "BBOXY_SOCK", "BBOXY_CAPABILITY", "BBOXY_SEAT_ID", "BBOXY_GENERATION",
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
temporary = path.with_suffix(".writing")
temporary.write_text(json.dumps(report, indent=2) + "\n")
temporary.rename(path)
# Keep the overlay alive until its owner has inspected it; no desktop input.
deadline = time.monotonic() + 15
while not path.with_suffix(".release").exists():
    if time.monotonic() > deadline:
        raise TimeoutError("fixture owner did not release the child")
    time.sleep(0.01)
