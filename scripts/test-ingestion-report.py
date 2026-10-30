"""Refusal controls for the R5 ingestion evidence reader.

bboxy450-term — https://github.com/bboxy450/bboxy450-term
LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""
import copy
import sys
sys.dont_write_bytecode = True
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('ingestion', Path(__file__).with_name('analyze-ingestion.py'))
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


def fixture():
    names = ['Admitted', 'Written', 'LogicOutputRead', 'GuiReceived', 'ModelPublished',
             'LatestTaken', 'ProjectionFinished', 'Encoded', 'GpuComplete']
    stamps = {name: (i + 1) * 1_000_000 for i, name in enumerate(names)}
    stamps['LogicRingRead'] = stamps['LogicOutputRead']
    probe = {'id': 1, 'ring_sequence': 3, 'seat_u128': '1', 'generation': 1, 'stamps': stamps}
    report = {'kind': 'real-pty', 'rows': 967, 'real_ptys': 967, 'watched': 50,
              'owned_producer_pids': list(range(1, 968)), 'timing_gated': True,
              'host_load_before': [1, 1, 1], 'failures': [],
              'metrics': {'input_to_gpu_complete_ms': 8, 'input_budget_ms': 26.666667,
                          'diagnostics': {'enabled': True, 'process_id': 42, 'probe': probe}}}
    row = dict(seat='00000000000000000000000001', generation=1, observed_ns=1,
               notified_sequence=3, target_sequence=3, captured_sequence=3,
               committed_sequence=3, applied_sequence=1, parsed_byte_end=20,
               checkpoint_sequence=0, checkpoint_paused=False)
    owner = dict(enabled=True, process_id=42, seats=[row], missed=1, overwritten_spans=10,
                 untracked_seat_samples=0, spans=[])
    capture = {name: {'logic': copy.deepcopy(owner)} for name in
               ('capture_at_measure', 'capture_at_stop', 'capture_at_settled')}
    return report, capture


class Evidence(unittest.TestCase):
    def test_missing_spans_are_unknown_and_cursor_domains_remain_distinct(self):
        result = reader.analyze(*fixture())
        self.assertIsNone(result['owner_windows']['capture_at_stop']['logic']['retained_queue_max'])
        self.assertEqual(result['owner_windows']['capture_at_stop']['logic']['phases'], {})
        self.assertEqual(result['cursor_cuts']['capture_at_stop']['captured_minus_applied_sequences'], 2)
        self.assertTrue(result['input_budget_met'])
        self.assertTrue(result['ingestion_budget_met'])
        r, c = fixture()
        c['capture_at_stop']['logic']['seats'][0]['captured_sequence'] = 0
        self.assertEqual(reader.analyze(r,c)['cursor_cuts']['capture_at_stop']['captured_minus_applied_sequences'], -1)

    def test_missing_or_reordered_endpoints_refuse(self):
        for stage in ('Written', 'LogicOutputRead', 'GpuComplete'):
            r, c = fixture()
            r['metrics']['diagnostics']['probe']['stamps'][stage] = None
            with self.assertRaises(reader.Unproven):
                reader.analyze(r, c)
        r, c = fixture()
        r['metrics']['diagnostics']['probe']['stamps']['Written'] = 9_000_000
        with self.assertRaises(reader.Unproven):
            reader.analyze(r, c)

    def test_identity_count_budget_and_summary_controls_refuse(self):
        for mutation in ('count', 'budget', 'latency', 'generation'):
            r, c = fixture()
            if mutation == 'generation':
                c['capture_at_stop']['logic']['seats'][0]['generation'] = 2
            elif mutation == 'count':
                r['owned_producer_pids'].pop()
            elif mutation == 'budget':
                r['metrics']['input_budget_ms'] = 100
            else:
                r['metrics']['input_to_gpu_complete_ms'] = 0
            with self.assertRaises(reader.Unproven, msg=mutation):
                reader.analyze(r, c)

    def test_independent_owner_clocks_are_not_compared_to_probe_stamps(self):
        r, c = fixture()
        c['capture_at_stop']['logic']['process_id'] = 43
        c['capture_at_stop']['logic']['spans'].append(dict(
            phase='read_ring_rpc', turn=1, start_ns=1_000_000_000_000,
            elapsed_ns=1_000_000, seat='00000000000000000000000001'))
        result = reader.analyze(r,c)
        self.assertEqual(result['input_to_gpu_ms'], 8)
        self.assertEqual(result['owner_windows']['capture_at_stop']['logic']['phases']['read_ring_rpc']['inclusive_ms'], 1)
        self.assertNotIn('observed_inclusive_overlap_ms', result)

    def test_overlapping_phases_are_reported_inclusively(self):
        r, c = fixture()
        for phase in ('logic_turn', 'read_ring_rpc'):
            c['capture_at_stop']['logic']['spans'].append(dict(
                phase=phase, turn=1, start_ns=2_000_000, elapsed_ns=1_000_000,
                seat='00000000000000000000000001', pending=50, ready=50))
        result = reader.analyze(r, c)
        self.assertEqual({k:v['inclusive_ms'] for k,v in result['owner_windows']['capture_at_stop']['logic']['phases'].items()}, {'logic_turn': 1, 'read_ring_rpc': 1})
        self.assertEqual(result['owner_windows']['capture_at_stop']['logic']['retained_queue_max'], {'pending': 50, 'ready': 50})


if __name__ == '__main__':
    unittest.main()
