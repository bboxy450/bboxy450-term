---
type: report
title: Q2 durable-close repair and preserved full-gate failure
author: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
created: 2026-11-06T16:45:18.889Z
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot.md]]'
commits:
- 0fd56ba54ac2f03cc26ecc4db19fe527c952d12e
verify:
  command: make test-dream-c1
  exit_code: 2
id: 01M6RW05EQ2MVW8DTZ5J9VJWXV
updated: 2026-11-06T16:45:18.889Z
number: 93377961366045
created_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
updated_by: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
authorship:
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  kind: seat
  public_key: p1xg5YnZmsPv/B3q4mjUXp1coPPED4IjwlMtDzJrDaE=
  process_generation: '1'
provenance:
  digest: sha256:c709d586d7e37150d60efa4f80beaf707f8450cbdacaf91fff13044ea7f1c0af
  frontmatter_digest: sha256:2a3da24cb980a382a2aceeafdeb99a90287db82ff9a75b5e6f28bd44c3268d7a
  signer: seat:01M6RNNVQX1JAY0X2TX1GFEGE4
  sig: GcKXZhR9/T6ika9qHq/KHEd0oEQQDjZFkyPZ/YREsPhSj3zZqR3RiyJ6D9KVw6F8d4NTKYDvAvKd57Bei/6eBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149597
  event:
    id: 01M6RW069A0H2E74WE185X1A25
    sequence: 149597
    hash: 89e4d609bc48964217165908eeb0c94108b1ccf860baab25fabfc18869cdefe8
---
Durable-close compatibility checkpoint for [[tasks/2026-11-05-dream-mcp-verified-snapshot]]. Signed `0fd56ba54ac2f03cc26ecc4db19fe527c952d12e` (%G? G, approved Boxy key). This is unfinished task evidence; no task_report/landing.

`src/gateway/mod.rs:132` captures the acquire closed flag BEFORE queue/terminal drain. A publication racing an empty pop now returns Empty and drains the actual queued durable envelope on the next observation. The deterministic held-order regression exposes the old EOF race without sleeps.

`src/gateway/unix.rs:173` retains the existing1s drain after published SessionClosed when the face requests close before the Unix writer finishes. `lifecycle_terminal` requires durable publication or actual actor fault; admission sealing plus observer EOF resubscribes. A held terminal writer proves the eager-cancel ordering, and a sealed-but-unpublished control proves terminal authority remains distinct. The original5s fixture/1s drain bounds are unchanged. The production actor still emits its one durable SessionClosed, then closes the hub; no synthetic close is added. Test-only engine/transport markers and seat_process diagnostics retain phase/session/last seq/durable event structure/child status/stderr. `src/frontends/headless.rs` is unchanged.

Actual scoped checks (the3passed Unix run is the final lifecycle source; earlier process runs precede that last narrowing):

- `cargo test --offline --locked --release --jobs 2 --target-dir target --lib gateway::close_tests -- --test-threads=1 --nocapture` — exit0, 1passed; `target/dream-read-finish-recovery/session-close-gateway-unit.log`, SHA256 `8acd90e6725a3952061de7acb4df2113f91d7a33d2ca62a6363ef2a8eb7a7a77`.
- `cargo test --offline --locked --release --jobs 2 --target-dir target --lib gateway::unix::close_tests -- --test-threads=1 --nocapture` — exit0, 2passed; `target/dream-read-finish-recovery/session-close-unix-unit.log`, SHA256 `2373a04d8879eb7838e0590673f7e837ea9486505c7290eb25ec0514acde8910`.
- `cargo test --offline --locked --release --jobs 2 --target-dir target --features test-fixtures --test seat_process dream_c1_retained -- --test-threads=1 --nocapture` — exit0, 7passed; `target/dream-read-finish-recovery/session-close-retained-process.log`, SHA256 `7a2e49dcd24b06d5b41e53a2bc63d9e97c61c969538f518f6b90e3b759ac87d2`.
- `cargo test --offline --locked --release --jobs 2 --target-dir target --features test-fixtures --test gateway_unix transport_replay_and_duplicate_admission_equal_inproc -- --test-threads=1 --nocapture` — exit0, 1passed; `target/dream-read-finish-recovery/session-close-unix-process.log`, SHA256 `11214b9954bd9273c3785db12572b40fd6016830511736d6b222f2962ab27957`.
- `cargo test --offline --locked --release --jobs 2 --target-dir target --lib gateway::unix::close_tests -- --test-threads=1 --nocapture` — exit0, 3passed; `target/dream-read-finish-recovery/session-close-terminal-lifecycle-unit.log`, SHA256 `7e8cf9a938d4114ccaafb2b256fb9bd2add65154655451a27ce957213357764e`.

After the final lifecycle narrowing, current Unix unit family3passed, exit0; `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/session-close-terminal-lifecycle-unit.log`, SHA256 `7e8cf9a938d4114ccaafb2b256fb9bd2add65154655451a27ce957213357764e`. Earlier process checks were not repeated for luck; the canonical full gate below was the current-source check.

`close_receive(0/1)` and `close_retirement(0/1/2)` register two finite good models and three named safety controls with explicit atomic publication/enqueue/closed release, captured observation, local face, held writer, actual write, explicit deadline and fault. Only internal read/drain is weakly fair. Native unit tests2passed and five full native/TLC graphs, labelled edge sets and named verdicts agreed: receive54states/76edges, late-read62/76, retirement164/313, eager-cancel164/301, sealed-EOF174/333. Good Java verdict0, all intended safety controls12, graph checks0. `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/session-close-model/evidence.json`, SHA256 `6cce4ffab104c4f23e83326e285a33e73cf6ceebcde1f0300fa225653262b766`. Its exact source hashes equal the signed0fd commit. Scope is these five models, not full repository formal parity. Native unit logs remain in session-close-model/native-tests.log; the intermediate native summary JSON was overwritten by the parity receipt, so its original command summary is only tool readback, not a retained original file.

`make test-dream-c1` — actual exit2, `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-finish-recovery/current-c1-after-close-repair.log`, SHA256 `4c73c2c8653488b62e26b260571ed48b1628592e3cbca18f330b4500a8a25c5f`. Bootstrap/C1/fingerprint unit stages pass, but the current C1 integration family30/31 fails `source_blob_sequence_version_and_custody_controls` at tests/dream_c1.rs:681 with `native birth verdict refused grant: native birth allocation`. This run never reaches the later retained-host fixture commands. That is a preserved NEW failure, not evidence that the socket fix passed the complete gate. No lucky retry or threshold change. The compiled ordinary worker is target/dream-c1-native-profile/worker-18017264d6bf9a74fcca0fa7a236ce2adb19ee89390e051c9c72f2244a8127b2, SHA256 suffix, source commit0fd.

The original failed close evidence was preserved before tests in `target/dream-read-finish-recovery/session-close-pretest-1793981743152645000`. Historical output omitted child PIDs/A-vs-B phase; EOF alone does not identify historical publication vs transport failure. The deterministic controls prove two concrete source races; they do not retroactively identify that schedule.

Remaining original gates/real qualification/publication/resource witnesses are unchanged and unfinished. New native-allocation failure is under scoped diagnosis; subsequent helper-accounting/IO optimizations are separate working changes and not this commit. The immutable C input handed to Root/builder remains signed a386 and is not modified by this compatibility checkpoint.
