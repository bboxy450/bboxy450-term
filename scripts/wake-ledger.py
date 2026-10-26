"""Private #334 wake attribution. This is a projection, never extra inference cost.

One stage=wake line owns each attempt. Append-only kind=wake-usage revisions
fill its input tokens; consumers use the latest revision per wake_id. Native
queue ids are NOT turn ids. Only a native client message id inside a typed turn
scope (Codex), or a transcript UUID parent chain (Claude), can join the two.
Missing/conflicting identities remain unknown, including after a restart.
"""
import json
import os
from pathlib import Path
import stat
import time


class Wakes:
    def __init__(self, progress, directory, digest, now, deadline):
        self.state = progress.setdefault('wake_accounting', dict(turns={}, last={}))
        self.digest, self.now = digest, now
        self.wakes = {}
        self.budget_exhausted = False
        for path in sorted(Path(directory).glob('*/ledger.jsonl')):
            try:
                fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
            except OSError:
                continue
            with os.fdopen(fd) as stream:
                if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
                    continue
                while time.monotonic() < deadline:
                    line = stream.readline(1_048_577)
                    if not line:
                        break
                    if len(line) > 1_048_576 or not line.endswith('\n'):
                        break
                    try:
                        row = json.loads(line)
                        if row.get('kind') == 'stage' and row.get('stage') == 'wake':
                            # Whitelist metadata; never keep a serialized Work or note.
                            item = {k: row.get(k) for k in ('id', 'attempt', 'harness', 'session', 'at', 'evidence')}
                            if all(isinstance(item[k], str) and 0 < len(item[k]) <= 256
                                   for k in ('id', 'attempt', 'harness', 'session', 'at')):
                                prior = self.wakes.get(item['id'])
                                self.wakes[item['id']] = item if prior is None or prior == item else dict(item, conflict=True)
                    except (ValueError, TypeError, AttributeError):
                        pass  # A concurrent partial append is observed next night.
                if time.monotonic() >= deadline:
                    self.budget_exhausted = True
                    break

    def turn(self, harness, session, identity):
        key = self.digest([harness, session, identity])
        return self.state['turns'].setdefault(key, dict(harness=harness, session=session,
            turn=identity, clients=[], requests={}, complete=False))

    @staticmethod
    def usage(turn, identity, values):
        if not isinstance(identity, str) or not identity or len(identity) > 256:
            return
        old = turn['requests'].get(identity)
        # A response is counted once, not once per streamed/display copy.
        turn['requests'][identity] = values if old is None or old == values else {
            'input_tokens': None, 'input_cached_tokens': None,
            'input_uncached_tokens': None, 'conflict': True}

    def observe_codex(self, row, context):
        payload = row.get('payload') or {}
        kind, event = row.get('type'), payload.get('type')
        session = context.get('session')
        if not session:
            return
        if kind == 'event_msg' and event == 'task_started' and payload.get('turn_id'):
            context['wake_turn'] = payload['turn_id']
        elif kind == 'event_msg' and event == 'user_message':
            identity = context.get('wake_turn')
            if identity:
                turn = self.turn('codex', session, identity)
                client = payload.get('client_id')
                if isinstance(client, str) and 0 < len(client) <= 256:
                    if client not in turn['clients']:
                        turn['clients'].append(client)
                else:
                    turn['unknown_client'] = True
        elif kind == 'event_msg' and event in ('task_complete', 'turn_aborted'):
            identity = payload.get('turn_id') or context.get('wake_turn')
            if identity:
                self.turn('codex', session, identity)['complete'] = True
            context.pop('wake_turn', None)
        if kind != 'token_usage_record' or not payload.get('turn_id'):
            return
        # 0.163 protocol::TokenUsageRecord supplies exact thread/turn/response ids
        # and per-response usage. Do not turn its native queue id into a turn id.
        thread = payload.get('thread_id') or session
        turn = self.turn('codex', thread, payload['turn_id'])
        values = payload.get('usage') or {}
        total, cached = number(values, 'input_tokens'), number(values, 'cached_input_tokens')
        self.usage(turn, payload.get('response_id'), dict(input_tokens=total,
            input_cached_tokens=cached, input_uncached_tokens=(total-cached
                if total is not None and cached is not None and total >= cached else None)))

    def observe_claude(self, row, context):
        session = context.get('session')
        if not session:
            return
        nodes = context.setdefault('wake_nodes', {})
        identity, parent = row.get('uuid'), row.get('parentUuid')
        root = nodes.get(parent)
        if row.get('type') == 'user' and not row.get('tool_result_present'):
            root = identity  # A new user turn breaks ancestry to the prior wake.
        if isinstance(identity, str) and 0 < len(identity) <= 256:
            nodes[identity] = root
        if not root or row.get('type') != 'assistant':
            return
        message = row.get('message') or {}
        values = message.get('usage')
        if not isinstance(values, dict):
            return
        turn = self.turn('claude', session, root)
        turn['clients'] = [root]
        fresh, creation, cached = (number(values, name) for name in (
            'input_tokens', 'cache_creation_input_tokens', 'cache_read_input_tokens'))
        uncached = fresh+creation if fresh is not None and creation is not None else None
        self.usage(turn, row.get('requestRef') or message.get('id'), dict(
            input_tokens=cached+uncached if cached is not None and uncached is not None else None,
            input_cached_tokens=cached, input_uncached_tokens=uncached))
        if message.get('stop_reason') in ('end_turn', 'stop_sequence'):
            turn['complete'] = True

    def rows(self):
        rows = []
        linked = {}
        for turn in self.state['turns'].values():
            for client in turn['clients']:
                linked.setdefault((turn['harness'], turn['session'], client), []).append(turn)
        for identity, wake in sorted(self.wakes.items()):
            matches = linked.get((wake['harness'], wake['session'], wake['attempt']), [])
            reason = 'unknown; no exact native message/turn linkage'
            tokens = dict.fromkeys(('input_tokens', 'input_cached_tokens', 'input_uncached_tokens'))
            complete, native_turn = False, None
            if len(matches) == 1 and not wake.get('conflict'):
                turn = matches[0]
                native_turn = turn['turn']
                if turn['clients'] == [wake['attempt']] and not turn.get('unknown_client'):
                    observations = list(turn['requests'].values())
                    for key in tokens:
                        if observations and all(type(value.get(key)) is int for value in observations):
                            tokens[key] = sum(value[key] for value in observations)
                    complete = turn['complete']
                    reason = ('exact native message and turn metadata' if tokens['input_tokens'] is not None
                              else 'unknown; linked turn has missing/conflicting usage')
                else:
                    reason = 'unknown; turn contains multiple or unidentified inputs'
            elif len(matches) > 1 or wake.get('conflict'):
                reason = 'unknown; conflicting native message/turn linkage'
            value = dict(kind='wake-usage', wake_id=identity, native_turn=native_turn,
                         token_attribution=reason, complete=complete, **tokens)
            # Unknown is reported once, then revised when later metadata arrives.
            if self.state['last'].get(identity) != value:
                self.state['last'][identity] = value
                rows.append(dict(value, at=self.now(), id='wake-usage:' + self.digest(value)))
        return rows


def number(values, key):
    value = values.get(key)
    return value if type(value) is int and value >= 0 else None
