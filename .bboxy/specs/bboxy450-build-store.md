---
type: spec
title: bboxy450-build-store — one disk budget for every worktree's build tree (spike)
status: proposed
id: 01M6HD7Y2TKS8XEQK1EGP8NNMA
updated: 2026-11-03T19:12:43.674Z
created: 2026-11-03T19:12:43.674Z
number: 15489
created_by: seat:01M6HCZWWK2PPMNFN1SJKW21BH
updated_by: seat:01M6HCZWWK2PPMNFN1SJKW21BH
authorship:
  signer: seat:01M6HCZWWK2PPMNFN1SJKW21BH
  kind: seat
  public_key: EdCHgspnF2HeQMN0lvmNqxQy6st++RXk6dJTFu1mWis=
  process_generation: '1'
provenance:
  digest: sha256:1cbe141dba061e28a1afd3984cdaf1d1cab5104672b215fefee60af74fc758fe
  frontmatter_digest: sha256:da14228faf1caa2c80db3e8257b055b97a2b585b6b935cd3982085c8636ddce0
  signer: seat:01M6HCZWWK2PPMNFN1SJKW21BH
  sig: VSRgrg3oipDtAkd6HYxXU/JvZ7YrABIJkjeaxaNcWQcnM3g2XN5y2ppaDJ3dX1Ry4IaivBDW4rcBgHM99txxCQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#92410
---
# bboxy450-build-store (spike, design only)

Crate `bboxy450-build-store`, dir `crates/bboxy450-build-store`, Respectful Slop License like the workspace. It starts with Rust/Cargo and stays open to other toolchains. **Nothing gets built until the gate in §5 passes.**

## 1. Problem, measured 2026-11-03

| Fact | Value | Source |
|---|---|---|
| Data volume | 413 of 460 GiB used, 18 GiB free (96%) | `df -h ~` |
| Worktree target dirs | 22 dirs, 99.5 GiB logical | `du -sk ~/mydev/*/target` |
| Files over 256 KiB | 97.2 GiB; 28.1 GiB (29%) repeat by name and size | `find … stat`, name+size proxy, not hashed |
| sccache | 10 GiB, cap 20G | `scripts/cache-env.sh:44-45` |
| Incremental | off (`CARGO_INCREMENTAL=0`) | `scripts/cache-env.sh:20` |

`du` counts APFS clones once per path, so these are logical bytes. Physical sharing is unmeasured. Incremental sessions are not the cost here. Duplicated finished artifacts and stale worktrees are.

## 2. Prior art (claims from operator-supplied research; verify before relying on them)

- **Kache**: Rust compiler cache with a CAS, CoW restore and worktree seeding. Its documented gap is that the store budget does not cap target-dir usage. This is the baseline to beat.
- **cargo-warm**: APFS clone or reflink seeding of new worktrees; GC only for removed worktrees.
- **sccache**: already in use. Skips incremental builds and linked crates.
- **Cargo `build.build-dir`** (stable since 1.91) and the upstream cross-workspace-cache goal: upstream may make this crate unnecessary.
- **Nix GC roots, min-free/max-free; Bazel AC/CAS split; Buck2 deferred materialization**: lifecycle and policy models.

## 3. Shape, if built

Three layers, each useful on its own:

1. **Inventory** (read-only): sizes by worktree, profile and artifact class; real content hashes; clone-sharing probe; reclaimable estimate.
2. **Compaction**: whole-file dedupe of *finished, unlocked* artifacts with `clonefile` (APFS) or `FIDEDUPERANGE` (Linux). Never hardlinks. Replace only under Cargo's build lock, and keep mtime/permissions.
3. **Budget**: one controller over all target dirs plus sccache. Free-space watermarks (low/high, with hysteresis). Evict whole worktree targets or profiles, oldest-idle first, merged worktrees first. Verify by actual `statfs` free-space delta.

Action caching is out of scope. sccache or Kache keeps that job.

## 4. Invariants (each needs a check before code)

- I1: A file Cargo may write is never replaced. Compaction runs only when no Cargo lock is held on that target dir.
- I2: Compaction preserves byte content, mtime and mode. A post-check hash matches.
- I3: Copy fallback is reported, never silent.
- I4: Eviction never touches a target dir whose worktree has a live process (lsof) or holds uncommitted work markers.
- I5: Reported reclaimed bytes equal the measured `statfs` delta, with estimates labeled as estimates.

TLA+ model (per d365): variables `files`, `locks`, `clones`, `freeBytes`; actions Build, Lock/Unlock, Compact, Evict, CrashMidCompact. Safety is I1 and I2. Broken control: Compact without a lock check.

## 5. Go/no-go gate (cheap, in order)

1. **Hygiene first.** Remove target dirs of merged or abandoned worktrees, which disk-cleanup authorization already covers. Measure the GiB freed. If that restores more than 25% free, the spike's urgency drops.
2. **Real dedupe ratio.** Hash finished artifacts across the 22 targets. Proceed only if real duplicate bytes are at least 20 GiB after step 1.
3. **Baseline.** Trial Kache, and separately `build-dir` plus cargo-warm seeding, on three worktrees. If either reaches at least 80% of the step-2 savings, adopt it and close this spike with a decision. No crate.
4. Otherwise build layer 1, then 2, then 3, each landing separately.

## 6. Non-goals

FUSE, sub-file or compiler-aware chunking, cross-machine caches, replacing Cargo planning, non-Rust toolchains (until the Rust path proves out).
