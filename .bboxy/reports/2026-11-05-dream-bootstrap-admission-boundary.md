---
type: report
title: Disposition first controlled Dream startup and parent-child paid handoff
task: '[[tasks/2026-11-05-dream-bootstrap-admission-boundary]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T20:07:41.820Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6PN5XTV8TYK6JR24RKX39QJ
updated: 2026-11-05T20:07:41.820Z
number: 93377961365189
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:65f2e51b67266dcd60c11ec476a7fd4b3839238cf38ea4961826d32c1af25be1
  frontmatter_digest: sha256:8a482d1b505c9a0d7814e389befa36ac63c661e85fda50fd5e385fb406164d9a
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: qmk3Bac5HSPRGMp5o3Y1aG0Yik8UKMQkN0+s3EUH4j+sLBc5j4nEDwujx8AUAX9Uyb8M7n9NnMhG4lkbnKe/DQ==
  log: .bboxy/log/2026-11-05.jsonl#141545
---
# Dream worker bootstrap admission disposition

Task [[tasks/2026-11-05-dream-bootstrap-admission-boundary]]. Knowledge-only; commits:[]; no source/build/test/service/assignment/merge action.

## Decision / scope

[[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]] chooses one per-job inherited pipe with a fixed128-byte permit and open revocation lease, profile dream-bootstrap/1. The gate precedes **early argv Vec/PathBuf/Arc construction**, covering the first declared startup work, not only run/Builder. Actual FD plus birth/run/nonce/incarnation/context/worker binding is required; env labels alone admit nothing.

Parent retains actual Child/effects, evaluates inclusive creation cost and pays all work debt before one permit attempt. It transfers expensive ownership to child and does control only until paid child bootstrap Hello. Parent then prepares/pays and retires metadata ownership before existing Start transfers work back. Permit/lease/Hello/Start I/O is explicitly measured≤1ms control bookkeeping inside existing reserves, without recursive work permits; setup/hash/native/cleanup cannot hide there. No arbitrary dyld/kernel preemption, all-times process-memory proof or whole-process10% claim.

The decision fixes descriptor/env/layout bounds, original identity/effect custody, partial/duplicate/EOF/revoke/expiry/clock/crash/restart behavior, retained slot, public-currentness/ABA and unchanged250ms readiness/30s/native/app/heap/control/reap gates. Approved source/binary/profile support precedes execution of any probe; no unsafe old-binary gate discovery. Same≤1/account source-free probe uses paid permit→Hello→EOF/reap. Missing gate/profile makes managed Dream unavailable while ordinary seats/Legacy continue. Manual new worker without gate refuses before controlled startup. Q2 uses this same producer-owned handoff; Git/GPG helper65126 remains separate.

Rook notification01M6PMXWYWZC7P0K5PDGQBXJDB replies to delegation05CRMXH7936R25H962429XDRE0 with the exact disposition. Tool returned stored/accepted, not proof of model receipt. Oracle amends live component scopes/checks; original accepted tasks and prior fixed-review report remain unchanged.

## Changed records / complete readbacks

| Record | Current SHA256 record digest / pages |
|---|---|
| decisions/d93377961365166-dream-bootstrap-private-permit-handoff.md | ee7aca9df05a97d83ab1082971c61f84fd62641a5a19d21b714489aa1036b47e /1 |
| specs/dream-term-c1.md | d1abb6706471c4a6345a0c804815126ffa3e51170c363605b118ade6f40f8879 /2 |
| interfaces/dream-term-c1.md | 6d2a062e3161450ae00500574e7f3500427e48b1f35ceb315dec6d0c46d00118 /2 |
| specs/dreaming-mcp.md | 13d880c67a67e9bf27e928b7e3fa944773bce3b13354d4377143e857e91aaf6a /2 |
| interfaces/dreaming-mcp.md | 67bdba9b7d70b13a4bdcea58cf5150a5248f4a157c3d1df2c6149660a68bb93f /3 |

All full readbacks equal submitted text. Existing ref/etag/frontmatter preserved. Only bootstrap clauses and C1's cleared-environment sentence changed; reversing these exact edits reproduces each prior complete body. C1/read public controls/serde/schema/IR/algorithm, proof hashes and shared hints are unchanged. Responsibility claims link all five records to this task. One claim was explicitly refused workspace_pending, then succeeded on the same op_id after orient confirmed the unchanged root/workspace; no unknown outcome or bypass.

## Source / cutoff / race

- Consumer /Users/feral/mydev/bboxy450-term-dream-c1 HEADd156e0cbae84aa5a9af768220d8200cf62c200de plus active author changes. wake-up src/dream_c1.rs:1283-1359 prepares/spawns; :1392-1411 gives250ms Hello readiness. mux logic/dream_c1.rs:4811-4859 in the inspected fragment retains Child, checks native verdict, pays parent debt then reads Hello; :4945-4959 prepares publication after Hello. There is no inherited pre-run permit in that snapshot.
- Producer /Users/feral/mydev/bboxy450-harness-dream-native-c1 git object206c8bff6a3746c77bad88d909e610669cee84a5 c1.rs:405-439 performs measured/rested argv/path allocation before run; :2349-2438 prepares parked compute/Hello. Child spans can run before parent spawn returns/pays debt. Exact c1.rs SHA25646b9e3c5ecefabe6884fb0f395f741770d61a8a3df4a55711b16e74437c39b00. This is fresh boundary inspection, not certification of4575/206c fixes to the accepted16d review.
- Q2 /Users/feral/mydev/bboxy450-harness-dream-read-q2 HEAD47182320be6435c860bc2176c2ddda582029fa63 plus active edits. c1.rs:2217-2312 ParkedCompute/run<J> creates the read prestart owner before Hello; read.rs:62-77 selects it; worker.rs facade reexports the ownership. Initial hashes: c1.rs6a1806390c9e39b8e9f7c379ff0a5bea3eef9f439d084b48ceb7df789fde3d0c; worker.rs9d042a9c0d8f1c8494b26a19bed9aa9eaa355dec6ffd7e7093001601cd91f80f; read.rsd1e3210a4fd5746b62eff00cd76e561d38abde1f5f742ac88a4efb0558b2ac9b. No claim that this branch already implements the new native/bootstrap contract.

Consumer bytes changed while designing. Initial inspected wake-up/mux hashes099f60952b6b832ab91b19a7aee84a28983c6b7e787415701e2fb9ce5d9a4eab /13e862eda6a1dd0754d37484f7dcfa779988adbfa3c707858d549ab334e2282b; at20:05:05Z final hashes126fdef141a357281eb10f834a02ae49b055e43eac655d0b0237e9e9d5246961 /77687d9e09e1819f36c1d4cfd0dc54feb1db4d685d3d1bc99d60619df03749f8. HEAD remainedd156; source/test/verification changes belong to the author. Producer HEAD remained206c, only .bboxy/runs/formal-native.json dirty. Observations are pinned; unseen later changes are not reviewed.

## Actual verification / remaining evidence

git diff --check in exact /Users/feral/mydev/bboxy450-term-dream-c1 — **exit0**, no output, adjacent snapshot20:05:05Z. Checks current tracked diff only. Read-only Git/source/hash inspections also ran; no product build/test/formal replay/benchmark or provider/credential/operator action.

Decision/ICD require finite birth/permit/costknown/debt/Hello/Start/owner/currentness models and causal controls for bypass-before-argv, unpaid grant, concurrent parent, debt reset, ABA/duplicate and unknown birth. A real private process witness holds parent immediately after actual spawn return, shows no controlled child startup before grant using source-bound markers (silence alone insufficient), retains independent Status/Cancel and then verifies paid handoffs/actual reap. Include delayed/partial/stale/revoked/EOF permit, failed native clock, probe and current four-job/large-input/foreground/native-frame/heap gates. Use canonical make test-dream-c1 on both sides and existing source-bound formal/native-TLC/integration gates; no new target or green result invented here.

Remaining work is Oracle live amendments, producer/consumer/Q2 gate glue/conformance and fresh ordinary-release/private-fixture/source/profile process evidence. This disposition does not accept or broaden a component, rerun its gates, or change earlier review evidence.
