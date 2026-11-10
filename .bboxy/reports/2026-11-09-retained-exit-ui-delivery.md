---
id: 01M721QHPDDFQ3M97WFYX1AAWG
number: 1794291517133963
type: report
title: Startup restored with Supervisor and all working harnesses preserved
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
supersedes: '[[reports/2026-11-09-retained-exit-workspace-restore]]'
commits:
- 1881181123d1dca79ec1f2fcfa2d96831243e244
verify:
  command: /Users/feral/Applications/bboxy450-term.app/Contents/MacOS/bboxy450-term ctl app status
  exit_code: 0
author: seat:01M71Y252EEXCTDHVJTJSX7JVZ
created: '2026-11-10T06:18:37.133762+00:00'
---
Direct operator request: fix failed session restoration without restarting Supervisor or the harnesses doing work. Delivered source 1881181123d1dca79ec1f2fcfa2d96831243e244; this author is a standalone execution identifier, not a registered native-seat signature. The historical task is delivery context only.

The fix in [[reports/2026-11-09-retained-exit-workspace-restore]] treats Supervisor-retained exited sessions as history instead of fresh workspace-writer launch confirmations. The reverse indexes and current writer permissions remain intact. The old exited seat can no longer abort startup by trying to reclaim a reassigned workspace. All live seat/generation/PID refusals remain strict.

Verified delivery:
- Eighteen focused workspace regressions passed; the initial test-only ExitInfo import error was corrected before that passing run. Native Rust and pinned Java TLC match the complete selected ownership graphs: good 16 states / 28 edges, broken 25 states / 40 edges with the expected safety refusal. Full CI/formal/Yarn, lint, smoke and visual suites were not run; no make land was invoked.
- The first app build exited 2 because the shared target/release/deps directory disappeared while rustc was emitting a Term object. The source was unchanged. An isolated, protected worktree target avoided that shared cleanup; canonical make -o check-licenses bundle exited 0 and reported 35m06s for app compilation. Ordinary bundled-worker preparation also exited 0. Its scope is Ordinary release worker preparation only; consumer verification incomplete; no native Dream consumer-conformance claim is made. Missing-libLLVM stripping and unused Dream helper warnings were nonfatal.
- Canonical make install-prepared BUNDLE=/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-restore-fix-20261109-x6y64eqb/term/target/release/bboxy450-term.app exited 0 with approved Apple Development identity 3943AF130A015B36824B7FBC86AA672E7B25F3C4. Both installed executable hashes match the signed bundle, and strict installed-signature verification exited 0. Bundled harness revision is 20e52d0f937c7ed19abbf506564a3dd8a0c8cdd4.
- Logic was absent after the startup failure. The existing reopen-gui.py launch/ready functions performed guarded GUI replacement using the freshly captured public GUI owner; normal startup adopted the live Supervisor and started corrected Logic. The live-Logic preflight of the full refresh helper was inapplicable, so it was not weakened or fabricated. No --recover-seats, ctl app stop, make restart-recover, OS clicks or desktop capture was used.
- New GUI 35862 / 01M721J7FTWGW6A1H45H1PHFNA renders the exact source commit above. New Logic is 36049. Supervisor remains PID 84759 with birth [1794273616,96520]. Harnesses 86143, 9999, 34927 and 19453 retain their exact kernel start times and Supervisor parent. No harness was restarted or relaunched.
- Canonical --gui-handoff-probe --preserve-backend-check exited 0. ctl app status exited 0 twice, with running/backend_ready/gui_ready/events_ready all true; rendered frames advanced 658 to 664. All seven process records restored and the four original live seats are running. The repository is retained.

Installer backup /Users/feral/Applications/.bboxy450-term-install.whUYNe/previous-Contents is intentionally retained because the unchanged Supervisor/harnesses may still map its old executables. The private source/build worktree is removed only after its commits are on main, its tree is clean and lsof shows no users. Other source worktrees and live application data are preserved.

Public process identities, installed executable hashes, exact command/exit results and selected formal evidence hashes are committed in .bboxy/runs/2026-11-09-retained-exit-ui-delivery.json. Raw logs and native/TLC graphs remain private at /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-restore-fix-20261109-x6y64eqb. This evidence-only [skip ci] commit does not change the installed source cut.
