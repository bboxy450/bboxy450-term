# coverage: full
# M4: the operator's real task door keeps the declared verification guards and op replay.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh 'M4 verification fixture') || fail 'fixture seat spawn refused'
python3 -I - "$BBT" "$E2E_WORK" "$seat" <<'PY'
import json, os, shlex, subprocess, sys, time
from pathlib import Path

binary, work, seat = sys.argv[1], Path(sys.argv[2]), sys.argv[3]
trace = work / 'm4-calls.jsonl'
def op():
    n = (int(time.time() * 1000) << 80) | int.from_bytes(os.urandom(10), 'big')
    return ''.join('0123456789ABCDEFGHJKMNPQRSTVWXYZ'[(n >> (5*i)) & 31] for i in range(25, -1, -1))
def ctl(*args, exit=0):
    p = subprocess.run([binary, 'ctl', *args], text=True, capture_output=True, timeout=30)
    with trace.open('a') as f:
        f.write(json.dumps(dict(args=args, exit=p.returncode, stdout=p.stdout, stderr=p.stderr)) + '\n')
    rows = [json.loads(line) for line in p.stdout.splitlines() if line.strip()]
    assert len(rows) == 1, (args, p.returncode, p.stdout, p.stderr)
    reply = rows[0]
    assert p.returncode == exit and reply['ok'] == (exit == 0), (args, p.returncode, reply, p.stderr)
    return reply
def status(task):
    reply = ctl('task', 'status', task)
    assert reply['data']['kind'] == 'item', reply
    return reply['data']['item']

verify_file = work / 'fixture-verify.sh'
verify_file.write_text('#!/bin/sh\nprintf "M4 verification ran\\n"\nexit 0\n')
verify = shlex.join(['sh', str(verify_file)])
created = ctl('task', 'create', 'M4 verification guard', '--verify', verify)['data']['record']
task = '#' + str(created['number'])
ctl('task', 'delegate', task, seat)
initial = status(task)
assert initial['metadata']['status'] == 'claimed', initial
assert initial['metadata']['verify'] == verify, initial
for locator in (initial['id'], initial['path']):
    assert status(locator) == initial, 'status locators disagree'

for command, code, reason in (
    ('different verification command', 0, 'verify_result command does not match task verify'),
    (verify, 1, 'verify_result exit_code is 1; acceptance requires 0'),
):
    proof = dict(command=command, exit_code=code, evidence=[])
    ctl('task', 'report', task, '--summary', 'M4 invalid verification control',
        '--verify-result', json.dumps(proof))
    before = status(task)
    assert before['metadata']['status'] == 'reported', before
    assert before['metadata']['verify_result'] == proof, before
    operation = op()
    args = ('task', 'accept', task, '--op', operation)
    refused = ctl(*args, exit=1)
    assert refused['op'] == operation, refused
    assert refused['error']['kind'] == 'refused', refused
    assert refused['error']['message'] == 'rejected: ' + reason, refused
    assert refused['error']['hint'], refused
    assert ctl(*args, exit=1) == refused, 'refusal replay changed'
    assert status(task) == before, 'refused accept changed the task'
    print('M4 named refusal: ' + reason, flush=True)

ran = subprocess.run(['sh', str(verify_file)], text=True, capture_output=True, timeout=10)
assert ran.returncode == 0 and ran.stdout == 'M4 verification ran\n', ran
proof = dict(command=verify, exit_code=ran.returncode,
             evidence=[dict(criterion='M4', reference='private fixture stdout: M4 verification ran')])
report_op = op()
report = ('task', 'report', task, '--summary', 'M4 single valid report',
          '--verify-result', json.dumps(proof), '--op', report_op)
reported = ctl(*report)
assert reported['data']['kind'] == 'applied', reported
before = status(task)
assert before['metadata']['status'] == 'reported', before
assert before['metadata']['verify_result'] == proof, before
assert before['metadata']['report']['op_id'] == report_op, before
assert before['metadata']['report']['reported_by'].startswith('operator:'), before
assert before['body'].count('M4 single valid report') == 1, before
assert ctl(*report) == reported, 'same-op report did not replay'
assert status(task) == before, 'report replay duplicated the report'
conflict = ctl('task', 'report', task, '--summary', 'M4 changed retry', '--op', report_op, exit=1)
assert conflict['error']['kind'] == 'op_id_reused', conflict
assert status(task) == before, 'changed same-op report mutated the task'

accept = ('task', 'accept', task, '--op', op())
accepted = ctl(*accept)
assert accepted['data']['kind'] == 'applied', accepted
after = status(task)
assert after['metadata']['status'] == 'accepted', after
assert after['metadata']['verify'] == verify, after
assert after['metadata']['verify_result'] == proof, after
assert ctl(*accept) == accepted, 'same-op accept did not replay'
assert status(task) == after, 'accept replay duplicated its effect'
again = ctl('task', 'accept', task, exit=1)
assert again['error']['message'] == 'rejected: task must be reported before acceptance', again
assert status(task) == after, 'a fresh accept applied twice'
print('M4 PASS: both named refusals retained the report; valid verification accepted exactly once; same-op retries unchanged')
PY
[ "$?" -eq 0 ] || fail 'M4 task verification or operation identity assertion failed'
