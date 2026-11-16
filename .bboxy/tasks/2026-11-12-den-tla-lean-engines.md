---
type: task
title: 'Den and Yarn: liveness, TLA+ export, Lean proofs and trace conformance over one model'
verify: cargo nextest run -p bboxy450-tlc -p bboxy450-den -p bboxy450-yarn && make test-formal
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
id: 01M78ECVP5ZVY6F98EM82AAQBM
status: open
delegated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
swarm: 01M649102FBS60QWNPZS5VAGF9
number: 1794292861627493
created: 2026-11-12T17:55:25.484Z
updated: 2026-11-12T18:30:44.535Z
created_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
updated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
authorship:
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  kind: seat
  public_key: lZh8jo1S9rWtfvb9+EUefnBFfbJ6FtBuf6dXC19gMpk=
  process_generation: '1'
provenance:
  digest: sha256:768ea2f4dbba1f75c37983fb109720ad656c6affcd315576d1e11f13561754af
  frontmatter_digest: sha256:d8ec4b170986a671753183c2be20030ef41a1a8939baac518366c9d7e7d706a5
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  sig: 3vYIEX0OGUKgaH/Xb4CQ2Ynyho7sToINbsR7fnghYmegcOjAksyoljSq2re01hCW0eZwcJexScQl5IoJ77KoBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151640
  event:
    id: 01M78GDHNR70YH6W887GRP06RN
    sequence: 151640
    hash: 46229d124dbcf1d931de492226fca17d4b74e32a1246cb4bc4082b73a2f99a2a
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
