"""T1's deterministic executables behind the real launcher and PTY host.

They report their own terminal, process and input. They do not impersonate a
model or return a canned ctl/MCP result; model behaviour belongs to L4.
"""
import os
from pathlib import Path
import socket
import sys


kind, *args = sys.argv[1:]
if args == ["--version"]:
    print(f"{kind} T1 launch fixture")
    raise SystemExit(0)

home = Path(os.environ["HOME"])
assert (home / ".t1-private-home").read_text() == "T1 throwaway harness home\n"

if kind == "codex" and args[:2] == ["app-server", "--listen"]:
    assert args[2].startswith("unix://")
    path = args[2].removeprefix("unix://")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.bind(path)
        listener.listen()
        while True:
            stream, _ = listener.accept()
            stream.close()

assert kind in ("claude", "codex")
assert os.isatty(0) and os.isatty(1), "launch did not supply a real PTY"
seat = os.environ["BBOXY_SEAT_ID"]
print(f"T1_READY {kind} {seat} {os.getpid()} {os.ttyname(0)}", flush=True)
for line in sys.stdin:
    if line.startswith("T1_PING "):
        print("T1_REPLY " + line.strip().removeprefix("T1_PING "), flush=True)
