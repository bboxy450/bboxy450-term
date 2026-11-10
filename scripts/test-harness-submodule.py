#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Pinned-submodule admission and model-omission negative controls."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('harness_submodule', Path(__file__).with_name('harness-submodule.py'))
harness = importlib.util.module_from_spec(spec)
spec.loader.exec_module(harness)


class SubmoduleTests(unittest.TestCase):
    def setUp(self):
        # Admission mocks exercise the default pinned path even when the parent
        # gate is explicitly testing a signed private candidate.
        env = patch.dict(harness.os.environ, {'BBOXY_HARNESS_CANDIDATE': ''})
        env.start()
        self.addCleanup(env.stop)

    def test_real_parent_acp_agent_features_are_explicit_and_legacy_acp_is_rejected(self):
        root = Path(__file__).resolve().parent.parent
        harness.check_json_features(root)
        rejected = subprocess.run([sys.executable, '-I', str(Path(__file__).with_name('harness-submodule.py')),
                                   '--check-json-features', '--acp-control'], cwd=root, text=True, capture_output=True)
        self.assertEqual(rejected.returncode, 1, rejected.stdout + rejected.stderr)
        self.assertIn('bboxy450-harness/acp', rejected.stderr)
        print(f'harness: real ACP consumer control exits 1: {rejected.stderr.splitlines()[-1]}', flush=True)

    def test_missing_json_feature_inventory_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'feature inventory is missing'):
            harness.validate_json_features({})

    def test_unexpected_ordering_feature_without_acp_agent_is_rejected(self):
        data = {'serde_json': {'preserve_order'}, 'bboxy450-harness': {'embedded'}}
        with self.assertRaisesRegex(ValueError, 'serde_json/preserve_order'):
            harness.validate_json_features(data)
        data['bboxy450-harness'].add('acp-agent')
        self.assertEqual(harness.validate_json_features(data), ['preserve_order'])

    def test_workspace_acp_default_does_not_leak_into_consumer_features(self):
        workspace = {'packages': [{'id': 'h', 'name': 'bboxy450-harness'}],
                     'resolve': {'nodes': [{'id': 'h', 'features': ['default', 'acp', 'acp-agent', 'embedded']}]}}
        consumer = 'bboxy450-term v0.1.1|default\nbboxy450-harness v0.1.0 (/own path/harness)|embedded\nserde_json v1.0.154|default,std\n'
        with patch.object(harness, 'workspace_metadata', return_value=workspace) as metadata, patch.object(subprocess, 'check_output', return_value=consumer) as tree:
            self.assertEqual(harness.check_json_features(Path('/own path')), ['default', 'std'])
        metadata.assert_not_called()
        self.assertEqual(tree.call_args.args[0], ['cargo', 'tree', '-p', 'bboxy450-term', '--locked', '-e', 'normal,build',
                                                '--prefix', 'none', '-f', '{p}|{f}'])
        self.assertEqual(tree.call_args.kwargs['cwd'], Path('/own path'))

    def test_consumer_legacy_acp_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'bboxy450-harness/acp'):
            harness.validate_json_features({'serde_json': {'std'}, 'bboxy450-harness': {'embedded', 'acp'}})

    def test_consumer_feature_instances_are_merged_without_dedupe_marker(self):
        consumer = 'bboxy450-harness v0.1.0|embedded,acp-agent\nserde_json v1.0.154|std\nserde_json v1.0.154|std,preserve_order (*)\n'
        with patch.object(subprocess, 'check_output', return_value=consumer):
            self.assertEqual(harness.check_json_features(Path('/own')), ['preserve_order', 'std'])

    def test_malformed_consumer_feature_output_is_rejected(self):
        with patch.object(subprocess, 'check_output', return_value='serde_json std\n'):
            with self.assertRaisesRegex(ValueError, 'malformed consumer feature row'):
                harness.check_json_features(Path('/own'))

    def test_missing_checkout_fails_after_initialization_instead_of_skipping(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=["crates/bboxy450-harness"]\n')
            with patch.object(subprocess, 'check_output', side_effect=[harness.URL + '\n', '160000 ' + '1' * 40 + ' 0\t' + harness.MEMBER + '\n']), patch.object(subprocess, 'run') as update:
                with self.assertRaisesRegex(ValueError, 'checkout is missing'):
                    harness.initialize(root)
                self.assertEqual(update.call_args.args[0][1:5], ['submodule', 'update', '--init', '--recursive'])

    def test_non_gitlink_or_foreign_url_refused_before_fetch(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=["crates/bboxy450-harness"]\n')
            for url, entry in [('https://example.invalid/foreign', '160000 abc 0\tx'), (harness.URL, '100644 abc 0\tx'), (harness.URL, '')]:
                with patch.object(subprocess, 'check_output', side_effect=[url, entry]), patch.object(subprocess, 'run') as update:
                    with self.assertRaisesRegex(ValueError, 'unapproved pinned submodule'):
                        harness.initialize(root)
                    update.assert_not_called()

    def test_generated_evidence_is_retained_but_tracked_evidence_is_never_moved(self):
        with tempfile.TemporaryDirectory() as own:
            child, artifacts = Path(own) / 'child', Path(own) / 'artifacts'
            evidence = child / 'verification/evidence/hash'
            evidence.mkdir(parents=True)
            (evidence / 'evidence.json').write_text('{"graph_equal": true}')
            with patch.object(subprocess, 'check_output', return_value=b'tracked/evidence'):
                with self.assertRaisesRegex(ValueError, 'refuse to move tracked'):
                    harness.archive_generated_evidence(child, artifacts)
            self.assertTrue(evidence.is_dir())
            self.assertFalse(artifacts.exists())
            with patch.object(subprocess, 'check_output', return_value=b''):
                harness.archive_generated_evidence(child, artifacts)
            self.assertEqual((artifacts / 'formal-evidence/hash/evidence.json').read_text(), '{"graph_equal": true}')
            self.assertFalse((child / 'verification/evidence').exists())

    def test_missing_control_duplicate_or_wrong_verdict_refuses_import(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            inventory = root / harness.MEMBER / 'verification'
            inventory.mkdir(parents=True)
            (inventory / 'corpus.toml').write_text('[[model]]\nname="HarnessEmbedLifecycle"\nexpected="pass"\n[[model]]\nname="BrokenHarnessEmbedLifecycle"\nexpected="safety"\n')
            source = root / 'native'
            source.mkdir()
            for text in ['HarnessEmbedLifecycle\tpass\t1\t1\n', 'HarnessEmbedLifecycle\tpass\t1\t1\n' * 2, 'HarnessEmbedLifecycle\tpass\t1\t1\nBrokenHarnessEmbedLifecycle\tpass\t1\t1\n']:
                (source / 'manifest.tsv').write_text(text)
                with self.assertRaisesRegex(ValueError, 'native model'):
                    harness.import_models(root, source, root / 'target')


if __name__ == '__main__':
    unittest.main()
