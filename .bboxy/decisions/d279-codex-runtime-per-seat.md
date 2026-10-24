---
id: 01M5QN3ZPZT9210SSZKGHNBFWE
type: decision
number: 279
title: "Codex wake: a seat's TUI and its native control share one explicitly selected runtime per seat, never the operator's user-global daemon; notLoaded is unknown, never idle"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[harness-kits]]", rel: decides }
  - { to: "[[harness-adapters]]", rel: decides }
  - { to: "[[d265-codex-seat-trust]]", rel: amends }
  - { to: "[[w2-5-mcp-and-seat-wiring]]", rel: constrains }
created: "2026-10-24T19:10:06Z"
updated: "2026-10-24T19:10:06Z"
---
# Decision 279: Codex wake binds to one runtime per seat

Ruled by the Architect on 2026-10-24 from Sable's live evidence: after `SessionStart` rebinds the thread id, native control connected to the operator's **user-global** Codex daemon socket (`/private/tmp/codex-daemon-501/…`, owner pid 13411, the same daemon serving the operator's own Codex tabs). There the seat's thread was `notLoaded` while the seat's TUI (pid 94186) was running it, so every wake preflight was rejected and Codex moved only after a `ctl` nudge.

**Alternatives.** (a) Keep discovering the user-global daemon and treat `notLoaded` as a reason to load the thread there. (b) The seat's TUI and its native control are bound to **one explicitly selected runtime per seat**, chosen at launch and recorded in the seat's binding, either a seat-private Codex app-server the wake-up kit starts with a per-seat socket under the seat's overlay and the TUI attached to it, or the TUI pinned to a known socket through a **documented** Codex mechanism that keeps the operator's login without copying `auth.json` (#265). The kit discovers nothing user-global for a seat. Id and status validation stay; `notLoaded` is `unknown`, and a wake against an unknown thread is `NativeUnknown`, stop and show, never a retry and never idle. (c) Drive Codex through the PTY only.

**Choice.** (b).

**Reason.** The operator's own daemon is the operator's: loading a seat's thread into it, or depending on it, puts a seat's state into a process we do not own and the operator did not hand us, the same reasoning as #260 and #265. A runtime per seat is what the binding (seat, generation, session) already describes; discovery against a global locator is the bug. (c) throws away the native path that works. **Which documented Codex 0.163 mechanism** yields the per-seat runtime, a seat-private `codex app-server` with the TUI attached, or a socket or home selection that shares the login without a copied credential, is a read-only research item for Sable against the Codex docs with permalinks, before code; the record is [[harness-adapters]]'s Correction section or a new research record, and the first shape that is documented wins. Rejected: inventing a flag; loading threads into the operator's daemon; `notLoaded` as idle.

**Rules.** [[harness-kits]] §2: the wake-up crate's Codex adapter takes the runtime endpoint from the launch plan and never from a user-global path; `WakeMethod::CodexRpc` is reported only when the seat's own runtime answers `idle | active` for the bound thread. [[harness-adapters]]: `notLoaded` maps to `unknown`, never to idle. The e2e promise L4 (first-turn `orient` for a Codex seat) reads against it.

Owner: Corvin Duskmantle <ARCHITECT>.
