---
type: report
title: Fixed Dream C1 producer boundary review with rejected-result cleanup finding
task: '[[tasks/2026-11-05-dream-c1-final-producer-review]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T16:17:05.037Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6P7ZQKNA1KZD3PDYFAY4ZRF
updated: 2026-11-05T16:17:05.037Z
number: 93377961364776
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:a85aa6baebdd1afea81fe07b9789560c4cc4c35eed40dae3e7686c8ea87bbfd8
  frontmatter_digest: sha256:4ce7102e63e598d830abccb5df41115b8503e19e813264e5c30dd4758117105e
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: 3NBkBMLdF2gP+MKomhX71A+LAssoEvpW6VoCGELseD8qF/Xk/SPSEnr5iTVCU4c78yA2zAakREL0S45tjSyVCg==
  log: .bboxy/log/2026-11-05.jsonl#138162
---
# Fixed C1 producer boundary review

Task: [[tasks/2026-11-05-dream-c1-final-producer-review]] (01M6P7JTDZEY6MC9YBS8TAV5T8).
Reviewed /Users/feral/mydev/bboxy450-harness-dream-c1 at exact HEAD **4bdafb676467f4847d42c1ace154e8fe31521eb0**, baseline a2ca8f393630081b6be583151293ae286dc2419e. Initial/final tracked status and diff against fixed HEAD were empty. No component acceptance or consumer bump is authorized by this report.

**One unresolved static CPU-accounting/pacing defect, R1.** Other inspected producer mechanisms and retained evidence support their bounded claims below. No tests, builds or formal exploration were rerun.

## R1: rejected owned return value bypasses measured cleanup

src/dream/c1.rs:288-310 calls f(), samples its elapsed CPU, then returns resource_limit on elapsed/heap failure. The owned Result<T> is implicitly destroyed on that return, outside the measured work interval and before any cleanup_unit wait/accounting. The later cancellation cleanup at:311-313 does not cover this branch. The unavailable end-clock branch at:294-295 has the same ownership issue.

This is a real static accounting/pacing gap, reachable through the Envelope-returning b.track at:870-875. An over-budget large parse can finish with owned data, be rejected, and immediately dispose of that data outside recorded cleanup. Review does **not** establish how many milliseconds a real admitted record's destructor consumes. Existing actual_budget_rejects_cpu_burst_control at:1717-1727 returns (), so its causal control cannot detect owned-result disposal.

Required bounded correction/witness: retain large returned ownership in an externally paced slot across rejection (or equivalent local mechanism), including unavailable-clock/failure paths. Add an over-budget unit returning an owned Drop probe; assert disposal receives pacing and contributes cleanup/max CPU before settlement. Pair with a near-limit parsed-envelope rejection/cancel case and preserve unchanged limits. This is not an allocator-proof project. R1 should be disposed before claiming complete CPU/teardown coverage; Oracle decides component acceptance.

Sent within current scope to Rook and producer. Durable message IDs: 01M6P7TWTKN6YA9ASKXHBPK4PA and 01M6P7TWTZ7PW16MJE8X5Z8VFS. Tools returned stored/accepted; that is not proof of model receipt. No acknowledgement or author delay was requested.

## Reviewed requirements and concrete evidence

Paths below are relative to the producer tree; log is target/dream-c1-final-4bdafb67.log.

| Requirement / disposition | Source mechanism + retained witness |
|---|---|
| Isolated bounded CPU, tiny batches and foreground pause — R1 exception above | c1.rs:55-87 enters the macOS worker without general runtime; :258-315 checks thread CPU, foreground/deadline and ≥9ms rest; :1028-1038 establishes nice+10/utility; :814-917 separates framing/preflight/parse/observation and tiny≤8KiB/128-value batches. :320-375 and d0.rs:1099-1127 drain retained owners in measured batches. Source unit/inline-reactor controls at c1.rs:1717-1786,1794-1808 really exercise budget assertions. Log:6-17 records nine passing controls; :73,81,93,98 records multi-MiB sampled units with overruns0. Those samples do not clear R1. |
| Independent controls, EOF/cancel/deadline and real owner settlement — inspected | c1.rs:1326-1411 flat preflight; :1419-1680 nonblocking independent control/output loop, bounded lanes, monotonic matching Activity and nonce, EOF/partial-frame cancellation, h.is_finished()+join before Done. Parent must still observe actual subprocess exit. tests/dream_c1.rs:587-649 checks pause≤100ms, stale revision, EOF/deadline and actual child.wait; :1289-1379 checks four processes, independent cancellation and retained-state teardown. Log:57-77,91-92 records corresponding passes; no parent/UI result follows from these alone. |
| Exact closed source/raw/blob proof and fork subset — inspected | c1.rs:431-477 nofollow/uid/mode/nlink/fd-path identity; :646-996 streams≤64KiB, hashes original LF-excluded line bytes, validates final closed seq/hash, complete typed blob inventory and exact blob bytes. inspect.rs:47-264 incrementally checks transcript lifecycle; :403-433 enforces supported version/session/repo and explicit fork/native exclusion. This is the stated nonfork subset, not authored-IR or arbitrary protocol equivalence. Actual Store fork/raw CRLF/source mutation/blob controls appear in log:55,67-71,103-105. |
| Selected D0 computation, output custody and one close owner — inspected | d0.rs:312-353 shares the observer with exact streamed origin; :885-1083 bounds selected rows/aggregation and paced disposal, without replay(paths)/task/cache join. c1.rs:479-555 exclusive no-clobber publication, :1154-1210 pages then descriptor. dream/mod.rs:137-150 gates BOTH legacy D0/advice; engine/mod.rs:444, backend/acp_agent.rs:417 and import/watch.rs:83 call it. Log:134-155 records large-blob/200-round managed close witnesses; :168 selected semantics; :79 immutable output. Root owns remaining raw closure/whole integration review. |
| Declared credits and honest heap domain — inspected | c1.rs:661-745 reserves line/retained overlap; :1070-1202 reserves source/result/page/index/descriptor capacities. :1876-1929 CAS admission/shrink/drop preserves declared ownership; :1984-2136 structural preflight precedes serde. The causal missing-reservation control at:2180-2193 compares actual owned capacity against the ledger, and really omits the reserve in its negative branch. tests/dream_c1.rs:1235-1285 asserts78MiB work,16MiB helpers,2MiB control, measured96MiB ceiling, zero terminal live credits and no publication on dense overflow. Log:57,62,73,81,93 retains heap/RSS/credit/CPU diagnostics. Opaque/native allocations remain measured headroom, not a universal allocator or all-times128MiB proof. |
| Finite formal guards and source binding — inspected, not rerun | tests/formal/dream_c1.rs:65-103 changes close/raw/clobber guards; :155-195 changes foreground/early-Done/cancel-publication guards; :229-277 deliberately omits growth-overlap credit. The named invariants catch these effects, rather than compile failures. Log:362-372 records positive Storage45/67, Inspection41/56, Worker63/97 states/edges, with corresponding causal safety controls and parity. :584-585 records227 variants/70 contracts. Receipt/source binding below matches selected reviewed files; byte/CPU/native memory/syscall implementation refinement remains explicitly unproven. |

No additional acceptance-blocking defect was identified in the inspected scope. Large JSON/law operations may honestly fail resource_limit; this review does not promise all in-bound inputs complete on every machine. General native/library overhead and unsampled transient peaks are not silently reclassified as credit invariants.

## Fixed evidence and hashes

Final source/verification snapshot:2026-11-05 16:15:28 UTC. git diff --check in the **producer tree** returned **exit0**, empty output. Read-only Git/nl/rg/hash/JSON-inspection command batches also returned0. No suite was replayed.

| File | SHA256 |
|---|---|
| src/dream/c1.rs | d394a2626062208edff4929b1f2a5905ad836d8030ef36e4840d7c8dca05b68e |
| src/dream/inspect.rs | 2df2abfcc17df836f575ad3a725ab80bb2d9c283ed600d1e5e7e6b19f5570bc8 |
| src/dream/d0.rs | af7fc383efbe3f46a172838897192d5b9acadb1801c18ef053bdfa942ac125b4 |
| tests/dream_c1.rs | d954661c74fb4fea1f5a521c8b04596976a85793dfa31a5ab736bf62b96522f3 |
| tests/formal/dream_c1.rs | 570543f4dd58f00be858d03b6ce2c564c71b8ebefa2bd3d74e39b2f680caa460 |
| tests/formal/models.rs | b37d84c2dedae211bcaebe79c584cc0ffeb7ac39f2158df10e2693c53e3a1a50 |
| target/dream-c1-final-4bdafb67.log | cb6b887911c2f60c51136728edb14735c420d5772a21f1e60d0916de880e2ea9 |

Retained formal receipt verification/evidence/930f78a4919fbe1a3e2a05e2263e45413f3718e740103230518cf4584f8469b6/evidence.json has that exact SHA256, source4bdafb67 and dirty=false. Its input_sha256 entries match all six source/test hashes above. Relevant native/TLC graph_equal=true, positive tlc_exit0 and broken safety tlc_exit12 are present. No new formal verdict was computed, and no complete TLC replacement/Rust refinement is claimed.

Governing records read fully: [[specs/dream-term-c1]] digest4cab897d305e24a55c165a2e0221c6393ad44c885475214b37f5eca56ca490f5; [[interfaces/dream-term-c1]] c951b5417e78e5317192648a3a3bb84114f8759d2ee4b737924122936650f0c0; fork64460, sole-owner64464 and corrected memory/public-authority64581 decisions. Producer report71df9ef1bc3bc77f02627912ddda4a6c1bd30475daa587a7c289d0bed2f65df3 was compared to actual source/log, not treated as proof by itself.

## Limits and handoff

The author reports the required compound gate exit0; the retained log shows its named tests/parity/build-check results. This review did not independently execute those commands or recover a separate shell exit sentinel. Signature verification, whole-corpus/raw log/closure audit and landing baseline remain Rook's checks.

Parent32MiB attribution/combined128MiB samples, account/root admission, public authority/ABA, ≥240 native frames, UI responsiveness and final structured host hints cannot be certified by producer-only data. The consumer/shared integrated gate remains incomplete. No source edits, commit, builds/tests, provider/service/credential/operator access, install, merge/push, delegation, acceptance or consumer bump. Under the later refresh broadcast this read-only seat has no writing branch to rebase; the fixed producer snapshot was preserved. Any corrective source change requires new source-bound evidence and a new review disposition.
