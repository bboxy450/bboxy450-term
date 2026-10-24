---
id: "01M5NE355RPRC0YM0TFH86EPGR"
type: "decision"
number: 44
title: "Attention: a waiting seat swells, pulses and notifies; the notification focuses it"
status: "decided"
decided: "2026-10-23"
operator:
  - "if harness has a question for the user - its name in the tab left bar and its borde should autoswell and pulsate a lot, and user should get desktop notification that this harness needs attention - and clicking on that notification would focus the harness terminal"
links:
  - { to: "[[2026-10-22-architecture-round-1#44]]", rel: "supersedes-as-index" }
  - { to: "[[attention]]", rel: "decides" }
created: "2026-10-23T22:28:51Z"
updated: "2026-10-24T15:36:34Z"
---
# Decision 44: Attention: a waiting seat swells, pulses and notifies; the notification focuses it

**Attention: a waiting seat swells, pulses and notifies; the
notification focuses it (operator, 2026-10-23, verbatim: "if harness
has a question for the user - its name in the tab left bar and its
borde should autoswell and pulsate a lot, and user should get desktop
notification that this harness needs attention - and clicking on that
notification would focus the harness terminal").** Spec [[attention]]:
trigger is the existing `Activity::Waiting(kind)` on the events lane,
no new detection; the row and block loop the swell in waiting amber
until the seat leaves waiting or is focused; paint only, never focus or
layout ([[d29-explicit-seat-placement|#29]]); one `UNUserNotificationCenter` notification per episode,
identifier = seat id, click → `Focus(seat)` through the sidebar-click
path. Mechanism decision: the build produces `bboxy450-term.app` with an
`Info.plist` for every run, ad-hoc signed (Developer ID stays deferred,
[[d24-defer-code-signing|#24]]), because a bare binary has no bundle identifier and cannot post a
clickable notification; an unbundled run swells in the window and
posts nothing (no `osascript` fallback: `AGENTS.md` bans desktop
scripting, Oracle's correction). Ad-hoc signing is not identity
signing, so [[d24-defer-code-signing|#24]] holds (Oracle). Muted seats never notify. Rejected: polling
the harness for questions (the adapters already report them); a
notification that steals focus on its own; a separate notification
daemon.

**Amended 2026-10-24 (probe with the operator present, Vesper's report):** the bundle is signed with the approved Apple Development identity, never ad-hoc (probe with the operator present, 2026-10-24, Vesper's report: an ad-hoc-signed bundle gets "Notifications are not allowed for this application" from `UNUserNotificationCenter` and macOS never shows a prompt; the same bundle signed with the approved Apple Development identity, team UQB3368A84, works first time). Recorded in [[desktop-notifications]] §3 and §5.
