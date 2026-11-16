---
type: task
title: 'Dream C1: a CPU overrun is measured, not refused; restore the Dream tests'
verify: cargo nextest run -p bboxy450-term-mux --lib --retries 0
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
id: 01M78WFC54943HGK458NRD1SHF
status: open
delegated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-12T22:01:28.010Z
created: 2026-11-12T22:01:28.010Z
number: 1794292861627495
created_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
updated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
authorship:
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  kind: seat
  public_key: lZh8jo1S9rWtfvb9+EUefnBFfbJ6FtBuf6dXC19gMpk=
  process_generation: '1'
provenance:
  digest: sha256:717548573519e58df0938c480226cd1e4a02848cede8f0e4b5b4ace3acd64098
  frontmatter_digest: sha256:c0c53cb91dce57387b92046e0bbb40e3ed8e89f15205a5ef112d4fc42f890739
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  sig: /w8077pn3KmJjf6kDL0Hv5E0Bt07VXOF8AMXexPVEFwxHEFhx8UuICtQ8VsCvQpzKDm4LLL/QK15jiIXcU8SCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151644
  event:
    id: 01M78WFD309J1RM25Q9ED9HN4K
    sequence: 151644
    hash: 7cbda76a1312c7574cb04c2227cd26bcffddcd6eec6f669531a861a83e674dc4
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
