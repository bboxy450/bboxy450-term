---
type: report
title: INPUT61 exact pins ready;66 offline tests pass, native attempt blocked by disk gate
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-08T09:47:11.841Z
commits:
- 1691e3947ca04739f1301b596eae388f9c6f2aef
supersedes: '[[reports/2026-11-08-dream-proof-tools-input60-attempt.md]]'
verify:
  command: python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target
  exit_code: 0
id: 01M6X8VZV3GAW8ABWGVP9MZRXQ
updated: 2026-11-08T09:47:11.841Z
number: 93377961366352
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:99b00c81948a673ea3b05b19222b159b8e2db1edad69b131535a1f2367bdf30b
  frontmatter_digest: sha256:79f860920bf3aca0a9d49374b4dc98e82088bc8a5b74bc48ac90c69c0c16f889
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: F97BQEFVAfekLIvXUgjb6eh9gm3LM+GPwQna1wHSuVTzdgGT/VaQWZjC5brhzAJaKyAq6x2dpYisfUTou9DQDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150625
  event:
    id: 01M6X8W162QWMT2Q52E9WW1QDD
    sequence: 150625
    hash: 39dc133debf70c99f5e2e267858af86030567c80909c9fa8ec6a6fb6ed9e8b7a
---
INPUT61 is pinned, signed and verified offline. The single authorized native attempt is still **unconsumed** because the task's strict >8GiB pre-build disk gate fails. No canonical fetch/build or candidate/probe qualification was launched and no retained artifact was cleaned up.

Signed code commit `1691e3947ca04739f1301b596eae388f9c6f2aef` changes only `scripts/dream-proof-tools.py` ROOT_Q2_INPUT and `tests/fixtures/dream-proof-tools/sources.lock.json` canonical_q2. The 68f6d062 checker, exact terminal fields and decision66331 budget interpretation are unchanged. Source/header/attribution, guards, models, DTO/version and runtime authority are not changed; runtime_approved=false.

## Exact frozen input

The task's ORACLE FREEZE, INPUT61 supersedes INPUT60:
- Source/preparation commit `f6dffea0ff8038240de796e237362577e58fe843`.
- C328721B SHA `9bacbb0483ab328798af2bc98225bdf8f50f930e6d2ecda6779cec3afec75042`.
- Supplied probe251064B SHA `c50c2ae7d6f8aad7f3c0c99214e0683ea7b78cdf686f6acba8ca31fe340a2009`.
- Receipt530062B SHA `eb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987`, under Q2's `target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/prepared-input61/dream-read-c-preparation/compile-wcb1m3yz`.
- Other9 named inputs are byte-identical to INPUT60; the terminal format string is byte-identical and its field set matches the unchanged checker. Source-domain digest remains `8e6d9207723751cfb936587ecac7f06673321acbca05be739613e0736c2e4ee8`, an exact proposal snapshot rather than runtime approval.

E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input61-20261108`.

| Verification | Real result and retained evidence |
|---|---|
| Selected signature, main binding | git verify-commit exit0/Good approved Boxy fingerprint9B07E849480C68328CA20684844ECC0587DE7AB0; INPUT60→INPUT61 ancestry0; source→local origin/main ancestry0. E/preflight.json SHA `bb1434b1ee4783a7312a258d507bd950d7a0eb48a9955a8bf1e2c90e324c9a00`. No remote fetch/current-remote claim. |
| Exact source and codec authority | All10 git-show checks exit0, nine unchanged inputs; both codec blob checks0 preserve b26d7cd3 blob authority. E/source-byte-checks.json SHA `2d0388505e4f97d041b410b49550b9e57c4c1e0d890e703959d25c283072f376`. |
| Complete immutable ingestion | exit0 through source/executable/receipt identities and hashes, private root, all selected projection bindings, compiler/linker/SDK/platform/header/retained-output/Mach-O checks. E/ingestion.json SHA `f74badbe7e8ba94874dbc4133017287199cd3be2e7d4fc40d8ed1036f704d86a`; selection.json SHA `1c8b5fb173c6e2454509ac79e5ab5494d25dd58f11b993c3c6b0bd657247a6a6`. No executable rebuild/substitution. |
| Parser compatibility | Exact frozen terminal format/field equality, no parser change. E/terminal-binding.json SHA `bcc85453f47220c568044c3d0e7411c2fae771aeb201fd79b306aa83ca0d9cd1`. Result/exit-tail compatibility independently read from frozen source; no controller/decoder fork. |
| Offline controls | `python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`:66 tests exit0, E/unit.log SHA `1de2fba6d4e9aed1eb7ae8cc4a115c036be0aecafe38c29f38d2c14a48f11e88`; unit-exit.json SHA `73380caf0106d31dbf2b0263661dff509c7d353afc9c035e484882edebaf5fd1`. Synthetic/offline controls do not qualify native performance. |
| Prior effects and clean signed source | All11 historical ledgers preserve dev/inode/length/SHA. E/preserved-ledgers.json SHA `77329ea7194d7f3bdbb09ba9ccc30c989bf5996a28a76c12c90683fc1799a244`. HEAD signature0/Good Boxy, git diff --check0, git status --short0/empty; E/final-state.json SHA `2766808c7ff471065d82a6604c21d1a68e79dc91eefefbeb86ac3ae572be4fd3`. Recipe SHA `05d89c75aa4a1a7f3497e88b4e56ac14f9379017baa77076c3b370231496d34e`. |

## Exact blocker and decision

`/bin/df -k /Users/feral/mydev/bboxy450-harness-dream-proof-tools` exited0 with **7,883,320KiB available**, below the task's strict **>8,388,608KiB (8GiB)** gate. E/disk-gate.json SHA `4e3d6d12eddd2f6ef2978613e072276abcaf01d8208fb63c067c10c57d299b01` retains stdout, sample time, the false gate predicate, native_attempt_started=false and no_cleanup_performed=true.

All authorized pinning/offline checking is complete and concrete for review. Required action/decision: space released above the existing gate, or exact authorized disposable paths/disposition. No deletion of retained failures, sources, prefixes or peer files was attempted. Blocker DM `01M6X8QM8ZYM2E17WVYAZFC3KP` gives the exact gate and ready commit. It is not permission to widen or bypass the disk threshold.

No INPUT61 native basis/descriptor/result exists yet. Derived G, final helper/raw/floor debt and process_create excess are unobserved for this tuple. The prior ruling permitting a clearly labelled source/basis-derived G remains applicable; no new counter/schema amendment is needed. The source's ESRCH repair is frozen and inspected but not yet exercised in a real private positive here. No claim is made about full30s fit.

The full task command remains `make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`; it was NOT run on1691e394. Last actual full verification is INPUT60/68f6d062, exit2, retained in [[reports/2026-11-08-dream-proof-tools-input60-attempt.md]] (#93377961366345). Current unit0 does not replace that failed full-task result or supply fresh final source-bound formal evidence. Fresh crypto/build, positive/components, formal parity/license final gates, qualification/receipt/landing remain outstanding.

This append-only handoff records a pre-build capacity block, not task completion. Evidence dirs/files remain0700/0600. No native attempt, retry, source/probe substitution, threshold waiver, qualification, runtime approval, push/merge/install/release or peer cleanup occurred.
