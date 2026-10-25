"""A real seat calls its inherited MCP bridge; only raw protocol bytes are returned.

The controlling script starts this through ctl seat type/key. No second app,
operator endpoint, credential read or model self-report is involved.
"""
import json
from pathlib import Path
import subprocess
import sys


binary, request, output, tag = sys.argv[1:]
calls = json.loads(Path(request).read_text())
frames = [
    {"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "messaging-night-fixture", "version": "1"}}},
    {"jsonrpc": "2.0", "method": "notifications/initialized"},
]
for number, call in enumerate(calls, 1):
    frames.append({"jsonrpc": "2.0", "id": number, "method": "tools/call",
                   "params": {"name": call["tool"], "arguments": call["args"],
                              "_meta": {"bboxy450/op_id": call["op"]}}})
result = subprocess.run([binary, "mcp"],
                        input="".join(json.dumps(row) + "\n" for row in frames),
                        text=True, capture_output=True, timeout=30)
rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
Path(output).write_text(json.dumps({"exit": result.returncode, "responses": rows,
                                   "stderr": result.stderr}, indent=2) + "\n")
print("MESSAGE_FIXTURE_DONE " + tag, flush=True)
