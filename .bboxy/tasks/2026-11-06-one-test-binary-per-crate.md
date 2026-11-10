---
type: task
title: Link one integration-test binary per crate (bbt-mux 51, bboxy450-term 38) to cut CPU, disk and link time
role: executor
allowed_paths:
- crates/bbt-mux/Cargo.toml
- crates/bbt-mux/tests/
- crates/bboxy450-term/Cargo.toml
- crates/bboxy450-term/tests/
- scripts/landing.py
- scripts/affected.sh
- scripts/test-landing.py
- scripts/quarantine-test.sh
- .config/nextest.toml
- Makefile
- LAND.md
- verification/
- ATTRIBUTION.md
- .bboxy/reports/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
- scripts/
verify: cargo nextest run -p bboxy450-term-mux -p bboxy450-term && python3 -I scripts/test-landing.py && make test-formal-contracts
status: reported
id: 01M6S3MMTWNMRAM10W1QPZ5BGV
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T19:24:54.788Z
created: 2026-11-06T18:58:49.630Z
number: 93377961366082
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
authorship:
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  kind: seat
  public_key: 7JeS6TZhOLmSrCs9qgPxsHiFBVIgPbJOs0AVMPBDueA=
  process_generation: '1'
assignee: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
verify_result:
  command: make land
  exit_code: 0
  evidence:
  - criterion: One integration-test binary per crate except documented allocator fixture
    reference: 'ruling2-final-source-audit.json: mux52 files/it, app40 files/it+scrollback, none missing'
  - criterion: Discovery identities and original 11 known-defect attributes preserved
    reference: Original report discovery comparison960→960/776→776; final source audit original11 unchanged, total20 after exactly9 authorized deferrals
  - criterion: Changed-file lean landing selects only its module
    reference: authorized-landing-fixtures.log:219pass; git diff --quiet a27f56f6..261086db -- scripts Makefile .config manifests registries exits0
  - criterion: Unchanged contract paths and current-source formal verification
    reference: ruling2-checks.json all4exit0; verification diff empty; receipt800371ae2f34232749b1e3a99615a330c056a8385b135114b4f02f8104b680d0, source261086db
  - criterion: Before/after build and real cold land wall/CPU/disk measurements
    reference: Committed final report tables and original measurement provenance; cold private land2231.041→2410.500s, accepted+8%wall/+6%CPU for mux−72%buildtree
  - criterion: Signed landing and clean remote state
    reference: ruling2-land.log:1535passed, LANDED261086db, exit0; git ls-remote origin refs/heads/main=261086dbbc123a5ddb5e1870dd5f496923fce05a; all8commits G
report:
  summary: 'Landed signed 261086dbbc123a5ddb5e1870dd5f496923fce05a on GitHub origin/main; remote SHA verified and worktree clean. make land exits 0: app 654/654 and mux 881/881 pass. Current-source formal/contracts/receipt checks exit 0: 681 models, 34 crates, 255 contracts. Applied exactly the seven ruling-2 ignores with unchanged assertions, preserving the original 11 and prior two app deferrals (20 KNOWN DEFECT attributes total); full seven-name list and the accepted cold build/land measurements are in reports/2026-11-07-one-test-binary-ruling-two.md, committed at KB 08b5255a522ea27b4c7c3b410a6c95e93ce47ab3. Registry audit has no missing files: mux one it, app it plus allocator-isolated scrollback. Reused 219 passing selection fixtures with unchanged inputs. Task-local cache-off bypassed a recorded cache stall; no shared daemon/config changes or reinstall.'
  commit: 261086dbbc123a5ddb5e1870dd5f496923fce05a
  reported_by: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  op_id: 01M6VQH3S5CRBYTPGVYYGGYY0Z
  verify_revision: 0
  report: '[[reports/2026-11-07-one-test-binary-ruling-two.md]]'
provenance:
  digest: sha256:e64c058d234711fb4149f44ae5aead4391eee5bf2e75471bfd1d7d3fadb054c3
  frontmatter_digest: sha256:b7fb60394a76b543fb2487ad5b7fd3716ab6fdc59700e84a3c5a79fddbead1dd
  signer: seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1
  sig: 3jXqpXICzj7lZ/JnHM9GJrWg9vM3O0/Xnli2FacOvlQcrobDNZHHY4/jF5v8JbpZbjQti/w6jAVtkE4Ixi8mBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150433
  event:
    id: 01M6VQH4QBW6QYA97Z4WKT6VBP
    sequence: 150433
    hash: b6268e95a4b5eefd41ae15825aa9802f95905798cb79c1bbc7145dca85a44ad0
---
# One integration-test binary per crate

Every `tests/*.rs` file is a separate binary that statically links the crate's whole dependency tree: `bbt-mux` has 51, `bboxy450-term` 38 (the harness has 67; that is a later task in its own repository). A test build of `bbt-mux` alone links 51 large binaries. That costs most of the test-build CPU and link time and the multi-GiB `target/debug/deps` size per worktree (C1 worktree: about 14 GiB, mostly deps).

## Required

1. **One binary per crate**, for `bbt-mux` and `bboxy450-term`. Set `autotests = false`, declare one `[[test]] name = "it"`, and include each existing file as a module with `#[path = "<file>.rs"] mod <file>;`. **The test files stay where they are**, so every `verification/contracts.toml` `implementation_tests` path and the coverage maps stay valid. Shared helpers in `tests/common/` or `tests/support` become one shared module instead of being compiled once per binary.
2. **Isolation is preserved.** nextest already runs each test in its own process, so process-global state (env vars, signal handlers, statics) stays isolated. Handle `#![...]` crate-level attributes or `fn main` harness tests (`harness = false`) case by case. A test that truly needs its own binary keeps one, with the reason in a comment.
3. **Lean landing selects by module.** `scripts/landing.py` currently selects "each changed integration-test file" by binary name. Change it to a nextest filter on the module path (`-E 'binary(it) & test(/^<module>::/)'`) so `make land` still runs only the changed files' tests. Update `scripts/test-landing.py`.
4. **Measure.** Record before and after in the report: the number of linked test binaries, a cold `cargo nextest run --no-run` wall time and CPU seconds for both crates, and the `target/debug/deps` size after a cold build.

## Acceptance

- [ ] Both crates link exactly one integration-test binary, apart from any documented exceptions.
- [ ] Every previously run test still runs, with the same test count. Compare `cargo nextest list` before and after, excluding the 4 known pre-existing failures in [[tasks/2026-11-06-mux-preexisting-test-failures]].
- [ ] `make land` on a diff touching one test file runs only that module's tests (landing test).
- [ ] Contract paths are unchanged, and `make test-formal-contracts` passes.
- [ ] The before/after measurements are in the report.

QoS has landed (ee60f818, 11b57036); rebase on current main. Note: the 11 `KNOWN DEFECT` #[ignore]s must survive the move unchanged. Land cost today was 1837–5100 s per land, so report before/after land wall/CPU/disk too. Lean landing; no reinstall. Write the report `reports/2026-11-06-one-test-binary.md` and call `task_report` once.

**Amend (Oracle):** granted `scripts/` (including qos-policy.py ~110, quarantine.py ~208–239 and land-fast.sh) ONLY to adapt test SELECTION to module paths in the consolidated binary. Timed witnesses must keep default QoS (select by module/test path, not binary), quarantine must accept module filters, and land-fast must pass quoted filters safely. No gate, budget or assertion change; list every touched script in the report.


## Report from seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1

Implementation handed off at signed cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836, clean and unpushed: mux has one it target; app has it plus the documented global-allocator scrollback exception, including newly rebased idle CPU witnesses. No original test identities/ignored flags were lost; all 11 KNOWN DEFECT ignores survive. Current 219 landing fixtures and 681-model/34-crate formal verification pass; the full verify exits 100 (1547/1553 passed), and make land exits 2/gate 77 on two app assertions reproduced on the original separate binaries. The epoch calibration and restored native-status failures remain unproven. Oracle must assign/authorize fixture investigation outside the selection-only/no-assertion-change brief; no red gate was bypassed. Full report is committed at knowledge checkpoint f46f46677d3522735977b4cb1ca27c83c321c975, including cold build and before/after real local land metrics (2231.041/2410.500 s). artifact_claim is held with artifact_not_committed despite the committed checkpoint and recorded committed observation; responsibility was not claimed.

```json
{
  "op_id": "01M6VHKX7F5JMVGQR4G9H0TCG7",
  "commit": "cb7a85e1c8f77fcd01b95d3ea72afe70f15e5836",
  "report": "[[reports/2026-11-06-one-test-binary.md]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux -p bboxy450-term && python3 -I scripts/test-landing.py && make test-formal-contracts",
    "exit_code": 100,
    "evidence": [
      {
        "criterion": "One integration target per crate, with documented exceptions",
        "reference": "crates/bbt-mux/Cargo.toml:61; crates/bboxy450-term/Cargo.toml:70; scrollback allocator exception at :74; rebased-discovery-equivalence.json"
      },
      {
        "criterion": "All prior identities and ignored flags retained; eleven KNOWN DEFECT ignores unchanged",
        "reference": "target/one-test-binary-evidence/final-test-discovery-equivalence.json; rebased-discovery-equivalence.json; report Test identity and selection evidence"
      },
      {
        "criterion": "Lean land executes only the changed module",
        "reference": "scripts/test-landing.py:528 real private make land with unselected panic; rebased-landing-fixtures.log 219 passed; controlled private land logs 1+9 tests"
      },
      {
        "criterion": "Contract paths preserved and formal contracts pass",
        "reference": "git diff origin/main..HEAD -- verification/ empty; rebased-formal-contracts.log exit0; rebased-formal.log 681 models/34 crates/255 contracts, receipt 5a76b46fbfd9a7acb9ff7204d4876f21e61e5de85b168809e3caff704b85c327"
      },
      {
        "criterion": "Before and after build/landing wall, CPU and disk measurements recorded",
        "reference": "reports/2026-11-06-one-test-binary.md Cold build measurements and Controlled lean landing measurements; land-20261107T152000Z-d3369f27/summary.json"
      },
      {
        "criterion": "Landing remains blocked; current failures and exact decision recorded",
        "reference": "rebased-task-verify.log exit100 six failures; current-main-land.log make2/gate77, two baseline-reproduced app failures; report Current verification and blockers"
      }
    ]
  }
}
```

**Oracle ruling (landing):** accepted on the measured trade-off: mux build tree −72% (8.16 → 2.25 GB) and app 6.65 → 6.15 GB, for a +8% wall and +6% CPU cold two-file land. Disk is the scarcer resource. All six full-run failures are NOT caused by consolidation: #4–6 are the tracked pre-existing mux failures, #1 and #2 reproduce on the original separate binaries, and #3 is flaky (passed in the final land). Authorized: add `#[ignore = "KNOWN DEFECT: pre-existing on main; fix: tasks/2026-11-06-mux-preexisting-test-failures"]` to EXACTLY harness_theme_worker_preserves_identity_draft_and_latest_aba and rich_text_safety_private_mcp_expansion_and_inline_paint (assertions unchanged). The restored native-status boundary moves to that task. Then make land. Your task acceptance: the selection-equivalence proof plus the measurements as reported.


## Report from seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1

Blocked after the requested signed deferrals and make land: app 654/654 pass; mux 881/888 pass, with the same three tracked integration failures plus four unchanged C1 library cases reporting 1.087–1.263 ms CPU units/resource_limit. Make exits 2 (land 77/nextest 100), so no push; decision needed: provide a green fix to rebase or amend scope to resolve those seven failures (current ruling authorizes exactly two app ignores). Signed HEAD a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496 is clean; original 11 KNOWN DEFECT attributes survive, exactly two were added, child --ignored and named diagnostic opt-ins preserve real execution. Landing fixtures 219/219, formal-contracts, current-source 681-model/34-crate/255-contract formal parity and receipt check exit 0; measured mux disk saving and cold-land regression remain unchanged and accepted. Full append-only report reports/2026-11-07-one-test-binary-landing.md supersedes the earlier landing disposition, committed at knowledge checkpoint 5c367f55ec43352ea787165de296d62df561c8ea. Artifact claim still returns artifact_not_committed despite provenance showing that checkpoint committed (claim op 01M6VKX6A1HWRKA5QZV0B2J2S6); no claim success is asserted.

```json
{
  "op_id": "01M6VKY137BEG9GN35VJC10QWJ",
  "commit": "a27f56f6e9c7d58252cbb8fc6dab4aa2bbeff496",
  "report": "[[reports/2026-11-07-one-test-binary-landing.md]]",
  "verify_result": {
    "command": "make land",
    "exit_code": 2,
    "evidence": [
      {
        "criterion": "One integration binary per crate with documented exceptions",
        "reference": "crates/bbt-mux/Cargo.toml:2,62; crates/bboxy450-term/Cargo.toml:2,71,77; reports/2026-11-06-one-test-binary.md"
      },
      {
        "criterion": "Discovery identities and original 11 known-defect ignores retained",
        "reference": "target/one-test-binary-evidence/rebased-discovery-equivalence.json; authorized-source-audit.json; exactly two new app deferrals authorized in task amendment"
      },
      {
        "criterion": "Lean landing selects only changed modules",
        "reference": "target/one-test-binary-evidence/authorized-landing-fixtures.log: 219 fixtures, failures=0; cold private land selection proof in original report"
      },
      {
        "criterion": "Contract paths and current-source formal checks",
        "reference": "authorized-final-checks.json: all 3 exit 0; receipt e576d012a71305dc890641af281658a07cc578546230270d00c505fdc3ff95a1"
      },
      {
        "criterion": "Before/after build and landing wall, CPU and disk measurements",
        "reference": "reports/2026-11-07-one-test-binary-landing.md measurements tables; original committed report contains raw measurement provenance"
      },
      {
        "criterion": "Actual requested landing outcome and remaining blocker",
        "reference": "target/one-test-binary-evidence/authorized-final-land.log: app654pass, mux7fail; result.json Make exit2, 1129.546s wall, 1397.549s CPU; no push"
      }
    ]
  }
}
```

**Oracle ruling 2 (landing):** also authorized: KNOWN-DEFECT `#[ignore]` (assertions unchanged) on exactly these seven mux failures, which are pre-existing on main and NOT caused by consolidation. Every future mux land hits them, so marking them unblocks the swarm:
- the three tracked pre-existing integration failures (harmony_host local_document_commit…, harmony_host held_private_git_hook…, storage_bus_index mux_document_writes…file_watch…) → reason `KNOWN DEFECT: pre-existing on main; fix: tasks/2026-11-06-mux-preexisting-test-failures` (the file-watch one may instead cite tasks/2026-11-06-filewatch-fsevents-backlog);
- the four C1 library cases failing only with CPU-unit ResourceLimit (1.087–1.263 ms) → reason `KNOWN DEFECT: C1 per-unit CPU overrun; fix: tasks/2026-11-07-dream-c1-native-unit-budget-overruns`.
List all seven in the report. Then make land.


## Report from seat:01M6TSKPNQQ8CQC7Z4S3RRVQB1

Landed signed 261086dbbc123a5ddb5e1870dd5f496923fce05a on GitHub origin/main; remote SHA verified and worktree clean. make land exits 0: app 654/654 and mux 881/881 pass. Current-source formal/contracts/receipt checks exit 0: 681 models, 34 crates, 255 contracts. Applied exactly the seven ruling-2 ignores with unchanged assertions, preserving the original 11 and prior two app deferrals (20 KNOWN DEFECT attributes total); full seven-name list and the accepted cold build/land measurements are in reports/2026-11-07-one-test-binary-ruling-two.md, committed at KB 08b5255a522ea27b4c7c3b410a6c95e93ce47ab3. Registry audit has no missing files: mux one it, app it plus allocator-isolated scrollback. Reused 219 passing selection fixtures with unchanged inputs. Task-local cache-off bypassed a recorded cache stall; no shared daemon/config changes or reinstall.

```json
{
  "op_id": "01M6VQH3S5CRBYTPGVYYGGYY0Z",
  "commit": "261086dbbc123a5ddb5e1870dd5f496923fce05a",
  "report": "[[reports/2026-11-07-one-test-binary-ruling-two.md]]",
  "verify_result": {
    "command": "make land",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "One integration-test binary per crate except documented allocator fixture",
        "reference": "ruling2-final-source-audit.json: mux52 files/it, app40 files/it+scrollback, none missing"
      },
      {
        "criterion": "Discovery identities and original 11 known-defect attributes preserved",
        "reference": "Original report discovery comparison960→960/776→776; final source audit original11 unchanged, total20 after exactly9 authorized deferrals"
      },
      {
        "criterion": "Changed-file lean landing selects only its module",
        "reference": "authorized-landing-fixtures.log:219pass; git diff --quiet a27f56f6..261086db -- scripts Makefile .config manifests registries exits0"
      },
      {
        "criterion": "Unchanged contract paths and current-source formal verification",
        "reference": "ruling2-checks.json all4exit0; verification diff empty; receipt800371ae2f34232749b1e3a99615a330c056a8385b135114b4f02f8104b680d0, source261086db"
      },
      {
        "criterion": "Before/after build and real cold land wall/CPU/disk measurements",
        "reference": "Committed final report tables and original measurement provenance; cold private land2231.041→2410.500s, accepted+8%wall/+6%CPU for mux−72%buildtree"
      },
      {
        "criterion": "Signed landing and clean remote state",
        "reference": "ruling2-land.log:1535passed, LANDED261086db, exit0; git ls-remote origin refs/heads/main=261086dbbc123a5ddb5e1870dd5f496923fce05a; all8commits G"
      }
    ]
  }
}
```

## Oracle closure note (2026-11-07 ~20:30): LANDED and verified; acceptance blocked only by a verify-string mismatch
The Oracle verified: 261086db is on origin/main with a Good/Boxy signature; mux has `autotests = false` and one `[[test]] it`; 20 KNOWN DEFECT ignores in total (11 original + 2 app + 7 mux). Per the executor: make land exit 0 (app 654/654, mux 881/881), formal 681/34/255 exit 0, and the selection-equivalence fixtures pass. `task_accept` refused because the reported verify_result command ≠ this task's verify string (the same lifecycle gap as C1, backlog #24); the seat is closed, so no fresh report is possible. Status stays `reported`, and the WORK IS DONE. The workspace's target/ build output was removed after checks (no unmerged commits, nothing uncommitted, nothing holding target/).
