---
type: task
title: 'Lean land plan: a manifest change that only adds a workspace member affects only that member'
role: executor
allowed_paths:
- scripts/landing.py
- scripts/test-landing.py
- LAND.md
- verification/contracts.toml
- crates/bbt-mermaid/tests/
- .bboxy/reports/
verify: python3 -I scripts/test-landing.py && make test-formal-infrastructure
id: 01M731QJQJ3A2RGJSGACQDTDPD
status: claimed
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T15:38:40.965Z
created: 2026-11-10T15:37:53.007Z
number: 1794292861627449
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
assignee: seat:01M731RY7V5R0Y66H8G05M8EEJ
provenance:
  digest: sha256:216a845708b4c263893c2cf067f80bbf27b49c823b77d2872efa7da311295ac9
  frontmatter_digest: sha256:460a79ddb431298eb4a89c8947f575d71ef087918cef613e0edfe16f034e901e
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: 9Jl/ejAjMQWawtUja47qN/P9z20k2mvh5NL+o50sx/hrkBun54fbhGlSkvWGEO/Uebwkc0JUInDoX3j7Fj+oBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151485
  event:
    id: 01M731S22302PHZY2VY92YNXNR
    sequence: 151485
    hash: 9b7a1f12900b4a6882878b3d0c77b4e1bc264797f2e890b664cd8768310f45fb
---
ROLE: EXECUTOR

# Goal

Stop the lean landing gate from treating "a new crate was added" as "every crate changed".

Today `scripts/landing.py` `affected()` marks any change to the root `Cargo.toml` or
`Cargo.lock` as *broad* and puts every workspace package in the plan
(`if broad: touched.update(packages)`). Adding a crate always edits both files, so a new
leaf crate that nothing depends on must pass every other crate's tests to land. On
2026-11-10 that made [[tasks/2026-11-09-bspec-checker-crate]] fail five landings on
tests it cannot affect.

Decision (Oracle, under the operator's standing delegation, recorded here and reversible):
a manifest change that **provably only adds workspace members** is not broad. Everything
else about `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` and `.cargo/` stays broad
exactly as now.

Worktree: the managed workspace this seat was launched in. Base `origin/main`.

# The rule, exactly

Let OLD and NEW be the file contents at the two ends of the range `affected()` is given.
The root manifest pair is **member-only** when all of these hold:

1. `Cargo.toml`: parse OLD and NEW with `tomllib`. NEW's `workspace.members` is OLD's with
   one or more entries appended or inserted, none removed or changed. With
   `workspace.members` set equal in both, the two parsed documents are equal. (So a new
   `[workspace.dependencies]` entry, a version change, a profile change or a feature
   change is not member-only.)
2. `Cargo.lock`: parse OLD and NEW with `tomllib`. Every `[[package]]` table in OLD is in
   NEW unchanged (same name, version, source, checksum and dependency list). Every extra
   table in NEW has no `source` key (it is a path package) and its `name` is the package
   name of one of the members added in step 1. Top-level keys other than `package` are
   equal.
3. If only one of the two files changed, the other is trivially fine; the changed one must
   still satisfy its own rule. A `Cargo.lock` change with no `Cargo.toml` change is
   member-only only if it adds nothing (so it is never member-only in practice: keep it
   broad).
4. `rust-toolchain.toml` and `.cargo/` are never covered by this rule.

When the pair is member-only, remove `Cargo.toml` and `Cargo.lock` from `broad` for the
purpose of `touched.update(packages)`. The new member's own files already put it in
`touched` through the existing path-to-package loop, and `dependent_closure` already adds
anything that depends on it. Keep both files in `shared` (the tooling list) so the fast
invariants still see a manifest change.

Any parse failure, any missing OLD file, any doubt: broad. The rule only ever narrows on
a positive proof.

# Do it in this order

1. Read `affected()` (near line 706) and the two other places that test for
   `("Cargo.toml", "Cargo.lock")` (near lines 1141 and 1313, recheck and moved-main
   logic). Understand what each decides. Change only the decision "which crates are
   tested". If the other two sites would still force every crate's tests for a
   member-only change, apply the same predicate there; if they decide something else
   (for example whether evidence may be reused), leave them and say so in the report.
2. Write one pure function, for example
   `member_only_manifest_delta(old_toml, new_toml, old_lock, new_lock, packages) -> bool`,
   taking file texts (or `None` when a file did not change) and returning the verdict. No
   git and no I/O inside it, so it is unit-testable.
3. Call it from `affected()` with the texts read by `git show <base>:<path>` and the
   worktree files for the range ends. Reuse the module's existing `git` helper.
4. In `scripts/test-landing.py`, next to the existing `affected` fixtures (the helper near
   line 311), add cases. Each is a broken control for the rule except the first:
   - (a) add a leaf crate: new member line, new lock stanza, new crate directory →
     `touched` is exactly the new crate; tier is not `wide`.
   - (b) (a) plus a changed dependency version in root `Cargo.toml` → every package.
   - (c) (a) plus a new third-party `[[package]]` in `Cargo.lock` → every package.
   - (d) (a) plus an existing lock stanza gaining a dependency → every package.
   - (e) a member removed → every package.
   - (f) `rust-toolchain.toml` changed alongside (a) → every package.
   - (g) `Cargo.lock` changed alone → every package.
   - (h) a new member that an existing crate now depends on: that crate's `Cargo.toml`
     changes and, with the lock regenerated, its existing lock stanza gains the
     dependency → every package (clause 2 fails). Amendment 1, 2026-11-10: the first
     wording expected a narrow plan here and contradicted clause 2 and case (d). Clause 2
     wins. The rule narrows only for a pure leaf: a new member nothing else depends on.
     Build this fixture with a correctly regenerated lock, never a stale one.
   Also unit-test the pure function directly on small TOML strings for each rule clause.
5. Update `LAND.md` where it describes what the lean gate tests: one short paragraph
   stating the rule and that anything else in the root manifest stays broad.
6. If `verification/contracts.toml` has a contract whose `assumptions` or `property` text
   describes the affected-set computation, update the text to state the new rule; do not
   weaken any other sentence. If none describes it, change nothing there and say so.
7. Run the verify command. Commit signed. `make land`. Because this change touches only
   scripts and docs, its own plan should not include crate tests; if it does, send me the
   plan lines before letting it run.

# Latitude

You decide, without asking: function and test names, how fixtures build their temporary
repositories, wording in `LAND.md`. A Mermaid fixture sync after a rebase is yours.

Stop and message only if: an existing landing test fails for a reason you cannot tie to
this change; the rule as written cannot be implemented; or `make land` fails. For a land
failure send the failing step and its last 15 lines at once, without investigating.

Land with the seat's context scrubbed, exactly:

```
env -u BBOXY_CAPABILITY_FILE -u BBOXY_GENERATION -u BBOXY_PROCESS_GENERATION -u BBOXY_ROLE -u BBOXY_RUN_DIR -u BBOXY_SEAT_ID -u BBOXY_SOCK make land
```

(see [[bugs/harness-python-execute-tests-leak-seat-context]]). Before starting it run
`df -h ~`; under 8 GiB free, tell me first.

# Acceptance

- [ ] Case (a) plans exactly the new crate; cases (b) to (h) plan every package.
- [ ] The pure function has a unit test per rule clause, including parse failure → broad.
- [ ] Every existing test in `scripts/test-landing.py` still passes, unchanged.
- [ ] `LAND.md` states the rule.
- [ ] The verify command exits 0 and the change is landed.

# Out of scope

Any other gate behaviour; quarantine; the formal stage; `make land-full`; any Rust source.

# Report

`reports/2026-11-10-land-plan-member-only-manifest.md`, then `task_report`: mechanism,
what each of the three manifest checks in `landing.py` decides and what you changed in
each, every command with its exit status, the plan `make land` printed for this change,
and what you did not check.
