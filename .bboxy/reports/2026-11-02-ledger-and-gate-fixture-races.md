---
id: "01M6D8GKKAMEGT1JMM884AZH3Z"
number: 5172
type: "report"
title: "Ledger signal ownership and deterministic gate fixtures"
task: "[[tasks/2026-11-02-term-resource-ledger-race]]"
supersedes: "[[reports/2026-11-02-resource-ledger-race]]"
commits: ["300026133ac696975699515297fa28bbc31c21f4", "ea90d343226e9b4361fee4434a3362ada115dbb7", "585ac6c7e2c48910fd3a1657cd5a8c7ca9db1d4b", "ba4d98d52bc5c0d698f7b9d19ef9339495bbde1b", "83f6f601078e0a7e535fcb41db12c81c32169b0e"]
verify: {"command": "python3 -I scripts/test-resource-ledger.py", "exit_code": 0}
author: "seat:01M6ADTTF9QWWFZFVZJHJ3DD8E"
created: "2026-11-02T04:33:06.666629Z"
---
# Ledger and gate-fixture races

Executor E1 harness-core, worktree `/Users/feral/mydev/bboxy450-term-ledger-race`,
branch `resource-ledger-race`. This combined report covers four delegated tasks:
[[tasks/2026-11-02-term-resource-ledger-race]] (#5117),
[[tasks/2026-11-02-term-plugin-hook-log-partial-line]] (#5165),
[[tasks/2026-11-02-term-writer-tick-ordered-commits]] (#5169), and
[[tasks/2026-11-02-term-plugin-fixture-single-write]] (#5171).
The latter three task records were copied unchanged from the authenticated
assignment root. Oracle ruling 01M6D6XAJNWE6S7SN07MP65E8Q combines the work on
this branch; #5171 adds the fixture writer's scope. No merge or land.sh was run.
The containing signed commit and its Report trailer bind this report without a
self-referential SHA. All five preceding commits have valid Boxy signatures.

## Files and mechanisms

- `scripts/resource-ledger.py::run`: SIGINT/SIGTERM handlers are installed before
  accounting and child creation. Signals received before publication of the child
  handle are retained, with duplicates coalesced to at most two distinct signals,
  then forwarded through the original descendant-only path. Exit normalization
  remains 128+signal. Child stdout readiness cannot beat handler installation.
- `scripts/test-resource-ledger.py`: a real Popen interception sends SIGTERM
  immediately before/after spawn, with an owned pipe holding the child and no
  timing sleep. Each suite tests 32 after-spawn and eight before-child deliveries,
  all requiring 143. Moving registration after spawn fails specifically at
  `spawn-window signal took default action`, with -15 rather than 143. A copied
  fixture's missing wake-ledger dependency was corrected before counting control
  evidence. The follow-up disables source bytecode writes and makes the CPU-units
  witness use its known own process identity while retaining real libproc and
  getrusage measurements and every numeric assertion. Production ps behavior is
  unchanged; the original failures remain in the archive.
- `crates/bbt-mux/tests/plugin_host.rs:50`: read byte records ending in LF and
  decode only those complete records. A trailing partial JSON or UTF-8 scalar
  waits for the next poll; a malformed completed record still panics. The real
  channel-paused writer witness reads while half a record is held, then joins the
  writer and reads both records in order. It also tests a split Unicode scalar.
- `crates/bbt-plugin-fixture/src/reference.rs:37`: serialize JSON and LF into
  one buffer, then call write_all once on the O_APPEND file. The newline reader
  remains necessary for partial/short writes. Scope came from separately
  delegated #5171 before this file was edited.
- `crates/bbt-supervisor/src/ring/writer.rs:845`: the writer normally commits
  on its 20 ms timer or durability pressure. At :855-882 the independent 200 ms
  idle path may project already-ordered tail records into the ring. That calls
  Ring::commit(false), `ring.rs:1142`, incrementing the same ordered_commits
  counter again. Splitting the two appends across timer ticks cannot itself give
  two commits for one single-record seat: commit_group increments only when its
  sequence advances (:919). The forced idle tick reproduces the actual 2-versus-1
  result, establishing the projection mechanism rather than assuming double
  scheduling is a product defect.
- The one-tick fixture now opts into cfg(test)-only manual ticks before admitting
  records. It waits until both seats have captured seq1 with committed seq0,
  explicitly ticks once, then retains committed seq1, durable_commits==0 and
  ordered_commits==1 for both seats. Manual mode suppresses automatic ordering,
  durability and idle-projection rounds, so elapsed scheduler time cannot move
  the measurement boundary. All production automatic-tick behavior is unchanged.
  `mux-daemon.md:161` still governs ordered tick commits and durable points.
- `scripts/test-ledger-fixture-controls.py`: restores and hashes source in finally,
  requires cargo exit101 at the named failure, then requires the restored witness
  to exit0. The writer control forces an additional automatic idle-projection
  tick using a backdated fixture observation; it retains the exact ==1 assertion.

## Commands, exits and repetition evidence

The committed command/exit/hash inventory is
`.bboxy/runs/2026-11-02-ledger-and-gate-fixture-races.json`.
Raw local output is outside Git and target under
`.bboxy/runs/clones/e1-ledger-race/`, in `resource-ledger-race/` and
`combined-ledger-fixtures/`. The archive manifest hashes every preserved old log;
individual repetition rows hash every current log. No raw account transcript or
credential value is committed.

| Check | Actual result |
| --- | --- |
| `python3 -I scripts/test-resource-ledger.py`, 50 consecutive runs | 50 exits0; 1,850 tests, 2,000 forced forwarded SIGTERMs, 50 late-handler negative detections |
| Late-handler mutation / restored boundary witnesses | 1 -> 0 at -15-versus-143 assertion; Python unittest uses exit1 |
| `cargo test -p bboxy450-harness-host --lib ring::writer`, 50 runs without added stress | 50 exits0, 250 tests passed |
| Same writer command, 50 runs with two owned CPU workers | 50 exits0, 250 tests passed; workers joined, source bytes unchanged |
| `cargo test -p bboxy450-term-mux --test plugin_host`, 20 consecutive runs after buffered writer change | 20 exits0, 320 tests, no ignored cases; split-record witness included |
| `python3 -I scripts/test-ledger-fixture-controls.py --case writer --out <raw-dir>` | Driver0; mutant101 at `one tick ordered each seat exactly once` (2 versus1), restored0 |
| Same control driver with `--case hook` | Driver0; mutant101 at `EOF while parsing`, restored0 |
| `make test-boundaries` before these fixture-only additions | 0 on ba4d98d5; real process checks retained |

Writer repetitions each retain two existing ignored release-only crash tests;
these are not counted as passed. "Without added stress" is not a claim that the
shared machine was idle: every repetition records ambient load. Targeted Cargo
uses two build jobs and a worktree-local target; tests use two test threads.
Timeouts are hang guards, not percentile or wall-clock performance gates.
The initial task's nonexistent package bboxy450-term-supervisor exited101 before
building; the real package is bboxy450-harness-host. Oracle correction
01M6D7M8P9NMHMRHZTJEFM1F1Q directs #5169's task_report to use the actual writer
command above and cite the shared full gate. No failing command is called green.

## Full gate history, formal scope and handoff boundary

Both earlier exact `make test` attempts exited2 on ba4d98d5. The first completed
workspace checks, native/TLC parity and smoke, then the checkpoint fixture refused
free disk of 8,599,150,592 bytes below its 10 GiB guard. Space recovered without
threshold changes, and `sh scripts/invariants.sh checkpoint-bounded` then exited0.
The second full run exposed the writer assertion now fixed by #5169. Three isolated
original writer runs passed; those did not establish a green full gate. Neither
failed full result is discarded or substituted by a partial pass. The earlier
boundary initialization failure and its canonical make init-submodules recovery
are retained in the superseded report and archive.

The earlier full formal pass explored 205 models across 32 crates and 84 contracts,
with zero reused verdicts and native/TLC graph agreement, on ba4d98d5. That is
historical evidence, not a receipt for this containing commit. The subsequently
checked current receipt named another source and was refused; its failure is not
claimed green. A fresh current-head formal receipt is required by the final full
gate and push target.

No model in the corpus encodes this Python ledger signal lifecycle. Its real POSIX
witness and late-handler control are explicitly implementation-only in
verification/coverage.toml. Framing/serialization are specialized parsing tests;
manual writer timing exists only in the test fixture. Production lifecycle,
retry, persistence and arbitration semantics are unchanged. A separate multiline
implementation-only inventory entry names the new fixture sources, assumptions,
bounds and controls. Existing models do not claim OS scheduling, syscall timing,
whole-Rust refinement or a proof of the ordered-commit metrics. No model property
was weakened and no additional formal proof is claimed for the fixture mode.

The completed gate logs were copied and SHA-256 inventoried before deleting only
this worktree's old target directory, per 01M6D47CM089VWRV8PD8FGJPGH. Other worktrees
and shared sccache were untouched. The new target must likewise be removed after
recording the branch's final gate log. Raw logs remain locally under the ignored
archive above; only summaries enter Git.

This immutable report precedes its containing commit. The one shared exact
`make test` must run after that signed commit and only when no other full run is
active, as #5169 requires. Its actual exit, exact source, log hash, current formal
receipt, PR URL and cleanup result belong in each final task_report. This report
does not claim that future gate has passed. No PR, publication or merge is claimed
by this pre-gate record.

The live MCP task_report schema still lacks the optional report locator, and a
Report record's task field is singular. This common report names all four assigned
tasks in its body; each one-paragraph handoff carries the qualified Git report
locator and the same full gate evidence. Native Report publication is not claimed.
The report number is the local next-number candidate; corpus checks must reject
any collision rather than silently reallocating a published record.
