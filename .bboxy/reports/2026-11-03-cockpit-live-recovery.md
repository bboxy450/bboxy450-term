---
id: 2XNZBT3C1J0HQTN1MBDGVG153R
number: 5002236
type: report
title: Signed recovery install and live cockpit readiness with manual session recovery
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w3-2d-7-modules]]"
supersedes: "[[reports/2026-11-03-cockpit-recovery]]"
commits: ["2e481e4fe3554a1368c61da47da898b9f01c02f4"]
created: "2026-11-03T22:49:33Z"
verify:
  command: "'/Users/feral/Applications/bboxy450-term.app/Contents/MacOS/bboxy450-term' ctl app status"
  exit_code: 0
---
# Signed recovery install and live cockpit readiness

The operator changed the recovery instruction: stop the current supervisor and
leave session recovery manual. No seat restart, resume, continuation prompt or
new seat launch was issued after that instruction. The original three open
registry records remain at generation 1 with their native session IDs intact.

The explicitly recorded old supervisor 24081 and blocked logic 24082 received
SIGTERM after checking their owner records and executable paths. Both exited;
the desktop helper followed its supervisor. No broad process kill or SIGKILL
was used. This later stop had current operator authorization; it does not erase
the earlier interruption of live work recorded in the superseded report.

`make test-jsonl-recovery` exited 0 on final code: three projection tests and
the native JsonlRepair/broken-control witness passed. `make test-licenses`
exited 0: six policy fixtures, nine notice fixtures and all 34 workspace crate
inventories passed. Full CI and Java/TLC differential verification were omitted
under the operator's explicit request; there is no claim of a new full formal
receipt. The source diff was checked and the signed commit verified by
`python3 -I -B scripts/verify-signatures.py origin/main..HEAD` before pushing.

`make install SIGNING_IDENTITY=B7B799761F58D5315C2D19162A44550E1677D325`
exited 0, including the 51.75-second optimized build, strict bundle validation
and installation. The operator previously approved this available same-Team
Apple Distribution certificate. Info.plist records source `2e481e4fe3554a1368c61da47da898b9f01c02f4` and harness
`af65c6cf82d9dde6314b1771b9314061b0e24212`; Team UQB3368A84 and
`com.bboxy450.term` remain the designated identity.

The first launch reported ready, but the follow-up found its entire process
group gone and status returned ok:false. That transient reply was not accepted
as sustained readiness. A detached launcher using start_new_session started the
same canonical installed `ctl app start --ctl` operation. Its launcher PID was
25403; the continuing GUI is 25404, supervisor 25571, logic 25572 and desktop
helper 25583. Logic reports retained-owner marker 15.

Actual follow-up commands now returned exit 0 and ok:true:

- `ctl app status`: running, backend_ready, gui_ready and events_ready all true;
  one window, generation 1, frame sequence 384 at the recorded observation.
- `ctl seat ls`: zero live seats, matching manual recovery rather than an
  automatic restart of the interrupted sessions.
- `ctl ui layout`: a solved live window tree and frame, with no startup wait.

A separate read-only SQLite observation found published projection cursor
229154 equal to canonical head 229154, with no repair pending. The retained
activity view caught up while control/UI requests remained responsive. No
production history, session record or conversation file was edited or deleted.
No staging deployment was retried and no ATC workflow was invoked.

Raw outputs, process samples, exact stop record, launcher record and prior
cleanup backups remain outside Git in
`~/.local/state/bboxy450-term/worktree-cleanup-2026-11-03-05xew3wt/`.
This record changes evidence only; the installed executable remains the tested
source commit above. The main checkout is the sole remaining worktree.
