#!/usr/bin/python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Vendor CLI compatibility child: consume real launch context and call admitted MCP."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid

args = sys.argv[1:]
root = Path(os.environ['HOME']).parent
if os.environ.get('BBOXY_SEAT_ID') and args[0] != 'app-server':
    (root / f"context-{os.environ['BBOXY_SEAT_ID']}.entry.json").write_text(json.dumps({
        'pid': os.getpid(), 'generation': os.environ.get('BBOXY_GENERATION'), 'cwd': os.getcwd()}))
if args[0] == 'app-server':
    os.environ['BBOXY_SETTINGS_CODEX_SOCKET'] = args[2].removeprefix('unix://')
    os.execv(str(root / 'fixture-test'), [str(root / 'fixture-test'), '--ignored', '--exact',
                                       'settings_codex_service_fixture', '--nocapture'])

def flag(key):
    return args[args.index(key) + 1]

if '--append-system-prompt' in args:
    kind = 'claude'
    prompt = flag('--append-system-prompt')
    skill = (Path(flag('--plugin-dir')) / 'skills/bboxy450/SKILL.md').read_text()
    server = json.loads(Path(flag('--mcp-config')).read_text())['mcpServers']['bboxy']
    command = [server['command'], *server['args']]
    assert '--dangerously-skip-permissions' in args
    native_session = flag('--resume') if '--resume' in args else flag('--session-id')
else:
    kind = 'codex'
    config = {}
    for at, arg in enumerate(args[:-1]):
        if arg == '-c':
            key, value = args[at + 1].split('=', 1)
            try:
                config[key] = json.loads(value)
            except json.JSONDecodeError:
                config[key] = value
    prompt = config['developer_instructions']
    skill = prompt
    # The managed remote TUI's approval selectors live in the per-thread plan;
    # the production proxy forwards these on start/resume, not CLI permission flags.
    if '--remote' in args:
        launch = Path(flag('--remote').removeprefix('unix://')).parent / 'codex-launch.json'
        config.update(json.loads(launch.read_text())['config'])
    command = [config['mcp_servers.bboxy.command'], *config['mcp_servers.bboxy.args']]
    assert config['approval_policy'] == 'never'
    assert config['sandbox_mode'] == 'danger-full-access'
    assert config.get('mcp_servers.bboxy.default_tools_approval_mode',
                      config.get('mcp_servers', {}).get('bboxy', {}).get('default_tools_approval_mode')) == 'approve'
    session_file = root / f"context-{os.environ['BBOXY_SEAT_ID']}.session"
    native_session = args[args.index('resume') + 1] if 'resume' in args else str(uuid.uuid4())
    if 'resume' in args:
        assert session_file.read_text() == native_session
    session_file.write_text(native_session)
    # A CLI compatibility child emits the documented native SessionStart hook.
    # Without a native thread identity, Restart deliberately becomes a shell.
    subprocess.run([command[0], 'hook', '--agent', 'codex', json.dumps({
        'hook_event_name': 'SessionStart', 'session_id': native_session})], check=True)

client = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def request(number, method, params):
    client.stdin.write(json.dumps({'jsonrpc': '2.0', 'id': number, 'method': method, 'params': params}) + '\n')
    client.stdin.flush()
    while True:
        line = client.stdout.readline()
        assert line, 'admitted MCP exited'
        reply = json.loads(line)
        if reply.get('id') == number:
            assert 'error' not in reply, reply
            return reply['result']

request(1, 'initialize', {'protocolVersion': '2025-11-25', 'capabilities': {},
                         'clientInfo': {'name': 'private-vendor-context-fixture', 'version': '1'}})
client.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
client.stdin.flush()
tools = request(2, 'tools/list', {})
seat = os.environ['BBOXY_SEAT_ID']
trigger = root / f'context-{seat}.refresh'
number = 3
task = None
while True:
    orient = request(number, 'tools/call', {'name': 'orient', 'arguments': {}})
    number += 1
    value = {'kind': kind, 'cwd': os.getcwd(), 'prompt': prompt, 'skill': skill,
             'orient': orient, 'tools': tools,
             'task': task,
             'generation': os.environ['BBOXY_GENERATION'], 'pid': os.getpid()}
    value['native_session'] = native_session
    temporary = root / f'context-{seat}.tmp'
    temporary.write_text(json.dumps(value))
    temporary.replace(root / f'context-{seat}.json')
    while not trigger.exists():
        time.sleep(0.02)
    task_id = trigger.read_text()
    trigger.unlink()
    # Task bodies are deliberately separate from orient's lean task cards.
    until = time.monotonic() + 10
    while True:
        task = request(number, 'tools/call', {'name': 'task_status', 'arguments': {'task_id': task_id}})
        number += 1
        if 'PRIVATE_TASK_CONTEXT_20261103' in json.dumps(task):
            break
        assert time.monotonic() < until, task
        time.sleep(0.05)
