"""Regression for a hook review that appears after Codex's initial ready prompt."""
import importlib
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
with patch.dict(sys.modules, {'native_checks': types.SimpleNamespace()}):
    turn = importlib.import_module('native_turn')


class NativeTurnTests(unittest.TestCase):
    def test_late_review_is_approved_once_before_the_single_answer_marker(self):
        screens = [{'rows': [text]} for text in [
            'Ask Codex to do anything', 'Hooks need review',
            'Hooks need review', 'Reply L3_NATIVE_OK. Use no tools.', '• T6_BEFORE']]
        app = types.SimpleNamespace(ctl=Mock(side_effect=screens))
        approve = Mock(return_value=True)
        with patch.object(turn.native, 'approve_codex_hooks', approve, create=True), \
                patch.object(turn.time, 'sleep'):
            self.assertEqual(turn.wait_codex_turn(app, 'seat', 'T6_BEFORE', 't6'), screens[-1])
        approve.assert_called_once_with('seat', marker='T6_BEFORE', prefix='t6')
        self.assertEqual(app.ctl.call_count, 5)

    def test_an_unrelated_prompt_times_out_without_approval(self):
        app = types.SimpleNamespace(ctl=Mock(return_value={'rows': ['Sign in to continue']}))
        approve = Mock()
        with patch.object(turn.native, 'approve_codex_hooks', approve, create=True), \
                patch.object(turn.time, 'monotonic', side_effect=[0, 31]):
            with self.assertRaisesRegex(AssertionError, 'Codex turn T9_BEFORE'):
                turn.wait_codex_turn(app, 'seat', 'T9_BEFORE', 't9')
        approve.assert_not_called()

    def test_the_existing_hook_recipe_refusal_propagates(self):
        app = types.SimpleNamespace(ctl=Mock(return_value={'rows': ['Hooks need review']}))
        approve = Mock(side_effect=AssertionError('unexpected hook count'))
        with patch.object(turn.native, 'approve_codex_hooks', approve, create=True):
            with self.assertRaisesRegex(AssertionError, 'unexpected hook count'):
                turn.wait_codex_turn(app, 'seat', 'T6_AFTER', 't6-after')
        self.assertEqual(app.ctl.call_count, 1)


if __name__ == '__main__':
    unittest.main()
