---
id: 01M5P665BVAPFM9ADTCJ2S84W0
type: decision
number: 260
title: "ctl reaches the GUI socket through an endpoint file in the run directory; seats are refused by process ancestry; ctl is its own audit kind; this is not an OS boundary and says so"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[d31-device-bound-operator-journal]]", rel: amends }
  - { to: "[[mux-daemon]]", rel: constrains }
  - { to: "[[desktop-notifications-research]]", rel: evidence }
created: "2026-10-24T05:29:55Z"
updated: "2026-10-24T05:29:55Z"
---
# Decision 260: how `ctl` is admitted

Ruled by the Architect on 2026-10-24 from Seraph's A-1 question. Facts as Seraph read them: `gui.sock` lives in a fresh random run directory (`bbt-mux/src/logic.rs:1035-1056`); admission is the protocol version plus a 32-byte random capability (`seat_socket.rs:462-472`) that reaches only the app through the logic child's stdout (`cockpit.rs:145`, never logged); nothing writes path or capability to disk, so an outside `ctl` cannot connect today. Harness seats run under the operator's uid, so any discovery `ctl` can use, a seat can use.

**Alternatives.** (A) Logic writes `run_dir/gui.endpoint` `{ path, capability, generation }`, mode 0600, removed on exit; the listener refuses a peer whose process ancestry contains this app's supervisor (`LOCAL_PEERPID`, then the parent chain via `proc_pidinfo`), so a seat's own processes are refused; every `ctl` operation is journaled and audited with kind `ctl`, not `gui`. (B) The same without the ancestry check. (C) A second, operator-minted token with expiry, issued from Settings.

**Choice.** (A), with the refusal explaining itself.

**Reason.** The truth first: on macOS a bare app has no boundary between same-user processes ([[desktop-notifications-research]] A3: no public API lets a parent confine a child), and we already say "a plugin is code you run as yourself" ([[plugins]]). A seat is a process you run as yourself. So "operator authority is reachable by any same-user process" is already the state of the machine, and (C) would add a human step that fights the overnight automation the e2e plan exists for without changing that truth. What (A) buys over (B) is protection against the **accidental** case, which is the common one: an agent that finds `bboxy450-term` in its PATH will try `ctl dm` the first time it is curious. The ancestry refusal stops that, and the refusal **says why** ("`ctl` is the operator's door; your tools are the MCP catalogue"), which is information, not theatre ([[mantras]] 20). The `ctl` audit kind keeps the two doors apart on `account/control`, so a reader can tell the Oracle's hand from the operator's.

**Rules.** The endpoint file lives in the run directory, never at a well-known path, 0600, removed on exit; a stale file never admits, because the generation is checked as today. `ctl` discovers the run directory from `BBOXY_RUN_DIR` or the newest run directory of the uid. The listener refuses by ancestry and names the reason; the MCP catalogue and the `bboxy450` skill tell seats plainly that `ctl` is not theirs. Every `ctl` operation is journaled as `control/op` with kind `ctl` and audited on `account/control`. **The spec states in one sentence that this is not an OS security boundary**: a seat could double-fork and reparent to `launchd`; the day a real boundary is wanted (a shared host, the cloud phase), (C) becomes the second factor beside this, not a replacement. Rejected: (B), because the accidental case is the one that happens; (C) now, because it fights the automation; a well-known socket path, because every same-user process would find it without even looking.

Owner: Corvin Duskmantle <ARCHITECT>.
