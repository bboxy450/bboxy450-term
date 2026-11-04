---
id: 09E4DSEDJS9T3TNS7NQTA5FDH3
number: 5002230
type: report
title: Codex launch fix validation and blocked main push
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w2-3-adapters-claude-codex-zsh]]"
supersedes: "[[reports/2026-11-03-codex-launch-gate]]"
commits: ["b6afb0227df8d15ae22f826e844ca0d12cb76ab9", "d858af269eac48853a13daf055b677a22660ae5b"]
created: "2026-11-03T19:44:17Z"
verify:
  command: "python3 -I scripts/test-formal.py"
  exit_code: 0
---
# Codex launch fix validation and blocked main push

Implementation and native startup evidence: [[reports/2026-11-03-codex-launch]].
Startup-modal gate repairs: [[reports/2026-11-03-codex-launch-gate]].
Same external-conversation author identifier; no native seat grant is claimed.
The commits above are the rebased candidates on main's `b342be1e` base.

Second make land attempt, clean committed source:

- Strict all-target Clippy for harness-wake-up, mux and app passed.
- Harness unit tests: 19 passed.
- App lib/bin tests: 431 passed, ten ignored/night cases skipped.
- Mux lib/MCP tests: 384 passed, two failed, twenty ignored/night cases skipped.
  The launch-door regression passed in this batch as well as its focused run.
- Both failures were already listed in [[quarantine]]. The held-import test
  passed its isolated retry. The 250-record import test failed alone again at
  `crates/bbt-mux/tests/mcp.rs:234`, first_publication, on the existing 10-second
  MCP read deadline; original-operation lookup remained unknown. The make
  command returned 2 and its gate reported 77. No push occurred.
- The remaining corpus check was run directly: cargo test --locked -q -p
  bboxy450-doc --test corpus returned 101 (seven passed, two failed). Main's
  startup-modal and eventlog-startup reports both use number 5002227; the
  startup-modal report's task is plain direct-brief text rather than a resolvable
  task locator. The current-commit tree check also finds that inherited invalid
  report. No main history was rewritten and no append-only report was edited.

The first make test-formal attempt stopped at its self-tests: this change had
omitted the new model pair from verification/corpus.toml and used the wrong
mandatory-check declaration. Both are now corrected; scripts/test-formal.py
returned 0, all 22 self-tests passed. Full native/TLC verification is rerun on
the signed commit containing this correction; its result remains in the local
formal log, not a fabricated success claim here.

Logs are outside Git under this worktree's target/: codex-launch-land.log,
codex-launch-land-2.log, codex-corpus.log, codex-launch-formal.log and
codex-formal-inventory.log. make test-licenses passed earlier. No full make test,
installation, release or deployment was run. The requested main push remains
blocked by the import gate and inherited report corpus. Worktree cleanup is
conditional on landing; retain the signed branch and worktree while unlanded.
