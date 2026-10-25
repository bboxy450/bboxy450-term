# coverage: partial; harness: stand-in; missing: a real Worker and real Oracle harness ask for the unassigned inventory
# M5: the operator sees the exact cross-swarm inventory; admitted seat MCP calls are refused.
. "$(dirname "$0")/lib.sh"
python3 -I -B - "$(dirname "$0")" <<'PY'
import json, os, sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from task_checks import agent_call, ctl, operation, seats, spawn, status, successful

oracle_a = spawn('oracle', 'M5 oracle A stand-in', operation())
oracle_b = spawn('oracle', 'M5 oracle B stand-in', operation())
worker = spawn('executor', 'M5 worker stand-in')
records = seats()
assert records[oracle_a]['record']['swarm'] != records[oracle_b]['record']['swarm'], records
expected = set()
for suffix, owner in (('a', oracle_a), ('b', oracle_b)):
    created = successful(agent_call(owner, 'knowledge_write', {
        'path': 'tasks/m5-unassigned-' + suffix + '.md',
        'frontmatter': {'type': 'task', 'title': 'M5 open unassigned ' + suffix,
                        'status': 'open', 'assignee': 'none', 'role': 'executor', 'verify': 'true'},
        'body': 'Private M5 task created by the admitted oracle of its own swarm.\n',
    }))['record']
    assert created['frontmatter']['delegated_by'] == 'seat:' + owner, created
    assert created['frontmatter']['status'] == 'open' and created['frontmatter']['assignee'] == 'none', created
    fetched = ctl('record', 'fetch', '#' + str(created['number']))['data']
    assert fetched['id'] == created['id'] and fetched['frontmatter'] == created['frontmatter'], fetched
    expected.add(created['id'])

def assigned(title):
    record = ctl('task', 'create', title, '--verify', 'true')['data']['record']
    handle = '#' + str(record['number'])
    ctl('task', 'delegate', handle, worker)
    return handle, status(handle)['id']

claimed, claimed_id = assigned('M5 assigned control')
closed, closed_id = assigned('M5 accepted control')
proof = json.dumps(dict(command='true', exit_code=0, evidence=[]))
ctl('task', 'report', closed, '--summary', 'M5 closed inventory control', '--verify-result', proof)
ctl('task', 'accept', closed)
assert status(closed)['metadata']['status'] == 'accepted'

def ids(*args):
    return {card['id'] for card in ctl('task', 'list', *args)['data']}

assert ids('--unassigned') == expected, 'operator cross-swarm inventory differs'
assert ids('--unassigned', '--status', 'open') == expected, 'open inventory filter differs'
assert ids('--seat', worker, '--status', 'claimed') == {claimed_id}, 'seat/status filter lost assigned control'
assert ids('--seat', worker, '--status', 'accepted') == {closed_id}, 'seat/status filter lost accepted control'
assert not (expected & {claimed_id, closed_id}), 'fixture IDs overlap'

refusals = []
for seat in (worker, oracle_a):
    response = agent_call(seat, 'task_list', {'unassigned': True, 'limit': 128})
    assert 'error' not in response, response
    result = response['result']
    assert result['isError'] is True, result
    data = result['structuredContent']
    assert data['status'] == 'rejected' and data['error']['code'] == 'rejected', data
    assert data['error']['message'] == 'unassigned task inventory requires the account operator', data
    assert set(data) == {'op_id', 'status', 'error'}, 'refusal leaked task data'
    assert len(result['content']) == 1, result
    assert result['content'][0]['type'] == 'text' and json.loads(result['content'][0]['text']) == data, result
    for task_id in expected | {claimed_id, closed_id}:
        assert task_id not in json.dumps(result), 'seat refusal leaked a task identifier'
    refusals.append(dict(seat=seat, role=records[seat]['record']['role'], response=response))
    print('M5 named refusal (' + records[seat]['record']['role']['class'] + '): ' + data['error']['message'], flush=True)
assert ids('--unassigned') == expected, 'seat refusal changed the operator inventory'
Path(os.environ['E2E_WORK'], 'm5-seat-refusals.json').write_text(json.dumps(refusals, indent=2) + '\n')
print('M5 PASS (harness: stand-in): exact inventory in two swarms, assigned/accepted filters, Worker and Oracle real-MCP refusals without data; partial: real harnesses')
PY
[ "$?" -eq 0 ] || fail 'M5 inventory or admitted seat authority assertion failed'
