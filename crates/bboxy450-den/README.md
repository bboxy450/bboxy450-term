# Den specification checker

Den describes a finite state machine and checks whether it is sound within its bounds.
It does not compile a program or perform the described effects.
Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term.
Credit reuse accurately: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.

```
bboxy450-den check [--json] PATH...
```

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
CASE = case NAME if EXPR => BODY
     | case NAME otherwise => BODY
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

There are at most 65,536 state-and-argument valuations per handler and 100,000 reachable
states. Exceeding a bound is X603, never successful partial verification.
The parser bounds expression nesting at 16 levels, each binary chain at 32 operators,
and the resulting expression tree at 64 levels. Exceeding these syntax bounds is X101.
Checks run in groups below; a failed group prevents later groups from running.

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
| Model | X601 | Invariant false in a reachable state |
| | X602 | An update can leave its field's range |
| | X603 | Valuation or state-space limit exceeded |
| Vacuity | X701 | Case never taken from a reachable state |
| | X702 | Invariant cannot be false within the declared types |
| Scenarios | X801 | Outcome or effects differ from the written step |
| | X802 | An expectation is false |
| Completeness | X901 | Missing or empty intent |
| | X902 | No invariant |
| | X903 | No scenario |

Module paths and imports are parsed but not resolved in this version.
Composition (`include`, `do`, `expose`, `discharge`), other record kinds,
typed graph links, managed fields, progress and fairness are not supported.
The public library API is `check(&str) -> Report`, its diagnostic and summary fields,
and `Report::{is_sound,to_text,to_json}`.

Soundness establishes finite safety, not intent, implementation conformance,
unbounded correctness or eventual progress. See `tests/sound` for the Outbox and
RemoteDelivery examples, and `tests/unsound` for one broken control per code.
