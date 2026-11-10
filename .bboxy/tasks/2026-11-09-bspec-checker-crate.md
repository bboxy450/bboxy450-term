---
type: task
title: 'bspec v1: the spec language checker crate and make check-specs'
role: executor
allowed_paths:
- crates/bboxy450-den/
- Cargo.toml
- Cargo.lock
- Makefile
- verification/coverage.toml
- verification/contracts.toml
- licensing.json
- ATTRIBUTION.md
- crates/bbt-mermaid/tests/
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-den && cargo clippy -p bboxy450-den --all-targets -- -D warnings && make check-den && make test-licenses
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
id: 01M71JZAEJPC72B5QT15N0E09Q
number: 1794274420276157
created: 2026-11-10T02:00:43.822Z
updated: 2026-11-10T07:45:31.055Z
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
verify_revision: 1
provenance:
  digest: sha256:cfce71ca362f7b07a900d7f2fc2409ba42250cbe99d4f6ed1a2afc18bd0caa12
  frontmatter_digest: sha256:0a44a31094a56703f5e4e3235ef0457fcb16987043491f722f22be9759655fba
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: xdeEvS76Bkrc4mfxm2aEuZmohGuhXsunUmKn9nz39blKxY1/sbxF99G2AUg5RKODGfbQzsVJMwDiIQk3hqaZBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151402
  event:
    id: 01M726PNGX6B00Y13XR9SC3WJN
    sequence: 151402
    hash: 15ee0fe6b08471d5353c3c1afa9993430e62068cb409a48d539f5769c8ad5fd1
---
ROLE: EXECUTOR

# Goal

Build `crates/bboxy450-den`: the parser and soundness checker for **Den**, the
knowledge-graph language defined in [[spec-language]]. This task is migration step 1 of
that record: the `spec` record kind only. (The record's title still says "bspec"; the
language was renamed Den. Use Den everywhere.)

**Read [[spec-language]] first and treat it as the definition.** Where this brief and that
record disagree on syntax or meaning, the record wins and you note it in the report.

No compiler, no code generation, no execution. The crate answers one question about a
`.den` file holding a `spec`: is it sound?

Worktree: the managed workspace this seat was launched in. Base `origin/main`.

# Scope of this step

In: `module` and `import` lines (parsed, not resolved); one `spec` record per file with
`intent`, `enum`, `state`, `event`, `effect`, `on`/`case`, `invariant`, `assume`,
`scenario`; types `Bool`, `Int[lo..hi]`, an `enum`, `Set[Enum]`.

In, as checks: X101, X201–X204, X301–X303, X401, X402, X501–X503, X601–X603, X701, X702,
X801, X802, X901–X903.

Out, each a later task: resolving imports and the Graph group (X151, X351–X355);
`include`, `do`, `expose`, `discharge` (X504, X604, X904); the other record kinds (`epic`,
`loop`, `task`, `report`, `invariant`, `interface`, `e2e`, `decision`) and the Swarm group
(X371–X379); the knowledge commit gate; any change to `protocol!`, `bbt-yarn` or
`bboxy450-tlc`. A file using an out-of-scope construct gets one diagnostic `X190` "not
supported in this version" at that construct, never a crash and never silence.

# Latitude

You decide, without asking, and note it in the report: module layout, type names, helper
functions, test structure, diagnostic wording, and any extra file a new workspace crate
needs so the repository's gates pass (licence files, source headers, the
`verification/coverage.toml` row, a Mermaid fixture if a record you did not write makes
`every_bboxy_mermaid_block_has_a_fixture` fail after a rebase).

If the language definition is ambiguous or contradicts itself, choose the reading that
keeps "one way to say each thing", implement it, and list it under "Spec questions" in
the report. Do not stop for it.

Stop and message only if a check in scope cannot be implemented as defined, or the verify
command cannot pass without dropping a check. For any `make land` failure outside your
diff, send the failing step and its last 15 lines at once; do not investigate first.

# Shape of the crate

- Dependency: `bboxy450-tlc` only. Hand-written lexer and recursive-descent parser. No
  parser generator, no `syn`, no regex crate, no serde.
- Public API, and nothing else public:

```rust
pub fn check(source: &str) -> Report;

pub struct Report { pub records: Vec<Summary>, pub diagnostics: Vec<Diagnostic> }
impl Report {
    pub fn is_sound(&self) -> bool;   // no diagnostics
    pub fn to_text(&self) -> String;  // one line per diagnostic: "L:C CODE message. Repair: ..."
    pub fn to_json(&self) -> String;  // stable field order, hand-written
}
pub struct Diagnostic { pub code: &'static str, pub line: u32, pub column: u32,
                        pub record: String, pub message: String, pub repair: String }
pub struct Summary { pub kind: &'static str, pub name: String, pub intent: String,
                     pub domains: Vec<(String, usize)>, pub states: usize,
                     pub transitions: usize, pub invariants: Vec<(String, String)>,
                     pub assumptions: Vec<(String, String)>, pub scenarios: usize }
```

- Binary `bboxy450-den`: `bboxy450-den check PATH...`. A directory is walked for `*.den`.
  Prints `PATH:` before each diagnostic line. Exit 0 all sound, 1 any unsound, 2 usage or
  I/O error. `--json` prints one JSON object per file.
- Limits: 65,536 valuations per `on` block for totality; 100,000 reachable states. Above
  either is X603.

# Syntax notes for the parser (from the record, restated so nothing is guessed)

- One statement per line or separated by `;`. Newlines inside `( )`, `[ ]`, after a binary
  operator, after `=>`, after `=`, and inside `"""…"""` do not end a statement.
  Indentation never matters. `//` starts a comment.
- Names: `[A-Za-z_][A-Za-z0-9_]*`; a module path segment may also be a backquoted string.
  Claim ids are ordinary names (`OUTBOX_001`).
- A case is `case NAME if EXPR => BODY` or `case NAME otherwise => BODY`. BODY is
  `{ stmt; stmt }`, or `reject NAME`, or `ignore NAME`.
- Statements: `x = e`, `s += e`, `s -= e` (sets only), `emit Effect(args)`.
- Expressions, loosest to tightest: `==>` (right-assoc); `||`; `&&`; `!`; `==` `!=` `<`
  `<=` `>` `>=`; `+` `-`; application and primary. Primary: `true`, `false`, integer,
  name, `Enum.Member`, `Set()`, `Set(e, …)`, `s(e)` (membership when `s` is a set),
  `if c then a else b`, `(e)`. Quantifiers: `forall x: Enum. e`, `exists x: Enum. e`,
  extending as far right as possible.
- Scenario step: `Call ~> accepted`, `Call ~> accepted emits Call, Call`,
  `Call ~> rejected NAME`, `Call ~> ignored NAME`; and `expect EXPR`.
- `invariant ID "text" = EXPR` and `assume ID "text"`.

# Do it in this order

Commit after each numbered step that leaves the crate's tests green.

1. **Scaffold.** Copy the crate conventions from `crates/bboxy450-tlc` (Cargo fields,
   source header, licence files). Add the workspace member and the coverage row: parsing
   and type checking are implementation-tested; reachable-state checks run on
   `bboxy450-tlc`.
2. **Lexer and parser** to an AST where every node keeps line and column. X101, X190.
   Fixture `tests/sound/outbox.den`: the `Outbox` spec from [[spec-language]], verbatim.
3. **Names and types.** X201–X204, X301–X303.
4. **Interpreter.** Evaluate an expression in a state; `step(state, event instance)` gives
   accepted (new state, emitted effects), rejected NAME or ignored NAME.
5. **Totality and steps.** X401, X402 by enumerating every value of the state types and
   arguments, with the offending state and arguments in the message. X501–X503.
6. **Model.** Lower the spec to a `bboxy450_tlc::Model`. Read how
   `crates/bbt-yarn-macros/src/protocol.rs` lowers sets and domains, match it, and credit
   it "Adapted from" in the source. X601 with the shortest trace as event names, X602,
   X603, X701, X702.
7. **Scenarios and completeness.** X801, X802, X901–X903.
8. **Report.** `to_text`, `to_json`, `Summary`.
9. **Binary and Make.** `make check-den` runs the binary over the repository's `*.den`
   files (today: the crate's `tests/sound`); add it to `make help`.
10. **Fixtures as broken controls.** `tests/unsound/X###-short-name.den`, at least one per
    code in scope. One test walks the directory and asserts each file produces exactly the
    code in its name and no other.
11. **Anchor.** `tests/sound/remote_delivery.den` (below) is sound, and reaches the same
    number of states as `bboxy450_term_remote::delivery::RemoteDelivery::model()`
    (dev-dependency; 52 at the time of writing). Compare transitions too; if the two
    count rejected and ignored events differently, compare state-changing transitions and
    say so.
12. **Differential.** For every file in `tests/sound`, a breadth-first walk with the
    step-4 interpreter reaches the same number of states as the lowered model.
13. **Reference.** `crates/bboxy450-den/README.md`: grammar, meaning rules and the code
    table for this step, under 200 lines, written so a model can write a sound spec from it.
14. Clippy, the verify command, signed commits, `make land`, report.

# The RemoteDelivery fixture

```scala
module specs.delivery.RemoteDelivery

spec RemoteDelivery {
  intent "A message accepted locally is applied on the remote at most once and acknowledged only after it was applied, across duplicate frames, link drops and remote crashes."

  enum Msg { A, B }

  state {
    linkUp: Bool = true
    unacked: Set[Msg] = Set()
    inflight: Set[Msg] = Set()
    applied: Set[Msg] = Set()
    acked: Set[Msg] = Set()
    ever: Set[Msg] = Set()
    doubled: Bool = false
  }

  event Submit(m: Msg)
  event Send(m: Msg)
  event Arrive(m: Msg)
  event AckArrive(m: Msg)
  event Drop
  event Reconnect
  event RemoteCrash
  effect Apply(m: Msg)

  on Submit(m) {
    case accept if !unacked(m) && !acked(m) => { unacked += m }
    case known otherwise => reject AlreadySubmitted
  }
  on Send(m) {
    case send if linkUp && unacked(m) && !inflight(m) => { inflight += m }
    case hold otherwise => ignore NotSendable
  }
  on Arrive(m) {
    case fresh if linkUp && inflight(m) && !applied(m) => {
      applied += m
      ever += m
      doubled = doubled || ever(m)
      emit Apply(m)
    }
    case duplicate if linkUp && inflight(m) && applied(m) => ignore AlreadyApplied
    case lost otherwise => ignore NotInFlight
  }
  on AckArrive(m) {
    case ack if linkUp && inflight(m) && applied(m) => {
      unacked -= m
      inflight -= m
      acked += m
    }
    case stale otherwise => ignore NoAck
  }
  on Drop {
    case drop if linkUp => { linkUp = false; inflight = Set() }
    case down otherwise => ignore AlreadyDown
  }
  on Reconnect {
    case up if !linkUp => { linkUp = true }
    case live otherwise => ignore AlreadyUp
  }
  on RemoteCrash {
    case crash if true => { linkUp = false; inflight = Set() }
  }

  invariant REMOTE_DELIVERY_001 "a message is applied at most once" = !doubled
  invariant REMOTE_DELIVERY_002 "acknowledged implies applied" =
    forall m: Msg. acked(m) ==> applied(m)
  invariant REMOTE_DELIVERY_003 "acknowledged leaves the outbox" =
    forall m: Msg. acked(m) ==> !unacked(m)

  assume REMOTE_DELIVERY_A1 "the link does not forge Applied frames"

  scenario "a duplicate after a crash is applied once" {
    Submit(Msg.A)    ~> accepted
    Send(Msg.A)      ~> accepted
    Arrive(Msg.A)    ~> accepted emits Apply(Msg.A)
    Arrive(Msg.A)    ~> ignored AlreadyApplied
    RemoteCrash      ~> accepted
    Reconnect        ~> accepted
    Send(Msg.A)      ~> accepted
    Arrive(Msg.A)    ~> ignored AlreadyApplied
    AckArrive(Msg.A) ~> accepted
    expect acked(Msg.A) && !unacked(Msg.A)
  }
}
```

If this text is unsound under the rules as defined, do not bend the checker: report which
code fires and why, fix the fixture minimally, and say what you changed.

# Acceptance

- [ ] Every code in scope has a fixture producing exactly that code; an out-of-scope
      construct produces X190.
- [ ] `outbox.den` and `remote_delivery.den` are sound.
- [ ] The anchor and the differential tests pass; the counts are in the report.
- [ ] `make check-den` exits 0 on the repository; show a run on a seeded unsound file
      exiting 1 with its line number.
- [ ] The crate depends only on `bboxy450-tlc`; nothing but the API above is public.
- [ ] `README.md` reference is under 200 lines.
- [ ] The verify command exits 0 and the work is landed with `make land`.

# Report

Follow `AGENTS.md`: stay silent while working. When done: `make land`, write
`reports/2026-11-09-den-checker-crate.md`, then `task_report`. Include: files and
mechanisms, every command with its real exit status, state and transition counts for both
sound fixtures, "Spec questions" (every ambiguity you resolved and how), and what you did
not check.
