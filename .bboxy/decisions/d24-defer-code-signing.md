---
id: "01M5K5V3TGP3QZQNY6XAH0ZQPG"
type: "decision"
number: 24
title: "No code signing until we distribute"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#24]]", rel: "supersedes-as-index" }
created: "2026-10-23T01:26:10Z"
updated: "2026-10-24T15:36:34Z"
---
# Decision 24: No code signing until we distribute

**No code signing until we distribute (operator, 2026-10-22; velocity
first).** The PKG0 cargo runner is removed from main the day it landed,
PKG1 is deferred, `scripts/sign-run.sh` is kept for later; nothing may
depend on a signed build or a keychain prompt. Custody stays at tier 2
with ID1e's non-interactive, bounded keychain access on every developer
machine; tier 1 returns with the signed bundle at distribution. [[d17-device-key-custody|#17]]'s
tiers are unchanged, only their timeline. Rejected: keeping the runner
for its keychain-ACL stability (it put a signing step and a certificate
dependency in every cargo test for a benefit ID1e made unnecessary).

**Amended 2026-10-24 (probe with the operator present, Vesper's report):** a Development identity signature on the local bundle is now required for notifications, which is not distribution signing; Developer ID for distribution stays deferred (probe with the operator present, 2026-10-24, Vesper's report: an ad-hoc-signed bundle gets "Notifications are not allowed for this application" from `UNUserNotificationCenter` and macOS never shows a prompt; the same bundle signed with the approved Apple Development identity, team UQB3368A84, works first time). Recorded in [[desktop-notifications]] §3 and §5.
