---
id: 69FMWB2DCC4NBVSECKG4GTH20F
number: 5002235
type: report
title: Unblock cockpit attachment during retained activity projection recovery
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w3-2d-7-modules]]"
supersedes: "[[reports/2026-11-03-startup-progress]]"
commits: ["e0e5841c0b709c6b34d21a0ef6f7869ae2c3271f"]
created: "2026-11-03T22:45:29Z"
verify:
  command: "make test-jsonl-recovery"
  exit_code: 0
---
# Unblock cockpit attachment during activity projection recovery

The previous installation was rebuilt and signed, but the backend was not healthy.
The operator's screenshot remained on cockpit attachment. A two-second sample of
the actual logic PID 24082 placed almost every logic-worker sample in
`provenance::projection::recover` / `Projection::append` / `File::sync_all`.
The signed activity database contained 229150 rows. The old repair rebuilt the
entire retained stream synchronously and synced every row, blocking GUI/control
requests on the logic owner.

`crates/bbt-mux/src/provenance/projection.rs:222` now starts a staged repair with
a fixed head and persists its temporary-directory ID, verified cursor, hash and
part stamps between pages. Each call processes at most 128 rows, stops after an
8 ms work budget, and syncs touched files once before committing that page.
Regular appends use the same bounded batching. Every row still verifies its
signature and exact sequence/hash predecessor. An incomplete repair exposes no
published references; complete parts replace their destinations before the
published cursor commits. Invalid staged files restart from canonical bytes.
The budget does not bound individual OS I/O or filesystem-sync latency.

`crates/bboxy450-term/src/cockpit.rs:511` advances the retained-logic marker to
15 so the existing logic-only refresh path activates this fix while preserving
the PTY supervisor. The provenance specification and JsonlRepair finite contract
record yielding, publication ordering and fair progress; its broken control
removes the yield guard and must violate the owner-turn invariant.

`make test-jsonl-recovery` exited 0 on the final source: three projection tests
passed (318 unrelated tests skipped), and the focused native formal test passed
(13 unrelated tests filtered). The native witness reopens the store mid-repair,
checks the same staged directory and increasing cursor, and verifies every
published signature/sequence. Existing partial/duplicate and symlink controls
remain green. Full CI and Java/TLC parity were not run under the operator's
explicit instruction to skip the full suite. No broader formal receipt is claimed.

Incident: the earlier `ctl app stop` stopped three active seats across two
swarms and their backend owner. That was a full stop, not a GUI-only restart.
The operator reported interrupted staging work. Their saved registry records,
original native session IDs, recovery launch contexts and native conversation
files remain present. The current recovery must preserve supervisor PID 24081,
refresh only blocked logic, resume the original seats and inspect actual live
state before retrying deployments. Successful restoration is not claimed by
this source/check report; installation and live resumption need separate evidence.

Raw process samples, test output and lifecycle replies remain outside Git in
`~/.local/state/bboxy450-term/worktree-cleanup-2026-11-03-05xew3wt/`.
