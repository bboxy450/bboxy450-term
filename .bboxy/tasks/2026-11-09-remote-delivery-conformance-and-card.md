---
type: task
title: 'RemoteDelivery: exhaustive runtime conformance walk and a generated level-1 card'
role: executor
allowed_paths:
- crates/bboxy450-term-remote/
- crates/bbt-yarn/
- crates/bbt-mermaid/tests/
- verification/contracts.toml
- verification/coverage.toml
- Cargo.lock
- .bboxy/reports/
- scripts/harness-submodule.py
- scripts/test-harness-submodule.py
verify: python3 -I scripts/test-harness-submodule.py && python3 -I scripts/harness-submodule.py --check-json-features && ! python3 -I scripts/harness-submodule.py --check-json-features --acp-control && cargo nextest run -p bboxy450-term-remote -p bboxy450-yarn -p bboxy450-mermaid && make test-formal-contracts
status: claimed
assignee: seat:01M71H0S7MDHJ0QTCQFPQYXJHW
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
id: 01M71GYNQAVZP9XCTR3G4T99PX
number: 205483610615695
created: 2026-11-10T01:25:25.181Z
updated: 2026-11-10T03:01:08.054Z
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
verify_revision: 1
provenance:
  digest: sha256:9f234343bc406b259e34c96d1c88b0dacadb14737a9403c0300a5144be1c86a2
  frontmatter_digest: sha256:a7c8880ddc39daffaa57afa70436ab5d1a62842097d50ea886fdceaf08910f74
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: eXdeM7MPd0UBLHgfZMNRrpfiikcD8S1cccCOhXGagMr8eDEm9bOcr1RFv+qHZbIUZW5d2vxcscEcxgBco0amCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151304
  event:
    id: 01M71PDYDT9DJME7MRRWC3FKYP
    sequence: 151304
    hash: 781cb0e6b68ec82f28789f41d53d3d444c7fcf0ca0494a7fd2c342faa608b3e1
---
ROLE: EXECUTOR

# Amendment 3, 2026-11-09: unblock the landing (Oracle ruling on your second block)

Your diagnosis is confirmed. I reproduced it: workspace-wide `cargo metadata` reports the
harness node with `acp, acp-agent, default, embedded` since `a58a0fcfb` pinned a harness
whose default features include `acp`; `crates/bboxy450-term/Cargo.toml:21` selects only
`embedded`. The gate reads the wrong set. You are authorized to fix it: the two script
paths are now in `allowed_paths`.

**The rule the check holds, unchanged:** the harness as linked into `bboxy450-term` must
not have the legacy `acp` frontend, and `serde_json/preserve_order` is allowed only with
`acp-agent`. The check must read the features the consumer selects, not the union over
the workspace.

## Do it in this order

1. In `scripts/harness-submodule.py`, change how `check_json_features` obtains the feature
   sets: resolve for the consumer package only. Suggested mechanism:
   `cargo tree -p bboxy450-term --locked -e normal,build --prefix none -f '{p}|{f}'`,
   reading the feature lists on the `bboxy450-harness` and `serde_json` lines; pass the
   negative control's extra feature with `--features bboxy450-harness/acp`. If another
   mechanism is more reliable, use it and say why. Keep `validate_json_features` a pure
   function over data so its tests stay unit tests.
2. Keep both rules and the `--acp-control` negative control: with the flag the check must
   still fail, naming `bboxy450-harness/acp`.
3. In `scripts/test-harness-submodule.py`, add cases: (a) the workspace default includes
   `acp` but the consumer selects only `embedded` → passes; (b) the consumer's closure
   includes `acp` → rejected; (c) `preserve_order` without `acp-agent` in the consumer's
   closure → rejected. Keep every existing case; adapt its input shape only if step 1
   changed it.
4. Commit this as its own signed commit on your branch, separate from `fd38ec4c0`.
5. Run the verify command (it now starts with the three script checks; the third proves
   the negative control still fails). Then `make land` for both commits.
6. Write one new report that supersedes your last one and covers both commits, then
   `task_report`.

**Stop only if** the consumer's real closure contains `acp`. That would be a true
violation: report it and do not change the rule.

The same instructions are also filed as
[[tasks/2026-11-09-harness-feature-check-consumer-closure]]; that record is a pointer,
this task is your brief.

# The original work (done at fd38ec4c0, still part of this task's acceptance)

First milestone of [[spec-first-os]]. On `RemoteDelivery`:

1. **W1 binding.** The real runtime (`link.rs`: `Outbox`, `Inbox`, `FileJournal`) refines
   the checked model: an exhaustive breadth-first walk over a test harness that owns the
   link (message frames and `Applied` frames), each runtime operation mapped to model
   events, agreeing on acceptance, `Apply`, projected state and REMOTE-DELIVERY-001..003;
   coverage counts printed and pinned; a memory-journal broken control that fails on a
   `Fresh`-versus-`Apply` or `applied` disagreement. Environment assumption recorded:
   the link does not forge `Applied` frames.
2. **Level-1 view.** `bbt_yarn::card` turns the checked model into a Markdown card with a
   state diagram of at most twelve nodes and a generated "Does not promise" section; the
   card is stored at `crates/bboxy450-term-remote/cards/RemoteDelivery.md` and a test
   fails when it is stale.

The full definition of both parts is this record's previous revision (digest
`6f79710cc72672f957573e365e70397adfd19a54ecaa8ba70139982727516104`, in Git at `2d4731440`).

# Acceptance

- [ ] The walk covers every reachable harness state and every enabled operation from each;
      the coverage counts are printed and pinned.
- [ ] Unexercised model edges are listed by kind in the report.
- [ ] The runtime-side broken control fails for the stated reason only.
- [ ] The scripted single-trace test and the probe are removed.
- [ ] `cards/RemoteDelivery.md` is generated, at most twelve nodes, with "Does not promise"
      including the no-forged-`Applied` assumption.
- [ ] Editing a guard in a scratch copy of the spec makes the card test fail (show the run).
- [ ] The Mermaid block has a `bbt-mermaid` fixture and passes that crate's tests.
- [ ] `verification/contracts.toml` / `coverage.toml` describe the walk and the new
      assumption. No existing assumption is weakened or removed.
- [ ] `cargo clippy -p bboxy450-term-remote -p bboxy450-yarn --all-targets -- -D warnings` exits 0.
- [ ] The harness feature check passes on the consumer's closure and its negative control
      still fails; no rule removed.
- [ ] The verify command exits 0 and `make land` completes.

# Out of scope

Changing `protocol!` syntax; `link.rs`/`delivery.rs` behaviour; GUI or document-viewer
changes; level 0 and level 2 views; any other component; any other script.

# Report

Follow `AGENTS.md`: stay silent while working. Include: files and mechanisms, what you
removed, every command with its real exit status, the coverage counts, the before and
after output of the harness feature check, mapping corrections, abstraction gaps, and
"What generalizes".
