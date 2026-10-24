---
id: 01M5Q7XSKZXP3YG34H7JN01GB0
type: decision
number: 270
title: "ctl can inspect any seat's screen, send it text and terminal codes, and wait on it, under the operator principal; the Oracle answers native approval prompts through it with one guard"
status: decided
decided: "2026-10-24"
operator:
  - "find a way to auto approve them yourself! bboxy450-term should be remotely controllable, with ability to inspect any seat and its output and send it terminal codes! so use that!"
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[d260-ctl-authority]]", rel: amends }
  - { to: "[[d265-codex-seat-trust]]", rel: amends }
  - { to: "[[d269-ctl-door-and-issuer]]", rel: builds-on }
  - { to: "[[c1-live-seats]]", rel: builds-on }
created: "2026-10-24T15:19:32Z"
updated: "2026-10-24T15:31:48Z"
---
# Decision 270: ctl inspects and drives seats

Decided by the operator on the morning of 2026-10-24, when the Codex trust prompt stood between an unattended e2e seat and its first turn. Written by the Architect at the Oracle's ask.

**What `ctl` gains.** `ctl screen <seat>` reads a seat's visible screen (the same `screen_read` the MCP tools serve). `ctl input <seat> <text|codes>` sends text and terminal codes to a seat through **the same `GuiAction::Input` the window's keyboard uses**, so nothing reaches a PTY by a second path. `ctl wait <seat> <condition> [--timeout ms]` blocks until the seat reaches a state (`idle`, `waiting`, `exited`) or its screen matches a text, and returns the state and the screen at that moment, or a named timeout. All under the operator principal, `door: ctl`, `issued_by` carried, every call journaled and audited (#260, #269).

**No new authority.** The operator can already read every pane and type into every pane in the window; `ctl` reaches the same two things through the same actions. **The no-desktop-driving rule is unchanged**: this is our own PTY inside our own app, never the operator's desktop, never another application, never `osascript`.

**#265 amended: delegated approval with one guard.** Codex's native hook review stays, and the operator now delegates answering it to the Oracle through `ctl screen` and `ctl input`. The guard: the Oracle approves only when the review is provably all ours, and otherwise stops and reports with the screen. Codex 0.163 shows only a count and no per-hook list (Seraph's probe, same day), so "provably all ours" means: the count equals the number of hooks the launch overlay installed (`adapters/launch.rs`, generated, never a literal) and no repo-level `.codex` hook source exists under the seat's working directory; a changed user hook raises the count and fails by itself; the approved list is written to the run log and the approval is journaled like every `ctl input` (Seraph's addition, same day). The same shape applies to any native prompt a harness raises on launch: read it, match it against what we installed, answer only an exact match, report everything else.

**The e2e consequence.** In-app testers can be driven and inspected end to end from `ctl`: spawned, delegated to, watched by screen, typed at when a harness prompt stands in the way, and waited on. The unattended night needs no human at the keyboard and no desktop automation.

**Rejected.** Reading seats through a separate inspection socket (a second door); a `ctl input` path that bypasses `GuiAction::Input` (two keyboards); approving a review without matching it against the installed hooks (that is the bypass flag with extra steps, #265).

Owner: Corvin Duskmantle <ARCHITECT>.
