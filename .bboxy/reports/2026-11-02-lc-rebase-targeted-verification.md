---
{
  "author": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "authorship": {
    "kind": "seat",
    "process_generation": "1",
    "public_key": "srV0sf6OqVcpHr6OW6bpLTluWr3qGWNk1q9J8nzC9/Q=",
    "signer": "seat:01M6CKGYEPHYP25GFF6H7B1X23"
  },
  "commits": [
    "c66e9c45b1c17bf0348e194a3a8e2e3915ada390"
  ],
  "created": "2026-11-02T17:03:30.298Z",
  "created_by": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "id": "01M6EKEGKSVSFDE4PFDKG549K1",
  "number": 5192,
  "provenance": {
    "digest": "sha256:8daf09680d00bd40f5e34c6f21719c05b208dad873aa25c82cf39e86044e6270",
    "frontmatter_digest": "sha256:b89ccff58d35525de432847602bbf39c3f3c6730f76294c1102ffea7b576d490",
    "log": ".bboxy/log/2026-11-02.jsonl#34654",
    "sig": "lClU7+Qa1sb/m4ugaVJbsFkiGit2VKrRbShsZnqmH7QhzcZcSpoP70WxXTktpdHBCOdKGGmH56KaOdwk0OHmAg==",
    "signer": "seat:01M6CKGYEPHYP25GFF6H7B1X23"
  },
  "supersedes": "[[reports/2026-11-02-lc-port-verification]]",
  "task": "[[tasks/2026-11-01-lc-port-into-term]]",
  "title": "LC port: main rebase and targeted converter verification",
  "type": "report",
  "updated": "2026-11-02T17:03:30.298Z",
  "updated_by": "seat:01M6CKGYEPHYP25GFF6H7B1X23",
  "verify": {
    "command": "make test-log-converter",
    "exit_code": 0
  }
}
---
<!--
Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
-->

# LC rebase and targeted verification

Oracle follow-up 01M6DHH92TQK8CRXD26HACG8E0 instructs rebasing PR #91 onto
origin/main, running targeted converter checks only, sending the new SHA, then
marking the PR ready. The Oracle owns the single full gate of the combined
#91 / #95 / P3 candidate. This replaces the earlier full-gate rerun instruction
for this follow-up; no full gate was run here.

Worktree: /Users/feral/mydev/bboxy450-term-log-converter, branch log-converter.
Previous published head: 082ca4aeaa1e48ef4a2b80490637130b4afa3e6c.
Rebased implementation/test head: c66e9c45b1c17bf0348e194a3a8e2e3915ada390.
Base: 0ab3930cf5bfd028c5d63e440fe3448973b16b5f, the current origin/main at
fetch time; it contains the requested 3f025918. The old branch is retained as
e6-log-converter-before-main-rebase-20261102. All seven rebased commits have
valid G signatures from approved key 9B07E849480C68328CA20684844ECC0587DE7AB0.

The required advance notice was sent in reply to the Oracle before rebasing.
The published update uses an explicit lease against the old 082ca4ae head.
The signed report-only commit containing this record is identified by its
Report trailer and the final handoff; it changes no tested product/model input.

## Conflict resolution

- Makefile retains main's harness-branding targets and formal-stage timing/
  reuse wrappers. The converter exporter joins that wrapper as
  converter-contracts. Its existing targeted/native targets remain available.
- scripts/ci-local.sh retains main's FORMAL_REUSE=0 and both full-run converter
  targets. The full-run/presubmit boundary is unchanged.
- verification/contracts.toml and corpus.toml retain every main record,
  including the new GitKB models, and append LcAppendLog plus all three
  controls. Static parsing finds 63 unique contracts, 148 unique model entries
  and 33 crate inventory entries.
- scripts/test-landing.py keeps main's formal-stage fixture commands and adds
  both converter target names to its fixture stubs. The failed-native-gate
  control is retained. Recovery expected-command sequences apply unchanged.
- Cargo.lock and the remaining formal/coverage changes merged automatically.
  Git range-diff shows changes only in the three integration commits above.
  The RecordId, recovery-fixture, cursor-fix and historical-report patches
  replay unchanged.

The entire crates/bboxy450-log-converter tree is byte-identical to the previous
published head (git diff against the backup for that path is empty). No IR
fields, permanent IDs, wire bytes, golden fixtures, stateful model, or native
probe logic were changed by this rebase.

## Targeted verification

On clean c66e9c45, from the term worktree:

`make test-log-converter` — exit 0.

It runs cargo-metadata dependency/provenance/secret guards, converter fmt,
all-target/all-feature Clippy with -D warnings, and cargo test for only
bboxy450-log-converter. Default test concurrency is two; CI profiles use
opt-level 1/debug 0 with debug assertions enabled. The worktree-local target
and two Cargo jobs are retained.

Result: 34 Rust test functions passed, zero failed, two intentional full-run
witnesses ignored (upstream zstd CLI and the 100 MB corpus). The golden test
contains all 68 fixed cases. Native LcAppendLog and all three broken controls
are included in the crate test. Executable test-time summaries total 33.64 s,
below the 60 s affected-test allocation.

Complete stdout/stderr: /tmp/lc-rebase-targeted-20261102.log.
SHA-256: fb148749026b0a97ee8aeda1a9c937288b15fba0fdff12baa8de575ca500a0d0.
Static Python AST/TOML parsing and git diff --check also pass.

No workspace test suite, full gate, Java parity rerun, real-client native
target or 100 MB test was run in this follow-up, as requested. Historical
receipts remain tied to their original pre-rebase SHAs and are not relabelled.
The Oracle's combined candidate needs its own current-source full/formal
receipt. Prior failures in the old immutable reports remain historical facts;
this report does not claim a current full-CI result.

PR #91 is to be marked ready after the rebased SHA is sent to the Oracle.
No merge, release or land.sh operation is authorized or performed.
