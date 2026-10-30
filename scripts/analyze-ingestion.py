#!/usr/bin/env python3
"""Source-bound R5 input/ingestion evidence; inclusive spans never imply exclusive CPU.

bboxy450-term — https://github.com/bboxy450/bboxy450-term
LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""
import argparse
import hashlib
import json
from pathlib import Path


class Unproven(ValueError):
    pass


def require(condition, reason):
    if not condition:
        raise Unproven(reason)


def analyze(report, capture, seats=967, watched=50):
    require(report.get('kind') == 'real-pty', 'real_pty_report_required')
    require(report.get('rows') == report.get('real_ptys') == seats
            and report.get('watched') == watched, 'fixture_count_mismatch')
    pids = report.get('owned_producer_pids', [])
    require(len(pids) == len(set(pids)) == seats and all(type(p) is int and p > 0 for p in pids),
            'owned_producer_set_incomplete')
    metrics = report['metrics']
    diagnostics = metrics['diagnostics']
    require(diagnostics.get('enabled'), 'probe_disabled')
    probe = diagnostics['probe']
    stamps = probe['stamps']
    order = ['Admitted', 'Written', 'LogicOutputRead', 'GuiReceived', 'ModelPublished',
             'LatestTaken', 'ProjectionFinished', 'Encoded', 'GpuComplete']
    require(all(type(stamps.get(k)) is int and stamps[k] > 0 for k in order),
            'authoritative_probe_endpoint_missing')
    require(all(stamps[a] <= stamps[b] for a, b in zip(order, order[1:])),
            'probe_endpoints_reordered')
    require(probe['id'] > 0 and probe['ring_sequence'] > 0, 'uncorrelated_probe')
    source = [name for name in ('LogicRingRead', 'LogicLiveEvent')
              if stamps.get(name) == stamps['LogicOutputRead']]
    require(source, 'ingestion_read_source_missing')
    seat_number = int(probe['seat_u128'])
    require(0 <= seat_number < 2**128, 'probe_seat_invalid')
    alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
    seat = ''.join(alphabet[(seat_number >> (5 * i)) & 31] for i in reversed(range(26)))
    cuts, windows = {}, {}
    names = ['capture_at_measure', 'capture_at_stop', 'capture_at_settled']
    names += [n for n in ('capture_at_probe_admitted', 'capture_at_probe_read') if capture.get(n)]
    for name in names:
        owner = capture[name]['logic']
        require(owner.get('enabled'), 'capture_owner_disabled')
        rows = [s for s in owner['seats'] if s['seat'] == seat and s['generation'] == probe['generation']]
        require(len(rows) == 1, 'probe_generation_cursor_missing')
        row = rows[0]
        cuts[name] = {k: row[k] for k in ('observed_ns', 'notified_sequence', 'target_sequence',
                                        'captured_sequence', 'committed_sequence', 'applied_sequence',
                                        'parsed_byte_end', 'checkpoint_sequence', 'checkpoint_paused')}
        cuts[name]['captured_minus_applied_sequences'] = row['captured_sequence'] - row['applied_sequence']
        windows[name] = {side: owner_window(data) for side, data in capture[name].items()
                         if side in ('logic', 'supervisor') and data}
    def leg(a, b):
        return (stamps[b] - stamps[a]) / 1e6
    total = leg('Admitted', 'GpuComplete')
    require(abs(total - metrics['input_to_gpu_complete_ms']) < 0.001,
            'reported_latency_disagrees_with_endpoints')
    budget = metrics['input_budget_ms']
    require(type(budget) in (int, float) and 0 < budget <= 26.666668,
            'input_budget_weakened_or_invalid')
    return {'probe': {'seat': seat, 'generation': probe['generation'], 'id': probe['id'],
                      'ring_sequence': probe['ring_sequence'], 'read_source': source},
            'legs_ms': {a + '->' + b: leg(a, b) for a, b in zip(order, order[1:])},
            'input_to_gpu_ms': total, 'input_budget_ms': budget,
            'input_budget_met': total <= budget,
            'ingestion_budget_ms': 8.333333, 'ingestion_budget_met': leg('Written', 'LogicOutputRead') <= 8.333333,
            'timing_gated': report['timing_gated'], 'host_load_before': report['host_load_before'],
            'fixture_failures': report['failures'], 'cursor_cuts': cuts,
            'owner_windows': windows,
            'coverage_note': 'R5i probe stamps and capture-owner timestamps have independent origins. No cross-clock subtraction or probe-window overlap is inferred. Each owner window summarizes its entire retained span buffer, which can include work outside the probe. Spans can be overwritten/missed; absence is unknown. Durations are inclusive and cannot be added as exclusive CPU. Named cuts are causal observations, not atomic cross-process snapshots.',
            'interfaces': ['.bboxy/interfaces/supervisor-logic.md', '.bboxy/interfaces/logic-gui.md']}


def owner_window(owner):
    phases = {}
    turns = []
    for span in owner['spans']:
        phase = phases.setdefault(span['phase'], {'completed_spans': 0, 'inclusive_ms': 0, 'max_span_ms': 0})
        phase['completed_spans'] += 1
        phase['inclusive_ms'] += span['elapsed_ns'] / 1e6
        phase['max_span_ms'] = max(phase['max_span_ms'], span['elapsed_ns'] / 1e6)
        if span['phase'] == 'logic_turn':
            turns.append(span)
    return {'process_id': owner['process_id'],
            'clock': 'capture-owner origin; independent of R5i probe stamps',
            'sampled_ns': owner.get('sampled_ns'),
            'retained_spans': len(owner['spans']),
            'recorder': {k: owner[k] for k in ('missed', 'overwritten_spans', 'untracked_seat_samples')},
            'phases': phases,
            'retained_queue_max': ({'pending': max(s['pending'] for s in turns),
                                    'ready': max(s['ready'] for s in turns)} if turns else None)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('report', type=Path)
    parser.add_argument('--capture', type=Path)
    parser.add_argument('--seats', type=int, default=967)
    parser.add_argument('--watched', type=int, default=50)
    args = parser.parse_args()
    capture_path = args.capture or args.report.with_suffix('.capture.json')
    try:
        result = analyze(json.loads(args.report.read_text()), json.loads(capture_path.read_text()),
                         args.seats, args.watched)
        result['source_sha256'] = {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                   for p in (args.report, capture_path, Path(__file__))}
        print(json.dumps(result, sort_keys=True))
        return 0 if (result['input_budget_met'] and result['ingestion_budget_met']
                     and result['timing_gated'] and not result['fixture_failures']) else 1
    except (Unproven, KeyError, TypeError, ValueError, OSError) as error:
        print(json.dumps({'status': 'unproven', 'reason': str(error)}))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
