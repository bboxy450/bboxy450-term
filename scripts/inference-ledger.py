"""Nightly #333 usage accounting. No message/tool/think text is inspected or saved.

Reads harness usage and timing metadata, writes only under the private ledger.
Launch-time Git snapshots are not evidence for a long-lived session's later cuts.
Optional ledger/inference-bindings.json is a list of explicit, non-overlapping
{harness, session, agent, start, end, branch, sha} intervals (UTC timestamps).
Without a binding, the cut stays unknown. Never look up today's worktree HEAD.
"""
import argparse
from collections import defaultdict
from datetime import datetime, timedelta, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('resource_ledger', Path(__file__).with_name('resource-ledger.py'))
ledger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ledger)
TOKENS = ('input_cached_tokens', 'input_uncached_tokens', 'output_reasoning_tokens', 'output_visible_tokens')
IDLE = 600  # agent_time.py's short-gap rule; an estimate, not server compute time.


def stamp(value):
    value = datetime.fromisoformat(value.replace('Z', '+00:00'))
    if value.tzinfo is None:
        raise ValueError('timestamp needs a timezone')
    return value.astimezone(timezone.utc)


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def metadata(pairs):
    # Drop bodies at JSON decode, including API request blobs and instructions.
    # The reader never indexes their text or classifies a tool by its arguments.
    excluded = {'content', 'text', 'summary', 'base_instructions', 'instructions',
                'developer_instructions', 'input', 'arguments', 'output', 'params',
                'last_agent_message', 'hookAdditionalContext', 'data', 'blob'}
    return {k: v for k, v in pairs if k not in excluded}


def number(values, key):
    value = values.get(key)
    return value if type(value) is int and value >= 0 else None


def normalized(harness, values):
    if harness == 'codex':
        total, cached = number(values, 'input_tokens'), number(values, 'cached_input_tokens')
        output, reasoning = number(values, 'output_tokens'), number(values, 'reasoning_output_tokens')
        uncached = total - cached if total is not None and cached is not None and total >= cached else None
    else:
        fresh, creation = number(values, 'input_tokens'), number(values, 'cache_creation_input_tokens')
        cached = number(values, 'cache_read_input_tokens')
        uncached = fresh + creation if fresh is not None and creation is not None else None
        output = number(values, 'output_tokens')
        reasoning = number(values.get('output_tokens_details') or {}, 'thinking_tokens')
    visible = output - reasoning if output is not None and reasoning is not None and output >= reasoning else None
    if visible is None:
        reasoning = None
    return dict(zip(TOKENS, (cached, uncached, reasoning, visible)))


def private_json(path, data):
    temporary = path.with_suffix('.tmp')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, 'w') as stream:
        json.dump(data, stream, separators=(',', ':'))
    os.replace(temporary, path)


def recover(directory):
    """Replay an interrupted append by stable row id before advancing offsets."""
    pending = directory / 'inference-pending.json'
    if not pending.exists():
        return
    batch = json.loads(pending.read_text())
    known = {}
    for row in batch['rows']:
        day = row['at'][:10]
        if day not in known:
            path = directory / day / 'ledger.jsonl'
            known[day] = set()
            if path.exists():
                with path.open() as stream:
                    for line in stream:
                        try:
                            known[day].add(json.loads(line).get('id'))
                        except ValueError:
                            pass
        if row['id'] not in known[day]:
            ledger.append(row)
            known[day].add(row['id'])
    private_json(directory / 'inference-progress.json', batch['progress'])
    pending.unlink()


class Reader:
    def __init__(self, progress, project, bindings):
        self.progress, self.project, self.bindings = progress, project, bindings
        self.rows = {}
        self.issues = defaultdict(int)

    def belongs(self, cwd):
        if not isinstance(cwd, str):
            return False
        path = Path(cwd)
        return path.is_relative_to(self.project) or (path.parent == self.project.parent and
            path.name.startswith(self.project.name + '-'))

    def attribution(self, harness, session, at, cwd, branch=None):
        matched = [b for b in self.bindings if b['harness'] == harness and b['session'] == session
                   and stamp(b['start']) <= at < stamp(b['end'])]
        if len(matched) == 1:
            b = matched[0]
            return b['agent'], dict(branch=b.get('branch'), sha=b.get('sha')), 'explicit time-bounded binding'
        if matched:
            self.issues['ambiguous_binding'] += 1
            return 'unattributed', dict(branch=None, sha=None), 'conflicting explicit bindings'
        agent = 'worktree:' + cwd if cwd and Path(cwd) != self.project else 'unattributed'
        if harness == 'claude' and isinstance(branch, str) and branch and branch != 'HEAD':
            return agent, dict(branch=branch, sha=None), 'per-message transcript branch; SHA unavailable'
        return agent, dict(branch=None, sha=None), 'cut unattributed; no unambiguous contemporaneous binding'

    def charge(self, harness, context, at, tokens=None, wall=None, ended=None):
        if not self.belongs(context.get('cwd')):
            return
        agent, cut, basis = self.attribution(harness, context['session'], at, context['cwd'], context.get('branch'))
        key = json.dumps([at.date().isoformat(), harness, agent, cut], sort_keys=True)
        if key not in self.rows:
            self.rows[key] = dict(kind='stage', stage='inference', at=at.isoformat(), ended=at.isoformat(),
                agent=agent, cut=cut, harness=harness, attribution=basis,
                tokens={k: 0 for k in TOKENS}, token_unknown_observations={k: 0 for k in TOKENS},
                wall_seconds=0., wall_observations=0, cpu_user_seconds=None, cpu_system_seconds=None,
                peak_rss_bytes=None, disk_bytes_written=None, target_bytes_after=None, exit=None,
                token_accounting=('canonical request ledger cumulative deltas; legacy display totals only until available'
                    if harness == 'codex' else 'per-message usage deltas; visible corrections signed; cache creation is uncached input'),
                wall_accounting=('short timestamp gaps <600s without open tool ids; estimate, async tool time may leak'
                    if harness == 'codex' else 'request id/ref timestamp span; API wall, not server compute time'))
        row = self.rows[key]
        row['at'] = min(row['at'], at.isoformat())
        row['ended'] = max(row['ended'], (ended or at).isoformat())
        if tokens is not None:
            for k, value in tokens.items():
                if value is None:
                    row['token_unknown_observations'][k] += 1
                else:
                    row['tokens'][k] += value
        if wall is not None:
            row['wall_seconds'] += wall
            row['wall_observations'] += 1

    def wall(self, harness, context, start, end):
        """Intervals crossing midnight or a binding are charged on each side."""
        points = {start, end}
        midnight = (start + timedelta(days=1)).replace(hour=0, minute=0, second=0, microsecond=0)
        while midnight < end:
            points.add(midnight)
            midnight += timedelta(days=1)
        for binding in self.bindings:
            if binding['harness'] == harness and binding['session'] == context['session']:
                for name in ('start', 'end'):
                    boundary = stamp(binding[name])
                    if start < boundary < end:
                        points.add(boundary)
        points = sorted(points)
        for left, right in zip(points, points[1:]):
            self.charge(harness, context, left, wall=(right-left).total_seconds(), ended=right)

    def codex(self, d, context, at):
        p = d.get('payload') or {}
        kind, event = d.get('type'), p.get('type')
        old_cwd = context.get('cwd')
        if kind == 'session_meta':
            context.update(session=p.get('id') or p.get('session_id'), cwd=p.get('cwd'))
        elif kind == 'turn_context' and p.get('cwd'):
            context['cwd'] = p['cwd']
        if not context.get('session') or not self.belongs(context.get('cwd')):
            return
        if old_cwd and old_cwd != context.get('cwd'):
            context.pop('previous', None)
            context.pop('open', None)
        previous = context.get('previous')
        wall_key = 'codex-wall:' + context['session']
        high = self.progress.setdefault('wall', {}).get(wall_key)
        if previous and not context.get('open'):
            start = stamp(previous)
            gap = (at - start).total_seconds()
            if 0 < gap < IDLE:
                start = max(start, stamp(high)) if high else start
                if start < at:
                    self.wall('codex', context, start, at)
        if high is None or at > stamp(high):
            self.progress['wall'][wall_key] = at.isoformat()
        context['previous'] = at.isoformat()
        opened = context.setdefault('open', [])
        if kind == 'response_item':
            call = p.get('call_id')
            if event in ('function_call', 'custom_tool_call') and call and call not in opened:
                opened.append(call)
            elif event in ('function_call_output', 'custom_tool_call_output') and call in opened:
                opened.remove(call)
        if kind == 'event_msg' and event in ('task_complete', 'task_started', 'turn_aborted'):
            opened.clear()
        formats = self.progress.setdefault('formats', {})
        if kind == 'token_usage_record':
            formats[context['session']] = 'request-ledger'
        elif event == 'token_count' and formats.get(context['session']) == 'request-ledger':
            return  # Display totals can omit usage included by the request ledger.
        values = p.get('thread_token_usage') if kind == 'token_usage_record' else (
            (p.get('info') or {}).get('total_token_usage') if event == 'token_count' else None)
        if not isinstance(values, dict):
            return
        key = 'codex:' + context['session']
        before = self.progress['usage'].get(key, {})
        # Subtract raw counters before splitting cached/visible to avoid
        # subtracting a different request's cache mix. Display events are a
        # legacy fallback, never added to the canonical request ledger.
        raw, delta = {}, {}
        for k in ('input_tokens', 'cached_input_tokens', 'output_tokens', 'reasoning_output_tokens'):
            value = number(values, k)
            old = before.get(k, 0)
            raw[k] = max(old, value) if value is not None else old
            delta[k] = value - old if value is not None and value >= old else None
            if value is not None and value < old:
                self.issues['codex_counter_regression'] += 1
        self.progress['usage'][key] = raw
        if raw != before:
            self.charge('codex', context, at, normalized('codex', delta))

    def claude(self, d, context, at):
        if d.get('cwd'):
            context['cwd'] = d['cwd']
        context['session'] = d.get('sessionId') or context.get('session')
        if not context.get('session'):
            return
        starts = context.setdefault('requests', {})
        if d.get('type') == 'api-request' and d.get('id'):
            starts[d['id']] = at.isoformat()
        if d.get('type') != 'assistant':
            return
        context['branch'] = d.get('gitBranch')  # Unlike Codex's stale launch snapshot, this is per message.
        if not self.belongs(context.get('cwd')):
            return
        message = d.get('message') or {}
        if not message.get('id') or not isinstance(message.get('usage'), dict):
            return
        key = 'claude:' + context['session'] + ':' + message['id']
        before = self.progress['usage'].get(key, {})
        values = normalized('claude', message['usage'])
        totals, delta = {}, {}
        for k, value in values.items():
            old = before.get(k, 0)
            # A later block may fill in thinking usage, moving already billed
            # output from visible to reasoning. Preserve that signed revision.
            revise = k == 'output_visible_tokens'
            totals[k] = (value if revise else max(old or 0, value)) if value is not None else old or None
            delta[k] = (value - (old or 0)) if value is not None else None
            if delta[k] is not None and not revise:
                delta[k] = max(0, delta[k])
        start = starts.get(d.get('requestRef'))
        wall = max(0., (at - stamp(start)).total_seconds()) if start else None
        old_wall = before.get('wall_seconds')
        totals['wall_seconds'] = max(old_wall or 0., wall) if wall is not None else old_wall
        if totals != before:
            self.charge('claude', context, at, delta)
            if wall is not None and wall > (old_wall or 0.):
                self.wall('claude', context, stamp(start) + timedelta(seconds=old_wall or 0.), at)
            self.progress['usage'][key] = totals
        if wall is None and not before:
            self.issues['claude_request_start_unavailable'] += 1

    def read(self, harness, path, deadline):
        identity = digest([harness, str(path)])
        stat = path.stat()
        before = self.progress['files'].get(identity, {})
        same = before.get('inode') == [stat.st_dev, stat.st_ino] and before.get('offset', 0) <= stat.st_size
        context = dict(before) if same else {}
        context['inode'] = [stat.st_dev, stat.st_ino]
        with path.open('rb') as stream:
            stream.seek(context.get('offset', 0))
            while stream.tell() < stat.st_size and time.monotonic() < deadline:
                offset = stream.tell()
                line = stream.readline(stat.st_size - offset)
                if not line.endswith(b'\n'):
                    break  # A writer's incomplete record is read next night.
                context['offset'] = stream.tell()
                try:
                    d = json.loads(line, object_pairs_hook=metadata)
                    if not isinstance(d, dict) or not d.get('timestamp'):
                        continue  # Ordinary headers/last-prompt rows have no time.
                    at = stamp(d['timestamp'])
                    (self.codex if harness == 'codex' else self.claude)(d, context, at)
                except (ValueError, KeyError, TypeError, AttributeError):
                    self.issues['invalid_metadata_record'] += 1
        self.progress['files'][identity] = context


def collect(codex_root, claude_root, project, seconds=120):
    directory = ledger.state_dir() / 'ledger'
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    # An overlapping observational run skips immediately; no workload waits.
    fd = os.open(directory / 'inference.lock', os.O_CREAT | os.O_WRONLY, 0o600)
    with os.fdopen(fd, 'w') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return dict(kind='inference-summary', skipped='reader already active')
        recover(directory)
        checkpoint = directory / 'inference-progress.json'
        progress = json.loads(checkpoint.read_text()) if checkpoint.exists() else dict(files={}, usage={})
        bindings_path = directory / 'inference-bindings.json'
        bindings = json.loads(bindings_path.read_text()) if bindings_path.exists() else []
        before = digest(progress)
        reader = Reader(progress, project, bindings)
        deadline = time.monotonic() + seconds
        sources = [('codex', codex_root.glob('**/*.jsonl')),
                   ('claude', claude_root.glob('**/*.jsonl'))]
        for index, (harness, paths) in enumerate(sources):
            harness_deadline = time.monotonic() + max(0., deadline-time.monotonic()) / (len(sources)-index)
            for path in sorted(paths):
                if time.monotonic() >= harness_deadline:
                    reader.issues['scan_budget_exhausted'] += 1
                    break
                try:
                    reader.read(harness, path, harness_deadline)
                except OSError:
                    reader.issues['source_unavailable'] += 1
        batch = digest([before, progress])
        rows = []
        for key, row in sorted(reader.rows.items()):
            row['id'] = 'inference:' + digest([batch, key])
            row['token_known_lower_bounds'] = dict(row['tokens'])
            for k, count in row['token_unknown_observations'].items():
                if count:
                    row['tokens'][k] = None
            if row['wall_observations'] == 0:
                row['wall_seconds'] = None
            rows.append(row)
        summary = dict(kind='inference-summary', rows=len(rows), files_seen=len(progress['files']),
                       issues=reader.issues)
        rows.append(dict(summary, at=ledger.now(), id='inference-import:' + batch))
        private_json(directory / 'inference-pending.json', dict(rows=rows, progress=progress))
        recover(directory)
        return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--codex-root', type=Path, default=Path.home() / '.codex/sessions')
    parser.add_argument('--claude-root', type=Path, default=Path.home() / '.claude/projects')
    parser.add_argument('--project', type=Path)
    parser.add_argument('--max-seconds', type=float, default=120)
    args = parser.parse_args()
    try:
        project = args.project or Path(ledger.output(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'])).parent
        print(json.dumps(collect(args.codex_root, args.claude_root, project.resolve(), args.max_seconds)))
    except Exception as error:
        print('ledger: inference unavailable (' + type(error).__name__ + '); continuing', file=sys.stderr)
    return 0  # Accounting adds no correctness gate or operator interaction.


if __name__ == '__main__':
    sys.exit(main())
