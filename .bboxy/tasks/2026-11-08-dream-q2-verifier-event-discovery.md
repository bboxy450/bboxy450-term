---
type: task
title: Q2 event-driven verifier discovery (kqueue NOTE_FORK) for GS1 custody, hand off INPUT62
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- tests/fixtures/dream-read/proof-helper-probe.c
- tests/fixtures/dream-read/private-engine-controls.c
- tests/fixtures/dream-read/check-private-engine-controls.py
- tests/fixtures/dream-read/prepare-c-probe.py
- tests/fixtures/dream-read/check-c-preparation.py
- tests/formal/dream_read.rs
- tests/formal/dream_c1.rs
- tests/formal/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-08-dream-verifier-event-discovery.md
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[tasks/2026-11-08-dream-q2-live-sample-esrch-race]]'
id: 01M6X9N6ZW1BT2K65JJ4265RS8
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T11:34:08.197Z
created: 2026-11-08T10:00:57.503Z
number: 93377961366356
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Leader watch before controlled continuation; bounded event wait; immediate strict census and child exec/exit watch; original queue custody and charged close
    reference: reports/2026-11-08-dream-verifier-event-discovery.md; proof-helper-probe.c SHA01e8578163768f9d463f70f1a1cc698a8f073e0ea91f911a7dd8c51465a5496d at2516/2552/2895/2919/3232/3286/3409; current C receipt677aab9ad9df97573aec9a29e0ddc09c10c5da7b3b6989a281b5eed35eab1eda,83/54/56/125 actual0
  - criterion: Real actual-between-census fork/exit witness; physical poll-only mutant; no-fork and failed registration controls, existing signature cases
    reference: Canonical actual child lifetime1404375ns with raw RI/timebase/PID/poll proof; current physical missing-tail5commands0 SHA1811573eb7f0395db973f04a4388d3be076f3e89310b92a49f7fe12975eb0a43; C24 original outer1 logd1244ae68183a48d8b73eedcca5b47ed69b5f1b0ab0dec752699b379bc49b9ff preserved under native Task amendment162e70d5d6a7c4a3333fc7c078f926bdde044e4c1dd0039d6fe700ad0cac6bea
  - criterion: Exact four ledger fields/types and distinct refusals; no new terminal fields or law changes
    reference: Report field table and C25 freezece8673224c959523b3a6880f960ba323e126415007e46a87d76ae4d53860d2a8; ledger kindverifier_event_discovery; leader_fork_events:u64,leader_exec_events:u64,child_discovered_via:none|poll|fork_event,fork_seen_child_missed:bool; terminal_fields_changed=false
  - criterion: Finite event-discovery model and five broken controls; mandatory full current native/reference parity
    reference: Formal freezef4abb7fd8a3167024c02f2c2405fd63cdde96ad610ed75c0f522c646367a083d;440models94contracts3376317states11364544labellededges; archive20b1ab965ec30a14a9450dbb85c3a9264317f5d1500790e7aee285e9cb6e6473; rawmanifest2fa73050f9bcafa70445fe5c4f88e6558c48e39d60c8c847eda99c1ea33602a2
  - criterion: Exact required four-command string ONCE at final signed HEAD, actual0; canonical Land0 and remote containsHEAD
    reference: event-exact-four-chain-v1.log SHA4d6663040e21ce643de2e88c7c9a74494f374f64cbf3cfb0d6a2486a196fabf4; event-canonical-land-v1.log SHA5766ac876f82f010352d790b707b90d2328576bd2537e0bac28f3d5ddc1ef18d; fresh remote-verification-detail-v1.json and report
  - criterion: ONE INPUT62 exact signed source/receipt/executable/ten inputs/compiler/header audit, runtimefalse/no builder
    reference: Stored1448/id01M6XEN9XDZ52N5XSD6DK948M3; tupleed5a20501df0327296578e30095521815b7125c78edd3fc5dc91e633d073c09b; receipt889764ffa41be7bbf486be80afb49e74e5b58d9f81a522a1516609043468dd1d; executablebab19b967fed5adc1ac82f278bc5869657fc54242ef522c0e4fe897a6b09523c; audit207d4db89f1789fb2c69fef6ee5aea88ff67f60ee50413dae8e61875f99786a5 actual997checks325headers0 with sys/event.h pinned
  - criterion: Full new committed report with actual failures/amendment/limits preserved
    reference: 'reports/2026-11-08-dream-verifier-event-discovery.md #93377961366359; KBb2e71b1d5e20d4e6623a5e513f36bd8326bdc2ee;26unchanged historicalignorednonpasses; no narrow baseline timing retry; runtimefalse,toolqualification/wholeprivate30sfit unproven'
report:
  summary: Landed signed7ab80fac868b53b1dfc690ef774fc3bfabbb18b8 (including signed4967eb4378e3166aa7532630ee427b69d41a7dae) on main. Owned leader kqueue watch now precedes controlled continuation; fork notifications immediately service the existing census/current-token child watch/sampler, with exact missed-fork versus no-verifier refusals and charged queue custody through original settlement. Four typed ledger diagnostics added; terminal fields/caps/debt/GS1GS2 unchanged. Current canonical C83/54mutants/56compiles/125commands0, exact model test1pass, full440-model/94-contract native-Java graph/verdict parity0, exact required four-chain ONCE0, literal make land0 and fresh remote/clean/signatures0. C24 original1.676834ms measured-witness outer1 preserved; authenticated amendedTask162e70d5 explicitly accepts actual between-poll lifetime, no narrow baseline timing retry. INPUT62 stored1448 with signed10-input/compiler/997-check325-header audit tuple; report committed KBb2e71b1d5e20d4e6623a5e513f36bd8326bdc2ee. Runtimefalse/no builder;26 historical deferrals and tool/whole-private30s gaps remain unproven.
  commit: 7ab80fac868b53b1dfc690ef774fc3bfabbb18b8
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6XEZEVWFNXBJS0WS1CYMHM8
  verify_revision: 0
  report: '[[reports/2026-11-08-dream-verifier-event-discovery.md]]'
provenance:
  digest: sha256:b365e85216b7468246df12c5b40725c909a119c47c057c8d9f9cbfd143cc24cc
  frontmatter_digest: sha256:1edca19bdd663260038a8f18fcd4c05f3d4373c842d4cbdc57b5289693e42146
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: qrX6qYZk/ofJRhk58VwZlxUckQzi84W1yFhUOO18LS8ha3AGRDc9RvKlz+/R/dR5BPo2UrHx5d//7NbOfFh5Cw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150658
  event:
    id: 01M6XEZV61ZR3044EGHJGNAE7D
    sequence: 150658
    hash: d5cb98be1db66e28bb38ae1a220a1e6212bd128a5fb377e3a4ddd5277fc52e35
---
ROLE: EXECUTOR

## Evidence (builder run 01M6X8ZSXWJQAF6DZH8VE88G5F on frozen INPUT61 f6dffea0; Oracle read the raw stderr and ledger)
- Ledger sha256 b3239d0a...d5b4. Controller wait4 exit 81. 15/15 helpers were created and reaped, and the 30 s deadline was never observed.
- Final paid debt was 23.140558494 s: helper 1.349 s + ordinary raw 3.196 s + floors 18.596 s. That fits under 30 s at this stage.
- Errors: coverage_unknown "signature lacks exactly one retained verifier" (proof-helper-probe.c:3388, form 12), then settlement_unknown x2 "GS2 requires actual leader reap and the retained GS1 descriptor".
- The signature leader (seq 16, pid 73489) had wait4 CPU 23.306 ms (8.877 user + 14.427 system) and 5 sample checks, with a max gap of 23.068 ms.
- retained_child_accounting shows child_created=0, and the census child_count is 0.
- engine_sample (~2805) discovers members[1] ONLY by polling engine_list_children. A gpg verifier that git forks, execs and reaps between two samples is never seen. Its CPU still rolls into the leader's wait4, but GS1 custody (verifier argv/temp-file identity before it runs) cannot be established.
- From this evidence we cannot tell whether git never forked gpg or the fork was missed.

## Ruling (Oracle; stronger coverage, no relaxed law)
Add event-driven descendant discovery to the existing helper owner, keeping the existing sampler:
1. Register kqueue EVFILT_PROC on the original leader with NOTE_FORK | NOTE_EXEC | NOTE_EXIT (NOTE_EXITSTATUS if useful), at the attachment point after posix_spawn and before the leader's first CPU use that you control. Record the registration result; a failed registration is coverage_unknown.
2. The controller's ordinary wait/poll loop waits on that kqueue (bounded by the existing <=50 ms sample interval). On NOTE_FORK it runs the existing census immediately, then attaches EVFILT_PROC NOTE_EXEC|NOTE_EXIT to the discovered child, so GS1 runs as soon as possible after exec.
3. New diagnostics in the ledger (and the terminal line ONLY if needed; list exact names and types in the handoff):
   - leader_fork_events (u64)
   - leader_exec_events (u64)
   - child_discovered_via (none|poll|fork_event)
   - fork_seen_child_missed (bool): NOTE_FORK fired, but the child was already gone at the census.
4. Unchanged: the verifier must still be retained exactly once, and GS1/GS2 must hold. If a fork was seen but the verifier still could not be retained, refuse with a DISTINCT detail: "verifier forked and exited before census". If no fork was seen and the form needs a verifier, refuse with "signature leader created no verifier".
5. All law is unchanged: 66280 continuous running (no new SIGSTOP or grant), caps, 9x debt, 66331, the strict ^{tree} peel, 30 s, 16 MiB and reap/settlement. A kqueue fd is charged like any other descriptor.

## Controls (causal)
- A scripted leader that forks a fast child (exits in <1 ms) between samples: the old poll-only discovery misses it (mutant fails); the event path either retains it, or reports fork_seen_child_missed with the distinct detail.
- A leader that never forks: "created no verifier".
- A registration failure: coverage_unknown.
- The existing signature positive and negatives stay green.
- Update the finite model for discovery, with a broken control (poll-only).

## Handoff
ONE INPUT62 tuple, in the same format as INPUT61. Give exact new terminal field names and types, if any. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Report that exact string as verify_result.command; run it once on the final head. Land with make land, report in reports/, and task_report once. Keep only hashes and summaries of failed formal runs, not full caches (disk).


## Oracle amendment (2026-11-08): fast-child control criterion
In Controls, "exits in <1 ms" was an illustrative number, not law. It is replaced by: the scripted child is born AND exits strictly between two actual census polls, with its MEASURED RI lifetime (start to exit) recorded in the witness. There is no sub-millisecond requirement and no timing retry.
The measured witness event-c24-measured-physical-v1.json qualifies: lifetime 1,676,834 ns, strictly between polls, fork_events 1, fork_seen_child_missed=true, own wait4 and group gone, debt paid. Its log sha256 is d1244ae6...b9ff.
Keep unchanged the event, custody and cost assertions, the poll-only mutant (it must fail), and the rest of the control set.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed7ab80fac868b53b1dfc690ef774fc3bfabbb18b8 (including signed4967eb4378e3166aa7532630ee427b69d41a7dae) on main. Owned leader kqueue watch now precedes controlled continuation; fork notifications immediately service the existing census/current-token child watch/sampler, with exact missed-fork versus no-verifier refusals and charged queue custody through original settlement. Four typed ledger diagnostics added; terminal fields/caps/debt/GS1GS2 unchanged. Current canonical C83/54mutants/56compiles/125commands0, exact model test1pass, full440-model/94-contract native-Java graph/verdict parity0, exact required four-chain ONCE0, literal make land0 and fresh remote/clean/signatures0. C24 original1.676834ms measured-witness outer1 preserved; authenticated amendedTask162e70d5 explicitly accepts actual between-poll lifetime, no narrow baseline timing retry. INPUT62 stored1448 with signed10-input/compiler/997-check325-header audit tuple; report committed KBb2e71b1d5e20d4e6623a5e513f36bd8326bdc2ee. Runtimefalse/no builder;26 historical deferrals and tool/whole-private30s gaps remain unproven.

```json
{
  "op_id": "01M6XEZEVWFNXBJS0WS1CYMHM8",
  "commit": "7ab80fac868b53b1dfc690ef774fc3bfabbb18b8",
  "report": "[[reports/2026-11-08-dream-verifier-event-discovery.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Leader watch before controlled continuation; bounded event wait; immediate strict census and child exec/exit watch; original queue custody and charged close",
        "reference": "reports/2026-11-08-dream-verifier-event-discovery.md; proof-helper-probe.c SHA01e8578163768f9d463f70f1a1cc698a8f073e0ea91f911a7dd8c51465a5496d at2516/2552/2895/2919/3232/3286/3409; current C receipt677aab9ad9df97573aec9a29e0ddc09c10c5da7b3b6989a281b5eed35eab1eda,83/54/56/125 actual0"
      },
      {
        "criterion": "Real actual-between-census fork/exit witness; physical poll-only mutant; no-fork and failed registration controls, existing signature cases",
        "reference": "Canonical actual child lifetime1404375ns with raw RI/timebase/PID/poll proof; current physical missing-tail5commands0 SHA1811573eb7f0395db973f04a4388d3be076f3e89310b92a49f7fe12975eb0a43; C24 original outer1 logd1244ae68183a48d8b73eedcca5b47ed69b5f1b0ab0dec752699b379bc49b9ff preserved under native Task amendment162e70d5d6a7c4a3333fc7c078f926bdde044e4c1dd0039d6fe700ad0cac6bea"
      },
      {
        "criterion": "Exact four ledger fields/types and distinct refusals; no new terminal fields or law changes",
        "reference": "Report field table and C25 freezece8673224c959523b3a6880f960ba323e126415007e46a87d76ae4d53860d2a8; ledger kindverifier_event_discovery; leader_fork_events:u64,leader_exec_events:u64,child_discovered_via:none|poll|fork_event,fork_seen_child_missed:bool; terminal_fields_changed=false"
      },
      {
        "criterion": "Finite event-discovery model and five broken controls; mandatory full current native/reference parity",
        "reference": "Formal freezef4abb7fd8a3167024c02f2c2405fd63cdde96ad610ed75c0f522c646367a083d;440models94contracts3376317states11364544labellededges; archive20b1ab965ec30a14a9450dbb85c3a9264317f5d1500790e7aee285e9cb6e6473; rawmanifest2fa73050f9bcafa70445fe5c4f88e6558c48e39d60c8c847eda99c1ea33602a2"
      },
      {
        "criterion": "Exact required four-command string ONCE at final signed HEAD, actual0; canonical Land0 and remote containsHEAD",
        "reference": "event-exact-four-chain-v1.log SHA4d6663040e21ce643de2e88c7c9a74494f374f64cbf3cfb0d6a2486a196fabf4; event-canonical-land-v1.log SHA5766ac876f82f010352d790b707b90d2328576bd2537e0bac28f3d5ddc1ef18d; fresh remote-verification-detail-v1.json and report"
      },
      {
        "criterion": "ONE INPUT62 exact signed source/receipt/executable/ten inputs/compiler/header audit, runtimefalse/no builder",
        "reference": "Stored1448/id01M6XEN9XDZ52N5XSD6DK948M3; tupleed5a20501df0327296578e30095521815b7125c78edd3fc5dc91e633d073c09b; receipt889764ffa41be7bbf486be80afb49e74e5b58d9f81a522a1516609043468dd1d; executablebab19b967fed5adc1ac82f278bc5869657fc54242ef522c0e4fe897a6b09523c; audit207d4db89f1789fb2c69fef6ee5aea88ff67f60ee50413dae8e61875f99786a5 actual997checks325headers0 with sys/event.h pinned"
      },
      {
        "criterion": "Full new committed report with actual failures/amendment/limits preserved",
        "reference": "reports/2026-11-08-dream-verifier-event-discovery.md #93377961366359; KBb2e71b1d5e20d4e6623a5e513f36bd8326bdc2ee;26unchanged historicalignorednonpasses; no narrow baseline timing retry; runtimefalse,toolqualification/wholeprivate30sfit unproven"
      }
    ]
  }
}
```
