---
allowed_paths:
- crates/bboxy450-log-converter/
- Makefile
- scripts/
- verification/
- Cargo.lock
- .bboxy/
assignee: seat:01M6CKGX6ME342X90F0N1YA5TE
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0Yj/k2rOV2tLHQm2aES5d/roSsYDRC/E70ukAxvovgo=
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
created: 2026-11-01T23:15:40.998Z
created_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
delegated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
id: 01M6CPBC1D9QV9GXAYEXVECZWB
number: 5098
provenance:
  digest: sha256:05b2f8a2fa3e00411dd6f2cd524586af76f084098d5759d1f94b360add8a4d44
  frontmatter_digest: sha256:636e682aa1778d7482c5a4db580b95b2effe1e782a857b4e9b0aa58b615eef1a
  log: .bboxy/log/2026-11-01.jsonl#33468
  sig: nC6M4b2xLu4UeL+X0v1InLLXGLCvErsLnSQjnZxS12pJsFVlJ5zKZkW1tq0ZSxbikrE4+8HOrlxBBFpulpaMBA==
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
role: executor
status: claimed
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'LC-P4 (bboxy450-term): Codex parse/lower into crates/bboxy450-log-converter with review fixes'
type: task
updated: 2026-11-01T23:18:01.553Z
updated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
verify: make test
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-lc-p4 -b lc-p4-codex origin/log-converter`, once E6 has pushed `log-converter`; rebase onto main after the port lands. Open a bboxy450-term PR. Do not merge or run land.sh. This supersedes #5080, which was for the old repo.

Binding: d5087, the spec at harness f3e04a6 or later (§7 P4), lc-ir at 8c44ce3 or later, and d5089.

Scope:
1. Port P4 (bboxy450-log-converter 05438543: parse::codex and lower::codex) into the crate.
2. Fold in the review fixes from my messages 01M6CP8DWN and 01M6CPA22V:
   - event_msg of every type → Native, kept raw, with no duplicate Message/Reasoning;
   - non-JSON function_call arguments → ToolCall with text input;
   - token_count with info null is a known shape;
   - role_tag `replacement-N`;
   - an EOF/finalize Notice for a truncated tail;
   - an oversize line is skipped with a Notice and the parse advances;
   - the cursor stays consistent after an I/O error;
   - the malformed-line key gets its own namespace, with a fixture using non-contiguous ordinals;
   - tests: 2 replacements give distinct stable ids (control: empty tags collide), and a paired response_item + event_msg gives exactly one Message and one Native.
3. The real-Codex resume witness goes under `make test-log-converter-native`. A labelled SKIP never counts as proof, and tests never write into the repo.

Acceptance:
- Full bboxy450-term `make test` exits 0.
- `cargo test -p bboxy450-log-converter` passes.
- All commits are signed.
- The report goes in bboxy450-harness `.bboxy/runs/2026-11-01-lc-p4-term-report.log` (tell me the commit).
- Send one task_report with verify_result.command exactly `make test`.