#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Real signatures in disposable repos; no operator key, login, network or prompt."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SOURCE = Path(__file__).with_name("verify-signatures.py")
spec = importlib.util.spec_from_file_location("pinned_signatures", SOURCE)
verifier = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = verifier
spec.loader.exec_module(verifier)


def run(argv, *, cwd=None, data=None, env=None):
    result = subprocess.run(argv, cwd=cwd, input=data, env=env, capture_output=True, timeout=180)
    if result.returncode:
        raise AssertionError(result.stderr.decode("utf-8", "replace"))
    return result.stdout


class SignatureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Darwin agent socket names must fit sockaddr_un; the normal TMPDIR is long.
        cls.directory = tempfile.TemporaryDirectory(prefix="bboxy-sig-", dir="/tmp")
        cls.root = Path(cls.directory.name)
        cls.home = cls.root / "signing"
        cls.home.mkdir(mode=0o700)
        cls.gpg = shutil.which("gpg")
        cls.env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        cls.env.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1")
        # Adapted from bboxy450-term scripts/test-landing.py:fixture_identity.
        # Only this fixture's agent and disposable secret keys are created.
        # Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        cls.options = [cls.gpg, "--no-options", "--homedir", str(cls.home), "--batch",
                       "--no-tty", "--pinentry-mode", "loopback", "--passphrase", ""]
        cls.addClassCleanup(cls.close)
        cls.approved = cls.key("Approved Fixture", "approved@bboxy450.invalid")
        cls.legacy = cls.key("Historical Fixture", "historical@bboxy450.invalid")
        cls.unknown = cls.key("Other Fixture", "other@bboxy450.invalid")
        cls.expired = cls.key("Expired Fixture", "expired@bboxy450.invalid", expired=True)
        cls.wrapper = cls.root / "sign"
        cls.wrapper.write_text("#!/bin/sh\nexec " + " ".join(shlex.quote(x) for x in cls.options) + ' "$@"\n')
        cls.wrapper.chmod(0o700)
        cls.expired_wrapper = cls.root / "sign-expired"
        cls.expired_wrapper.write_text("#!/bin/sh\nexec " + " ".join(shlex.quote(x) for x in cls.options)
                                       + ' --faked-system-time 20100101T000000 "$@"\n')
        cls.expired_wrapper.chmod(0o700)
        agent = run(["gpg-connect-agent", "--homedir", str(cls.home), "GETINFO pid", "/bye"])
        (cls.root / "owned-agent.pid").write_bytes(agent)

    @classmethod
    def close(cls):
        subprocess.run(["gpg-connect-agent", "--homedir", str(cls.home), "KILLAGENT", "/bye"],
                       capture_output=True, timeout=10)
        cls.directory.cleanup()

    @classmethod
    def key(cls, name, email, expired=False):
        clock = ["--faked-system-time", "20100101T000000"] if expired else []
        run([*cls.options, *clock, "--quick-generate-key", f"{name} <{email}>", "ed25519", "sign",
             "1d" if expired else "0"])
        listing = run([*cls.options, "--with-colons", "--list-keys", email]).decode()
        fp = next(row.split(":")[9] for row in listing.splitlines() if row.startswith("fpr:"))
        (cls.root / f"{fp}.asc").write_bytes(run([*cls.options, "--armor", "--export", fp]))
        return verifier.Identity(fp, name, email)

    def repository(self):
        repo = Path(tempfile.mkdtemp(prefix="repo-", dir=self.root))
        run(["git", "init", "-q", "--initial-branch=main", str(repo)], env=self.env)
        hooks = self.root / "empty-hooks"
        hooks.mkdir(exist_ok=True)
        for key, value in [("user.name", self.approved.name), ("user.email", self.approved.email),
                           ("user.signingkey", self.approved.fingerprint), ("gpg.format", "openpgp"),
                           ("gpg.program", str(self.wrapper)), ("commit.gpgsign", "true"),
                           ("core.hooksPath", str(hooks))]:
            run(["git", "config", key, value], cwd=repo, env=self.env)
        return repo

    def commit(self, repo, identity=None, *, unsigned=False, wrong_author=False, expired=False):
        identity = identity or self.approved
        author = "Wrong Fixture <wrong@bboxy450.invalid>" if wrong_author else f"{identity.name} <{identity.email}>"
        run(["git", "-c", f"user.signingkey={identity.fingerprint}", "-c", f"commit.gpgsign={not unsigned}",
             "-c", f"gpg.program={self.expired_wrapper if expired else self.wrapper}",
             "commit", "-q", "--allow-empty", "--author", author, "-m", "private signature fixture"],
            cwd=repo, env=self.env)
        return run(["git", "rev-parse", "HEAD"], cwd=repo, env=self.env).decode().strip()

    def policy(self, *, legacy=None, approved=None, keys=None):
        approved = approved or self.approved
        identities = [approved, *([self.legacy] if legacy is not None else [])]
        public = tuple(self.root / f"{identity.fingerprint}.asc" for identity in identities)
        return verifier.Policy(approved, keys or public, self.legacy if legacy is not None else None, legacy)

    def test_valid_untrusted_pin_passes_and_empty_range_refuses(self):
        repo = self.repository()
        self.commit(repo)
        self.assertEqual(verifier.verify(repo, policy=self.policy()), {"commits": 1, "historical": 0})
        with self.assertRaisesRegex(verifier.Refusal, "empty signature range"):
            verifier.verify(repo, "HEAD..HEAD", self.policy())

    def test_empty_repository_and_unsigned_commit_refuse(self):
        repo = self.repository()
        with self.assertRaises(verifier.Refusal):
            verifier.verify(repo, policy=self.policy())
        self.commit(repo, unsigned=True)
        with self.assertRaisesRegex(verifier.Refusal, "signature status 'N'"):
            verifier.verify(repo, policy=self.policy())

    def test_bad_signature_refuses(self):
        repo = self.repository()
        sha = self.commit(repo)
        raw = run(["git", "cat-file", "commit", sha], cwd=repo, env=self.env)
        raw = raw.replace(b"private signature fixture", b"changed signature fixture")
        bad = run(["git", "hash-object", "-t", "commit", "-w", "--stdin"], cwd=repo, data=raw,
                  env=self.env).decode().strip()
        with self.assertRaisesRegex(verifier.Refusal, "signature status 'B'"):
            verifier.verify(repo, bad, self.policy())

    def test_unknown_key_and_even_trusted_unapproved_g_refuse(self):
        repo = self.repository()
        self.commit(repo, self.unknown)
        with self.assertRaisesRegex(verifier.Refusal, "signature status 'E'"):
            verifier.verify(repo, policy=self.policy())
        raw = run(["git", "show", "-s", "--format=%H%x00%G?%x00%GF%x00%GP%x00%an%x00%ae", "HEAD"],
                  cwd=repo, env=self.env).decode().strip().split("\0")
        self.assertEqual(raw[1], "G")  # Real crypto/trust in this fixture's own keyring.
        with self.assertRaisesRegex(verifier.Refusal, "primary fingerprint"):
            verifier.check_records([raw], self.policy())

    def test_legacy_exception_requires_exact_hash_and_author(self):
        repo = self.repository()
        sha = self.commit(repo, self.legacy)
        self.assertEqual(verifier.verify(repo, sha, self.policy(legacy=sha))["historical"], 1)
        with self.assertRaisesRegex(verifier.Refusal, "primary fingerprint"):
            verifier.verify(repo, sha, self.policy(legacy="0" * 40))
        future = self.commit(repo, self.legacy)
        with self.assertRaisesRegex(verifier.Refusal, "primary fingerprint"):
            verifier.verify(repo, future, self.policy(legacy=sha))
        other = self.repository()
        wrong = self.commit(other, self.legacy, wrong_author=True)
        with self.assertRaisesRegex(verifier.Refusal, "author outside"):
            verifier.verify(other, wrong, self.policy(legacy=wrong))

    def test_approved_signature_with_wrong_author_refuses(self):
        repo = self.repository()
        self.commit(repo, wrong_author=True)
        with self.assertRaisesRegex(verifier.Refusal, "author outside"):
            verifier.verify(repo, policy=self.policy())

    def test_expired_key_refuses(self):
        repo = self.repository()
        self.commit(repo, self.expired, expired=True)
        with self.assertRaisesRegex(verifier.Refusal, "signature status '[XY]'"):
            verifier.verify(repo, policy=self.policy(approved=self.expired))

    def test_revoked_key_refuses(self):
        repo = self.repository()
        self.commit(repo)
        home = self.root / "revoked-public"
        home.mkdir(mode=0o700)
        options = [self.gpg, "--no-options", "--homedir", str(home), "--batch", "--no-tty",
                   "--no-autostart", "--disable-dirmngr"]
        public = (self.root / f"{self.approved.fingerprint}.asc").read_bytes()
        run([*options, "--import"], data=public)
        certificate = (self.home / "openpgp-revocs.d" / f"{self.approved.fingerprint}.rev").read_bytes()
        certificate = certificate[certificate.index(b"-----BEGIN PGP PUBLIC KEY BLOCK-----"):]
        run([*options, "--import"], data=certificate)
        revoked = self.root / "revoked.asc"
        revoked.write_bytes(run([*options, "--armor", "--export", self.approved.fingerprint]))
        with self.assertRaisesRegex(verifier.Refusal, "signature status 'R'"):
            verifier.verify(repo, policy=self.policy(keys=(revoked,)))

    def test_replacement_objects_and_shallow_history_do_not_hide_unsigned_commits(self):
        repo = self.repository()
        signed = self.commit(repo)
        unsigned = self.commit(repo, unsigned=True)
        run(["git", "replace", unsigned, signed], cwd=repo, env=self.env)
        with self.assertRaisesRegex(verifier.Refusal, "signature status 'N'"):
            verifier.verify(repo, policy=self.policy())
        (repo / ".git/shallow").write_text(unsigned + "\n")
        with self.assertRaisesRegex(verifier.Refusal, "shallow history"):
            verifier.verify(repo, policy=self.policy())

    def test_inherited_keyring_and_git_verifier_cannot_supply_trust(self):
        repo = self.repository()
        self.commit(repo)
        run(["git", "config", "gpg.program", "/usr/bin/false"], cwd=repo, env=self.env)
        with patch.dict(os.environ, {"GNUPGHOME": str(self.home), "GIT_CONFIG_GLOBAL": "/missing/fixture-config"}):
            self.assertEqual(verifier.verify(repo, policy=self.policy())["commits"], 1)

    def test_unapproved_bundle_and_private_armor_refuse(self):
        repo = self.repository()
        self.commit(repo)
        unknown = self.root / f"{self.unknown.fingerprint}.asc"
        with self.assertRaisesRegex(verifier.Refusal, "bundle does not match"):
            verifier.verify(repo, policy=self.policy(keys=(unknown,)))
        fake = self.root / "not-public.asc"
        fake.write_bytes(b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nnot key material\n")
        with self.assertRaisesRegex(verifier.Refusal, "public armor"):
            verifier.verify(repo, policy=self.policy(keys=(fake,)))


class ProjectHistoryTests(unittest.TestCase):
    def test_actual_full_history_and_public_key_provenance(self):
        root = SOURCE.parent.parent
        expected = int(run(["git", "rev-list", "--count", "HEAD"], cwd=root))
        self.assertGreaterEqual(expected, 2202)
        self.assertEqual(verifier.verify(root), {"commits": expected, "historical": 1})
        provenance = json.loads((verifier.KEYS / "provenance.json").read_text())
        for key in provenance["keys"]:
            self.assertEqual(hashlib.sha256((verifier.KEYS / key["file"]).read_bytes()).hexdigest(), key["sha256"])


if __name__ == "__main__":
    unittest.main()
