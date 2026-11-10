---
type: note
title: Dream landing and cleanup evidence, 2026-11-06
id: 01M6RPWYNDC0D31C2W5YZ3WJB2
updated: 2026-11-06T15:16:10.587Z
created: 2026-11-06T15:16:10.587Z
number: 93377961366016
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:25cfe22120a6a4910f68a0b3e7441d1268bd7bd7d418f88657f6f73eb0821763
  frontmatter_digest: sha256:58884025fe56c3433328765a47be27ba1c18c8bf4b8c359826870754da8ff70f
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: PlG8eKqTm8i+lpi3MTuR466nCP/nabZqZZZjdlELX8/tfnCG+y/IXeHFnGTqOZzSIc+lc1qAtwYRQHrdECplBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149495
  event:
    id: 01M6RPWZCKQYMQDG59QXP5QKSW
    sequence: 149495
    hash: 2106e1206dfd8686d334c0a545b55a41b7795b5adbcc80e68bbbe1afbf175562
---
# Landing and cleanup

Operator requested merging completed Dream work and closing finished Executors. This checkpoint records actions, not full Dream runtime acceptance.

- Local term main fast-forwarded from de089ec2 to already-published2847e213 with git merge --ff-only origin/main exit0. Harness origin/main fetched to a75eabfa. No fresh Dream source landing has completed in this checkpoint. gh api user returned bboxy450; source signatures use Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0.
- Preserved consumer11dirty source paths and Q2/builder generated state before recovery in target/dream-merge-handoff-gc2afvl6/manifest.json and exact full-index binary patches. Consumer source HEAD5dffa17aceebeed957f21cb4de9903c332f2a206; patch121930B/SHA6259d95e42741e5e2fde577554e287189081a75f2535305366dfd8ae4dbd983b. Backups are evidence, not instructions to replay over current work.
- Verified old producer47182320 and native producerd06e696f are ancestors of harness origin/main and git cherry is empty. Their only dirty state is a generated formal receipt in native. Exact worktree root directories remain open in app PID89696 (fd179/321), so whole worktrees were retained. No application restart, broad process kill or force removal occurred.
- After lsof on each exact child directory returned1 with empty output/errors, removed ONLY these accepted producers' target/debug and target/release compiler caches. Preserved source, target/dream-c1-native-profile workers, formal/metrics/raw logs and Root hardlink archives. df available rose6.3GiB to13GiB (approximately7GiB reclaimed). Exact removals/heads/checks are target/dream-merge-handoff-gc2afvl6/closed-producer-cache-cleanup.json.
- Old consumer01M6NPQVR1STF4MY8C3TV382TJ and Q2 author01M6PBF0EX2R5RGQAXNQTR96Y0 had exited. Preserved source, spawned builtin Codex replacements and reassigned the existing native tasks: Dream Term Finish01M6RMV7DJWATTAHBDSXQZT35Q and Dream Read Finish01M6RNNVQX1JAY0X2TX1GFEGE4. New Q2 assignment0NHKC9BN7NPV370N6G77XY66AQ has application acknowledgement. Old close requests returned incomplete UI effects, but fresh seat_get confirms durable closed_grace (operations01M6RMW78WRKDHXSKQ2AXMAQ43 and01M6RNW47R4R3TCS3M5E4SZCSD). Physical UI completion was not claimed or blindly retried.
- Quota-limited Architect01M6QBVWBSF0CCV4HGVC3V9JQY task reconciled done/superseded for remaining C work, with A/B retained and C explicitly continued by its two source owners. seat_close01M6RNW79R7SDMB7DB4CCBRB7K applied. Root's shared main worktree is retained.
- [[decisions/d93377961366007-dream-c-closure-codec-authority]] gives one builder-owned signed closure schema/vector authority within fixed65962 design and existing budgets. Current builder01M6Q0FHRZV6MQA8PBPXG408N7 and Q2 replacement have direct handoff routing; no existing signed C artifacts or qualifying real-pair result is claimed. Root coherent-Q2 pre-birth pin remains required.
- Full previous consumer/Q2 scope bodies were preserved in [[notes/2026-11-06-dream-consumer-scope-snapshot]] and [[notes/2026-11-06-dream-q2-scope-snapshot]]. Compact briefs retain acceptance, fix oversized native wakes, and authorize each author to land only after actual green canonical gates. Private KB checkpoint face46a6a75e3f5574efd35603938c629cb3262f committed seven exact records without remote publication.

## Entry gate remains unproven

The rebased signed d027a16b Entry cut's make check-embedded-build && make land exited2. Embedded/strict Clippy and116 library tests passed; prepared C1 integration30/33 passed. immutable_results, near_limit_rejected_envelope and source_blob_sequence refused the parent native-birth predicate; old code merged CPU>2ms and wall>25ms without numbers. Formal stage was not reached; no push. Log /private/tmp/dream-entry-c1-landing-2026-11-06/verify-land-1.log109352B/SHA5f67520f0cbbb87adac45ac8e57c0e18955353d952ed46e227c9152b7229ef40.

Root authorized signed lossless diagnostic correction9165b7d50818d11d11eb6a0d9fb753f8c54d805e and one focused diagnostic. Boundary/unknown/broken-guard checks and Clippy exit0 without births. Canonical focused immutable_results command exit0 using unchanged ordinary worker3511aa485d47535ed2607c9083a587603dea3fb0b90e0bf6a9d471d67eda280d: PID40851 CPU303916ns/wall1371917ns/exit0; PID41035 CPU339625ns/wall1036083ns/exit1. Both original predicates false, permit attempted and exact wait4 settlement. Log6351B/SHAde0fc8633b59412d0bd0b06544d225c26d1b0abff49d7324108c958a378c4e74. Receipt and births JSON beside it retain diagnostic_only=true/runtime_conformance=false. Root read actual diff, receipt and rows. d027 cause stays unknown.

The live Entry brief now permits ONE full canonical gate of the corrected lossless-controller cut, retaining unchanged limits and every per-birth row. Any refusal ends that attempt with exact numeric/effect evidence; no repeat of unchanged failed source. A future green landing certifies that measured run, not a repaired historical cause or universal OS latency. Original term12ba/16a launches stay stopped until a fresh source/compiler/hash tuple is explicitly admitted; no final term/native-frame/combined-memory or real lesson availability claim exists here.
