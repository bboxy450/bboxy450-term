---
id: "01M5JGX4C0ND5VBY5G5ZT04WPF"
type: "decision"
number: 12
title: "Audience and the plugin seam"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#12]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:20:16Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 12: Audience and the plugin seam

**Audience and the plugin seam (operator).** Hackers and furries first;
cloud and ssh support that never locks anyone in; then teams and
enterprises want it; opinionated, not locked; developers. **No plugin
system yet.** Shape recorded for later: a binary in a folder, the terminal
runs `binary --shape` to learn how to integrate, the plugin runs as a
subprocess of our runtime and may expose sockets, a CLI or a daemon.
First kind: cloud and ssh providers, the out-of-process form of [[d7-provider-supervisor-link|#7]].
Rejected: a plugin API now (scope); a scripting runtime (not our shape).
