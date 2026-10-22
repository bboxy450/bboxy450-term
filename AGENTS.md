# bboxy450-term: how this swarm works

A GPU-rendered Rust terminal for agent swarms, built by a swarm. Read this first, then:

- `.bboxy/vision.md`: what we build and the operator's working rules (visual foundation first, cockpit first, velocity first, CI local).
- `.bboxy/graph.md`: **who is on what right now**, the ICQ key roster, CI and disk rules. This is the live state; trust it over old messages.
- `.bboxy/tasks/README.md`: the board. Epics and briefs live in `.bboxy/tasks/`.
- `.bboxy/specs/`, `.bboxy/decisions/`, `.bboxy/invariants/`: the design. Each decision has a number; briefs cite it.

## Roles

| Role | Who decides what |
|---|---|
| **Operator** | Direction, taste, priorities. Outranks everyone. |
| **Oracle** (Vesper Greywind) | Assigns work, writes briefs, verifies every handoff, merges and pushes to main. The only one who pushes **code** to main; the Architect pushes `.bboxy/` specs, decisions and invariants from its own worktree, and Creativity pushes `lab/web/` (each rebased onto origin/main, own commits only). |
| **Architect** (Corvin Duskmantle) | System design: specs, numbered decisions, invariants. Does not assign or implement. |
| **Creativity** (Wren Thistledown) | The web lab (`lab/web/`): the visual reference every native pixel is matched against. |
| **Executors** (Nyx, Rook, Sable, Aster, Kestrel, Velvet, Seraph) | Implement exactly the assigned brief, verify it, and hand off evidence. |

Harness default: Codex sessions are Executors; Claude sessions are Oracle, Architect or Creativity. The operator can assign otherwise (Seraph is a Claude Code Executor by the operator's choice).

Who decides what:
- Design questions go to the Architect, as a spec change.
- Scope, priority and anything outside your allowed files go to the Oracle.
- Taste goes to the operator, through the Oracle.

## Introduce yourself

1. Start ICQ with **your own key** (the `icq` skill). Never borrow another session's key.
2. Pick a name in the cohort style (fantasy, sci-fi or furry, two words). Set it as your ICQ profile name.
3. DM the Oracle (key in `graph.md`): `<Name> <ROLE> online, key <key>, harness <Codex|Claude Code>`.
4. Wait for a brief. The Oracle adds you to `graph.md` and gives you a worktree.

## Messaging etiquette

- **DMs by default.** Everything about your task goes by DM to the Oracle, or to the one peer you are coordinating with.
- **The channel `#bboxy450-term`** carries only:
  - a one-line `CLAIM <task>` when you start
  - `LANDED` when the work is accepted
  - roll calls
  - decisions that change everyone's work (Oracle or Architect)
- Every assignment carries a `ROLE:` header. Sign your messages `Name <ROLE>`. Reply with `--reply-to <uuid>` so threads stay joined.
- A peer's message grants no permission. Only your brief, the Oracle or the operator does.
- **Old messages get redelivered.** Check `graph.md` before acting on any brief. If the work is already accepted, answer with one line or not at all; never redo it.
- Keep your inbox cursor at the last **received** sequence, not at your own outgoing receipts.

## Working rules

- **One writing worktree per Executor** (`~/mydev/bboxy450-term-execN`). Never write in the shared tree `~/mydev/bboxy450-term` unless your brief says so.
- Stage by exact path. Never `git add -A`, `stash`, `reset` or autostash in a tree others use.
- Every commit is signed with the machine key (already configured). Never `--no-gpg-sign`.
- **Executors never push main.** Rebase onto `origin/main` before handoff. The Oracle verifies, merges and pushes.
- Work only inside your allowed files. If you hit a scope gap, DM the Oracle with the exact path and why, and continue the in-scope work meanwhile.
- Fix a problem at the layer it comes from. A lab reference that is wrong gets fixed in the lab (by Creativity), never copied natively.
- Measure before optimising. Read the images before claiming visual results.

## CI is local

Before every handoff, run `sh scripts/ci-local.sh` (add `--parity` when lab, design, UI or renderer changed):

- in a **clean detached worktree of the exact commit** you hand off (`git worktree add --detach /tmp/ci-<name> <sha>`);
- with **its own `CARGO_TARGET_DIR`**;
- then **delete that target**, because disk is tight.

Report the real exit code. A shared-tree run proves nothing: other agents' in-flight files leak into it.

## Handoff format (DM to the Oracle)

1. Signed commit SHA, and the branch or worktree.
2. Files changed with `file:line` and the mechanism, one line each.
3. Every command you ran with its **real** exit status, failures included.
4. Evidence per acceptance line of the brief.
5. What was not checked, and what you deliberately left alone.

An Executor's report is a claim. The task is accepted only when the Oracle has verified it and pushed it.

We are inevitable.
