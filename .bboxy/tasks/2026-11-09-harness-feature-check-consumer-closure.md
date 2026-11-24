---
allowed_paths:
- scripts/harness-submodule.py
- scripts/test-harness-submodule.py
- .bboxy/reports/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-10T03:00:23.652Z
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M71PCJGMT10KBHEN361CCQKV
number: 1794274420276163
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: implemented
  evidence:
  - commit 97c1a4d87 — scripts/harness-submodule.py:88-102
  fresh_accepted: false
  kind: historical
  op_id: 01M878KJMKKSY7PD6EGQ1BDZ5G
  previous_status: open
  reason: Feature check reads the consumer closure.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Harness feature isolation check reads the term consumer's feature closure, not the workspace union
type: task
updated: 2026-11-24T17:10:42.025Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: python3 -I scripts/test-harness-submodule.py && python3 -I scripts/harness-submodule.py --check-json-features && ! python3 -I scripts/harness-submodule.py --check-json-features --acp-control
provenance:
  digest: sha256:78376255b2007cc13b596513a7d4647edbd822146a9476cb7e951f15beda0887
  frontmatter_digest: sha256:6733fcef9da838e7dedc54e2007d5875ef2d0c7e46958960ee32535937699bed
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: f5LY+RDoHXa+sJus09ZqGt5I21xGuUbC/R3tVWdUOpfCBzP1N5ZNLA7eqaxAtrihyNfD4DeFhM7+rvLJHxv1CQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1887
  event:
    id: 01M878KKRB0BKNVMAHKE3EZMXM
    sequence: 1887
    hash: 031be70d35c7e74d2aa397d5e11251ed3fabf63b58481ddddaea97676ea7e49b
---
ROLE: EXECUTOR

# Goal

Unblock landing on `main`. Your diagnosis is confirmed by the Oracle:

- `a58a0fcfb` pins a harness whose `Cargo.toml` has `default = ["acp-agent", "acp"]`.
- The harness is a workspace member, so workspace-wide `cargo metadata` reports its
  features as `acp, acp-agent, default, embedded`.
- `scripts/harness-submodule.py` `validate_json_features` rejects `acp` on that node.
- The only consumer, `crates/bboxy450-term/Cargo.toml:21`, selects
  `default-features = false, features = ["embedded"]`.

The rule the check exists for is: **the harness as linked into `bboxy450-term` must not
have the legacy `acp` frontend; `serde_json/preserve_order` is allowed only with
`acp-agent`.** The check must read the features the consumer selects, not the union over
the workspace. The rule is not weakened.

Work in your current worktree and branch, as a separate signed commit placed with
`fd38ec4c0`. This task is authorization for exactly the two script paths.

# Do it in this order

1. In `scripts/harness-submodule.py`, change how `check_json_features` obtains the
   feature sets: resolve for the consumer package only. Suggested mechanism:
   `cargo tree -p bboxy450-term --locked -e normal,build --prefix none -f '{p}|{f}'`,
   reading the feature lists on the `bboxy450-harness` and `serde_json` lines; pass the
   negative control's extra feature with `--features bboxy450-harness/acp`. If cargo's
   output makes another mechanism more reliable, use it and say why. Keep
   `validate_json_features` a pure function over data so its tests stay unit tests.
2. Keep both existing rules and the `--acp-control` negative control: with the control
   flag the check must still fail, naming `bboxy450-harness/acp`.
3. In `scripts/test-harness-submodule.py`, add cases: (a) workspace default includes
   `acp` but the consumer selects only `embedded` → passes; (b) the consumer's closure
   includes `acp` → rejected; (c) `preserve_order` without `acp-agent` in the consumer's
   closure → rejected. Keep every existing case; adapt its input shape only if step 1
   changed the shape.
4. Run the verify command. Then `make land` for both commits.

# Acceptance

- [ ] The verify command exits 0 (its last clause proves the negative control still fails).
- [ ] `make land` gets past `make init-submodules`.
- [ ] No rule removed; no path outside the two scripts changed by this commit.

# Stop only if

The consumer's real closure contains `acp`. That would be a true violation of the rule:
report it and do not change the rule.

# Report

One `task_report` per task: this one with `reports/2026-11-09-harness-feature-check-consumer-closure.md`
(mechanism, commands with exit status, the before and after output of the check), and
your existing RemoteDelivery report updated with the landed commit.
