---
type: task
title: 'bspec v1: the spec language checker crate and make check-specs'
role: executor
allowed_paths:
- crates/bboxy450-spec/
- Cargo.toml
- Cargo.lock
- Makefile
- verification/coverage.toml
- verification/contracts.toml
- licensing.json
- ATTRIBUTION.md
- .bboxy/reports/
verify: cargo nextest run -p bboxy450-spec && cargo clippy -p bboxy450-spec --all-targets -- -D warnings && make check-specs && make test-licenses
id: 01M71JZAEJPC72B5QT15N0E09Q
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T02:00:43.822Z
created: 2026-11-10T02:00:43.822Z
number: 1794274420276157
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:dbaaef76a7769312f48c455d4e6eaf12ce4a41bc5310f003f4a15206f8be94d8
  frontmatter_digest: sha256:6b8cb428c515caf721c92e6db678bc0e92f2f99063d6faa533d8d51b76b48dee
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: +F2ueruUssHjx3UZIzGuDI24YF5KAV9VMUMmvtjEeuaHlJHq/6quXYBZYAF2jy2zTHxq+26oXoRgVuaMfHV3Aw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151286
  event:
    id: 01M71JZBA5ZD7HFQRFBZF6JYMV
    sequence: 151286
    hash: 7a2b4c6b7a3d6fc607a381fdc610d0d3afdb0e2d9f18f4324a57c58718fdaede
---
ROLE: EXECUTOR

# Goal

Build `crates/bboxy450-spec`: the parser and soundness checker for the `bspec` language.
The language, its meaning and every check are defined in [[spec-language]]. **Read that
record first and treat it as the definition.** This task builds exactly that, v1, no more.

No compiler. No code generation. The crate answers one question: is this spec sound?

Worktree: the managed workspace this seat was launched in. Base `origin/main`.

# Latitude

You decide, without asking, and note it in the report: module layout, type names, helper
functions, test structure, diagnostic wording, and any extra file a new workspace crate
needs so the repository's own gates pass (licence files, headers, `coverage.toml` entry).

If the language definition is ambiguous or contradicts itself, choose the reading that
keeps "one way to say each thing", implement it, and list the ambiguity in the report
under "Spec questions". Do not stop for it.

Stop and message only if a check in the table cannot be implemented as defined, or the
verify command cannot pass without dropping a check.

# Shape of the crate

- Dependencies: `bboxy450-tlc` only. Hand-written lexer and recursive-descent parser. No
  parser generator, no `syn`, no regex crate.
- Public API, and nothing else public:

```rust
pub fn check(source: &str) -> Report;            // a file of one or more `spec` items
pub fn check_markdown(document: &str) -> Report; // every ```bspec fence in a Markdown document

pub struct Report { pub specs: Vec<Summary>, pub diagnostics: Vec<Diagnostic> }
impl Report {
    pub fn is_sound(&self) -> bool;   // no diagnostics
    pub fn to_text(&self) -> String;  // one line per diagnostic: path-less "L:C CODE message. Repair: ..."
    pub fn to_json(&self) -> String;  // stable field order, hand-written, no serde
}
pub struct Diagnostic { pub code: &'static str, pub line: u32, pub column: u32,
                        pub spec: String, pub message: String, pub repair: String }
pub struct Summary { pub name: String, pub intent: String,
                     pub domains: Vec<(String, usize)>, pub states: usize,
                     pub transitions: usize, pub invariants: Vec<(String, String)>,
                     pub assumptions: Vec<(String, String)>, pub scenarios: usize }
```

- `check_markdown` reports lines in the **document's** numbering. A document with no
  `bspec` fence is sound with zero specs.
- Binary `bboxy450-spec`: `bboxy450-spec check PATH...`. A `.md` path uses
  `check_markdown`; any other uses `check`; a directory is walked for `.md` and `.bspec`.
  Prints `PATH:` before each diagnostic line. Exit 0 all sound, 1 any unsound, 2 usage or
  I/O error. `--json` prints one JSON object per path.
- Limit: 65,536 valuations per `on` block for totality, and 100,000 reachable states.
  Above either is B603.

# Do it in this order

Commit after each numbered step that leaves the crate's tests green.

1. **Scaffold.** Copy the crate skeleton conventions from `crates/bboxy450-tlc` (Cargo
   fields, source header, licence files). Add the workspace member and a
   `verification/coverage.toml` entry: parsing and type checking are implementation-tested;
   reachable-state checks run on `bboxy450-tlc`.
2. **Lexer and parser** to an AST where every node keeps line and column. B101. Fixture:
   `tests/sound/counter.bspec`, the `Counter` example from [[spec-language]], must parse.
3. **Names and types.** B201–B204, B301–B303. Types: `Bool`, `Int[lo..hi]`, a domain,
   `Set<Domain>`. `+=` and `-=` are set insert and remove; on `Int` they are not allowed
   (write `n = n + 1`).
4. **Interpreter.** Evaluate an expression in a state; `step(state, event instance)` gives
   accepted (new state, emitted effects), rejected NAME or ignored NAME. Used by totality,
   scenarios and the differential test.
5. **Totality and steps.** B401, B402 by enumerating every value of the state types and
   arguments, with the offending state and arguments in the message. B501–B503.
6. **Model.** Lower the spec to a `bboxy450_tlc::Model` (read how
   `crates/bbt-yarn-macros/src/protocol.rs` lowers `Set` and domains and match it; credit
   it "Adapted from" in the source). B601 with the shortest trace as event names, B602,
   B603, B701, B702.
7. **Scenarios and completeness.** B801, B802, B901–B903.
8. **Report.** `to_text`, `to_json`, `Summary`.
9. **Markdown.** `check_markdown`.
10. **Binary and Make.** `make check-specs` runs the binary over `.bboxy`; add it to
    `make help`. It must exit 0 on the repository as it is.
11. **Fixtures as broken controls.** `tests/unsound/B###-short-name.bspec`, at least one
    per code in the table. One test walks the directory and asserts each file produces
    exactly the code in its name and no other code. A fixture that fails to parse is
    acceptable only under `B101-`.
12. **Anchor.** `tests/sound/remote_delivery.bspec` (text below) is sound, and its
    reachable-state count equals that of
    `bboxy450_term_remote::delivery::RemoteDelivery::model()` (dev-dependency). Compare
    transitions too; if the two count rejected/ignored events differently, compare the
    state-changing transitions and say so in the report.
13. **Differential.** For every file in `tests/sound`, a breadth-first walk with the
    step-4 interpreter reaches the same number of states as the lowered model.
14. **Reference.** `crates/bboxy450-spec/README.md`: the grammar, the meaning rules and
    the code table, under 200 lines, written so a model can write a sound spec from it.
15. Clippy, verify command, signed commits, `make land`, report.

# The RemoteDelivery fixture

```bspec
spec RemoteDelivery {
  intent "A message accepted locally is applied on the remote at most once and acknowledged only after it was applied, across duplicate frames, link drops and remote crashes.";

  domain Msg = { A, B };

  state {
    link_up: Bool = true;
    unacked: Set<Msg> = {};
    inflight: Set<Msg> = {};
    applied: Set<Msg> = {};
    acked: Set<Msg> = {};
    ever: Set<Msg> = {};
    doubled: Bool = false;
  }

  event Submit(m: Msg);
  event Send(m: Msg);
  event Arrive(m: Msg);
  event AckArrive(m: Msg);
  event Drop;
  event Reconnect;
  event RemoteCrash;
  effect Apply(m: Msg);

  on Submit(m) {
    case accept when not (m in unacked) and not (m in acked) { unacked += m; }
    case known otherwise { reject AlreadySubmitted; }
  }
  on Send(m) {
    case send when link_up and m in unacked and not (m in inflight) { inflight += m; }
    case hold otherwise { ignore NotSendable; }
  }
  on Arrive(m) {
    case fresh when link_up and m in inflight and not (m in applied) {
      applied += m;
      ever += m;
      doubled = doubled or m in ever;
      emit Apply(m);
    }
    case duplicate when link_up and m in inflight and m in applied { ignore AlreadyApplied; }
    case lost otherwise { ignore NotInFlight; }
  }
  on AckArrive(m) {
    case ack when link_up and m in inflight and m in applied {
      unacked -= m;
      inflight -= m;
      acked += m;
    }
    case stale otherwise { ignore NoAck; }
  }
  on Drop {
    case drop when link_up { link_up = false; inflight = {}; }
    case down otherwise { ignore AlreadyDown; }
  }
  on Reconnect {
    case up when not link_up { link_up = true; }
    case live otherwise { ignore AlreadyUp; }
  }
  on RemoteCrash {
    case crash when true { link_up = false; inflight = {}; }
  }

  invariant REMOTE-DELIVERY-001 "a message is applied at most once": not doubled;
  invariant REMOTE-DELIVERY-002 "acknowledged implies applied": forall m: Msg. m in acked => m in applied;
  invariant REMOTE-DELIVERY-003 "acknowledged leaves the outbox": forall m: Msg. m in acked => not (m in unacked);

  assume REMOTE-DELIVERY-A1 "the link does not forge Applied frames";

  scenario "a duplicate after a crash is applied once" {
    Submit(Msg.A) -> accepted;
    Send(Msg.A) -> accepted;
    Arrive(Msg.A) -> accepted emits Apply(Msg.A);
    Arrive(Msg.A) -> ignored AlreadyApplied;
    RemoteCrash -> accepted;
    Reconnect -> accepted;
    Send(Msg.A) -> accepted;
    Arrive(Msg.A) -> ignored AlreadyApplied;
    AckArrive(Msg.A) -> accepted;
    expect Msg.A in acked and not (Msg.A in unacked);
  }
}
```

If this text is unsound under the rules as defined, do not bend the checker: report which
code fires and why, fix the fixture minimally, and say what you changed.

# Acceptance

- [ ] Every code B101–B903 in [[spec-language]] has a fixture producing exactly that code.
- [ ] `counter.bspec` and `remote_delivery.bspec` are sound.
- [ ] The anchor and the differential tests pass; the counts are in the report.
- [ ] `make check-specs` exits 0 on the repository; show a run on a seeded unsound
      Markdown file exiting 1 with the document's line number.
- [ ] The crate depends only on `bboxy450-tlc`; nothing but the API above is public.
- [ ] `README.md` reference is under 200 lines.
- [ ] The verify command exits 0.

# Out of scope

The `knowledge_commit` gate (next task); any change to `protocol!`, `bbt-yarn` or
`bboxy450-tlc`; a formatter; progress and fairness; references between specs.

# Report

Follow `AGENTS.md`: stay silent while working. When done: `make land`, write
`reports/2026-11-09-bspec-checker-crate.md`, then `task_report`. Include: files and
mechanisms, every command with its real exit status, the state and transition counts for
both sound fixtures, "Spec questions" (every ambiguity you resolved and how), and what
you did not check.
