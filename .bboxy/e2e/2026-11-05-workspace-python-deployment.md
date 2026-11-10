---
type: e2e
title: Managed workspace and Python MCP main installation handoff
function: Managed workspace/Python MCP installation and retained-session GUI refresh
group: deployment
status: passed
coverage: full for this installation; current pixels occluded
last_run: 2026-11-06T05:11:11.148567+00:00
expected: Both latest signed apps, ready current-source GUI/logic and retained supervisor/seat identities
result: Passed source/signature/installed hash/readiness/owner checks; capture graded stale/occluded
run_by: operator
evidence:
- /Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/installed-verification.json
- /Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/owner-preservation.json
- /Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/startup-recovery-diagnosis.json
id: 01M6QM96HM2EGEZF1YKP1XHC8S
updated: 2026-11-06T05:16:03.885Z
created: 2026-11-06T05:11:11.480Z
number: 93377961364437
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
updated_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
authorship:
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  kind: local-dev
  principal: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  device_id: 01M62F1BBJT8MW0F3YMZJ8BENM
  public_key: a8bbsPrJyl7hZraHSoUBkc76MQiRVYXx/kgj6IWVFt0=
provenance:
  digest: sha256:7b234f7fd7d7f86e3e30d226eb5d302c6305c72acc78e5b441881f18fe6c4b78
  frontmatter_digest: sha256:23f73d1c0e920a0e62fe2738e65438ac60873c8a52a298759740905435737de1
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: ogfaHFmtvts03D06nUQ7l2bGr6GAjG3C+ucaxe6UKyGBapa2frwe75vcAGRtzIWNCADVfuGu4meHb3yHu25mAg==
  log: bboxy://activity/index/384191
  event:
    id: 01M6QMJ4FCBH5VP1GAVKXE4NMY
    sequence: 384191
    hash: b20e788526e6e8ef4364106533f2ea768c1d4908b43805c98b73b1350621ddbd
---
# Managed workspace and Python MCP installation handoff

Term main is `13933a9410322b25c72466bdc65284fc631eb69e`; its harness gitlink and standalone harness main are `a75eabfa4c19ab1af2cd192809d75fe510dff1e3`. Both canonical checkouts are clean and both remote refs and approved source signatures were independently verified.

The workspace MCP lifecycle and Python runtime are installed. Workspaces retain separate source/KB bindings, bounded provider IO, guarded writing generations, recovery and preservation checks. Python uses the admitted current workspace and harness MCP/command bindings. Remote workspace providers remain unavailable until implemented; no cloud provider is claimed.

## Verification

- Combined term `make land`: passed; 547 models, 203 contracts, 34 crates. Receipt `51eb2cbf25dc4638c8d1c21f91bad47edc4ef453b87ea6c851f9c086fce4182c`.
- Harness `make land`: passed; 270 models, 75 contracts. Receipt `1ee5b2a045ffe2e0e6119342a3686e12b1f261be048c82bf10a2a877aca45306`.
- Final terminal diagnostic: all 447 tests passed with no leak classification; the earlier two leak observations remain in the producer's logs.
- Deployment independently rechecked source-bound receipts, signatures and exact installed/prepared executable hashes. No broad suites or models were rerun during installation.

Both apps are installed in `~/Applications`, signed with the exact prior installed term leaf `B7B799761F58D5315C2D19162A44550E1677D325`, Team `UQB3368A84`, using canonical Make overrides. The stale default Development identity was unavailable; the existing trusted Distribution identity was explicitly authorized and verified. The designated requirements were unchanged.

## Live state and recovery

The first canonical GUI handoff timed out waiting for its pipe line and restored the old UI. That failed attempt is retained. Both old and candidate native product captures reported `occluded=true`; this explains why the first pre-live frame did not by itself establish the later readiness line.

The current signed GUI is PID `57560`, source `13933a94...`. Native `ctl app status` reports GUI, backend and events ready. The actual startup readiness line later reported a post-live presented frame `107`; the product capture retained frame `170` but was explicitly graded stale/occluded, with no claim of fresh current pixels. Normal product CLI replacement used the freshly probed exact prior owner; no timeout, source or readiness predicate was weakened.

Supervisor PID `80176`, birth `[1793887644,471579]`, is exactly preserved. Fresh compatible logic is PID `57760`, birth `[1793941432,119763]`; the current-source compatibility probe matches MCP control version 21. All 30 seat IDs, swarms and generations remain present. Nineteen previously retained closed seats stay closed; one already-closed grace entry finalized to retained. No seat was restored, resumed or sent user work.

The operator-owned unassigned workspace and Python tasks were historically reconciled as implemented through `ctl task reconcile`, with concrete main/formal/installation evidence. This is `done`, not a fabricated seat report or fresh task acceptance.

## Evidence and remaining limits

Detailed command outputs, signed source receipts, public owner/seat snapshots and installed hashes are local at `/Users/feral/.local/state/bboxy450-term/python-execute-20261105/deployment/`. Producer feature verification remains at its parent directory; workspace implementation evidence is at `/Users/feral/.local/state/bboxy450-term/workspace-provisioning-20261105/`.

The aggregate formal receipt includes two ignored generated harness evidence archives. Exact hash-verified APFS clones were retained in the canonical checkout so independent receipt verification and later producer cleanup preserve those inputs. There was no tracked source mismatch.

Mapped installation backups and the shared build target are retained. Both released implementation worktrees were removed after main-ancestry/cherry checks, clean source checks and fresh negative open-process checks. Git required its initialized-submodule force option; only verified committed source and owned generated outputs were removed. Shared target inode, exact canonical formal archives, mapped installation backups and every live session were retained. Worktree prune reported no unrelated stale metadata. Live sessions remain for the operator to resume manually.
