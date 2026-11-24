---
allowed_paths:
- crates/bbt-mux/**
- crates/bboxy450-harness-wake-up/**
- crates/bboxy450-term/**
- scripts/**
- Makefile
- LAND.md
- verification/**
- .bboxy/specs/**
- .bboxy/decisions/**
- .bboxy/bugs/**
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-12T22:01:28.010Z
created_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M78WFC54943HGK458NRD1SHF
number: 1794292861627495
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: implemented
  evidence:
  - commit ff8109e68 + decision d1794292861627496
  fresh_accepted: false
  kind: historical
  op_id: 01M878KMWRHJ19AABW6A0FGFWR
  previous_status: open
  reason: Refusal became a 50 ms OVERRUN measure (recorded deviation).
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Dream C1: a CPU overrun is measured, not refused; restore the Dream tests'
type: task
updated: 2026-11-24T17:10:44.110Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: cargo nextest run -p bboxy450-term-mux --lib --retries 0
provenance:
  digest: sha256:717548573519e58df0938c480226cd1e4a02848cede8f0e4b5b4ace3acd64098
  frontmatter_digest: sha256:8fb8c68c6e73308e3cd2abdc0d252278a6b15a15fb21979748cfff6241c4b994
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: 1t/o6t1QpSWNLKuNBe8zsK/alGtXZ8xxdvU0mjSeaj9t64TSECzx592nj4dQTYS6L0+xi9dKZdb8O4h21cyJAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1891
  event:
    id: 01M878KNS0VEG282H7G2J7G4V9
    sequence: 1891
    hash: 844f3d4f84a1da31bb95a40ee08b838d6b791b774594c30b1c10eee69fd5f890
---
## Goal

Operator, 2026-11-12: "fix C1 and get that entire thing working! dreaming is an important
part of our system", "simplify DREAMING and LESS is more ... build on working stuff rather
than try to ship not working complexity", "KISS".

Dream C1 refuses any step that used more than 1 ms of thread CPU (`Code::ResourceLimit`).
On a shared machine that is a property of the machine, not of the code: with retries off
the mux library failed 4 to 8 Dream tests per run, a different subset each time
([[bugs/mux-dream-c1-cpu-budget-tests-flaky]]), and the same refusal fires in the product.

## Change

A step over 1 ms is **counted and reported, never refused**. The counters and
`max_unit_cpu_ns` stay as measurements. The job deadline and cancellation, which already
exist, remain the only limits. No new constant, no hard cap, no debug/release split.

Then restore the Dream tests deleted in `f2d8e4076` from git, delete the tests whose only
purpose is to prove the refusal, and show the mux library passes repeatedly with retries
off.

Supersedes the "exact enforceable law" of
[[decisions/d93377961365086-dream-native-quantum-and-debt]] for the refusal only; the rest
of that decision (rest after work, native call accounting) is untouched here.

## Acceptance criteria

- [ ] No Dream C1 path returns `ResourceLimit` because a unit's CPU time exceeded 1 ms.
- [ ] Overruns are still counted and visible in the host metrics.
- [ ] The restored Dream tests pass in five consecutive mux library runs with `--retries 0`.
- [ ] A decision record states the change and what it gives up.

## Verification

`cargo nextest run -p bboxy450-term-mux --lib --retries 0`, five times.
