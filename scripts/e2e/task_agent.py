"""Night fixture: a PTY command calls the inherited, admitted seat MCP bridge.

Raw JSON-RPC responses return through message_send to the operator inbox. The ctl
script asserts those bytes; the fixture never decides a verdict or reads credentials.
"""
import json
import os
import subprocess
import sys
import time


def operation():
    n = (int(time.time() * 1000) << 80) | int.from_bytes(os.urandom(10), "big")
    return "".join("0123456789ABCDEFGHJKMNPQRSTVWXYZ"[(n >> (5 * i)) & 31]
                   for i in range(25, -1, -1))


def mcp(name, args):
    frames = [
        {"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "task-night-stand-in", "version": "1"}}},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": name, "arguments": args, "_meta": {"bboxy450/op_id": operation()}}},
    ]
    p = subprocess.run([os.environ["BBT"], "mcp"],
                       input="".join(json.dumps(frame) + "\n" for frame in frames),
                       text=True, capture_output=True, timeout=25)
    replies = [json.loads(line) for line in p.stdout.splitlines() if line.strip()]
    assert p.returncode == 0, (p.returncode, p.stderr)
    answers = [row for row in replies if row.get("id") == 1]
    assert len(answers) == 1, (replies, p.stderr)
    return answers[0]


print("TASK_FIXTURE_READY " + os.environ["BBOXY_SEAT_ID"], flush=True)
for line in sys.stdin:
    if not line.strip():
        continue
    try:
        request = json.loads(line)
        result = mcp(request["tool"], request["args"])
        text = json.dumps(dict(fixture="task-night-stand-in", tag=request["tag"],
                               seat=os.environ["BBOXY_SEAT_ID"],
                               tool=request["tool"], response=result))
        sent = mcp("message_send", {"destination": "operator/inbox", "text": text})
        assert sent.get("result", {}).get("isError") is False, sent
        print("TASK_FIXTURE_SENT " + request["tag"], flush=True)
    except Exception as error:
        # A script waiting on the tagged response fails with this real screen as evidence.
        print("TASK_FIXTURE_ERROR " + repr(error), flush=True)
