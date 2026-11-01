#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Owned seat/descendant fixture. Emit counts and booleans, never raw environments."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def scan(seat, supervisor):
    value = Path(os.environ['BBOXY_CAPABILITY_FILE']).read_bytes().strip()
    assert len(value) == 64
    rows = []
    pid = seat
    seen = set()
    while pid not in seen:
        seen.add(pid)
        output = subprocess.check_output(['/bin/ps', '-Eww', '-p', str(pid)])
        rows.append(dict(pid=pid, source='ps', capability_matches=output.count(value),
                         legacy_keys=output.count(b'BBOXY_CAPABILITY=')))
        if pid == supervisor:
            break
        pid = int(subprocess.check_output(['/bin/ps', '-p', str(pid), '-o', 'ppid=']))
        assert pid > 1, 'owned supervisor was not in seat ancestry'
    assert pid == supervisor
    output = subprocess.check_output(['/usr/bin/env'])
    rows.append(dict(pid=os.getpid(), source='env', capability_matches=output.count(value),
                     legacy_keys=output.count(b'BBOXY_CAPABILITY=')))
    return rows


if sys.argv[1] == '--scan':
    assert os.getppid() == int(sys.argv[2]), 'scanner is not a child of the seat'
    print(json.dumps(scan(int(sys.argv[2]), int(sys.argv[3]))))
    raise SystemExit(0)

kind = sys.argv[1]
root = Path(os.environ['CAPABILITY_FIXTURE_ROOT'])
generation = os.environ['BBOXY_GENERATION']
key = kind + '-' + generation
until = time.monotonic() + 45
while not (root / (key + '.go')).exists():
    assert time.monotonic() < until, 'fixture owner did not release its ready barrier'
    time.sleep(.002)
rows = json.loads(subprocess.check_output([
    sys.executable, __file__, '--scan', str(os.getpid()), os.environ.get('CAPABILITY_SUPERVISOR_PID', (root / 'supervisor.pid').read_text().strip())]))
app = os.environ['CAPABILITY_APP']
env = os.environ.copy()
mcp = app
if kind == 'codex' and (Path(env['BBOXY_CAPABILITY_FILE']).parent / 'bboxy-seat').exists():
    mcp = str(Path(env['BBOXY_CAPABILITY_FILE']).parent / 'bboxy-seat')
    # Native Codex starts its MCP process outside the TUI's process tree.
    # The generated wrapper must restore file admission from its own overlay.
    env = {k: v for k, v in env.items() if not k.startswith('BBOXY_')}
requests = [
    dict(jsonrpc='2.0', id=1, method='initialize', params=dict(protocolVersion='2025-11-25', clientInfo=dict(name='capability-file-fixture', version='1'), capabilities={})),
    dict(jsonrpc='2.0', id=2, method='tools/call', params=dict(name='orient', arguments={})),
]
wire = ''.join(json.dumps(r) + '\n' for r in requests)
authenticated = []
probe_status = []
for _ in range(2):
    result = subprocess.run([mcp, 'mcp'], input=wire, text=True, capture_output=True, env=env, timeout=30)
    replies = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    own = next((r for r in replies if r.get('id') == 2), {})
    actor = own.get('result', {}).get('structuredContent', {}).get('actor', {})
    authenticated.append(result.returncode == 0 and actor.get('seat_id') == os.environ['BBOXY_SEAT_ID'])
    probe_status.append(dict(exit=result.returncode, rpc_error=own.get('error'), result_error=own.get('result', {}).get('structuredContent', {}).get('error'), actor=actor, stderr_category='missing-file' if 'No such file' in result.stderr else 'other' if result.stderr else 'none'))
hook_env = os.environ.copy()
hook_env['BBOXY_SOCK'] = str(root / (kind + '-hook.sock'))
hook_env['BBOXY_HOOK_SOCKET_ONLY'] = '1'
hook = subprocess.run([app, 'hook', '--agent', 'claude'], env=hook_env, text=True,
                      input=json.dumps(dict(hook_event_name='PermissionRequest', tool_use_id=key)),
                      capture_output=True, timeout=30)
# Hook exit alone is deliberately not treated as authentication evidence:
# the owner independently checks the real seat endpoint's HookObserved reply.
proof = dict(kind=kind, generation=int(generation), scans=rows, orient=authenticated,
             hook_exit=hook.returncode, probe_status=probe_status, capability_file=os.environ['BBOXY_CAPABILITY_FILE'])
(root / (key + '.json')).write_text(json.dumps(proof) + '\n')
while True:
    time.sleep(1)
