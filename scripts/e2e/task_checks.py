"""Ctl assertions shared by the task night scripts; no second control or identity door."""
import json
import os
import subprocess
import time
from pathlib import Path


def operation():
    n = (int(time.time() * 1000) << 80) | int.from_bytes(os.urandom(10), "big")
    return "".join("0123456789ABCDEFGHJKMNPQRSTVWXYZ"[(n >> (5 * i)) & 31]
                   for i in range(25, -1, -1))


def ctl(*args, exit=0):
    p = subprocess.run([os.environ["BBT"], "ctl", *args],
                       text=True, capture_output=True, timeout=30)
    with (Path(os.environ["E2E_WORK"]) / "task-calls.jsonl").open("a") as log:
        log.write(json.dumps(dict(args=args, exit=p.returncode,
                                 stdout=p.stdout, stderr=p.stderr)) + "\n")
    rows = [json.loads(line) for line in p.stdout.splitlines() if line.strip()]
    assert len(rows) == 1, (args, p.returncode, p.stdout, p.stderr)
    reply = rows[0]
    assert p.returncode == exit and reply["ok"] == (exit == 0), (args, p.returncode, reply, p.stderr)
    return reply


def seats():
    return {row["record"]["id"]: row for row in ctl("seat", "ls")["data"]}


def spawn(role, name, swarm=None):
    args = ["seat", "spawn", "claude", "--role", role, "--place", "tab",
            "--cwd", os.environ["E2E_WORK"], "--name", name]
    if swarm is not None:
        args.extend(["--swarm", swarm])
    seat = ctl(*args)["data"]["seat"]
    ctl("seat", "wait", seat, "--text", "TASK_FIXTURE_READY " + seat, "--timeout", "20000")
    record = seats()[seat]["record"]
    assert record["role"] == {"name": role, "class": "oracle" if role == "oracle" else "worker"}, record
    if swarm is not None:
        assert record["swarm"] == swarm, record
    return seat


def agent_call(seat, tool, args):
    tag = operation()
    line = json.dumps(dict(tag=tag, tool=tool, args=args), separators=(",", ":"))
    assert len(line.encode()) < 3000, "fixture command must fit the PTY's canonical input line"
    ctl("seat", "type", seat, line)
    ctl("seat", "key", seat, "enter")
    deadline = time.monotonic() + 30
    while True:
        rows = ctl("message", "inbox")["data"]
        rows = rows if isinstance(rows, list) else [rows]
        for row in rows:
            envelope = row["message"]["envelope"]
            if envelope["seat"] != seat:
                continue
            text = envelope["body"].get("text", "")
            try:
                answer = json.loads(text)
            except ValueError:
                continue
            if answer.get("fixture") == "task-night-stand-in" and answer.get("tag") == tag:
                assert answer["seat"] == seat and answer["tool"] == tool, answer
                return answer["response"]
        assert time.monotonic() < deadline, ("seat MCP response absent", tag,
                                             ctl("seat", "screen", seat))
        time.sleep(0.05)


def successful(response):
    assert "error" not in response, response
    result = response["result"]
    assert result["isError"] is False, result
    return result["structuredContent"]


def status(task):
    reply = ctl("task", "status", task)
    assert reply["data"]["kind"] == "item", reply
    return reply["data"]["item"]
