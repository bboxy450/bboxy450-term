---
type: decision
title: Measure C1 with an optimized private fixture retaining assertions and the actual release worker
id: 01M6P5R8BQJ8R7QCVAS22BBP6K
updated: 2026-11-05T15:38:01.807Z
created: 2026-11-05T15:38:01.807Z
number: 93377961364684
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:421115be8fc47cdbd93ba11120b335a2c647d462ddf63c17f9af125109685f2f
  frontmatter_digest: sha256:1da1d4317a2c7c2c1d887139d325e9278be970d59d7e493f66154d6c0bedd7ce
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: qmiursT4Ho34KsZCjC9NAWWgzxcmBDY0rqJrxngR5gZft7UCDZXzOE866t8XdgyWLl2L624mh2nGym6fj4CBBQ==
  log: .bboxy/log/2026-11-05.jsonl#137244
---
## Decision and evidence
The private C1 responsiveness/performance witness uses a canonical optimized-with-assertions build instead of an unoptimized development executable. Keep every existing 1ms unit/9ms rest,30s deadline,guard/currentness,CPU/duty,heap,control,foreground,frame and settlement threshold. No cooperative-check/parser contract relaxation is selected.

Consumer log /tmp/dream-c1-guard-profile.log records an actual unoptimized test-profile failure: fresh public/actor checks cost roughly439–906us thread CPU, and combined unit costs1,028,625ns before refusing resource_limit. The terminal test summary is failed; this is baseline failure evidence, not a passing fixture or a recorded wrapper exit code. Root inspected Makefile exports, root Cargo.toml (no custom optimized fixture profile), script runner paths and cfg(debug_assertions) private hooks. Smaller artificial corpora or an omitted guard would hide the required behavior. Optimization is not presumed to cure every overrun; remeasure unchanged gates.

## Exact build contract
Add ONLY root Cargo.toml profile [profile.dream-c1-fixture] with inherits="release", debug-assertions=true and overflow-checks=true. Other optimization/codegen/runtime defaults inherit release. Do not change release/dev/test defaults, Cargo.lock, dependencies, toolchain, global Cargo config or product installed build. Private tests/native fixture use this explicit profile with debug assertions still compiled/enabled; never disable checks or remove private guards to get a green result.

Canonical entry remains make test-dream-c1. Wire the profile through existing Make/script/quarantine-nextest paths: cargo build --profile dream-c1-fixture; nextest uses its Cargo-build-profile selector (not its named nextest execution-profile selector), respecting the existing quarantine wrapper. Reuse available wrapper support rather than making another runner; only a demonstrably missing exact selector translation may need a further scoped glue disposition. Source-bound receipts include Cargo.toml/profile, effective profile and compiler/feature settings, source/tool hashes, exact binary digests and paths. No ambient CARGO_PROFILE_* or RUSTFLAGS override may silently change the claimed canonical fixture setting; detect/record/refuse conflicting overrides locally, never mutate global config.

Keep the actual production inspect-c1 worker built in ordinary release mode without test-fixtures; its Hello and positive joined worker/resource/settlement witnesses use that exact artifact and hash. The retained-Host managed SEAT fixture may separately use the same pinned source's test-fixtures executable built under dream-c1-fixture. If the existing private provisioning shim currently conflates the seat and worker executable, split those exact roles explicitly in existing allowed script/tests: fixed inspection argv selects the production release worker, private managed seat selects the feature-gated fixture executable; do not substitute a fake worker or alter either argv/environment policy. Record both hashes/build settings honestly. Private deliberate failure hooks, where necessary, remain labeled fixture controls; do not claim their artifact equals an installed production binary.

## Scope and acceptance
Authorize consumer root Cargo.toml profile stanza only; existing Makefile/scripts/test-dream-c1.py/private C1 source/test paths cover profile/artifact plumbing. All protocol/schema/source eligibility/authority/pause/currentness/credit/slot limits remain unchanged. Existing producer release and embedded compile checks remain required; no harness-manifest edit is needed for a workspace-root profile.

First run the previously failing focused API/page-owner witness and representative multi-MiB/parser path with unchanged limits under the new fixture profile, then the source-bound final C1 gate. Retain unoptimized failure logs and optimized actual results separately. A passing optimized test is not evidence about unoptimized builds; identify coverage exactly. A remaining genuine optimized overrun is fixed or reported, never excused or widened. Formal/model source evidence is refreshed for changed source hashes; profile selection is implementation/build evidence, not a new formal proof of latency. No new waiver, full unrelated CI run, operator prompt, install or publish is introduced.