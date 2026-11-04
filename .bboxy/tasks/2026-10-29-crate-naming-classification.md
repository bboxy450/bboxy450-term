---
acceptance:
- Every issue70 baseline package and current workspace package has one source-backed generic/app-specific/standalone-app classification and canonical name under decision362.
- The existing licensing inventory preserves origin/license notices and distinguishes published baseline, current local manifests, added/removed packages and unverified publication state.
- Exact proposed rename/dependency/Make verification scope and release ordering are recorded without manifest edits, publication, yank or unproven issue closure.
- Preserved numbered record identity, signed scoped source, actual license/integrity checks and remaining uncertainties are reported; no cold broad rerun.
allowed_paths:
- .bboxy/specs/licensing.md
- .bboxy/runs/issue-70-crate-classification.json
artifacts:
- '[[01M64S6AQZ1Z1K3YK0KPEEKXWY]]'
assignee: seat:01M649QP6AMVQWCG28X5MQP3JR
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T21:17:34.723Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64RCZ3SBN8XKMMZXG98T2YF
number: 470
provenance:
  digest: sha256:abdfe37e8917b06e39a90ab3c346e9fe0fbc115820b4263427f13b4db7e0c819
  frontmatter_digest: sha256:62f1ff17c24d061ea40f91dcb115dcc50dd377d98c36c3da1aabab240dc9c4a8
  log: .bboxy/log/2026-10-29.jsonl#2370
  sig: HSwF5NsZEmeMNkqcTtsXV5MR1pMbC/s7u+DPVKA3FX1gSqGshP2pRAqvq1V5g/a8RCro9WvUzrbcyLLoeFE1BQ==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
report:
  commit: 5d8be221d43d516c523d82e1e3721da86b78e224
  op_id: 01M64S7KEGKD8X77CQ4NV2N5TC
  reported_by: seat:01M649QP6AMVQWCG28X5MQP3JR
  summary: 'Signed5d8be221 in assigned writing worktree, clean; only licensing151 and allowed issue70 JSON changed. Usable signed draft4bdfa681 delivered promptly. licensing.md72–216 classifies29 rows covering27 current packages+2 absent proposed keychain-macos/linux, traces oldfirehose, cites actual API/dependency lines and canonical names. Justified local package renames mux→term-mux and proto→term-proto for present terminal aggregates; retain generic render/harness-host/wake-up; Book standalone package staysbboxy450-book and binbboxybook must becomebboxy450-book. Preserve #39 reusable suite intent and Rust aliases/directories/wire bytes; no generic-kit extraction or implementation done. Exact future manifest/lock/Make/script/README/fixture/package checks at164, ordered release and compatibility limits193/203. Verified remote main6641b5b7 at21:17:43Z, manifests unchanged from own014e-based code; inspected remote plugin/supervisor deltas, unlanded cohorts not credited. Credential-free primary crates API36 HTTP responses at21:19:22–25Z:24 published0.1.0 unyanked names (23 current+oldfirehose),12 API404 including4 current-local-only names+2 absent platform proposals+6 candidate names; no checked0.1.1 publication. Current27 manifests0.1.1/customLICENSE distinct from public0.1.0 MIT/Apache metadata. Public GitHub repo API200/privatefalse at21:21:07Z. Primary dependency API at21:25:32Z marks oldrender→oldfirehose as dev;69''s fresh-install-break causal claim not proven by that edge; required migration-before-yank policy retained, oldterm normal oldmux/proto requirements need compatibility/retention evidence. No owners/tokens/tarballs/install/publication inferred. Exact signed5d8be221 make test-licenses0 (6 fixture+9 notice tests,27 crates,notice changes0), target/issue70-licenses-final.log SHA256ce9111ee9b7d9400df93149ab92b0cf887cf6aea1507f1d1ae4a1e6b0fc2c255. Original frontmatter exceptupdated/body/license-origin-map/links,29-row inventory/public metadata hashes and27 attribution/notice/license copy checks0, target/issue70-inventory-final.log SHA25688e953650789d9a0fd48bd2437e3856ce6549ee75494460751f86c4185edd3ed; signature/diff/exacttwo-path recordhash0. Initial link check1 caught literal Cargo binary-table marker, corrected; retained actual result in summary. Public metadata summary outsideGit SHA256b31d65b4824313a3e4225ce691e4d8512cf613c3019bc6c08c683183520571ec. Artifact01M64S6AQZ1Z1K3YK0KPEEKXWY reported grade, record SHA256d666ebf9bb780b0c87951842a262eae5819764f425fc71d63b7b186e4f572b5a. Full gate not admitted: free15172837376<required19071156224 bytes,no debug target; historical failed broad law gates do not verify this source. Rename/package/registry/install/release acceptance remains unproven. No code,Cargo,lock,Make,script,lifecycle,credentials,live runtime,peer cleanup,recruitment,merge/push or closure.'
role: architect
status: accepted
title: 'Issue70: classify published and current crate names from their real boundaries'
type: task
updated: 2026-10-29T21:47:27.236Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
verify: make test-licenses
verify_result:
  command: make test-licenses
  evidence:
  - criterion: Every issue70 baseline package and current workspace package has one source-backed generic/app-specific/standalone-app classification and canonical name under decision362.
    reference: .bboxy/specs/licensing.md:72 — pinned code/public evidence and29-row table (27 current members+2 absent proposed platform packages,old firehose alias traced); :155 — six proposed candidates judged from actual API/dependency boundaries; :164 exact canonical rename/alias/bin paths.
  - criterion: The existing licensing inventory preserves origin/license notices and distinguishes published baseline, current local manifests, added/removed packages and unverified publication state.
    reference: .bboxy/specs/licensing.md:23/40 — original adoption/license/origin map preserved; :88 onward — primary API URLs/time distinguish24 published0.1.0 unyanked names from local0.1.1,3 added desktop packages,renamed unpublished firehose and2 absent keychain packages; platform Unsupported states explicit.
  - criterion: Exact proposed rename/dependency/Make verification scope and release ordering are recorded without manifest edits, publication, yank or unproven issue closure.
    reference: .bboxy/specs/licensing.md:164 — package-only mux/proto renames, retained Rust aliases/directories,Book binary correction and exact Make/script/test/package fixture scope; :193/203 — required newfirehose/dependent-before-yank ordering,dev-dependency causal limit and old-name registry compatibility remain unproven.No authority to release/yank/close.
  - criterion: Preserved numbered record identity, signed scoped source, actual license/integrity checks and remaining uncertainties are reported; no cold broad rerun.
    reference: .bboxy/runs/issue-70-crate-classification.json — source/command/exit/hash/public-status/capacity summaries; exact5d8be221 license0, inventory/frontmatter/old-body/links/attribution0,signature0,diff0,two-path clean scope. Current raw focused log hashes in report; no cold broad gate.
  exit_code: 0
---
# Crate names reflect actual app boundaries
ROLE: ARCHITECT — Nyx Copperwing. Oracle accepted467 at signedbd1439b7 after real source/log/signature review; native records design is ready for future Executor adoption, not implemented. Reuse your one writing worktree /Users/feral/mydev/bboxy450-term-mcp-recovery-spec-20261029 and branch. This is existing issue70 preparation while the Executors finish73/49 and74/43/77.

Read GitHub70, decision362 .bboxy/decisions/d362-crate-names-app-then-package.md, decision39 suite-is-product, current .bboxy/specs/licensing.md and actual public APIs/dependencies/manifests. Verify remote main at read time (last6641); your014e-based source is not automatically current. The issue's24-crate baseline differs from the current27 workspace. Include missing/renamed baseline platform packages honestly; never infer publication or implementation from a directory name.

Amend ONLY the existing licensing record with a complete compact classification: current package, generic versus this app versus a standalone named app, file:line API/dependency reason, canonical name, publication/source status, required local rename. Check the suggested six term-specific renames against actual coupling; don't rubber-stamp the proposal or rename reusable suite libraries. Keep origin/license attribution and old names/links traceable. Public crates.io/GitHub metadata may be read without credentials; report primary-source URL/time, distinguish public verification from a local version. No token lookup/login, owner changes, install or publication.

Name exact future Executor paths/Make checks for justified local renames, dependency aliases, fixtures/packages. Release ordering stays explicit: new firehose/dependents before any old-name yank; this grants no release/yank authority. Verify-fixes-before-closing stands: classification prepares70/69, not proof their release/rename acceptance is met.

Use make test-licenses plus frontmatter/inventory/attribution/diff integrity checks. Report exact signed source/log hashes and unresolved evidence promptly. No product code, Cargo/lock/Make/script changes, new process/service, cold broad build at insufficient capacity, lifecycle edits, peer cleanup, credentials, live runtime, recruitment, merge/push or closure. Scope gaps go to Oracle with exact file/decision; independent classification continues.

## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Signed5d8be221 in assigned writing worktree, clean; only licensing151 and allowed issue70 JSON changed. Usable signed draft4bdfa681 delivered promptly. licensing.md72–216 classifies29 rows covering27 current packages+2 absent proposed keychain-macos/linux, traces oldfirehose, cites actual API/dependency lines and canonical names. Justified local package renames mux→term-mux and proto→term-proto for present terminal aggregates; retain generic render/harness-host/wake-up; Book standalone package staysbboxy450-book and binbboxybook must becomebboxy450-book. Preserve #39 reusable suite intent and Rust aliases/directories/wire bytes; no generic-kit extraction or implementation done. Exact future manifest/lock/Make/script/README/fixture/package checks at164, ordered release and compatibility limits193/203. Verified remote main6641b5b7 at21:17:43Z, manifests unchanged from own014e-based code; inspected remote plugin/supervisor deltas, unlanded cohorts not credited. Credential-free primary crates API36 HTTP responses at21:19:22–25Z:24 published0.1.0 unyanked names (23 current+oldfirehose),12 API404 including4 current-local-only names+2 absent platform proposals+6 candidate names; no checked0.1.1 publication. Current27 manifests0.1.1/customLICENSE distinct from public0.1.0 MIT/Apache metadata. Public GitHub repo API200/privatefalse at21:21:07Z. Primary dependency API at21:25:32Z marks oldrender→oldfirehose as dev;69's fresh-install-break causal claim not proven by that edge; required migration-before-yank policy retained, oldterm normal oldmux/proto requirements need compatibility/retention evidence. No owners/tokens/tarballs/install/publication inferred. Exact signed5d8be221 make test-licenses0 (6 fixture+9 notice tests,27 crates,notice changes0), target/issue70-licenses-final.log SHA256ce9111ee9b7d9400df93149ab92b0cf887cf6aea1507f1d1ae4a1e6b0fc2c255. Original frontmatter exceptupdated/body/license-origin-map/links,29-row inventory/public metadata hashes and27 attribution/notice/license copy checks0, target/issue70-inventory-final.log SHA25688e953650789d9a0fd48bd2437e3856ce6549ee75494460751f86c4185edd3ed; signature/diff/exacttwo-path recordhash0. Initial link check1 caught literal Cargo binary-table marker, corrected; retained actual result in summary. Public metadata summary outsideGit SHA256b31d65b4824313a3e4225ce691e4d8512cf613c3019bc6c08c683183520571ec. Artifact01M64S6AQZ1Z1K3YK0KPEEKXWY reported grade, record SHA256d666ebf9bb780b0c87951842a262eae5819764f425fc71d63b7b186e4f572b5a. Full gate not admitted: free15172837376<required19071156224 bytes,no debug target; historical failed broad law gates do not verify this source. Rename/package/registry/install/release acceptance remains unproven. No code,Cargo,lock,Make,script,lifecycle,credentials,live runtime,peer cleanup,recruitment,merge/push or closure.

```json
{
  "commit": "5d8be221d43d516c523d82e1e3721da86b78e224",
  "op_id": "01M64S7KEGKD8X77CQ4NV2N5TC",
  "verify_result": {
    "command": "make test-licenses",
    "evidence": [
      {
        "criterion": "Every issue70 baseline package and current workspace package has one source-backed generic/app-specific/standalone-app classification and canonical name under decision362.",
        "reference": ".bboxy/specs/licensing.md:72 — pinned code/public evidence and29-row table (27 current members+2 absent proposed platform packages,old firehose alias traced); :155 — six proposed candidates judged from actual API/dependency boundaries; :164 exact canonical rename/alias/bin paths."
      },
      {
        "criterion": "The existing licensing inventory preserves origin/license notices and distinguishes published baseline, current local manifests, added/removed packages and unverified publication state.",
        "reference": ".bboxy/specs/licensing.md:23/40 — original adoption/license/origin map preserved; :88 onward — primary API URLs/time distinguish24 published0.1.0 unyanked names from local0.1.1,3 added desktop packages,renamed unpublished firehose and2 absent keychain packages; platform Unsupported states explicit."
      },
      {
        "criterion": "Exact proposed rename/dependency/Make verification scope and release ordering are recorded without manifest edits, publication, yank or unproven issue closure.",
        "reference": ".bboxy/specs/licensing.md:164 — package-only mux/proto renames, retained Rust aliases/directories,Book binary correction and exact Make/script/test/package fixture scope; :193/203 — required newfirehose/dependent-before-yank ordering,dev-dependency causal limit and old-name registry compatibility remain unproven.No authority to release/yank/close."
      },
      {
        "criterion": "Preserved numbered record identity, signed scoped source, actual license/integrity checks and remaining uncertainties are reported; no cold broad rerun.",
        "reference": ".bboxy/runs/issue-70-crate-classification.json — source/command/exit/hash/public-status/capacity summaries; exact5d8be221 license0, inventory/frontmatter/old-body/links/attribution0,signature0,diff0,two-path clean scope. Current raw focused log hashes in report; no cold broad gate."
      }
    ],
    "exit_code": 0
  }
}
```
