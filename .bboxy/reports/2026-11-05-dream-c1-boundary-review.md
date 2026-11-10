---
type: report
title: Dream C1 in-flight producer/consumer boundary review
task: '[[tasks/2026-11-05-dream-c1-boundary-review]]'
created: 2026-11-05T11:51:12.366Z
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
commits: []
verify:
  command: git diff --check && git -C /Users/feral/mydev/bboxy450-harness-dream-c1 diff --check && git -C /Users/feral/mydev/bboxy450-term-dream-c1 diff --check
  exit_code: 0
id: 01M6NRRWZDQS04GTG45RTW269D
updated: 2026-11-05T11:51:12.366Z
number: 93377961364466
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:1e749a30bcd89d337172a4075d5627bcf67e120f533948fe504c1ae67c9a1330
  frontmatter_digest: sha256:0100a1e75a8eeeff8f3a43858e08db2ff0d5c1d754855f8849a1957101d96b6b
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: DOqdJi1PM0cJYzi34+FeNFOXGGXUgBCLu6jhysxmPqPBhgZ7mjjlCXnGubO7y07jAynvRzyEaBWX/yF4LmKsDA==
  log: .bboxy/log/2026-11-05.jsonl#134731
---
# C1 boundary review

Static review of [[tasks/2026-11-05-dream-c1-boundary-review]], against [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]]. This is an in-flight snapshot, not final-source acceptance. Six actionable findings were sent directly to the responsible authors; shared-boundary matters were copied to Rook. No acknowledgements or author status were requested.

## Findings and disposition

Paths below are relative to P = /Users/feral/mydev/bboxy450-harness-dream-c1 or C = /Users/feral/mydev/bboxy450-term-dream-c1. Hash prefixes identify exact inspected bytes in the snapshot table.

| ID | Classification and source evidence | Fix / causal witness / disposition |
|---|---|---|
| F1 | Actual result-integrity defect: C registry/dream_c1.rs (855fb814):565-585 served completed page JSON after header checks without authenticating its result/page digests. | Reuse the off-owner read_page validator (C logic/dream_c1.rs d48a70a8:1043-1091). **Source correction observed** in registry 1fcf9d75:629-636. Still require a real completed run, valid-JSON row tampering preserving headers, and Status(page) refusal. |
| F2 | Actual supported-protocol/subset mismatch: P inspect.rs b791bfad:42 hardcodes inherited=false; :395 requires each envelope's child session. C Reader d48a70a8:231 likewise rejects inherited rows. Canonical P protocol/laws.rs:24-27,52-65 permits parent rows through fork.at; store/fork.rs:256-271 publishes those raw rows. | **Scope resolved** by [[decisions/d93377961364460-dream-c1-fork-source-subset]]: C1 must reject fork headers immediately as unsupported_ir. Consumer header/snapshot guards appeared during review; producer inspect hash remained unchanged at cutoff. Real Store::fork → child close → explicit refusal, unchanged bytes, no result, and a parent-row negative control remain required. Do not claim general law equivalence. |
| F3 | Actual restart reexecution defect in C registry e6408ec7:498-515: saved Record remained Deferred while only live Control tracked running; retry could recreate an owner after daemon loss. checkpoint (logic 2c4f1471:787-794) saves that Record, so persisting PID alone did not prevent it. | **Source correction observed** in registry 1fcf9d75:527-557: durable started/incarnation fence before execute, predecessor quarantine. Require a held real child after Start, daemon restart, original-op and new-op/same-source retries, with no second worker/replay or unverified result adoption. Crash windows and predecessor slot accounting remain implementation evidence, not proved here. |
| F4 | Actual cancellation defects: C registry e6408ec7:541-547 ignores Cancel.op_id and only sets an atomic flag, leaving a never-started foreground-deferred run without a settling owner; :526-528 also overwrote Cancelled as Failed. | **Partly corrected:** registry 1fcf9d75:569-571 preserves Cancelled. Its :584-590 still ignores cancel operation identity and does not durably settle a never-started cancel. Bind actor/op/request digest, settle deferred cancellation without launching, preserve terminal results. Witness original Cancel retry/restart, same op targeting another run → op_id_reused, and running cancel → settled Cancelled only after actual reap. |
| F5 | Actual timing-path mismatch: C harness_status 96400e82:85-124 performs legacy log.poll before C1 polling and sleeps 200 ms; updated 51e39531:89-133 retains that structure. A turn arriving just after a tick cannot reach Activity within the 100 ms pause-ack allocation. C1 follower also uses shared spawn_blocking, including Reader's 9 ms sleep, without dedicated utility scheduling. | Feed admitted foreground work through a tiny immediate atomic/revision update; isolate/bound the fallback follower. Keep closure authority in IR. Witness a turn just after a follower tick and a held legacy-log read, asserting real pause acknowledgement and owner/control latency. Injecting foreground directly into a worker proves only the downstream half. |
| F6 | Residual integration concern, not a duplicate D0 replay: P engine/mod.rs:450-455 still launches ir_scan on TermC1 close; ACP/import also do so. P ir_scan.rs:455-463 uses shared blocking work and :503-512 reads/hashes/parses an entire advertised JSON blob; :632-651 detaches close work without C1 duty/foreground control. | Requires Rook's scope disposition: gate this separate scanner for term-owned close, or place its close work under an explicit bounded/pause policy. A large JSON blob plus real SessionClosed/foreground traffic must witness total post-close behavior. No CPU-burst magnitude was measured. |

Feedback storage IDs: F1 consumer 01M6NR97TW3M85CB92CETAFGNM, Rook 01M6NR97V6YT6VZMPR52MW9F2D; F2 producer 01M6NREZG7JSQJP4SBFX98K71B, consumer 01M6NREZH18K6TMRXA2M4432CC, Rook 01M6NREZGQVE50TAAPHEKAGH2X; F3 consumer 01M6NRHEXV698F0EJ60RQ6MF04, Rook 01M6NRHEY33KJXYP5ZDX6J3GWV; F4 consumer 01M6NRHEYC88BWQ68N1T0604B5; F5 consumer 01M6NRJ96ZY0PKZGRB99P1178D, Rook 01M6NRJ991VGDFY1WXE1JTQRRF; F6 producer 01M6NRKHJN0T4RZY1Q96JJN8K8, Rook 01M6NRKHM4KYBYSC2GQTGCV2DN. These are durable storage evidence, not model-receipt claims.

## Boundary mechanisms inspected

- **Closure/ownership:** producer d0.rs 6e548af9:704-719 suppresses legacy replay before cache/root access for TermC1; LaunchOptions carries ownership through engine, ACP and imported watcher hooks. Consumer logic.rs launch appends --dream-owner term-c1 only after capability probing succeeds; its dream_owned set gates automatic proposals. Reader requires an explicit terminal SessionClosed, sequence continuity and a stable source file; EOF/idle/turn-end alone do not create a bookmark (logic d48a70a8:203-332). Journal register atomically stores SourceKey, operation alias, bookmark and close cursor (registry 855fb814:160-206). Inspect original-op lookup precedes source access. Restart ownership persistence still needs the F3 witness.
- **Authority/provenance:** MCP control parsing injects the admitted operation ID and rejects caller-supplied paths/context/seat (C mcp/dream_c1.rs:46-68). Authority.capture (registry 855fb814:235-327) checks generation, issuer, grants, swarm/root/provider revisions. Final authority is recaptured before presentation/publication. This is a source mechanism, not a demonstrated A→B→A witness. Manifest authority alone is never promoted to memory/trust.
- **Isolation/custody:** direct sibling runner uses cleared environment, absolute job path, private pipes/group and nice priority (C wake-up dream_c1.rs:563-614). Producer early headless dispatch precedes profile/session discovery. Producer c1.rs ca45ae7b:433-601 revalidates raw JSONL, close hash, typed blob inventory and actual blob bytes in bounded units; :607-753 emits event-only D0 episode/cluster pages and immutable result descriptor. Consumer snapshot/result validation lives off mux owner; per-request isolated threads perform SQLite/serialization/hashing rather than mux handlers. Slow storage/control starvation still requires measurement.
- **Settlement:** producer c1.rs ca45ae7b:950-987 joins compute before Done; input/control handling precedes bounded progress drain. Consumer logic d48a70a8:1263-1447 holds the scoped source owner and actual child through cancellation, drain, reap and output validation. Wake-up :755-777 targets only its unreaped owned group. Later logic b08210ae:1236-1291 explicitly retains failed-Hello/checkpoint child custody. Independent cancel/status are atomics/cached views, but the shared four-request admission cap, SQLite contention and saturated-output cases need real witnesses.
- **Schema compatibility:** producer c1_wire.rs owns serde/schema/fixtures; consumer public and worker mirrors use producer fixtures with independent canonical/hash checks (C proto/tests/dream_c1.rs:49-82; wake-up/tests/dream_c1.rs:53-118). Hello pins schema hash, algorithm and IR ceiling before source access (wake-up :624-644). Probe succeeds provisionally; old/missing runner must remain honestly unavailable. No separate installer was introduced.

## Verification evidence and limits

At 2026-11-05 11:48:59 UTC, the exact task verification command returned **exit 0**, no output:

```sh
git diff --check && git -C /Users/feral/mydev/bboxy450-harness-dream-c1 diff --check && git -C /Users/feral/mydev/bboxy450-term-dream-c1 diff --check
```

This checks tracked diff whitespace only; new untracked files are not covered. Read-only git status/rev-parse, diff/source reads and SHA-256 snapshot commands also returned exit 0. Exploratory path searches included misses (exit 1/2); oversized read outputs were truncated and were not treated as complete source evidence. No builds, unit/process/smoke/e2e/performance or formal runs were executed.

Inspected producer formal model tests/formal/dream_c1.rs:9-98,100-189 has causal close/raw/clobber and foreground/early-Done/cancel-publication controls. Producer process tests/dream_c1.rs:579-638 exercise downstream Activity/EOF/cancel/deadline; :656-674 inspect a multi-MiB corpus and metric fields. They do not establish upstream pause delivery, total post-close CPU or the 200-roundtrip/UI performance allocation. Consumer formal ownership model :351-449 abstracts admission/hash/exit, and :451 onward makes context replacement an atomic cancellation transition: mapping that assumption to actual runtime events needs a real context-cutover witness. Consumer model assertions :612-675 are model traces, not API restart/ABA evidence. Consumer MCP and ctl process witnesses existed but their early snapshots chiefly exercised missing-run/refusal paths. Coverage was being added; no missing witness here is a claim about later final source.

Future acceptance must consume the authors' current-source gate logs: producer make test-dream-c1, its formal/parity/embedded gates; consumer proposed make test-dream-c1 plus canonical formal/boundary/smoke and proportionate landing gates from its brief. This review does not assert that an in-flight target was installed or green. Rook owns final-source acceptance.

## Snapshot/provenance

Both author trees were dirty with tracked changes and new files; no author code or submodule was changed by this seat. No commit was created. P HEAD remained a3425ac0a08e7184dc0a9198ddfa71f94a15ad54; C and root HEAD remained ad243195daf74153756308346d01a56390c1261c. Root status contained only untracked knowledge records at cutoff. Reads spanned changing files; each finding binds to its cited bytes, not just HEAD.

| Snapshot anchor | SHA-256 |
|---|---|
| P c1.rs initial | ca45ae7b8e758d49dad860e5121505314d2d0a47050f0639ec742fb55437a9b9 |
| P inspect.rs | b791bfad4a6b08a1a803b1e6448c196cb5ae4ff055521921eeff5d8cc0b45134 |
| P d0.rs initial | 6e548af91a96b0f992e430eeab56fe916068814684f2faeb291943b8b6eb511e |
| P ir_scan.rs cutoff | 25b2340fb4e4d157c63195ef112fb14058b187877e52d2eaaa5a98784d9d9b52 |
| P c1.rs cutoff | 81210d0344eb7ac7c300d91c0e5476e1f31bba48890d9cb60c85894a6d5349b3 |
| P c1_wire.rs cutoff | 5ce52ef346cd3494688b16b30f20e0d707f621b238adf9b13c151621f58e1b02 |
| C registry initial / reviewed middle | 855fb814dbe6bb60889790ada8814636353f5a597057ace6cf9c3027c88f9ec4 / e6408ec74d971c0f812fea814eeb50d03a63c6a256d3eb5f63e21791c6a4ad32 |
| C registry correction snapshot | 1fcf9d7556328d252f4cd0a6e6e93ed8fd146abf51aa7864de7b8c1dcd434bab |
| C logic initial / middle | d48a70a8ae8ee1e48711be1d1af99342222dae1ed19c5463c49562391e197699 / 2c4f147122e300a7166ab1a53cc84b538e6169c6eb1c0bf5d7475222ecc7578a |
| C logic final inspected | b08210ae00cb7bfa2c4167828be94dbdae6e749640dc6a4d2ff6a036ac655b0a |
| C harness_status initial / updated | 96400e821d43097ca57fe8fc6f82d1ebe1ca091a38f9023ed929dc0263122888 / 51e39531ab9eb6656a7f9dcb2549dbfc4084df304ea8c99827c447642ee4d197 |
| C wake-up worker transport | 416a161672a4a56087d434dc83e541e721fd450e1f1ad0f1ba469fa2c4c0cd35 |
| C public wire | c6717dbc8a8e99aacfaa356405b9b7edafab7ec6a008312eb6273d248d192dd4 |

Acceptance evidence: findings/dispositions and hashes above satisfy source-grounded snapshot review; closure/owner/runtime/settlement/schema mechanisms were inspected on both sides; verification is explicitly limited to whitespace checks and static inspection. Deliberate exclusions: source/spec edits, test execution, implementation assignments, build contention, installation, credentials/account logs, commits/merge/push, and final acceptance. Later author changes stale this snapshot.
