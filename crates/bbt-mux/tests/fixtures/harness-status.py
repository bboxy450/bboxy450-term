#!/usr/bin/python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Private native seat: independently select OSC-only or log-only reports."""
import base64
import json
import os
from pathlib import Path
import socket
import sys
import threading
import time
import tty

home = Path(os.environ['HOME'])
root = home / '.bboxy450-harness'
session = os.environ['BBOXY_SEAT_ID']  # A fixture-owned, unique ULID.
(root / 'run').mkdir(parents=True, exist_ok=True)
(root / 'sessions').mkdir(exist_ok=True)
log = (root / 'sessions' / f'{session}.jsonl').open('a')
listener = socket.socket(socket.AF_UNIX)
endpoint = sys.argv[sys.argv.index('--socket') + 1]
listener.bind(endpoint)
listener.listen()


def hello():
    while True:
        peer, _ = listener.accept()
        with peer:
            request = json.loads(peer.makefile('rb').readline(256 * 1024))
            assert request['subscribe'].get('session', session) == session
            assert 'op' not in request
            peer.sendall((json.dumps({'v': {'major': 1, 'minor': 1},
                                     'meta': {'session': session}}) + '\n').encode())


threading.Thread(target=hello, daemon=True).start()
seq = 0


def emit(event, state, detail='', permission=False):
    global seq
    if (home / 'status-source').read_text() == 'log':
        seq += 1
        log.write(json.dumps({'v': {'major': 1, 'minor': 1}, 'session': session,
                              'seq': seq, 'epoch': 1, 'ts_ms': time.time_ns() // 1_000_000,
                              'event': event}) + '\n')
        log.flush()
    else:
        msg = ':msg=' + base64.b64encode(detail.encode()).decode() if detail else ''
        kind = ':kind=permission' if permission else ''
        sys.stdout.write(f'\x1b]7501;state={state}{msg}{kind}\x07')
        sys.stdout.flush()


tty.setraw(sys.stdin.fileno())
print(f'session {session}', flush=True)
pending = b''
commands = home / 'status-command'
commands.touch()
reader = commands.open('rb')
while True:
    data = reader.read(4096)
    if not data:
        time.sleep(0.01)
        continue
    pending = (pending + data).replace(b'\x1b]7501;?\x07', b'')
    while b'\r' in pending:
        command, pending = pending.split(b'\r', 1)
        if command == b'start':
            emit({'type': 'turn_started', 'turn': 't'}, 'working')
            emit({'type': 'item_started', 'turn': 't', 'item': 'tool',
                  'kind': 'tool_call', 'head': {'name': 'bash'}}, 'working', 'bash')
        elif command == b'approval':
            emit({'type': 'item_started', 'turn': 't', 'item': 'a',
                  'kind': 'approval', 'head': {'name': 'Allow bash'}},
                 'blocked', 'Allow bash', True)
        elif command == b'resume':
            emit({'type': 'item_completed', 'item': 'a'}, 'working', 'bash')
        elif command in (b'done', b'error'):
            error = command == b'error'
            emit({'type': 'turn_ended', 'turn': 't',
                  'reason': {'reason': 'error' if error else 'done',
                             'message': 'scripted failure'}},
                 'error' if error else 'idle', 'scripted failure' if error else '')
        elif command == b'closed':
            emit({'type': 'session_closed'}, 'idle')
