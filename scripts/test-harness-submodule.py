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
    def test_converter_secret_shapes_fail_without_exposing_values(self):
        harness.validate_converter_secret('synthetic public fixture')
        for seed in ['sk-' + 'x' * 24, 'ghp_' + 'x' * 32,
                     'api_key=' + 'a' * 64, '-----BEGIN ' + 'RSA PRIVATE KEY-----']:
            with self.assertRaisesRegex(ValueError, 'value suppressed') as error:
                harness.validate_converter_secret(seed)
            self.assertNotIn(seed, str(error.exception))

    def test_real_resolution_includes_converter_direction_and_d4949(self):
        harness.check_resolution(Path(__file__).resolve().parent.parent)

    def test_converter_upward_and_banned_transitive_dependency_controls(self):
        def graph(name, source=None):
            return {'packages': [{'id': 'lc', 'name': harness.CONVERTER},
                                 {'id': 'middle', 'name': 'synthetic-helper'},
                                 {'id': 'bad', 'name': name, 'source': source}],
                    'resolve': {'nodes': [{'id': 'lc', 'dependencies': ['middle']},
                                          {'id': 'middle', 'dependencies': ['bad']},
                                          {'id': 'bad', 'dependencies': []}]}}
        self.assertEqual(len(harness.validate_converter_dependencies(graph('serde'))), 3)
        for name in ['bbt-mux', 'bboxy450-term-mux', 'bboxy450-term', 'bboxy450-harness']:
            with self.assertRaisesRegex(ValueError, 'forbidden upward dependency'):
                harness.validate_converter_dependencies(graph(name))
        for name in harness.DENIED_CONVERTERS:
            with self.assertRaisesRegex(ValueError, 'd4949 denied dependency'):
                harness.validate_converter_dependencies(graph(name))
        with self.assertRaisesRegex(ValueError, 'd4949 denied dependency'):
            harness.validate_converter_dependencies(graph('innocent-alias', 'git+https://github.com/ctxrs/ctx'))

    def test_converter_copied_code_and_target_alias_controls(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            crate = root / 'crates' / harness.CONVERTER
            (crate / 'src').mkdir(parents=True)
            manifest = crate / 'Cargo.toml'
            manifest.write_text('[package]\nname="bboxy450-log-converter"\n')
            source = crate / 'src/lib.rs'
            source.write_text('// Inspired by ctx; independently implemented.\n')
            harness.validate_converter_sources(root)
            for verb in ['Copied', 'Adapted', 'Translated']:
                source.write_text('// ' + verb + ' from ctx\n// https://github.com/ctxrs/ctx\n')
                with self.assertRaisesRegex(ValueError, 'd4949 copied origin'):
                    harness.validate_converter_sources(root)
            source.write_text('')
            manifest.write_text('[target.\'cfg(unix)\'.dev-dependencies]\nrenamed={package="casr", version="1"}\n')
            with self.assertRaisesRegex(ValueError, 'd4949 denied manifest dependency'):
                harness.validate_converter_sources(root)

    def test_real_parent_json_features_are_isolated_and_acp_is_rejected(self):
        root = Path(__file__).resolve().parent.parent
        harness.check_json_features(root)
        rejected = subprocess.run([sys.executable, '-I', str(Path(__file__).with_name('harness-submodule.py')),
                                   '--check-json-features', '--acp-control'], cwd=root, text=True, capture_output=True)
        self.assertEqual(rejected.returncode, 1, rejected.stdout + rejected.stderr)
        self.assertIn('serde_json/preserve_order', rejected.stderr)
        self.assertIn('bboxy450-harness/acp', rejected.stderr)
        print(f'harness: real ACP metadata control exits 1: {rejected.stderr.splitlines()[-1]}', flush=True)

    def test_missing_json_feature_inventory_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'feature inventory is missing'):
            harness.validate_json_features({'packages': [], 'resolve': {'nodes': []}})

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
