---
type: report
title: RemoteDelivery refinement and card implemented; landing blocked by parent feature gate
task: '[[tasks/2026-11-09-remote-delivery-conformance-and-card]]'
author: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
created: 2026-11-10T02:58:02.192Z
commits:
- fd38ec4c0196cf3437653a05112938ab56fd6d49
verify:
  command: make land
  exit_code: 2
supersedes: '[[reports/2026-11-09-remote-delivery-conformance-and-card]]'
id: 01M71P87GG6QTPR89W0XB2REY1
updated: 2026-11-10T02:58:02.192Z
number: 1794274420276161
created_by: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
updated_by: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
authorship:
  signer: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
  kind: seat
  public_key: uACX3zuhcDlbiNfc2nk5cFskWaUe4+OKMqxc6XxLvtM=
  process_generation: '1'
provenance:
  digest: sha256:564ee5251849a103030e8d73142dc4af7496bd7656ab948314fd4b8cb9cb9a06
  frontmatter_digest: sha256:e3248914a52dc0dd426ff1e7f3c29efa84fe444cae9da6e8aa9d224b51479be5
  signer: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
  sig: r8IKB67i7osvO7XdVPERyPODC94aAMIcFuEg/I5jmG0smGBPnpEQWoeErUuLeCOx14KSMteWyCY0xMu1OQyUBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151297
  event:
    id: 01M71P88XEG2ZNDSBR2N5R3A4X
    sequence: 151297
    hash: 46a9595521b74bae5e49843dd821345003358f258bdfe20fa6ce0bd49e942cda
---
The amended implementation is committed, but landing is blocked by the rebased parent harness feature gate. This supersedes [[reports/2026-11-09-remote-delivery-conformance-and-card]]. The task is not complete.

## Landing blocker and exact decision

`make land` rebased the source commit to `fd38ec4c0196cf3437653a05112938ab56fd6d49` on `origin/main=a58a0fcfb37ca97a9b1bb900c9a5ecfd660065b7`, then failed in `make init-submodules`:

> harness: parent feature isolation rejected: bboxy450-harness/acp

Outer Make exit: **2**. The underlying land script exited **77**; nested init Make exited **2**. No push or formal receipt was produced.

Evidence: `scripts/harness-submodule.py:67` runs workspace-wide `cargo metadata`; `:88` rejects any resolved harness `acp` feature. The newly pinned harness `20e52d0f937c7ed19abbf506564a3dd8a0c8cdd4` declares defaults `["acp-agent", "acp"]` at `crates/bboxy450-harness/Cargo.toml:21`. However, `cargo tree --locked -p bboxy450-term -e normal,build,features -i bboxy450-harness` exits 0 and shows the term consumer selects **only embedded**; its manifest has default-features=false.

Decision needed: authorize/assign correction of the workspace-wide feature check to validate the actual embedded consumer (retaining its negative control), or provide a corrected parent baseline. The assignment excludes `scripts/harness-submodule.py`, its tests, and harness source. I did not broaden scope, alter the harness defaults, or bypass the gate.

The named verify command passed before this rebase. These component files did not change across the rebase, but the harness pin and repository records did; the broader evidence is not current-pin verification. Post-rebase checks and the formal receipt remain pending.

## Files and mechanisms

- `crates/bboxy450-term-remote/tests/delivery_witness.rs:210` — one explicit operation mapping drives actual Outbox/Inbox APIs; a batch flush maps to ordered Send events, and only actual Inbox arrivals create Applied frames.
- `delivery_witness.rs:376` — breadth-first enumeration of observation states, shortest-path replay on fresh runtime/journal instances, every enabled operation, decision/Fresh-Apply/state/invariant comparisons, and a check that the observation quotient preserves the model state. Uses the existing model encoding, action-event map and checked graph to measure model coverage.
- `delivery_witness.rs:456` pins counts; `:472` runs the same walk with a memory journal recreated empty on crash and requires an Apply/applied disagreement after a Crash.
- `crates/bbt-yarn/src/verify.rs:638` — new card module within the existing project-owned verification module; `src/lib.rs:65` reexports it as `bbt_yarn::card`. It reuses Model::check, ProtocolMetadata and the existing private token hash. No new crate, dependency, source parser or hash implementation.
- `verify.rs:658` — deterministic graph quotient, first matching view predicate, merged event-labelled edges, no self-loops, errors for unnamed states, violations, incomplete exploration and more than twelve nodes. The card includes claims, bounds, state count, declared temporal scope, all case labels, unsupported waivers, assumptions and source hash. Metadata has no outcome-kind field, so all case labels are displayed, including every reject/ignore case.
- `verify.rs:834` onward — tiny inline Switch protocol tests determinism, first-match collapse, edge deduplication, omitted loops, temporal scope, source identity, limits, unnamed states, violations and the thirteen-node rejection.
- `crates/bboxy450-term-remote/tests/delivery_card.rs:14` — generates message-A view predicates from model variable names; reads the RemoteDelivery registry assumptions using existing serde_json for the registry's single-line JSON-compatible TOML basic string; checks stored card and fixture. Only BBOXY_UPDATE_CARDS=1 writes them.
- `crates/bboxy450-term-remote/cards/RemoteDelivery.md` — five nodes, six quotient edges, Does not promise with the no-forged-Applied assumption; model source identity `fnv1a64:2e535fa905c04e68`.
- `crates/bbt-mermaid/tests/fixtures.rs:40` includes the cards fixture in all existing parse/layout/geometry/determinism checks; `:360` verifies it is exactly the card's Mermaid block. The generated fixture is `tests/fixtures/cards/RemoteDelivery.mmd`.
- Six literal fixtures under `tests/fixtures/bboxy/` restore the required corpus for already tracked Python-tool, spec-first and workspace-provisioning documents. The first verify run exposed these baseline omissions. No check was removed or narrowed. Their exact source/provenance is in `tests/fixtures/README.md`. New upstream repository records arrived during rebase; their corpus has not been rechecked.
- `verification/contracts.toml:897` and `coverage.toml:262` describe the runtime-operation walk and causal receipt assumption. Every existing assumption is retained. Yarn README documents the hosted API.

## Coverage

The real FileJournal walk printed these values and the test pins them:

| Measure | Checked count |
|---|---:|
| Harness observation states | 72 |
| Enabled harness transitions | 502 |
| Model states reached, including intermediate batch steps | 50 / 52 |
| Model edges exercised | 298 / 572 |
| Unexercised model edges | 274 |

Unexercised edges by event kind: AckArrive **84**, Arrive **84**, Drop **4**, Reconnect **4**, RemoteCrash **4**, Send **86**, Submit **8**. These are coverage facts, not failures: model-only independent sends, unavailable link deliveries, intermediate batch states, and stutters from empty flushes need not be runtime operations.

The runtime broken control fails at **Submit(a), Flush, Deliver(a), Crash**, specifically `FailureKind::Applied`: the new memory journal has no applied a while the model retains it. IO or unrelated failures do not satisfy that control. Both existing model-control and resume tests remain.

## Mapping corrections and abstraction gaps

- Accepted acknowledgements retire their message frame from the link projection to match model inflight retirement. Applied frames remain available to exercise duplicate acknowledgements. Late message frames after retirement are outside this link projection.
- Empty flush maps to no event (stuttering); it additionally checks every candidate Send is ignored. It does not pretend to execute all those model self-loops.
- Drop/reconnect return no runtime decision. Their accepted/ignored classification follows the harness-owned connection flag; future flush/ack operations check actual Outbox behavior. Private Outbox down/inflight fields are not directly exposed.
- The no-forged-Applied environment assumption is explicitly authorized in Amendment 1, stored in contracts.toml and emitted on the card.
- Two identities only. Journal-and-apply remains atomic in the model; real fsync precedes the caller's apply, and a crash in between can lose a turn, reported unknown. Mid-operation crashes, IO failure injection, arbitrary payloads/ids, transport authentication, SSH, epochs and arbitrary Rust execution are not proved by this walk.

## What the first walk found

The initial direct probes really failed: Submit(A), Send(A), AckArrive(A) accepts in Outbox while the model ignores; Submit(A), Submit(B), Send(A) sends both at runtime while the model sends A. Oracle Amendment 1 ruled these were mapping errors in the first brief. The amended walk uses batched refinement and a real causal link, without changing delivery.rs or link.rs.

## What was removed

Removed the scripted single-trace test and its step helper, the initial failing direct-API probe, and the temporary scratch integration tests. The attempted standalone card.rs was folded into the existing owned verification module after the source-notice gate identified the inherited Yarn file classification; this also removed unnecessary token_hash visibility widening. A duplicate module doc was removed to satisfy clippy, without a lint suppression. No production delivery behavior or protocol syntax changed.

## Commands and actual exits

Commands below sourced scripts/cache-env.sh for Cargo invocations. Logs stay outside Git.

| Command | Exit | Evidence / disposition |
|---|---:|---|
| `cargo nextest run -p bboxy450-term-remote --test delivery_witness --no-capture` | 0 | `/tmp/bboxy-refinement-walk-01M71H0S.log`   |
| `cargo nextest run -p bboxy450-yarn --features model --lib -E 'test(card::)' --no-capture` | 0 | `/tmp/bboxy-card-unit-01M71H0S.log`   |
| `BBOXY_UPDATE_CARDS=1 cargo nextest run -p bboxy450-term-remote --test delivery_card --no-capture` | 0 |  `/tmp/bboxy-card-generate-01M71H0S.log`, `/tmp/bboxy-card-generate-final-01M71H0S.log`  |
| `cargo nextest run -p bboxy450-term-remote --test delivery_card_scratch --no-capture` | 100 | `/tmp/bboxy-card-scratch-01M71H0S.log`  Compiled/executed scratch guard &&true; only card footer hash changed 2e535fa905c04e68 -> 4cc39d846ea92b48. Temporary integration file removed; scratch source /tmp/bboxy-card-scratch-01M71H0S.rs |
| `cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings` | 0 | `/tmp/bboxy-card-clippy-01M71H0S.log`   |
| `cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts` | 100 | `/tmp/bboxy-task-verify-01M71H0S.log`  125/126 passed; one preexisting corpus test failed: six newly tracked .bboxy Mermaid blocks lacked literal fixtures. Added six exact fixtures and companion provenance within permitted tests scope; no checks weakened. Retry underway log /tmp/bboxy-task-verify-final-01M71H0S.log |
| `cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts` | 0 | `/tmp/bboxy-task-verify-final-01M71H0S.log`  126 affected-crate tests passed; 45 native formal contract tests passed; app clippy and QoS witnesses/19 Python tests/positive and expected-failing QoS controls completed. Existing rust-objcopy strip warnings (missing libLLVM) did not prevent any check. |
| `make test-licenses` | 2 | `/tmp/bboxy-card-licenses-01M71H0S.log`  Existing source-notice classifier treats new bbt-yarn file as upstream-scoped; moved card inline into existing project-owned verify.rs, reexport root path, restored private token_hash. No license/check weakening. |
| `git diff --no-index --check /dev/null crates/bboxy450-term-remote/cards/RemoteDelivery.md` | 3 |   Detected generated extra blank line at EOF; corrected generator's terminal newline. |
| `make test-licenses` | 0 | `/tmp/bboxy-card-licenses-final-01M71H0S.log`   |
| `BBOXY_UPDATE_CARDS=1 cargo nextest run -p bboxy450-term-remote --test delivery_card --no-capture` | 0 | `/tmp/bboxy-card-generate-source-final-01M71H0S.log`   |
| `cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings` | 101 | `/tmp/bboxy-card-clippy-source-final-01M71H0S.log`  Inline module had duplicate inner/outer docs; removed duplicate inner doc rather than suppress lint. |
| `cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings` | 0 | `/tmp/bboxy-card-clippy-final-01M71H0S.log`   |
| `git add -- <17 exact source paths> && git diff --cached --check && git diff --cached --stat` | 0 |    |
| `cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts` | 0 | `/tmp/bboxy-task-verify-source-final-01M71H0S.log`  Final source: 126 affected-crate tests, 45 native formal tests, app clippy and all QoS positives/negative controls passed. |
| `cargo nextest run -p bboxy450-term-remote --test delivery_card_scratch --no-capture` | 100 | `/tmp/bboxy-card-scratch-final-01M71H0S.log`  Final implementation compiled/executed; checked stale-card assertion and both exact source hashes in a separate Python assertion (exit 0), then removed temporary scratch test. |
| `git commit -S -F /tmp/bboxy-card-commit-01M71H0S.txt && git verify-commit HEAD` | 0 |   29220555af79f797132315679986228879b54654; automatically rebased/re-signed by make land as fd38ec4c0196cf3437653a05112938ab56fd6d49; both good signatures from approved Boxy key. |
| `make land` | 2 | `/tmp/bboxy-card-land-01M71H0S.log`  Underlying land-fast exit 77; nested make init-submodules exit 2, after rebase onto a58a0fcfb37ca97a9b1bb900c9a5ecfd660065b7. Pinned harness 20e52d0f937c7ed19abbf506564a3dd8a0c8cdd4 defaults include acp; workspace-wide checker rejects acp although cargo tree -p term exposes only embedded. |
| `cargo tree --locked -p bboxy450-term -e normal,build,features -i bboxy450-harness` | 0 |   Only bboxy450-harness feature embedded is selected by term consumer. |

Additional checks: source edits/formatting and ordinary source/diff/date/hash inspections exited 0; `git diff --cached --check` and `git diff --exit-code` exited 0 before commit. The separate Python assertion over the final scratch log exited 0: it required completed compilation, a stale-card assertion, both exact source hashes (2e535fa905c04e68 → 4cc39d846ea92b48), and absence of a compile failure. Read-only diagnostic exceptions were no-match rg (1, first report), ps for already exited PIDs (1), and a nonexistent optional build-log read (1); none is test evidence.

The final pre-rebase verify run passed **126/126 affected-crate tests**, **45 native formal contract tests**, app clippy, QoS Rust witnesses, **19 Python QoS tests**, and both expected-failing QoS controls. Existing rust-objcopy debug-stripping warnings about missing libLLVM did not prevent execution; no toolchain or gate was changed.

Final log SHA-256:

| Log in /tmp | SHA-256 |
|---|---|
| bboxy-refinement-walk-01M71H0S.log | dd3e880d3b99cceadf55b99d065f3cc13cac9f3b5791f8e332b3270566d0aab4 |
| bboxy-card-scratch-final-01M71H0S.log | 0365d49f8138c3389db569317066ade27be103ef3854d46a66a7dbadbf6aad0e |
| bboxy-card-clippy-final-01M71H0S.log | 35cf8f59daa677e2c122eb13bfd78f4879a7c5a637ae110951dbf86217528c00 |
| bboxy-task-verify-source-final-01M71H0S.log | cfb5002d2a9073728c3e8582f0e46e795c8b3257ef86afdb29a5cdb2049301c9 |
| bboxy-card-licenses-final-01M71H0S.log | deb5fd75f7c254a7cb367323b62b4ce5b65d984610f8592f07b836e46c49271c |
| bboxy-card-land-01M71H0S.log | 2825be8e616f5085f2f148dc8b91b683e4eb5d5797d77b1f098fa86bfcc2b94f |

## Commit, remaining work and reuse

Current signed source commit: **fd38ec4c0196cf3437653a05112938ab56fd6d49**; original signed pre-rebase commit: **29220555af79f797132315679986228879b54654**. git verify-commit exits 0 for both, using approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0. Worktree is clean. Source is not on origin/main; task acceptance/landing is pending. This report is checkpointed separately through authenticated knowledge tools.

No fresh make test-formal receipt, successful land/push, or post-rebase verify is claimed. Full make test, installation, GUI parity and a separate make smoke were not run; runtime implementation and UI geometry are unchanged.

What generalizes: bbt-supervisor can use `bbt_yarn::card::generate`, existing metadata/model/check/hash primitives and the fixture corpus checks unchanged. Its view predicates and recorded assumptions are component-specific. The BFS/replay pattern generalizes; this remote operation alphabet, observation projection and journal factory remain test-local rather than becoming another production abstraction.
