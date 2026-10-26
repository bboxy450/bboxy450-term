"""L3's installed client, using only a throwaway HOME and a local model server.

The client consumes the app's original overlay and inherited MCP capability.
No stand-in makes its tool calls, and no credential is read or copied here.
"""
import json
import os
from pathlib import Path
import shutil
import sys


kind, *args = sys.argv[1:]
work = Path.cwd()
settings = work / "l3-native-provider.json"
if not settings.exists():
    os.execv(sys.executable, [sys.executable, "-I", "-B",
             str(Path(__file__).with_name("launch_probe.py")), kind, *args])

home = work.parent / "home"
assert Path(os.environ["HOME"]).resolve() == home.resolve()
assert Path(os.environ["CLAUDE_CONFIG_DIR"]) == home / ".claude"
assert Path(os.environ["CODEX_HOME"]) == home / ".codex"
assert (home / ".l3-private-home").is_file()
config = json.loads(settings.read_text())
port = config["port"]
assert isinstance(port, int) and 0 < port < 65536
base = "http://127.0.0.1:" + str(port)

# The promise's PATH begins with these wrappers. Resolve the next installed
# client explicitly; recursively invoking this wrapper would prove nothing.
wrapper = Path(__file__).with_name("L3.fixture-bin").resolve()
search = os.pathsep.join(p for p in os.get_exec_path()
                         if Path(p).resolve() != wrapper and not Path(p).name.endswith(".fixture-bin"))
executable = shutil.which(kind, path=search)
assert executable, "installed native client unavailable: " + kind
assert Path(executable).resolve().parent != wrapper

# Discard inherited provider credentials, even though the URL is loopback.
env = dict(os.environ)
for key in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "OPENAI_API_KEY",
            "OPENAI_ADMIN_KEY", "AZURE_OPENAI_API_KEY", "CODEX_API_KEY",
            "AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN",
            "GOOGLE_APPLICATION_CREDENTIALS", "CLAUDE_CODE_OAUTH_TOKEN",
            "OPENAI_BASE_URL", "OPENAI_API_BASE", "ANTHROPIC_BASE_URL",
            "ANTHROPIC_CUSTOM_HEADERS", "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_VERTEX", "CLAUDE_CODE_USE_FOUNDRY"):
    env.pop(key, None)
env.update(DISABLE_AUTOUPDATER="1", CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1")
first_turn = config.get("first_turn", False)
assert isinstance(first_turn, bool)
prompt = ("Orient, report the assigned L4 fixture task, then reply L4_NATIVE_OK."
          if first_turn else "Reply L3_NATIVE_OK. Use no tools.")

if kind == "claude":
    env.update(ANTHROPIC_BASE_URL=base,
               ANTHROPIC_AUTH_TOKEN="throwaway-local-fixture-only",
               ANTHROPIC_MODEL="claude-sonnet-4-6", ENABLE_TOOL_SEARCH="false")
    allowed = ["mcp__bboxy__orient"]
    if first_turn:
        allowed.append("mcp__bboxy__task_report")
    args += ["-p", "--verbose", "--output-format", "stream-json",
             "--permission-mode", "dontAsk", "--allowedTools", *allowed, "--", prompt]
    # Preserve the native seat PID: exec, rather than a wrapper-owned subprocess.
    output = work / (os.environ["BBOXY_SEAT_ID"] + ".native.jsonl")
    fd = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    os.dup2(fd, 1)
    os.close(fd)
else:
    assert kind == "codex", kind
    runtime = args[:2] == ["app-server", "--listen"]
    for value in [
        'model_provider="l3_fixture"', 'model="fixture-model"',
        'model_providers.l3_fixture.name="L3 local fixture"',
        'model_providers.l3_fixture.base_url=' + json.dumps(base + "/v1"),
        'model_providers.l3_fixture.wire_api="responses"',
        'model_providers.l3_fixture.requires_openai_auth=false',
        'model_providers.l3_fixture.supports_websockets=false',
    ]:
        args += ["-c", value]
    if not runtime:
        args += ["--no-alt-screen", "--", prompt]

os.execve(executable, [executable, *args], env)
