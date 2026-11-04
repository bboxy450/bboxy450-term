---
type: task
title: 'Historical operator brief: cockpit startup modal and visible progress'
role: executor
verify: make build && cargo test -p bboxy450-term startup_modal_tracks_progress_failure_retry_and_reattach --lib && make test-licenses
id: 01M6HGKT164568PMEN3VF4MBXM
status: done
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T20:11:40.199Z
created: 2026-11-03T20:11:38.141Z
number: 16564
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: Historical operator-request implementation already landed as 1affb986; create a resolvable record without asserting new acceptance or extending the old test waiver.
  evidence:
  - git commit 1affb9865d75d775a9a5f306c043da8a05bf8f7c — signed startup-modal implementation and original report
  - git verify-commit 1affb9865d75d775a9a5f306c043da8a05bf8f7c — exit 0; Boxy approved key 9B07E849480C68328CA20684844ECC0587DE7AB0
  - reports/2026-11-03-startup-modal.md — historical build/focused-test/license evidence and explicit formal/parity waiver
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6HGKTJBS649C89AWZJ6YGY4
provenance:
  digest: sha256:037701de72b3de74402e64407d080dbffcc97140f6909ecedd59cf99bcbe0004
  frontmatter_digest: sha256:91fc5424e44ee02fda216a13771bda35b22aea1bd9519072e3afd4ccb4cf2848
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: nnCR95ZSK1JcfzkJTLQ2Uyu98ilbMEzYAPnOqYJWzkkhrFssmS0194Gx4vfLgex7CF3lYxrsh/EPj/lvOeSqBA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#96020
---
# Historical startup-modal operator brief

Historical record created under [[decisions/d16393-hash-pinned-report-corrections]] to repair the task locator of the malformed immutable startup-modal report. Implemented by signed commit 1affb9865d75d775a9a5f306c043da8a05bf8f7c (original Git author Boxy; original seat identity unproven). This record preserves the exact original brief below. Its creation does not claim fresh verification or remove the recorded operator waiver: full test/formal/parity checks were waived for that landing and remained owed. Current follow-up regression work is separately scoped in [[tasks/2026-11-03-startup-modal-input-regressions]].

## Exact original brief

ROLE: EXECUTOR. Repo bboxy450/bboxy450-term, worktree /Users/feral/mydev/bboxy450-term-startup-modal, branch feat/startup-modal. Do not merge, push or install.

**Operator request (2026-11-03).** "Implement a loading modal with a blur effect showing what's happening while the UI is starting and connecting; otherwise it's hard to see what is what."

**Today's failure, for context.** The window drew a blank first frame (`first frame: native scene presented, seats=0`). It then sat silent for 30 s while logic verified event-log history, and exited with `logic startup timed out waiting for its readiness line`. Separately, a GUI/logic build skew produced a silent reconnect loop that was visible only as "GUI connection closed" in the Tasks panel.

**Required outcome.**
1. While the cockpit is not attached, show a centered modal over the retained or empty scene, which is blurred and dimmed behind it. This covers first start, logic refresh, reconnect, and "backend lost". The modal shows the actual current phase from the cockpit's own state, each with elapsed time: starting supervisor / adopting supervisor (pid) / starting logic / logic verifying history / attaching GUI / reconnecting (attempt N, last reason) / refreshing logic (reason).
2. On failure the modal stays up and shows the error text, the log path (`<run>/app.log`, `<run>/logic.log`), and Retry and Quit actions. It replaces exiting silently. Only explicit Quit exits.
3. Logic reports startup progress to the GUI before its readiness line (an additive progress line on the same stdout channel, never a renamed or changed existing field). That way "verifying history N/M" is visible, and a live-progress logic is not killed while it is advancing: the startup deadline becomes "no progress for 30 s", not "not ready in 30 s".
4. Design rules: no eyebrow text. Use the existing theme tokens, and render correctly in light and dark.

**Verify (operator exception 2026-11-03: "land loading modal really quickly while we are building, skip tests while merging").** SPEED FIRST. Keep it small: a modal with phase + elapsed + error/Retry/Quit, and the additive logic progress line. If the progress-based deadline is more than a small change, do only the modal and record the deadline as a follow-up in your report. Required: `make build` exits 0, plus one focused unit test of the modal model (`cargo test -p bboxy450-term <your test name>`). Do NOT run make test, make test-parity or make test-formal. Note in the report that the formal/parity evidence is waived by the operator for this merge and is still owed. Signed commit (Boxy), exact paths staged, report in .bboxy/reports/. Target: well under an hour.
