---
type: report
title: Member-only root manifests preserve narrow lean landing plans
task: '[[tasks/2026-11-10-land-plan-member-only-manifest]]'
author: seat:01M731RY7V5R0Y66H8G05M8EEJ
created: 2026-11-10T15:51:06.854Z
commits:
- 3d3a708b5d6b7ed9ce10df3ad1ed047502526715
verify:
  command: python3 -I scripts/test-landing.py && make test-formal-infrastructure
  exit_code: 0
id: 01M732FS7W46KYD287TKBTRRAF
updated: 2026-11-10T15:51:06.854Z
number: 1794292861627455
created_by: seat:01M731RY7V5R0Y66H8G05M8EEJ
updated_by: seat:01M731RY7V5R0Y66H8G05M8EEJ
authorship:
  signer: seat:01M731RY7V5R0Y66H8G05M8EEJ
  kind: seat
  public_key: ym9yQeNXPWyMX6qW4la2g0gA5teyC7I/mAj2fL59w2w=
  process_generation: '1'
provenance:
  digest: sha256:eb2c6639ad3bc01b81a737a9ff00f522ea8327c1799a16f4f4b30fa83a63bfa7
  frontmatter_digest: sha256:0b93efc930898b6ee131479a733e5b381cc860e5538c5bcf95405b850b2e7f9c
  signer: seat:01M731RY7V5R0Y66H8G05M8EEJ
  sig: wyOiMQjLxEicHVkjUEraRUFiYIHw8xSfzgWWqxiE1qmyGgOIfmoRaEMMrjIf/4BcESupVojXl60dnvCT5Al0Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151499
  event:
    id: 01M732FTES9K796ZCF4D3BYFSB
    sequence: 151499
    hash: 62e1f59b4905145da849e4447d16d47702088d1e84424f684b3a604841f69f60
---
Implemented Amendment 1 of [[tasks/2026-11-10-land-plan-member-only-manifest]]. Source commit `3d3a708b5d6b7ed9ce10df3ad1ed047502526715` is signed by Boxy with key `9B07E849480C68328CA20684844ECC0587DE7AB0` and landed on origin/main. The managed implementation worktree was clean immediately after landing.

## Changes and subtraction

- `scripts/landing.py:706` — one pure TOML predicate preserves the old ordered member list, compares every other manifest field, subtracts identical old lock tables with multiplicity, compares all remaining lock metadata, and permits only extra source-free tables named by added members. Exact member-directory metadata supplies package names; a directory basename is not assumed to be the package name. Unchanged lock pairs are allowed; lock-only changes, parse failures, missing endpoints, duplicate members and unknown member paths remain broad.
- `scripts/landing.py:748` — affected() reuses the existing git helper, path-to-package loop and dependent_closure. Explicit ranges read both committed endpoints, including historical moved-main ranges; BASE.. uses HEAD and a single BASE uses worktree files. Shared inputs are captured before removing proven member-only root manifests from broad. Toolchain and .cargo paths remain broad.
- `scripts/test-landing.py:303` — eight direct predicate tests exercise every rule clause, full lock-table identity/multiplicity, reordered lock tables, dependency-list order, malformed TOML and missing endpoints. Ten added affected-set tests exercise cases (a)–(h), .cargo, missing OLD lock and historical/omitted endpoints.
- `LAND.md:12` — states the member-only exception, shared validation inputs and conservative fallback.

Removed the unconditional whole-workspace expansion for a positively proven member-only manifest pair, the late shared-input calculation that would lose those manifests, and the documentation's blanket claim that every root-manifest change touches every crate. No files or pre-existing tests were deleted. Existing attribution headers and root ATTRIBUTION.md already identify the project, canonical source and reuse policy; they remain intact.

## All three manifest checks

| Site | Decision and disposition |
| --- | --- |
| `scripts/landing.py:757`, affected() | Which crates enter touched/tests. This is the only changed decision; positive member-only proof filters root manifests from broad. |
| `scripts/landing.py:1203`, parity_cases() | Which visual comparison cases are selected. Unchanged; it does not force crate tests. |
| `scripts/landing.py:1375`, clean_source() | Whether untracked files are source and must be refused. Unchanged; it does not compute crate scope. |

Moved-main logic already calls affected() and intersects its transitive dependency closures, so it inherits the corrected computation without a second predicate. No assumptions/property in verification/contracts.toml describes affected-set computation. LandingVerification at line 397 concerns source-bound verification/reuse; contracts and models were left unchanged.

## Acceptance evidence

| Criterion | Evidence |
| --- | --- |
| (a) exactly the added crate | `scripts/test-landing.py:417` asserts touched and affected equal [new-package], non-wide tier, shared root manifests and the sole lean line `test new-package --lib`. |
| (b)–(h) every package | `scripts/test-landing.py:426` through the named regenerated-consumer case at line 466 invoke assert_broad_manifest(), comparing touched to real Cargo metadata and requiring wide tier. Case (h) regenerates the lock offline and asserts the existing consumer gained the dependency; no stale lock is used. |
| Pure rule clauses and parse failure | `MemberOnlyManifestTests` at line 303: eight methods with clause-specific subtests; parse failures at line 382 return false/broad. |
| Existing tests unchanged/pass | Final required command: 244 fast landing fixture tests, zero failures, plus 33 formal infrastructure tests, OK. All pre-existing test bodies remain unchanged. |
| Documentation | LAND.md paragraph beginning at line 12. |
| Verification and landing | Required command exit 0; scrubbed make land exit 0, printed no affected crates and an empty lean plan, and pushed the signed source commit above. |

## Commands and actual results

| Command | Actual exit/result |
| --- | --- |
| `python3 -I scripts/test-landing.py MemberOnlyManifestTests AffectedTests` | 0; 31 tests, OK, 30.416s. Iteration check before the endpoint controls were strengthened. |
| `python3 -I scripts/test-landing.py && make test-formal-infrastructure` (first run) | 0; 244 landing tests/0 failures, 33 infrastructure tests/OK. Repeated after review corrected omitted-RHS endpoint handling and strengthened the valid lock-removal control. |
| `python3 -I scripts/test-landing.py && make test-formal-infrastructure` (final source) | 0; 244 landing tests/0 failures in 77.768s; 33 formal infrastructure tests/OK in 2.188s. |
| `git diff --check` and `git diff --cached --check` | 0. |
| `git add -- scripts/landing.py scripts/test-landing.py LAND.md` | 1 with an ignored-directory warning, but git status and cached diff confirmed exactly the three tracked files were staged. No force-add or ignore-policy change was used. |
| `git commit -m "Keep proven workspace-member additions narrow in lean land"` | 0; signed ff3a8f7ed before the authorized landing rebase; final signed source SHA is 3d3a708b5d6b7ed9ce10df3ad1ed047502526715. |
| `git verify-commit HEAD` (before and after rebase) | 0; good Boxy signature with the approved key. |
| `python3 -I scripts/landing.py affected origin/main..HEAD` | 0; touched=[], affected=[], parity=no, native_inputs=[]. |
| `python3 -I scripts/landing.py lean origin/main..HEAD` | 0; empty plan. |
| `df -h ~` | 0; 26 GiB available before landing, above the 8 GiB stop threshold. |
| `env -u BBOXY_CAPABILITY_FILE -u BBOXY_GENERATION -u BBOXY_PROCESS_GENERATION -u BBOXY_ROLE -u BBOXY_RUN_DIR -u BBOXY_SEAT_ID -u BBOXY_SOCK make land` | 0; initialized pinned harness, affected none, empty lean plan, LANDED 3d3a708b5d6b7ed9ce10df3ad1ed047502526715. Total wall 12.186s, child CPU 6.605s, target growth +4096 bytes. |
| `git status --short`, `git rev-parse HEAD`, `git rev-parse origin/main` | 0; clean worktree, both revisions equal the landed source SHA. |
| `shasum -a 256 target/member-only-final-verify.log target/member-only-land.log` | 0; hashes below. |

Source/Git/instruction reads and make help succeeded. Two optional filesystem probes returned 1 because the requested sample report and .bboxy/.git were absent; git check-ignore returned 1 because the named tracked scripts were not ignored. Neither was a verification failure.

The actual landing plan/output was:

```text
land: affected: none
land: lean plan:
  
LANDED 3d3a708b5d6b7ed9ce10df3ad1ed047502526715
land-cost: total wall=12.186s cpu=6.605s disk=+4096B exit=0
```

Private logs in the managed worktree target/ (not committed):
- member-only-final-verify.log SHA256 `66b2e416f5da034e98b0775c7f60b9f7214384c6250c5ee198c94ebc8fad4a13`.
- member-only-land.log SHA256 `cc165092cdf1fc5891c37e6f5a69f07cbe97bb10aec805030828bd3988f84ea5`.

## Limits

No Rust source, quarantine, formal-stage behavior, make land-full or other gate policy changed. The six extended --all moving-main/recovery fixtures, full TLC/Yarn corpus, crate suites, native UI, smoke and e2e were not run: the scoped change is a pure selection/parser predicate and its private Git/Cargo-metadata fixtures. Unknown member/glob mappings and three-dot ranges conservatively retain broad behavior. No new crates or stateful model transitions were introduced. Only the required formal infrastructure self-tests were run; this report does not claim full formal verification. Report checkpoint metadata and the final handoff carry the report's separate committed artifact identity.