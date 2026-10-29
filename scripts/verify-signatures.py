#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Verify Git history using public pins, without the operator's GPG state."""
import os
from pathlib import Path
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor


class Refusal(Exception):
    """A missing proof, invalid signature or identity outside the pinned policy."""


@dataclass(frozen=True)
class Identity:
    fingerprint: str
    name: str
    email: str


@dataclass(frozen=True)
class Policy:
    approved: Identity
    public_keys: tuple[Path, ...]
    historical: Identity | None = None
    historical_commit: str | None = None
    additional: tuple[Identity, ...] = ()


KEYS = Path(__file__).resolve().parent / "verification-keys"
POLICY = Policy(
    Identity("9B07E849480C68328CA20684844ECC0587DE7AB0", "Boxy", "bboxy450@gmail.com"),
    (KEYS / "boxy.asc", KEYS / "historical-618df56f.asc"),
    additional=(Identity("B9ABDC578390946B71B6C20C3526057098BAC1AD", "Maksim Soltan", "code@maksim.sh"),),
)


def command(argv, *, cwd=None, env=None, data=None):
    result = subprocess.run(argv, cwd=cwd, env=env, input=data, capture_output=True,
                            timeout=180)
    if result.returncode:
        detail = result.stderr[:2048].decode("utf-8", "replace").strip()
        raise Refusal(f"{Path(argv[0]).name} exit {result.returncode}: {detail}")
    return result.stdout


def check_records(records, policy):
    if not records:
        raise Refusal("empty signature range")
    historical = 0
    for sha, status, signing, primary, name, email in records:
        if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", sha):
            raise Refusal("malformed commit identity")
        if status not in {"G", "U"}:
            raise Refusal(f"{sha}: signature status {status!r} refused")
        if not re.fullmatch(r"[0-9A-F]{40}|[0-9A-F]{64}", signing):
            raise Refusal(f"{sha}: verified signing fingerprint missing")
        identities = (policy.approved, *policy.additional)
        if sha == policy.historical_commit and policy.historical is not None:
            identities = (policy.historical,)
            historical += 1
        matching = [identity for identity in identities if primary == identity.fingerprint]
        if not matching:
            raise Refusal(f"{sha}: primary fingerprint outside pinned policy")
        if not any((name, email) == (identity.name, identity.email) for identity in matching):
            raise Refusal(f"{sha}: author outside pinned policy")
    return {"commits": len(records), "historical": historical}


def verify(repo, revision="HEAD", policy=POLICY):
    if not revision or revision.startswith("-") or any(c.isspace() for c in revision):
        raise Refusal("invalid signature range")
    gpg = shutil.which("gpg")
    if gpg is None:
        raise Refusal("gpg unavailable")
    # No inherited Git/GPG overrides select a different object database, verifier,
    # keyring, agent or replacement object. The caller's Git config is unchanged.
    env = {k: v for k, v in os.environ.items()
           if not k.startswith("GIT_") and k not in {"GNUPGHOME", "GPG_AGENT_INFO", "GPG_TTY"}}
    env["GIT_NO_REPLACE_OBJECTS"] = "1"
    env["GIT_GRAFT_FILE"] = os.devnull
    env["GIT_CONFIG_GLOBAL"] = os.devnull
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    with tempfile.TemporaryDirectory(prefix="bboxy-public-signatures-") as directory:
        home = Path(directory)
        env["GNUPGHOME"] = str(home)
        options = [gpg, "--no-options", "--homedir", str(home), "--batch", "--no-tty",
                   "--no-autostart", "--disable-dirmngr", "--no-auto-check-trustdb",
                   "--no-auto-key-import", "--no-auto-key-retrieve"]
        public = bytearray()
        for path in policy.public_keys:
            if not stat.S_ISREG(path.lstat().st_mode):
                raise Refusal("public key input must be a regular file")
            with path.open("rb") as source:
                block = source.read(256 * 1024 + 1)
            if (not block.startswith(b"-----BEGIN PGP PUBLIC KEY BLOCK-----")
                    or b"PRIVATE KEY" in block or len(block) > 256 * 1024):
                raise Refusal("public key input is not bounded public armor")
            public.extend(block)
            public.extend(b"\n")
        command([*options, "--import"], env=env, data=bytes(public))
        listing = command([*options, "--with-colons", "--list-keys"], env=env)
        primary_keys = set()
        primary = False
        for line in listing.decode("utf-8").splitlines():
            fields = line.split(":")
            if fields[0] in {"pub", "sub"}:
                primary = fields[0] == "pub"
            elif fields[0] == "fpr" and primary:
                primary_keys.add(fields[9])
                primary = False
        expected = {identity.fingerprint for identity in (policy.approved, *policy.additional)}
        if policy.historical is not None:
            expected.add(policy.historical.fingerprint)
        if primary_keys != expected:
            raise Refusal("public key bundle does not match pinned primary fingerprints")
        wrapper = home / "verify-gpg"
        wrapper.write_text("#!/bin/sh\n# bboxy450-term public verification only; policy: "
                           "https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md\nexec "
                           + " ".join(shlex.quote(x) for x in options) + ' "$@"\n')
        wrapper.chmod(0o700)
        git = ["git", "--no-replace-objects", "-c", f"gpg.program={wrapper}",
               "-c", f"gpg.openpgp.program={wrapper}", "-c", "gpg.format=openpgp",
               "-c", "gpg.minTrustLevel=undefined"]
        shallow = command([*git, "rev-parse", "--is-shallow-repository"], cwd=repo, env=env)
        if shallow.strip() != b"false":
            raise Refusal("shallow history cannot prove full provenance")
        commits = command([*git, "rev-list", revision, "--"], cwd=repo, env=env).decode().splitlines()
        if not commits:
            raise Refusal("empty signature range")

        def batch(selected):
            raw = command([*git, "log", "--no-walk=unsorted", "-z", "--no-show-signature", "--no-color",
                           "--format=%H%x00%G?%x00%GF%x00%GP%x00%an%x00%ae", *selected, "--"],
                          cwd=repo, env=env)
            fields = raw.decode("utf-8").split("\0")
            if fields[-1] == "":
                fields.pop()
            if len(fields) % 6:
                raise Refusal("malformed signature result")
            return [fields[i:i + 6] for i in range(0, len(fields), 6)]

        # Full history remains the input. Two bounded readers avoid serial GPG
        # startup cost; exact set equality prevents a dropped or repeated batch.
        chunks = [commits[i:i + 128] for i in range(0, len(commits), 128)]
        with ThreadPoolExecutor(max_workers=2) as workers:
            records = [record for rows in workers.map(batch, chunks) for record in rows]
        if len(records) != len(commits) or {row[0] for row in records} != set(commits):
            raise Refusal("signature results do not cover the complete requested history")
        return check_records(records, policy)


def main():
    if len(sys.argv) > 2:
        raise Refusal("usage: verify-signatures.py [revision-range]")
    revision = sys.argv[1] if len(sys.argv) == 2 else "HEAD"
    result = verify(Path.cwd(), revision)
    print(f"signatures: {result['commits']} verified, {result['historical']} exact historical exception(s)")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (Refusal, OSError, UnicodeError, subprocess.TimeoutExpired) as error:
        print(f"signatures: refused: {error}", file=sys.stderr)
        sys.exit(1)
