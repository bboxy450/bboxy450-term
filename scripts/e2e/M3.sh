# coverage: partial; harness: stand-in; missing: real-harness receipt; the seat row shows the task
# M3: two locator forms, claimed frontmatter, assigned-task readback and one oracle envelope.
. "$(dirname "$0")/lib.sh"
python3 -I -B - "$(dirname "$0")" <<'PY'
import json, os, struct, sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from task_checks import agent_call, ctl, operation, seats, spawn, status, successful

seat = spawn('executor', 'M3 delegation stand-in')
role = seats()[seat]['record']['role']
tasks = []
for suffix, locator_kind in (('number', 'number'), ('path', 'path')):
    created = ctl('task', 'create', 'M3 delegation ' + suffix, '--verify', 'true',
                  '--brief', 'M3 private fixed task; preserve the declared verify command')['data']['record']
    locator = '#' + str(created['number']) if locator_kind == 'number' else created['path']
    args = ('task', 'delegate', locator, seat, '--op', operation())
    delegated = ctl(*args)
    assert delegated['data']['response']['kind'] == 'applied', delegated
    assert delegated['data']['task'] == locator and delegated['data']['seat'] == seat, delegated
    before = status(locator)
    metadata = before['metadata']
    assert metadata['status'] == 'claimed', before
    assert metadata['assignee'] == 'seat:' + seat, before
    assert metadata['delegated_by'].startswith('operator:'), before
    assert metadata['verify'] == 'true' and metadata['role'] == 'executor', before
    assert ctl(*args) == delegated, 'delegation retry did not replay its original result'
    assert status(locator) == before, 'delegation retry changed the record'
    tasks.append(before)

record = seats()[seat]['record']
assert record['role'] == role, 'delegation changed the seat authority'
cards = ctl('task', 'list', '--seat', seat, '--status', 'claimed')['data']
assert {card['id'] for card in cards} == {task['id'] for task in tasks}, cards

# Read the actual recipient MCP inbox, not the operator's send receipt or a fixture verdict.
inbox = successful(agent_call(seat, 'message_list', {'after': 0, 'limit': 128}))
assert inbox['kind'] == 'messages', inbox
page = inbox['page']
assert page['next'] is None, 'fixture inbox unexpectedly requires another page'
envelopes = [row['envelope'] for row in page['items']]
delegations = [env for env in envelopes if env['kind'] == 'task' and env['body']['event'] == 'delegate']
assert len(delegations) == len(tasks), ('duplicate or absent delegation', delegations)
for task in tasks:
    matching = [env for env in delegations if env['body']['task_id'] == task['id']]
    assert len(matching) == 1, (task['id'], matching)
    env = matching[0]
    assert env['priority_class'] == 'oracle' and env['role']['class'] == 'oracle', env
    assert env['door'] == 'ctl' and env['issued_by'].startswith('test-account:'), env
    assert env['body']['path'] == task['path'], env
    assert env['signature'], 'delegation envelope is unsigned'
    assert status(task['path']) == task, 'reading the inbox changed the task'

work = Path(os.environ['E2E_WORK'])
(work / 'm3-recipient-envelopes.json').write_text(json.dumps(delegations, indent=2) + '\n')
ctl('seat', 'focus', seat)
capture = work / 'm3-window.png'
ctl('seat', 'capture', 'window', str(capture))
png = capture.read_bytes()
assert png[:8] == b'\x89PNG\r\n\x1a\n' and all(struct.unpack('>II', png[16:24])), 'window capture invalid'
print('M3 PASS (harness: stand-in): #N/path claim, unchanged authority/verify, one oracle-class recipient envelope per op; partial: real-harness receipt and visible task row')
PY
[ "$?" -eq 0 ] || fail 'M3 delegation or recipient-envelope assertion failed'
