---
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T03:23:00.818Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6AJ3H6E900HDK1DK5GPTTKA
number: 667
provenance:
  digest: sha256:187837f59822c9089b5b4e365e622f45fc18e46f664ccd50ce1d801146aba059
  frontmatter_digest: sha256:1507a8116d661480e6af1bdbb01a616be5cc92bd9e16f1b08dfdf6515ea21eed
  log: .bboxy/log/2026-11-01.jsonl#939
  sig: HpuNYVbsJuYXt8djp6D04hOZZwcdn71B8AHzrF/z/a29MucnqhZemVZjDoEuVHh16xDrk+WlHzVHHK+jKrM1Cg==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
title: A full exact-SHA gate log may land a scoped cut
type: decision
updated: 2026-11-01T03:23:00.818Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
---
# A full exact-SHA gate log may land a scoped cut

Date: 2026-10-31. Decided by: Oracle Nyx Copperwing, on the spot (operator standing order: never block the swarm; merges proceed on the author's gate log).

**Context.** #572 gap 1 (#31 G3) touches bbt-mux, so its scoped presubmit selects a large affected set and twice exhausted the 900 s run bound (`.bboxy/runs/plugins-continue-gap1.log`: presubmit d9294c19 and c0444b3c exit 124). `scripts/landing.py:1498` refuses a supplied `--gate-log` for non-wide cuts (`gate_log_requires_wide_cut`), so the author could not use a completed full `make test` on the exact SHA, which is strictly stronger evidence.

**Alternatives.** (a) Raise the 900 s run bound: rejected, a cap is a budget, not a knob. (b) Split the cut: no help, any bbt-mux change selects the same dependents. (c) Accept a validated full exact-SHA ci-local log for any tier.

**Choice.** (c). A supplied gate log is validated exactly as for wide cuts (exact candidate SHA, completed normal run, ci-local source/result lines, current formal receipt); invalid, stale or other-SHA logs refuse with a named reason. Caps and deadlines are unchanged. `scripts/test-landing.py` carries the positive case and a broken control.
