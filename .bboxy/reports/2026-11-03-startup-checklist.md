---
id: 4EJVH95FZ7P4PYE5GY1FKPP7Z4
number: 5002233
type: report
title: Startup checklist, current harness pin and worktree cleanup
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w3-2d-7-modules]]"
commits: ["198b2f02e78fc8f27afc8dd370f5ef87e0bb7be6", "69f622b084e5364eaaf46eb9f421272e4f7a8bf7"]
created: "2026-11-03T21:39:03Z"
verify:
  command: "make parity-startup"
  exit_code: 0
---
# Startup checklist, current harness pin and worktree cleanup

Direct operator brief: preview the startup modal in Boxybook, improve its native
layout and show every startup step. The task link is historical module context;
this external conversation claims no authenticated native seat grant or task
assignment. The operator subsequently authorized updating main and the harness,
building/installing and restarting the backend, then explicitly selected the
main checkout for builds and requested removal of all old worktrees.

`crates/bboxy450-term/src/startup_modal.rs:11` projects the existing observations
into four ordered steps: connect supervisor, start runtime, verify signed history,
attach cockpit. Earlier stages get checks, one stage is active, later stages stay
pending and sticky failure highlights the affected stage. Retry keeps the existing
state machine and action identities. Supervisor adoption, reconnect and runtime
refresh retain their real details; errors and available log paths remain visible.

`crates/bbt-ui/src/modules/startup.rs` owns the production retained composition.
The heading, highlighted current row, reported history count/bar and separate
diagnostics replace the old text column. Error spacing keeps the tested long
failure and both logs within the app's 600px minimum height. Narrow 320px and
normal 560px content widths are checked without clipping children.

`lab/web/startup.html` and `startup.js` use existing public startup-test data.
`crates/bboxybook/src/startup.rs` registers ten states at four densities and two
themes, including the default parity catalogue. The capture host exports public
startup input and the native host calls the production component; DOM child
rectangles do not place native content. The shared host now handles decorative
boxes without text and neutral named action wrappers in geometry diagnostics.

Verification before the harness update: `make test-startup-modal` passed two
native layout tests plus two app phase/sticky-failure/retry tests; 440 unrelated
app tests were filtered. `make parity-startup` passed 80/80 captures and native
comparisons, zero geometry mismatches and zero pixel reviews, with unchanged
0.5px rect/baseline and 2% text-pixel policies. Dark history and light long-failure
native images were inspected. `make smoke` returned 0 with a private unsigned
app, real backend, no window, ten-second idle proof and orderly child reaping.
`make test-licenses` passed six policy fixtures, the 34-crate inventory and nine
source-notice fixtures. No broad CI or new full formal receipt is claimed here.

After the operator resumed work, main was pulled to 56387182 and these source
changes rebased cleanly. `make bump-harness` moved the gitlink from 0c40316 to
af65c6cf82d9dde6314b1771b9314061b0e24212, matching remote harness main; Cargo.lock
did not change. The public-key verifier passed all seven new harness commits.
`scripts/harness-submodule.py --check-resolution` returned 0: all seven toolkit
dependencies resolve in-tree. The four new broken controls are registered in
parent coverage; its 99 imported model names exactly match the pinned corpus.

The operator stopped the worktree install before completion. Its exact recorded
process tree was terminated (143); no installation success is claimed for it.
The changes were fast-forwarded into the authorized main checkout. Four old
worktrees were clean or had only the recorded triage submodule revision change;
no process held their files open. Their refs, triage patch and complete child Git
bundle, preview and logs were preserved outside Git in
`~/.local/state/bboxy450-term/worktree-cleanup-2026-11-03-05xew3wt/` before removal
and pruning. Only the main worktree remains. Installation/restart results follow
in a separate deployment record after the authorized main-checkout build.
