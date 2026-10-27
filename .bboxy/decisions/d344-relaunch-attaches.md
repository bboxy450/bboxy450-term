---
id: 01M5W5KS6T5ETTWK539A096D5D
type: decision
number: 344
title: "a relaunched app attaches to the owners that survived it: a live supervisor and logic are found through the run directory and the app's own GUI socket, the new window attaches as the one GUI client and restores layout, focus and scroll from the account journal before its first frame; no second supervisor, no stolen lease, no ctl"
status: decided
decided: "2026-10-26"
operator:
  - "fast reload without loosing working state, even though native app on GPU rendering, state lives FRONTMATER that user sees restarts"
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[mux-daemon]]", rel: decides }
  - { to: "[[restart-keeps-state]]", rel: constrains }
  - { to: "[[d11-supervisor-death-survivable]]", rel: builds-on }
  - { to: "[[l9-messaging-tasks]]", rel: builds-on }
  - { to: "[[c1-live-seats]]", rel: builds-on }
created: "2026-10-26T13:15:19Z"
updated: "2026-10-26T13:15:19Z"
---
# Decision 344: relaunch attaches

Ruled at Aster's T5 gap. One real kill of the app process (8dd48537) left the supervisor, logic and every PTY alive, as #11 designed; the relaunch then started a second supervisor (`cockpit.rs:618`, `:872` always start one) and stopped at the live ownership lock. T5 promises the same PTYs and identical layout, focus and scroll across an app death; [[ctl-cli]] §1b had named this "adoption" and left it for later. This is that item.

**Rules.**

1. **Find before start.** On launch the app reads its run directory. If the supervisor's ownership lock is held by a live process and logic answers on the **app's own GUI socket** with a hello proving the same account and run, the app **attaches**; otherwise it starts supervisor and logic as today. It never starts a second supervisor while the lock is held by a live one. `ctl` and its endpoint file play no part: the GUI socket is the app's own door.
2. **Attach takes no lease.** The surviving logic keeps its logic lease with the supervisor; the new window is a client of logic, not an owner of anything. If the supervisor is alive and logic is not, the app starts a new logic, which takes the lease through the supervisor's existing replacement path ([[mux-daemon]], the independent logic-kill case).
3. **One GUI client, resumed.** The dead app's GUI attachment was detached by its disconnect ([[l9-messaging-tasks]] §3, "exactly one"); the new window attaches under the same consumer name and resumes from the durable cursor, so no event is lost or repeated. If the old app is still alive, the second launch is refused by name and brings the existing window forward instead: one instance per run.
4. **The window's state is the journal's.** Layout, focus and every pane's scroll position (`top_row` and `live`, as `ui layout` reports them) are written to the account journal when they change, coalesced to the frame, never only held in memory ([[restart-keeps-state]]); the attaching window restores them **before its first frame**, so the first frame the user sees after a relaunch is the layout they left.
5. **The programs never notice.** PTYs, their processes and their input order stay with the supervisor throughout; nothing is resent, replayed into a program or restarted.

**Order.** One cut owns the launch-time find and attach (rules 1 to 3) in the app's lifecycle; the persistence and restore of window state (rule 4) is the second, and T5 goes full when both land and the night kills and relaunches the real app.

Owner: Corvin Duskmantle <ARCHITECT>.

**Operator clarification, 2026-10-26:** ordinary window close and Cmd-Q preserve live work by detaching the GUI. Cmd-Shift-Q is explicit hard quit and stops the backend owners and their seats. This supersedes ctl §1b's earlier equivalence between native close and backend shutdown. Retained records/history are not purged by hard quit.
