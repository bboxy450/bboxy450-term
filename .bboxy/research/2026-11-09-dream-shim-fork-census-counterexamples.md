---
type: research
title: Darwin NOTE_FORK coalescing and orphan escape contradict shim gate assumptions
derived_from:
- '[[decisions/d93377961366459-dream-external-helper-shim-enforcement]]'
- '[[tasks/2026-11-09-dream-exec-shim-enforcement]]'
id: 01M70FT04KSBM9YMXMNET7EZZ3
updated: 2026-11-09T15:46:21.808Z
created: 2026-11-09T15:46:09.261Z
number: 93377961366461
created_by: seat:01M70FJGXV7W40W80200Z0TDN5
updated_by: seat:01M70FJGXV7W40W80200Z0TDN5
authorship:
  signer: seat:01M70FJGXV7W40W80200Z0TDN5
  kind: seat
  public_key: ZqTue5OR8Q7BNcheRMErqTQtqa+AL9dxIChIyiTZDGM=
  process_generation: '1'
provenance:
  digest: sha256:cb6d79b3f9d84f1b22be02c6be2c3052fa4a81425a666186e490f6b40a72446e
  frontmatter_digest: sha256:eb7c123a137ddf9bc2281e16e1dca1c318d2c3cf8ff864cda78b20a1106bb1e0
  signer: seat:01M70FJGXV7W40W80200Z0TDN5
  sig: cPbVgNGzo6nZI+LDhhUSBZOM0ZUm/cMrjT1EW/uiH0B2T6eK+8fj7dUz9E0vPojZT32QPO8m3ooV9m1c93//Dw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151028
  event:
    id: 01M70FTD4AVSKPWS3SSYEJ1F9Z
    sequence: 151028
    hash: 802543c9db9a68545be3ee31f5448bf7d1c53ea3bd9e6914e70de9caab410b33
---
## Finding and required decision

The assigned 66459 mechanism cannot establish an actual one-fork bound by counting leader NOTE_FORK receipts, or establish absence of an undiscovered escaped orphan by leader-child/group census. These are measured Darwin API counterexamples, not product exploitation or a claim that the pinned Git/GPG currently do this. The decision needs an enforceable fork/escape mechanism, or an explicit supported-tool assumption and revised acceptance evidence. Worker scope does not permit choosing the assurance change.

## Reproduction and retained evidence

Worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2. Clean tracked HEAD 637cb234d93d08d0c8fcfb820b7e48f9538bc0e1 after git fetch origin and git rebase origin/main (up to date). Darwin 27.0.0 arm64 / xnu-13432.1.9~1.

Evidence directory: target/dream-exec-shim-decision-witness/.

- witness.c: SHA256 0c7335e5229c346480460c5484a37d4925c7430780c75a19dad14dc0e9d8e062.
- witness: SHA256 b53a77d487f022cce36290a07dcd753dfcaa26e00bfb393e0a60e4f6daeddbd4.
- result.jsonl: SHA256 24991fd56687af8ba0f1cca96f7fc8e9fe9e9590fba54f1acc9ab2004ccc703f.
- manifest.json: exact argv, exit statuses, stdout/stderr, timestamp and hashes.

Commands (actual subprocess return codes captured in manifest.json):

1. /usr/bin/clang -O2 -std=c11 -Wall -Wextra -Werror target/dream-exec-shim-decision-witness/witness.c -o target/dream-exec-shim-decision-witness/witness — exit 0.
2. target/dream-exec-shim-decision-witness/witness — exit 0; both causal assertions held; cleanup of the escaped owned child was confirmed by absence/zombie observation after its dedicated cleanup pipe was signaled.

An intermediate added cleanup assertion initially failed compilation because SZOMB was undeclared (exit 1); adding sys/proc.h corrected it. No failing product gate was retried or bypassed.

## Actual results

```json
{"case":"coalesced_forks","actual_leader_forks":2,"NOTE_FORK_receipts":1,"fflags":1073741824,"data":0,"child_census":0,"census_errno":0,"wait4_cpu_ns":1153000,"asserted":true}
{"case":"escaped_orphan","actual_leader_forks":1,"NOTE_FORK_receipts":1,"fflags":3221225472,"data":0,"child_census":0,"census_errno":0,"group_before_reap":1,"group_before_errno":0,"group_after_reap":0,"group_after_errno":0,"escaped_live_after_reap":1,"escaped_ppid":1,"escaped_pgid":77642,"leader_wait4_cpu_ns":500000,"asserted":true}
```

## Mechanism and source mapping

- witness.c:25 — registers EVFILT_PROC EV_ADD|EV_ENABLE|EV_CLEAR with NOTE_FORK|NOTE_EXIT while the leader is positively stopped.
- witness.c:45 — leader forks and reaps TWO children before controller drains its knote. Controller gets ONE NOTE_FORK flag, data=0, no current children. Thus adding a >1 receipt gate to proof-helper-probe.c:2982 does not establish a >1 actual-fork gate.
- witness.c:58 — a single child calls setsid; its leader then exits. Before consuming wait4 the original leader is still pinned by WNOWAIT, but its child census is empty and only its zombie remains in the original PGID. After reap the group census is empty while the child is alive with PPID=1 and its own PGID.
- witness.c:65 — the evidence-only PID side channel independently proves the escaped child is alive. Product census does not receive this PID; it is not evidence that production could discover the orphan.
- proof-helper-probe.c:66 and :69 — native_child_count uses proc_listchildpids, native_group_count uses proc_listpgrppids, the exact APIs tested.
- proof-helper-probe.c:2984 — leader_fork_events currently increments per delivered NOTE_FORK, rather than per actual fork.
- proof-helper-probe.c:4483 — engine_cleanup_escaped iterates already retained identities; it cannot recover a child that was never discovered.
- proof-helper-probe.c:4497 — engine_original_group_gone checks retained identities plus current original group membership.
- Local SDK sys/event.h:253 — explicitly documents that a fork hint's child PID is NOT passed through in the delivered kevent.
- Local SDK sys/event.h:357 — NOTE_TRACK/NOTE_TRACKERR/NOTE_CHILD unsupported since 10.5.

## Limits and untouched work

The witness controls scheduling to exhibit a permitted delayed-controller interleaving; it does not claim this scheduling happens on every run, or that pinned Git/GPG violate their own supported behavior. It establishes that the proposed OS gates alone cannot prove the stated guarantees.

No product, model, ledger or terminal field changed; no code commit, INPUT70, builder run, make land or acceptance claim. The required full verification command was not run because the contract mechanism needs disposition before implementation. runtime_approved=false. All prior target evidence is retained.