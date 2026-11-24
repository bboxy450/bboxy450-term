---
allowed_paths:
- crates/bboxy450-den/**
- crates/bboxy450-tlc/**
- crates/bbt-yarn/**
- crates/bbt-yarn-macros/**
- crates/bboxy450-term-remote/**
- verification/**
- scripts/**
- Makefile
- .bboxy/specs/**
- ATTRIBUTION.md
- Cargo.lock
- Cargo.toml
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-12T17:55:25.484Z
created_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M78ECVP5ZVY6F98EM82AAQBM
number: 1794292861627493
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: implemented
  evidence:
  - commit c02712486 — Den theorems in Lean 4
  - commit 794bf5ca6 — Yarn Lean proofs; make test-lean --expect-fail
  fresh_accepted: false
  kind: historical
  op_id: 01M878KQFYKX9S2FG60K0MF4R9
  previous_status: open
  reason: Den and Yarn proofs in Lean with an expect-fail control.
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Den and Yarn: liveness, TLA+ export, Lean proofs and trace conformance over one model'
type: task
updated: 2026-11-24T17:10:47.018Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: cargo nextest run -p bboxy450-tlc -p bboxy450-den -p bboxy450-yarn && make test-formal
provenance:
  digest: sha256:768ea2f4dbba1f75c37983fb109720ad656c6affcd315576d1e11f13561754af
  frontmatter_digest: sha256:6fd77e1b1bdc380567e5366e3feed1d57aff15eac7892cd91770141f93902319
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: SMeSaw7XnLS9A07aWMHpbfuigWnC06VRRnlJreI9GRSoVO5twdgTi12tYkEQl0c8IZGvrSo1BArVHyREHp4jCQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1895
  event:
    id: 01M878KRKN1TCE3EXJH1PKY75T
    sequence: 1895
    hash: 15dd49838f53e90812edc5fd6e1567c04380d638e98ff066fb0f1363844e11e3
---
## Goal

Operator, 2026-11-12: "can we make lean and tla+ work under .den? ... GO IMPLEMENT ALL OF THAT! and you can integrate LEAN into bboxy yarn crate as well".

Den stays the one language agents write for specs. TLA+ and Lean become engines under
it, per "One model, many engines" in [[spec-language]].

## Scope

1. **Liveness in Den.** `property` (leads-to) and fairness lines, lowered to the weak/strong
   fairness and response fields `bboxy450-tlc` already checks.
2. **TLA+ text for Den specs.** A Den spec exports the same model text Yarn exports, so it
   joins the native-vs-Java TLC parity corpus.
3. **Lean export, one generator for both front ends.** The finite `bboxy450_tlc::Model`
   has already flattened a set into one variable per member, so it cannot state a theorem
   for every domain size. The generator therefore takes the typed machine before that
   flattening (`bboxy450_tlc::lean::Machine`); Den and Yarn's `protocol!` each print their
   expressions as Lean terms. Den gains `theorem`; Yarn gains `#[invariant(.., theorem)]`.
   Lean runs headless from a pinned toolchain, never in the commit path or product runtime.
4. **Evidence states its strength.** Bounded model checking and an unbounded Lean theorem
   are distinguishable.
5. **Trace conformance.** A recorded event trace from Rust is replayed against a Den spec.

`crates/bboxy450-term-remote/**` is in scope for the real witness: the RemoteDelivery
declaration's theorem flag, its proof and its card hash.

## Acceptance criteria

- [ ] A Den spec with a `property` that fails without its fairness line is refused, with a broken control.
- [ ] Den specs appear in the Java TLC parity corpus and agree.
- [ ] `OUTBOX_001` is proved in Lean for every size of `Msg`, checked by a headless make target; a deliberately false theorem fails it.
- [ ] A Yarn protocol model exports to Lean through the same exporter and its theorem statement type-checks.
- [ ] A trace that the spec forbids is refused by the conformance check.
- [ ] `crates/bboxy450-den/README.md`, `verification/coverage.toml` and [[spec-language]] describe what is built.

## Verification

`cargo nextest run -p bboxy450-tlc -p bboxy450-den -p bboxy450-yarn && make test-formal`
