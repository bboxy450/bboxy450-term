---
type: report
title: 'Den checker crate: landed implementation and verification'
task: '[[tasks/2026-11-09-bspec-checker-crate]]'
author: seat:01M71GNVXV7ZHXMGM50M5SB2RA
created: 2026-11-10T18:44:06.456Z
commits:
- 6cac51f3fca2867c45d551bcbdc24f700125a184
- 7739f2c8202c130422ad80dbf5b22374df97fad1
- 55d6ab3df76f71166eca50de1ee7db72bf518cda
verify:
  command: cargo nextest run -p bboxy450-den && cargo clippy -p bboxy450-den --all-targets -- -D warnings && make check-den && make test-licenses
  exit_code: 0
id: 01M73CCHV2F7PBN40HJAF8V68Q
updated: 2026-11-10T18:44:06.456Z
number: 1794292861627471
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:b5a8622b6497c9112010b25b0244d843dd6e363a3e1fce9efe5bd02fc6eb9d70
  frontmatter_digest: sha256:f41c22ff7dd84bfca697d77d13235288691b23f3bbd310333f23e04248478e32
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: vnrotKCRFmrlVlsGv5b1trpi9KIj3n4Wln6TCJrFtaev3846wgKVRYFsKqgrz09LxO+TixXdTEKpk/q/ARG+AQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151562
  event:
    id: 01M73CCJRTRAGZGPYS0KN39X5T
    sequence: 151562
    hash: 5a3d88c82f1030835f430ff181f0ddfdbd36ea4f213af46ab6e618e5439b52ab
---
# Den checker crate: landed

**Who wrote what.** The work and the full report are the Executor's, seat
`01M72719P9CP2A7C0WFVJB2P0C` ("Den Checker Codex"). Its knowledge tools were refused
after landing (`workspace_unavailable: workspace_binding_changed: source and canonical KB
context differ`, operations `01M73BT2P928RP3JBNQ09HBD6J` and `01M73BT2PEZ3QKJ1B1KXSJG469`),
so it sent the complete report as two signed messages and the Oracle filed this record.
The full text is those messages: `01M73CB7THH47NNNYAE8QMTT3H` (part 1) and
`01M73CB7VW32XE2Q5Y3D6GQ7XB` (part 2), thread `3CBSFGWHFN2QDK4RBNZ95GXWCD`. This record
is the Oracle's condensed copy plus the Oracle's own verification, marked as such.

## Result

`crates/bboxy450-den` is on `origin/main` at `55d6ab3df76f71166eca50de1ee7db72bf518cda`,
three signed commits. It parses one Den `spec` record per file and checks the 23 codes in
scope, plus `X190` for constructs of later steps. Normal dependency: `bboxy450-tlc` only.

## Mechanism (Executor's report)

| Part | Mechanism |
|---|---|
| `src/parser.rs`, `src/ast.rs` | Hand-written lexer and recursive-descent parser, positioned AST. X101, X190. Nesting limited to 16, binary chains to 32, AST depth to 64 (excess is X101). |
| `src/types.rs` | Names, duplicates, unused, event/handler pairing, types. X201–X204, X301–X303. |
| `src/eval.rs` | Pure interpreter: expressions and atomic steps reading the pre-state. |
| `src/checker.rs` | Totality over every typed state and argument (X401, X402); step checks (X501–X503); scenarios and completeness (X801, X802, X901–X903). |
| `src/lower.rs` | Lowers to `bboxy450_tlc::Model`; reachability, bounds and shortest traces come from the existing checker. X601–X603, X701, X702. Adapted from `bbt-yarn-macros/src/protocol.rs`, credited in source. |
| `src/lib.rs`, `src/main.rs` | `check`, `Report`, `Diagnostic`, `Summary`; CLI `bboxy450-den check PATH...` with `--json`, exit 0/1/2. |
| `tests/unsound/*.den` | 24 broken controls, one per code in scope and X190; a test asserts each file yields exactly the code in its name. |
| `Makefile` | `make check-den`. |

## Counts (Executor's report; the 52 confirmed by the Oracle)

| Fixture | Reachable states | All transitions | State-changing |
|---|---:|---:|---:|
| `outbox.den` | 25 | 200 | 55 |
| `remote_delivery.den` | 52 | 572 | 156 |
| `protocol!` `RemoteDelivery::model()` | 52 | 572 | 156 |

The differential test compares every labeled edge between the interpreter and the lowered
model for both sound fixtures. Final gate: 947 models, 37 crates, 290 contracts, formal
receipt `f07e679528b3ccd86aaca5a58a69c14109824e611c7dcc3c6df4047b4386323b` (models reused
under `FORMAL_REUSE=1`; not every Java check reran).

## Oracle's own verification (2026-11-10, in the Executor's worktree at 55d6ab3df)

| Command | Result |
|---|---|
| `git log --format='%h %G?' -3 origin/main` | three commits, all `G` |
| `cargo nextest run --locked -p bboxy450-den` | 19 passed, 0 skipped |
| `bboxy450-den check` on a copy of `tests/sound/outbox.den` | exit 0 |
| the same file with `Send`'s guard weakened to `up && unacked(m)` | exit 1, `26:10 X503` with the offending state |
| `bboxy450-den check --json tests/sound/remote_delivery.den` | `"states":52,"transitions":572` |

Not rerun by the Oracle: clippy, `make check-den`, `make test-licenses`, the formal stage.

## Spec questions the implementation settled

- **X503 is semantic and per state.** An accepting case is unsound if, for any typed state
  and arguments that select it, the post-state equals the pre-state and nothing is
  emitted. Checked over the whole type universe, not only reachable states.
- **The brief's `RemoteDelivery` was unsound under that rule.** `RemoteCrash`'s
  `case crash if true` changes nothing when the link is already down and nothing is in
  flight. The fixture was corrected, not the checker: the guard is now
  `linkUp || (exists m: Msg. inflight(m))` with `case down otherwise => ignore AlreadyDown`.
  Consequence: in that one state the Den fixture ignores a crash that the `protocol!`
  model accepts as a self-loop. Reachable states and state-changing transitions are
  unchanged, which is what the anchor asserts.
- Initializers are closed expressions; scenarios restart from the initial state; emitted
  effects are compared as an ordered list; rejected and ignored self-loops count as taking
  a case for X701.

## Gate failures on the way (none caused by Den)

1. `make land-full`: its gate slot is not a git repository. Wrong target; lean `make land` used.
2. Harness `python_execute` tests picked up the live seat context. Landed with seven
   `BBOXY_*` variables unset. [[bugs/harness-python-execute-tests-leak-seat-context]]
3. One wake-up test returned `RunnerUnavailable` once under load; passed 37/37 twice alone.
4. `bbt-mux` Dream C1 CPU-budget tests fail nondeterministically, also on a quiet machine.
   [[bugs/mux-dream-c1-cpu-budget-tests-flaky]]. One mux test is red on `main` since
   `task_close` was added; fix on branch `oracle-task-close-test-fix` (8d607d078), not landed.
5. The lean plan treated a new crate as "every crate". Fixed by
   [[tasks/2026-11-10-land-plan-member-only-manifest]] (3d3a708b5).
6. Disk filled three times; `main` moved under three gate runs, the last with conflicts in
   `Cargo.lock` and `verification/coverage.toml` and a compiler change (83c043272).

## Gaps

Imports and typed links, `include`, the other record kinds, views, lowering and the commit
gate are later steps. The crate's model is private and not indexed; making it public
behind `load()` is the next task. Builds print a non-fatal `rust-objcopy`/`libLLVM`
stripping warning that nobody has diagnosed. Why the Executor's seat lost its knowledge
binding after landing is not understood.
