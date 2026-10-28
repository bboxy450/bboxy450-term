#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Disposable Git fixtures prove notice coverage without Cargo or operator input."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

CHECKER = Path(__file__).with_name("source-notices.py")
SPDX = b"LicenseRef-Respectful-Slop-1.0-Draft"
POLICY = b"https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md"


class SourceNotices(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)

    def put(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def run_checker(self, *args, expected=0):
        result = subprocess.run(
            [sys.executable, "-I", str(CHECKER), *args, str(self.root)],
            capture_output=True,
        )
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        return result

    def test_missing_owned_header_fails_then_sync_is_idempotent_and_preserves_body(self):
        body = b"//! A module.\n#![deny(unsafe_code)]\npub fn answer() -> u8 { 42 }\n"
        path = self.put("crates/demo/src/lib.rs", body)
        self.run_checker(expected=1)
        self.run_checker("--sync")
        attached = path.read_bytes()
        self.assertTrue(attached.endswith(body))
        self.assertIn(SPDX, attached.splitlines()[1])
        self.assertIn(POLICY, attached)
        self.run_checker("--check")
        self.run_checker("--sync")
        self.assertEqual(path.read_bytes(), attached)

    def test_wrong_license_or_policy_url_fails_check_without_editing(self):
        path = self.put("crates/demo/src/lib.rs", b"pub fn answer() {}\n")
        self.run_checker("--sync")
        correct = path.read_bytes()
        for original, replacement in [
            (b"/blob/main/LICENSE", b"/blob/main/wrong-license"),
            (b"/blob/main/ATTRIBUTION.md", b"/blob/main/wrong-policy"),
        ]:
            changed = correct.replace(original, replacement)
            path.write_bytes(changed)
            self.run_checker(expected=1)
            self.assertEqual(path.read_bytes(), changed)

    def test_unknown_copyright_or_spdx_refuses_ownership_claim_even_in_sync(self):
        path = self.put("crates/demo/src/lib.rs", b"// Copyright 2020 Someone Else\npub fn other() {}\n")
        for body in [path.read_bytes(), b"// SPDX-License-Identifier: BSD-3-Clause\npub fn other() {}\n"]:
            path.write_bytes(body)
            self.run_checker("--sync", expected=1)
            self.assertEqual(path.read_bytes(), body)

    def test_upstream_keyboard_and_vendor_files_remain_byte_identical(self):
        paths = [
            "crates/bbt-core/src/keyboard.rs",
            "crates/bbt-core/src/vendor/alacritty/term/mod.rs",
            "crates/bbt-core/src/vendor/vte/mod.rs",
            "crates/bboxy-firehose/reference/vendor/xterm.js",
        ]
        body = b"// Copyright 2020 Original contributors\n// SPDX-License-Identifier: Apache-2.0\noriginal();\n"
        for name in paths:
            self.put(name, body)
        self.run_checker("--sync")
        for name in paths:
            self.assertEqual((self.root / name).read_bytes(), body)
        bad = self.root / paths[0]
        bad.write_bytes(b"// SPDX-License-Identifier: " + SPDX + b"\n" + body)
        self.run_checker("--check", expected=1)

    def test_scoped_core_and_fork_headers_preserve_their_original_terms(self):
        cases = {
            "crates/bbt-core/src/lib.rs": b"// SPDX-License-Identifier: Apache-2.0\n#![doc = \"existing\"]\npub fn wrapper() {}\n",
            "crates/bbt-yarn/src/lib.rs": b"// Copyright 2026 Kittens contributors\n#![no_std]\npub fn kernel() {}\n",
            "crates/bbt-yarn-macros/src/lib.rs": b"// SPDX-License-Identifier: MIT OR Apache-2.0\npub fn expand() {}\n",
        }
        for name, body in cases.items():
            self.put(name, body)
        self.run_checker("--sync")
        for name, body in cases.items():
            attached = (self.root / name).read_bytes()
            self.assertTrue(attached.endswith(body))
            self.assertIn(POLICY, attached)
            self.assertNotIn(SPDX, attached)
        self.run_checker()

    def test_mirrors_generated_outputs_and_line_sensitive_fixtures_are_untouched(self):
        cases = {
            "crates/demo/release-inputs/lab/web/components.js": b"const mirror = 42;\n",
            "crates/bbt-yarn/tests/ui/borrow.rs": b"fn main() { fails(); }\n",
            "crates/demo/tests/fixtures/wire.json": b'{"exact":42}',
            "crates/bbt-mermaid/tests/fixtures/docs/example.mmd": b"graph TD\n A-->B\n",
            "crates/bbt-render/tests/fonts/test.ttf": b"\x00exact font\xff",
            "lab/web/tokens.js": b"// GENERATED by lab/web/gen-tokens.py from design/tokens.toml.\nwindow.TOKENS={};\n",
        }
        for name, body in cases.items():
            self.put(name, body)
        self.run_checker("--sync")
        for name, body in cases.items():
            self.assertEqual((self.root / name).read_bytes(), body)

    def test_unregistered_generated_comment_cannot_bypass_coverage(self):
        path = self.put("crates/demo/src/lib.rs", b"// Generated from somewhere.\npub fn actual() {}\n")
        self.run_checker(expected=1)
        self.run_checker("--sync")
        self.assertIn(SPDX, path.read_bytes())

    def test_shebang_encoding_bom_and_packaged_canonical_source_are_preserved(self):
        body = b"\xef\xbb\xbf#!/usr/bin/env python3\n# coding: utf-8\nprint('unchanged')\n"
        path = self.put("crates/demo/tools/probe.py", body)
        web = self.put("lab/web/components.js", b"(function () { return 42; }());\n")
        self.run_checker("--sync")
        self.assertTrue(path.read_bytes().startswith(b"\xef\xbb\xbf#!/usr/bin/env python3\n# coding: utf-8\n"))
        self.assertTrue(path.read_bytes().endswith(b"print('unchanged')\n"))
        self.assertIn(SPDX, web.read_bytes())
        self.run_checker()

    def test_old_project_only_license_notice_migrates_without_touching_code(self):
        body = b"//! The project's own boundary.\nuse core::fmt;\n"
        path = self.put("crates/bboxy450-term/src/keyboard.rs", b"// SPDX-License-Identifier: MIT OR Apache-2.0\n" + body)
        own = self.put("crates/demo/src/lib.rs", b"//! Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:\n//! " + POLICY + b"\npub fn own() {}\n")
        self.run_checker("--sync")
        self.assertTrue(path.read_bytes().endswith(body))
        self.assertTrue(own.read_bytes().endswith(b"pub fn own() {}\n"))
        self.assertNotIn(b"MIT OR Apache-2.0", path.read_bytes())
        self.run_checker()


if __name__ == "__main__":
    unittest.main()
