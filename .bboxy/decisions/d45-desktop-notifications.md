---
id: "01M5NG5SZR420ZH0YTG0QVC0BC"
type: "decision"
number: 45
title: "Desktop notifications are a facade over one crate per OS"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#45]]", rel: "supersedes-as-index" }
  - { to: "[[desktop-notifications]]", rel: "decides" }
  - { to: "[[desktop-notifications-research]]", rel: "evidence" }
created: "2026-10-23T23:05:15Z"
updated: "2026-10-24T15:36:34Z"
---
# Decision 45: Desktop notifications are a facade over one crate per OS

**Desktop notifications are a facade over one crate per OS (operator
via the Oracle, 2026-10-23: an OS-agnostic facade
`bboxy450-desktop-notifications` with per-OS sub-crates, macOS first
and real, Windows and Linux dummies marked "DO RESEARCH BEFORE
IMPLEMENTING").** Spec [[desktop-notifications]]: the facade holds
`Notifier`, `Notification`, `Response`, `Error` and selects the backend
by `cfg(target_os)`; `open` once at launch with a response callback the
caller forwards to its own loop; macOS on `UNUserNotificationCenter`
with the process as delegate, identifier = replace semantics, a bundle
check before the centre is touched (`NoBundle`); dummies answer
`Unsupported` by name at run time, an unknown target fails at compile
time; the four crates lead the layering order and carry no `bbt-*`
dependency; dual licence. **Use versus build: build**, from
[[desktop-notifications-research]] (same evening): the macOS crate sits
on `objc2-user-notifications` 0.3 directly, the objc2 family winit
already links, with a `define_class!` delegate set before
`EventLoop::run`; about 300 lines. Rejected on evidence: notify-rust
(its UN path is a preview feature with blocking handles and a delegate
it does not own), user-notify (LGPL-3.0, UUID identifiers so no
replace, tokio on every target), mac-notification-sys (deprecated
NSUserNotification behind a bundle-identifier swizzle),
usernotifications-rs (a day old, bespoke FFI). Two Apple behaviours
nobody documents are acceptance tests, not assumptions: ad-hoc bundle
sufficiency and delivered-identifier replace. Names are free on
crates.io (404 on all four). Rejected: notifications inside the app crate (unportable, and
attention is a kit consumer); one crate with `cfg` blocks per OS (the
seam hidden, the dummies invisible).

**Amended 2026-10-24 (probe with the operator present, Vesper's report):** the bundle is signed with the approved Apple Development identity, never ad-hoc (probe with the operator present, 2026-10-24, Vesper's report: an ad-hoc-signed bundle gets "Notifications are not allowed for this application" from `UNUserNotificationCenter` and macOS never shows a prompt; the same bundle signed with the approved Apple Development identity, team UQB3368A84, works first time). Recorded in [[desktop-notifications]] §3 and §5.
