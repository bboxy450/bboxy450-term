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
