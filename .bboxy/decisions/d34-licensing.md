---
id: "01M5MSD5JRC1ZGH64EARVGSTNM"
type: "decision"
number: 34
title: "Licensing: dual MIT OR Apache-2.0 by default, bbt-core Apache-2.0 only"
status: "decided"
decided: "2026-10-23"
operator:
  - "lets do dual license, be smart about this, ask architect as well what project will have what license"
links:
  - { to: "[[2026-10-22-architecture-round-1#34]]", rel: "supersedes-as-index" }
  - { to: "[[licensing]]", rel: "decides" }
created: "2026-10-23T16:27:19Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 34: Licensing: dual MIT OR Apache-2.0 by default, bbt-core Apache-2.0 only

**Licensing: dual MIT OR Apache-2.0 by default, `bbt-core` Apache-2.0
only (operator, 2026-10-23: "lets do dual license, be smart about this,
ask architect as well what project will have what license").** The map
is [[licensing]] §1. The one exception is forced by provenance: a fork
of Apache-2.0-only `alacritty_terminal` cannot be offered under MIT.
Every publishing crate carries its own license files; root gets
LICENSE-MIT, LICENSE-APACHE and a NOTICE inventory of vendored code;
vendored headers stay verbatim. The sweep is an Executor task with
`scripts/licenses.sh` as its gate. Supersedes the two earlier yarn
answers (MIT only; Apache only): yarn stays dual, as its upstream is.
Rejected: Apache-only for the whole project (drops GPLv2
compatibility and mismatches the ecosystem for a patent clause a user
can already elect); MIT-only (no patent grant, and impossible for
bbt-core); relicensing alacritty code (not ours to relicense).
