---
type: note
title: Native C1 metric interface reviewed; full corrected-source gate pending
links:
- to: '[[tasks/2026-11-05-dream-c1-native-debt-producer]]'
  rel: implementation
- to: '[[tasks/2026-11-05-dream-c1-native-fixed-review]]'
  rel: review
id: 01M6PKZK4AC8NGS24Z0Y64VMG7
updated: 2026-11-05T19:59:24.348Z
created: 2026-11-05T19:46:42.465Z
number: 93377961365144
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:e2a2a9a5128be4966ad5b5b433c7fae66df3f1c55523b0a444679283697d7e22
  frontmatter_digest: sha256:38d3be7bb9bd0b9f699edaa0315f8bc651a07609543817bf34d46fa5fc40d215
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: meJY5zqiw8cn3EuR2oJ3CnzbaFIsAyHTwkshI3uwXS0s0bErwAugR5JV8dqC3VqmdpIAxkwKKU4bY0S1QYkRCg==
  log: .bboxy/log/2026-11-05.jsonl#141268
---
Oracle read actual signed ffa09e52b8302ed74aaba4e4c7ede3bf653b7954 and16d7406dbb62c939933391783666bd7c3d36c2c1 (both G/approved844ECC0587DE7AB0) above accepted47182320; tracked tree clean at initial19:39:49 UTC.10changed paths fit the fresh producer task; no wire/schema edits. README commits c1-native-debt/1 exact bounded metric names/units/cutoffs; initial direct source review inspected native_call, Accounting charge/pay, strict unit/control/cleanup, parked-owner bootstrap/join, publication custody and pre-descriptor capture vs terminal metrics. This is an intermediate reviewed interface/source milestone, not final component acceptance or new consumer pin.

Actual canonical log target/dream-c1-native-evidence/test-dream-c1.log shows15unit passes, affected fingerprint/process checks and four_ordinary_release_workers_complete_multi_mib_with_independent_control_and_reap positive. In that ordinary cargo-build pass,200 reported Paused-frame timings p99~22us/max2180us (causal RTT UNPROVEN: initial queued compute Paused was not consumed before the timed requests), all4jobs complete/reap, individual source2503989B/202envelopes, no work overruns and terminal debt0. Terminal strict app maxima~0.25–0.29ms, control~0.045–0.073ms, observed heap<1MiB, RSS~10MiB. These are direct observed component log facts, not actual term admitted MCP/authority/nativeframes/shared combined conformance. The overall canonical gate was still running; no final exit0 claimed.

Earlier source-milestone artifact target/dream-c1-native-profile/worker-milestone SHA986fd7234573a67d05d402d620d98be979f85875886e550603f7469627d62920 came from cargo test --release, defaultacp-agent/no-test-fixtures/rustc1.99.0 b940084d7/aarch64-apple-darwin, per author's handoff. Feature unification may distinguish it from ordinary cargo build. Inspected Makefile now cargo-builds/copies target/dream-c1-native-profile/worker and stores source/compiler/hashes BEFORE cargo-test; DREAM_C1_RUNNER explicitly exercises that distinct retained ordinary worker. Final full source/features/hash/exit receipt remains required. No milestone artifact is silently promoted to the final canonical worker.

Concrete within-scope finding sent to author: fixed16d cleanup_unit:755-766 uses end.saturating_sub(at), turning a finite reversed CPU clock into0 and leaving cost_known true. Existing interval(at,end) already correctly produces unknown/MAX. Requested checked equivalent and scoped injected reverse-cleanup clock witness retaining effects/disposal, sticky failure and no zero substitution. Static clock-law defect only; no actual OS clock reversal or measured user latency failure claimed. Any16d passing gate remains baseline until this corrected signed source and fresh source-bound evidence. No new source scope or raised limits.

Architect [[tasks/2026-11-05-dream-c1-native-fixed-review]] assigned read-only FIXED16d interaction/coverage review; author may fix independently, later diffs are unreviewed until root reads them. No approval/status wait. Producer native/debt implementation, consumer parent/SQLite/parser/native integration, and Q2 selected-public-proof/helper work remain separately owned. Original accepted task stays frozen; no unreviewed submodule bump, install/release/landing or final Dream availability.

Separately consumed and accepted Architect [[tasks/2026-11-05-dream-q2-helper-budget-disposition]] after full report39598dea30ac7e49f183b03240fd1bc6d14165ad17c564bde820d339e1eb279f/current decision e8b872dc5ec5006d19cf33bfea70bd10f1c5aeb357ade6e19ee7fff2722775f2 review and own git diff --check0 in Q2 tree. Final decision adds actual fixed implicit GPG verification argv/options, job scratch/internal-buffer coverage and explicit model initial/safety/liveness statements to the earlier read version; no wire or original C1 change. Q2 signed amendment remains in force and points to the current decision; failed Q2 baseline remains failed. Receipt storage/claims are not model receipt or runtime proof.

## Subsequent fixed-review evidence correction
Root's earlier description of200 causal responsive roundtrips overstated the evidence and is withdrawn. [[reports/2026-11-05-dream-c1-native-fixed-review]] digest1cded51bba541ebb592024919371bb1d4425194c27f02897af1cd1a2881c5174 establishes F2 unavailable owner/process tail samples could retain cost_known=true/MAX passing >0, and F3 stale initial Paused may satisfy first timing request. Both are confirmed against fixed16d source and sent to author. Four actual successful completion/reap observations remain real. New signed45752489 attempts F1–F3 corrections; root has read their delta mechanisms but final source-bound gates/ordinary artifact and full causal witness still need verification, no acceptance claimed. Original focused/formal exit0 receipts are baseline (formal sourcef1b6), not proof for later corrections.

Consumer additionally found a missing cross-process bootstrap handoff: new child may create its compute owner before parent process-create inclusive cost/debt is known/paid. [[tasks/2026-11-05-dream-bootstrap-admission-boundary]] assigned to Architect to define minimal private gate or explicit fully charged startup interpretation, no silent bootstrap exemption/wire bump. Producers/consumer/Q2 continue independent work; final integration cannot claim exclusive/debt-correct startup until the contract is implemented and causally witnessed. Accepted fixed-review task records completed review, not component acceptance.