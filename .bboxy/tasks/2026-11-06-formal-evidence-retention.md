---
allowed_paths:
- scripts/formal.py
- scripts/land.sh
- scripts/landing.py
- scripts/test-landing.py
- scripts/test-formal.py
- Makefile
- LAND.md
- verification/README.md
- .bboxy/reports/
assignee: seat:01M6SEB3WJBTKYM886SJSSY7AE
authorship:
  kind: seat
  process_generation: '1'
  public_key: qxL/3a47UEqef+6VNGve7zFPxAJ4V2xtFwav0ldZ+l0=
  signer: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
closed:
  by: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
  op_id: 01M85CFF1VFBMB64TQBX0WQ7RM
created: 2026-11-06T19:10:15.571Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
id: 01M6S49JCH1GSW2SM7GNE9F6FF
number: 93377961366090
report:
  commit: 6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a
  op_id: 01M6SH6VXKF878J3FNQVJ3N0N2
  report: '[[reports/2026-11-06-formal-evidence-retention.md]]'
  reported_by: seat:01M6SEB3WJBTKYM886SJSSY7AE
  summary: 'Landed signed 6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a: compressed content-addressed graphs, live/pending-report retention, verified monthly tar.zst archives and budget-triggered background pruning. Actual retained output shrank 2,130,747,831 to 125,392,112 bytes (16.99×; 17.10× with inode dedupe). Exact assigned verification exited 0: 181 landing fixtures plus current-source evidence check; lean land passed Clippy, 447 native tests (12 skipped), and 570 native/TLC models across 34 crates/212 contracts. Compressed-store checker and 23 formal infrastructure tests also passed. Worktree clean; committed full report contains per-criterion evidence, receipt/log hashes and coverage limits.'
  verify_revision: 0
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Retention policy and make target for .git/bboxy-formal-evidence (22 GiB, 176 receipts, no pruning)
type: task
updated: 2026-11-23T23:39:52.550Z
updated_by: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
verify: python3 -I scripts/test-landing.py && make check-formal-evidence
verify_result:
  command: python3 -I scripts/test-landing.py && make check-formal-evidence
  evidence:
  - criterion: Live set and newest-N preservation
    reference: scripts/formal.py:495; scripts/test-landing.py:881
  - criterion: Verified archives, corruption refusal and DRY_RUN
    reference: scripts/formal.py:549; scripts/formal.py:581; scripts/formal.py:660; scripts/test-landing.py:938; scripts/test-landing.py:949; scripts/test-landing.py:957
  - criterion: Automatic background QoS above budget
    reference: Makefile:822; scripts/landing.py:1818; scripts/formal.py:704
  - criterion: Compressed, deduplicated graphs and decoded hash checking
    reference: scripts/formal.py:399; scripts/formal.py:324; scripts/test-formal.py:360; scripts/test-landing.py:967; target/formal-retention-summary.json (16.99×/17.10×)
  - criterion: Source-bound checks remain valid after prune
    reference: Formal receipt 3c10919b4c774134cab758c68a5479beea82b88690cf7ab29c19b115c56af618; make check-formal-evidence and compressed-store Make check exited 0
  - criterion: Signed lean landing
    reference: target/formal-retention-land.log SHA256 2a6c78c0b50252e9996e48d3f95f5cf81686511df839daf1c509379a06566e56; LANDED 6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a; git verify-commit HEAD exit 0
  exit_code: 0
provenance:
  digest: sha256:5b3013780a4264e03dd35e441b92627bb77ef80783dc44278859ae6d4f37db57
  frontmatter_digest: sha256:38ab9695a3e6fae3d8468592f99f8970479e8c32f0a8ee4ee238044a6af47c22
  signer: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
  sig: GNSQR4mzp3xCYOlG8Ks2X4XovFbKFdhdcHPZvasYK7BHVMlFz92YbWhnkjTQr+5GGg38e+HoMAvKWMT2OYw+AQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1416
  event:
    id: 01M85CFFZBDQQ7SMDHWK2444YG
    sequence: 1416
    hash: 17f84d3b7089b9d816b30792ae9e681ff3b4016ecd50c4654f00c78f9bd3dd12
---
# Formal evidence grows without bound

`bboxy450-term/.git/bboxy-formal-evidence` holds **22 GiB in 176 receipt directories** (Oracle `du`, 2026-11-06). Nothing ever prunes them, and the disk was down to 362 MiB free today.

## Required

1. **Live set.** Define it: the receipts that `make check-formal-evidence` and `land.sh` read for current `origin/main` and each live worktree HEAD, plus receipts cited by unaccepted task reports.
2. **`make prune-formal-evidence`.** Keep the live set and the newest N (default 10). Losslessly archive the rest as one `tar.zst` per month with a SHA-256 manifest, verify the archive (integrity test plus file count), then delete the originals. Support `DRY_RUN=1`. Never delete without a verified archive.
3. **Run it automatically.** After a successful `make land`, run it at background QoS when the evidence directory exceeds a budget (default 5 GiB).
4. **Tests.** The landing tests cover: live receipts are kept, cold receipts are archived and removed, a corrupt archive keeps the originals, and DRY_RUN changes nothing. `check-formal-evidence` still passes after a prune.

Lean landing; write the report `reports/2026-11-06-formal-evidence-retention.md` and call `task_report` once.

## Oracle amend (19:45, URGENT): the growth is per-receipt size, not just count
`.git/bboxy-formal-evidence` grew from 22 GiB to **39 GiB in about 90 minutes** on 2026-11-06. Today's receipts are **6.9 GiB each** (4c239f0d, 03538af1, d9e1192c, 2b91b751) and 2 GiB each (0fc6016c, 5dbb230b), while Nov 4 receipts were about 260 MiB. So every formal run on the full corpus writes about 7 GiB of uncompressed model graphs.
1. **Store compressed** (zstd per file or per receipt), and have the checker read through decompression. The receipt hash binds the uncompressed bytes, so decompress-then-hash keeps it valid. Target: ≥10× smaller.
2. **Dedupe:** identical model graphs across receipts (unchanged models between runs) are stored once, content-addressed (the `by-model/` store already exists, holding 5459 entries in 21 MiB; extend it to graphs).
3. The live set and archiving rules above still apply. The Oracle has already losslessly archived the 155 pre-Nov-4 receipts to `.git/bboxy-formal-evidence-archive/receipts-before-2026-11-04.tar.zst` (with list and SHA256SUMS).
Priority: start right after the C1 landing frees disk.


## Report from seat:01M6SEB3WJBTKYM886SJSSY7AE

Landed signed 6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a: compressed content-addressed graphs, live/pending-report retention, verified monthly tar.zst archives and budget-triggered background pruning. Actual retained output shrank 2,130,747,831 to 125,392,112 bytes (16.99×; 17.10× with inode dedupe). Exact assigned verification exited 0: 181 landing fixtures plus current-source evidence check; lean land passed Clippy, 447 native tests (12 skipped), and 570 native/TLC models across 34 crates/212 contracts. Compressed-store checker and 23 formal infrastructure tests also passed. Worktree clean; committed full report contains per-criterion evidence, receipt/log hashes and coverage limits.

```json
{
  "op_id": "01M6SH6VXKF878J3FNQVJ3N0N2",
  "commit": "6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a",
  "report": "[[reports/2026-11-06-formal-evidence-retention.md]]",
  "verify_result": {
    "command": "python3 -I scripts/test-landing.py && make check-formal-evidence",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Live set and newest-N preservation",
        "reference": "scripts/formal.py:495; scripts/test-landing.py:881"
      },
      {
        "criterion": "Verified archives, corruption refusal and DRY_RUN",
        "reference": "scripts/formal.py:549; scripts/formal.py:581; scripts/formal.py:660; scripts/test-landing.py:938; scripts/test-landing.py:949; scripts/test-landing.py:957"
      },
      {
        "criterion": "Automatic background QoS above budget",
        "reference": "Makefile:822; scripts/landing.py:1818; scripts/formal.py:704"
      },
      {
        "criterion": "Compressed, deduplicated graphs and decoded hash checking",
        "reference": "scripts/formal.py:399; scripts/formal.py:324; scripts/test-formal.py:360; scripts/test-landing.py:967; target/formal-retention-summary.json (16.99×/17.10×)"
      },
      {
        "criterion": "Source-bound checks remain valid after prune",
        "reference": "Formal receipt 3c10919b4c774134cab758c68a5479beea82b88690cf7ab29c19b115c56af618; make check-formal-evidence and compressed-store Make check exited 0"
      },
      {
        "criterion": "Signed lean landing",
        "reference": "target/formal-retention-land.log SHA256 2a6c78c0b50252e9996e48d3f95f5cf81686511df839daf1c509379a06566e56; LANDED 6df528c6d16b2f8c4b8fd9666f1b26c06a3cd51a; git verify-commit HEAD exit 0"
      }
    ]
  }
}
```
