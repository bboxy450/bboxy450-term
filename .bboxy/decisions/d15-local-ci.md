---
id: "01M5JH9DXRP40T1YM1BVC8NQTS"
type: "decision"
number: 15
title: "CI is local"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#15]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:26:59Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 15: CI is local

**CI is local (operator, 2026-10-22).** No cloud CI during heavy
development; GitHub Actions runs on manual dispatch only and comes back
per crate when it is open-sourced. `sh scripts/ci-local.sh` is the gate:
layering, build, test, ui-lint, and `--parity` when lab, design, UI or
renderer changed; exit 0 is green. Every handoff and merge reports
`sh scripts/ci-local.sh — exit 0 at <commit>` in the kb; that line is the
CI evidence. Found by the Architect: no cloud run had started since 07:27
UTC that day. Rejected: deleting the workflow; paying for cloud runners
now; a second script (the Architect's `ci.sh` was removed in favour of the
Oracle's, same gates).
