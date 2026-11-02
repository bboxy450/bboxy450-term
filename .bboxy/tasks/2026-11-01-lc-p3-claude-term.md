---
allowed_paths:
- crates/bboxy450-log-converter/
- Makefile
- scripts/
- verification/
- Cargo.lock
- .bboxy/
assignee: seat:01M6AMBPG3XD3CMJWHQCKWAG7M
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0Yj/k2rOV2tLHQm2aES5d/roSsYDRC/E70ukAxvovgo=
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
created: 2026-11-01T23:25:15.048Z
created_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
delegated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
id: 01M6CPWWQZQVHFKGF0BKTKG7DH
number: 5104
provenance:
  digest: sha256:67cbe93ca9bb67044a47836eb08b709352b228f8a5ce25a115bddbf515b9f1be
  frontmatter_digest: sha256:67047c96e3f869c0b01111a4cd8aeb3decf1412d30397c53be8523806f6e6c2a
  log: .bboxy/log/2026-11-01.jsonl#33500
  sig: QuFTK9egO0agJp2gu2/ywf41MryK/zSaAP+vZjSGJkQRQN9/pkX4IYY1tKNxmA6Fyn/mY1ippQYYezLcy14zCw==
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
role: executor
status: claimed
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'LC-P3 (bboxy450-term): Claude parse/lower + sanitize in crates/bboxy450-log-converter, from harness PR #21'
type: task
updated: 2026-11-01T23:25:17.165Z
updated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
verify: make test
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-lc-p3 -b lc-p3-claude origin/log-converter`, once E6 has pushed `log-converter`; rebase onto main after the port lands. Open a bboxy450-term PR. Do not merge or run land.sh.

Why: harness PR #21 (#4935, head 04edebdc) converts Claude → harness protocol directly. Under d5087 and the converter spec, the Claude parser lives in the converter crate and targets the shared IR. #21 is NOT merged into the harness: it stays open as the source for this port and is closed when P3 lands. Its IR → harness-protocol half becomes the basis of P5.

Binding: d5087, d4947 (sanitize), d4949, d5089, the spec at harness f3e04a6 or later (§7 P3), and lc-ir at 8c44ce3 or later (role_tag `block-N` for extra blocks in one record, framed EventId).

Scope:
1. `parse::claude`: Claude Code session JSONL → IR, porting #21's mapping logic and its fixes (repaired versus new turns, diff hunk positions, bounded large arguments).
   - The parentUuid DAG, sidechains, compaction (compact_boundary plus isCompactSummary), and api-request lines are Native.
   - Thinking keeps its text, with the signature as Sealed{provider claude}, never folded into message text.
   - `block-N` tags; incremental by byte offset for P7 tailing; a truncated tail gives a Notice.
2. `lower::claude`, same-provider: verbatim from raw. The P0 F2 path-staging resume witness goes under `make test-log-converter-native`.
3. `sanitize`, an IR pass per d4947: redactor patterns, API-key shapes, the capability, auth/bearer headers and PEM; /Users/<name> → ~; emails, hosts and IPs → placeholders; uuid/ulid/session ids → stable fakes; image metadata only; tool outputs truncated at 4 KiB. Deterministic, with a planted-secret test and a broken control.
4. Tests: synthetic fixtures only; round trip; determinism; the 100 MB memory witness in ci-local only.

Acceptance:
- Full bboxy450-term `make test` exits 0.
- All commits are signed.
- The report goes in bboxy450-harness `.bboxy/runs/2026-11-01-lc-p3-claude-report.log` (tell me the commit).
- Send one task_report with verify_result.command exactly `make test`.