# Den specification checker

Den describes a finite state machine and checks whether it is sound within its bounds:
safety by exhaustive exploration, progress under declared fairness. A `theorem` is also
exported to Lean 4 and proved for domains of every size, and a recorded run of an
implementation can be replayed against the spec. Den does not compile a program or
perform the described effects.
Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term.
Credit reuse accurately: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.

```
bboxy450-den check [--json] PATH...
bboxy450-den lean --out DIR [--expect-fail] PATH...
bboxy450-den replay SPEC TRACE...
```

`check` is the commit gate and never runs another tool. `lean` and `replay` are below.

A path is a file or a directory recursively walked for `*.den`, in sorted order.
Symbolic links are refused. Text output prefixes each diagnostic with its path;
`--json` emits one object per file, with `path` and `report` fields.
Exit 0 means every input is sound, 1 means a diagnostic, and 2 means usage or I/O failure.

## A complete spec

```scala
module specs.Toggle

spec Toggle {
  intent "Two flags toggle together."
  state {
    x: Bool = false
    y: Bool = false
  }
  event Toggle
  on Toggle {
    case toggle if true => { x = !x; y = !y }
  }
  invariant TOGGLE_001 "the flags agree" = x == y
  scenario "toggle both flags" {
    Toggle ~> accepted
    expect x && y
  }
}
```

## Grammar

A file has `module PATH`, zero or more `import PATH` declarations, and one `spec NAME`.
Paths use dot-separated names; a path segment may be a backquoted string.
Names match `[A-Za-z_][A-Za-z0-9_]*`. Indentation never matters. `//` starts a comment.
A newline or `;` separates statements. Newlines inside parentheses or brackets,
after binary operators, `=>` or `=`, and in triple-quoted text continue a statement.
Text uses `"text"` or `"""multiline text"""`.

A spec body contains these declarations:

```text
intent TEXT
enum DOMAIN { MEMBER, MEMBER, ... }
state { FIELD: TYPE = EXPR; ... }
event EVENT(PARAM: TYPE, ...)
effect EFFECT(PARAM: TYPE, ...)
on EVENT(BINDING, ...) { CASE; ... }
invariant ID TEXT = EXPR
theorem ID TEXT = EXPR
property ID TEXT = [forall X: ENUM. ]... EXPR leads to EXPR
assume ID TEXT
scenario TEXT { STEP; ... }
```

Parameterless events, effects and handlers omit parentheses.
A type is `Bool`, `Int[LO..HI]`, a declared enum, or `Set[ENUM]`.
Integer bounds are inclusive, signed, and finite; enum members use `Enum.Member`.
An enum is a finite model-checking domain, not an unbounded type.
Initializers are closed expressions: they may use enum members and local quantifier
bindings, but no state fields or event arguments. Initial values must fit their types.
Integer literals and bounds fit signed 64-bit integers; arithmetic is checked.

```text
CASE = [fair weak | fair strong] case NAME if EXPR => BODY
     | [fair weak | fair strong] case NAME otherwise => BODY
BODY = { STMT; ... } | reject REASON | ignore REASON
STMT = FIELD = EXPR | SET_FIELD += EXPR | SET_FIELD -= EXPR | emit EFFECT(ARGS)
STEP = EVENT(ARGS) ~> accepted
     | EVENT(ARGS) ~> accepted emits EFFECT(ARGS), EFFECT(ARGS), ...
     | EVENT(ARGS) ~> rejected REASON
     | EVENT(ARGS) ~> ignored REASON
     | expect EXPR
```

Parameterless scenario calls omit parentheses. Reject and ignore reasons are names.
An outcome inside a block is recognized so forbidden updates can be diagnosed as X502.

Expressions, from loosest to tightest binding:

| Level | Forms |
|---|---|
| Implication | `a ==> b`, right associative |
| Disjunction | `a || b` |
| Conjunction | `a && b` |
| Negation | `!a` |
| Comparison | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| Arithmetic | integer `+`, `-` |
| Application / primary | `true`, `false`, integer, name, `Enum.Member`, `(expr)` |

Other primary forms are `Set()`, `Set(e, ...)`, set membership `set(e)`,
`if condition then value else value`, `forall x: Enum. predicate`, and
`exists x: Enum. predicate`. A quantifier extends as far right as possible.
Use parentheses to delimit a quantified subexpression.

## Meaning and limits

For every state valuation and event argument combination exactly one case must apply.
`otherwise` applies only if all explicit guards are false; source order gives no priority.
All assignments and emitted arguments read the pre-step state. Updates commit together;
an unmentioned field keeps its value. `+=` inserts one enum member; `-=` removes one.
Reject and ignore leave state unchanged and emit nothing.
An accepting case must change state or emit an effect whenever it applies.
An assumption records an environmental promise; the checker does not prove it.

The checker uses `bboxy450-tlc` to exhaust the reachable graph, checks every invariant,
and reports a shortest event trace to a violation. Every case must be reachable.
An invariant must also be false in at least one hypothetical state of the declared types;
an always-true formula is rejected as vacuous. Scenarios restart at the initial state,
assert each outcome and emitted effects, then evaluate each `expect` at that point.

## Progress

A `property` says that whenever its request holds, its response eventually holds. Its
`forall` binders range over both sides, so it is checked once per member; a side that is
itself quantified needs parentheses. Without fairness every such property is false,
because nothing obliges any event to happen. `fair weak case` promises the case is
eventually taken if it stays selectable; `fair strong case` promises it if it keeps
becoming selectable, which is what survives an interruption such as a dropped link. One
case is one promise over all its argument values. Only an accepting case may be fair
(X505). A refuted property reports the events leading to a loop and the loop itself
(X651); a property whose request no reachable state satisfies is vacuous (X703).
Fairness is an assumption about the environment: record it as an `assume`.

```scala
on Send(m) {
  fair strong case send if up && unacked(m) && !inflight(m) => { inflight += m }
  case hold otherwise => ignore NotSendable
}
property OUTBOX_L1 "a submitted message is eventually acknowledged" =
  forall m: Msg. unacked(m) leads to acked(m)
```

## Theorems: Lean 4, for every domain size

`check` proves an invariant for the declared enum members only. `theorem` declares an
invariant that also owes a proof for every reachable state of the machine *at any domain
size*. It is still model-checked first; an unsound spec exports nothing.

`bboxy450-den lean --out DIR PATH...` writes, for each spec that declares a theorem:

| File | Content |
|---|---|
| `Den<Spec>.lean` | generated: `State` (a structure of propositions; a set is a predicate), `initial`, `Step` (one constructor per accepting case), `Reachable`, each invariant as a predicate, and `ID_statement` per theorem |
| `Den<Spec>Proof.lean` | a copy of the proof beside the spec (`outbox.den` → `outbox.lean`), which proves `theorem ID_proved : ID_statement` in namespace `Den<Spec>` |
| `Den<Spec>Check.lean` | generated: each `ID_proved` has exactly its statement, and prints the axioms it rests on |
| `lean.tsv` | one row per module for `scripts/lean.py --check DIR` |

An enum whose members are named by neither the transitions nor a theorem becomes an
abstract type, and the statement quantifies over it: `∀ (Msg : Type) ...`. Naming a member
(`Gate.Open`) fixes the enum to its declared members. An `Int[lo..hi]` field is a Lean
`Int`, and its range becomes the generated theorem `TYPE_OK`, which is then owed too.
Rejected and ignored cases change nothing and are not steps. Effects, scenarios and
progress properties are not exported. A missing proof file is X905.

`make test-lean` runs the pinned Lean 4 kernel (`verification/tools.lock.json`,
installed by `scripts/lean.py` into a shared machine cache). A theorem passes only when
all three files compile and the proof rests on nothing but `propext`, `Classical.choice`
and `Quot.sound`: `sorry`, `native_decide` and a declared axiom are refused.
`tests/unproved` holds the broken controls, exported with `--expect-fail`. Lean never runs
in `check`. The same generator, `bboxy450_tlc::lean`, serves Yarn's `protocol!`.

## Conformance: replaying a recorded trace

`bboxy450-den replay SPEC TRACE...` checks that a run recorded from an implementation is
a behavior the spec allows. A trace is the body of a scenario: one
`Event(args) ~> outcome [emits ...]` per line, optionally `expect EXPR`. The first step
the spec decides differently is reported at its line in the trace (X801), and the exit
status is 1. The library form is `replay(spec, trace) -> Report`.

## Bounds

There are at most 65,536 state-and-argument valuations per handler and 100,000 reachable
states. Exceeding a bound is X603, never successful partial verification.
The parser bounds expression nesting at 16 levels, each binary chain at 32 operators,
and the resulting expression tree at 64 levels. Exceeding these syntax bounds is X101.
Checks run in groups below; a failed group prevents later groups from running.

## Idioms

A cross-cutting rule that is not a property of the state ("a browser session
never reaches ATC") is checked as a guard audit: a Bool field that every
accepting case sets from its own guard, and an invariant that the field is
never true. The checker then refuses any later case that loosens the guard. It
costs one field; a per-subject log of grants would multiply the state space and
hit the valuation bound.

## Lowering to Markdown

```
bboxy450-den render --markdown [--out DIR] [--overwrite] [--no-mermaid] PATH...
```

One sound `.den` gives one `.md` beside it (or under `DIR`, mirroring the
path): a YAML header (`type: spec`, `title:` from the intent's first sentence,
`generated_from:` with the module and source SHA-256), the intent, domains,
state table, events, effects, handler cases, invariants, assumptions, numbered
scenario steps, a Mermaid flowchart per scenario under "Views" (dropped by
`--no-mermaid`) and the whole Den source in a fenced `den` block. The same
source gives the same bytes. A file that is not a previous lowering is kept
unless `--overwrite`; an unsound source writes nothing and exits 1.

## Diagnostics

Every diagnostic carries a stable code, line, column, record name, message and repair.

| Group | Code | Unsound condition |
|---|---|---|
| Syntax | X101 | Text does not parse |
| Version | X190 | Construct is not supported in this version |
| Names | X201 | Undeclared name |
| | X202 | Duplicate declaration in one scope |
| | X203 | Unused declaration or import |
| | X204 | Event lacks a handler, or handler lacks an event |
| Types | X301 | Wrong expression or field type |
| | X302 | Initial value outside its type |
| | X303 | Wrong call arity or argument type |
| Totality | X401 | Overlapping case guards |
| | X402 | State and arguments match no case |
| Steps | X501 | A case assigns one field twice |
| | X502 | Reject or ignore updates state or emits |
| | X503 | Accepting case changes nothing and emits nothing |
| | X505 | A case that rejects or ignores is declared fair |
| Model | X601 | Invariant false in a reachable state |
| | X602 | An update can leave its field's range |
| | X603 | Valuation or state-space limit exceeded |
| Progress | X651 | A property is refuted by a fair loop |
| Vacuity | X701 | Case never taken from a reachable state |
| | X702 | Invariant cannot be false within the declared types |
| | X703 | Property never requested in a reachable state |
| Scenarios | X801 | Outcome or effects differ from the written step |
| | X802 | An expectation is false |
| Completeness | X901 | Missing or empty intent |
| | X902 | No invariant |
| | X903 | No scenario |
| | X905 | A theorem has no proof file (`lean` only) |

Module paths and imports are parsed but not resolved in this version.
Composition (`include`, `do`, `expose`, `discharge`), other record kinds,
typed graph links and managed fields are not supported.
The public library API is `check(&str) -> Report`, its diagnostic and summary fields,
`Report::{is_sound,to_text,to_json}`, `model(&str)` (the finite model, also for a broken
control), `lean(&str)` (the Lean machine of a sound spec with theorems) and
`replay(spec, trace) -> Report`.

What each result establishes:

| Result | Establishes | Does not establish |
|---|---|---|
| `check` is sound | safety and progress for the declared members, under the declared fairness | anything beyond the bound; that the fairness holds in the world |
| `make test-lean` passes | each theorem in every reachable state, at any size of an unnamed enum | progress; that the Lean machine is the Rust program |
| `replay` is sound | the recorded runs are allowed by the spec | runs that were not recorded |

None of them establishes intent. `make test-formal` also exports the fixtures to TLA+ and
compares the native checker with Java TLC, state for state. See `tests/sound` for Outbox
(progress, two theorems), Turnstile (a named enum and an integer) and RemoteDelivery,
`tests/unsound` for one broken control per code, and `tests/unproved` for the Lean controls.
