# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""A private harness peer. Wait for both answers before returning either verdict."""
import json
import socket
import sys
import time
from pathlib import Path

path, session, item, opened = sys.argv[1:]
version = {"major": 1, "minor": 1}
with socket.socket(socket.AF_UNIX) as listener:
    listener.bind(path)
    listener.listen(1)
    listener.settimeout(30)
    connection, _ = listener.accept()
    with connection:
        connection.settimeout(30)
        with connection.makefile("rwb", buffering=0) as wire:
            def send(value):
                connection.sendall(json.dumps(value).encode() + b"\n")

            def event(seq, event, cause=None):
                send(dict(v=version, session=session, epoch=1, seq=seq, at=seq,
                          cause=cause, ts_ms=55 + seq, event=event))

            subscribe = json.loads(wire.readline())
            assert subscribe["subscribe"]["ephemeral"] is False
            assert subscribe["approver"]
            send(dict(v=version, meta={"session": session}, epoch=1, head=1))
            event(1, {"type": "session_started", "meta": {}})
            until = time.monotonic() + 15
            while not Path(opened).exists():
                assert time.monotonic() < until, "GUI did not subscribe"
                time.sleep(0.01)
            event(2, {"type":"item_started","kind":"question","item":item,
                      "head":{"kind":"question","message":"Which database?",
                              "schema":{"enum":["Postgres","SQLite"]}}})
            accepted = []
            while len(accepted) < 2:
                op = json.loads(wire.readline())
                assert op["session"] == session
                if op["op"]["type"] != "answer_question":
                    send({"op_id":op["op_id"],"ack":{"op_id":op["op_id"],"duplicate":False}})
                    continue
                assert op["op"]["question"] == item
                assert op["op"]["answer"]["action"] == "accept"
                accepted.append(op)
                send({"op_id":op["op_id"],"ack":{"op_id":op["op_id"],"duplicate":False}})
            winner, loser = accepted
            assert winner["op_id"] != loser["op_id"]
            by = winner["op"]["client"]
            event(3, {"type":"item_completed","item":item, "status":{"status":"completed"},
                      "body":{"kind":"question","answered_by":by,"answer":winner["op"]["answer"]}}, winner["op_id"])
            event(4, {"type":"op_rejected","op":loser["op_id"],
                      "reason":{"reason":"already_answered","by":by}}, loser["op_id"])
            # Connected until the test shuts its own supervisor down.
            assert wire.readline() == b"", "GUI replay reached the harness twice"
