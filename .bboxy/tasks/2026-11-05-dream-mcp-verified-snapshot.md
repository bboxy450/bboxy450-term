---
type: task
title: Q2 implement bounded already-published verified Dream read snapshots
role: executor
allowed_paths:
- tests/fixtures/dream-bootstrap/process.rs
- verification/upstream/scripts-test-formal.py
- verification/upstream/scripts-formal.py
- verification/upstream-sources.json
- scripts/formal.py
- tests/formal/dream_c1.rs
- tests/acp_launch_context.rs
- tests/fixtures/acp-agent/launch_parity.py
- tests/support/seat_socket.rs
- ATTRIBUTION.md
- Makefile
- src/main.rs
- src/frontends/headless.rs
- src/dream/mod.rs
- src/dream/read.rs
- src/dream/read_wire.rs
- src/dream/resolve.rs
- src/dream/audit.rs
- src/dream/graph.rs
- src/dream/recall.rs
- schema/dream-read-v1.schema.json
- tests/dream_read.rs
- tests/fixtures/dream-read
- tests/formal/dream_read.rs
- tests/formal/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-05-dream-mcp-verified-snapshot.md
- src/dream/c1.rs
- src/dream/worker.rs
- src/dream/trust.rs
- src/dream/receipt.rs
- src/dream/projection.rs
- src/dream/seal.rs
- tests/dream_c1.rs
- src/dream/bootstrap.rs
- scripts/land.py
- reports/2026-11-06-dream-mcp-verified-snapshot-landed.md
- tests/fixtures/dream-proof-tools/closure-boundary.schema.json
- tests/fixtures/dream-proof-tools/closure-boundary-vectors.json
- src/gateway/mod.rs
- src/gateway/unix.rs
- src/engine/mod.rs
- tests/seat_process.rs
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[research/dreaming-services-mcp]]'
- '[[specs/dreaming-mcp]]'
depends_on:
- '[[tasks/2026-11-05-dream-c1-producer]]'
id: 01M6P1PTMD3SDRJ4JQFFV2AXV8
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T05:54:52.670Z
created: 2026-11-05T14:27:22.336Z
number: 93377961364661
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
verify_revision: 2
assignee: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Distinct prepare-read wire, exact context/manifest binding and bounded immutable pages/descriptor.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#wire; src/frontends/headless.rs:32; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Finite selected IR/JSONL/blobs, private Git, public policy/keys, catalogue and artifact-to-file custody; no arbitrary receipt paths/fallback.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#bundle; src/dream/read.rs:295; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Compatible adoption, current record/trial/publication/receipt/trust/transitive proof, safe lesson fields and verified_at_revision.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#published_lessons; src/dream/read.rs:2286; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Shared64MiB input; parent32MiB/child96MiB, protected control/helper shares, bounded allocation/disposal, four jobs and actual joins.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#resource; src/dream/read_wire.rs:14; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Own-thread ordinary1ms/native2msCPU25mswall3msinclusive/full9x debt unchanged. Decision66280 removes external per-grant/Q/T/N; helpers run continuously at atomicBACKGROUND/observedPRI4 with250ms command/3s job totalCPU, actual per-member RI begin-to-begin+return gaps<=50ms, terminal originalwait4/rawovershoot/fresh9x postreap debt/terminalRSS16MiB.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#cost_debt; src/dream/c1.rs:748; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Canonical128B permit/lease, unchanged250ms readiness and30s maximum work deadline; no argv/channel/owner/source admission before permit, null-context probe rejection and actual settlement.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#bootstrap; src/dream/bootstrap.rs:16; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Original positivebirth/audit/PGID custody, bounded2members, positive ownWNOWAIT/actualownwait4, known-native census status, retainedchild positive exitedRI then fresh sameleader qualified componentwise rollup; live/escaped effects before reap seal, independentgroup proof/ownedquarantine. KnownleaderCPU paid even if unknownchild held; no postownreap signals.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#helper_custody; src/dream/c1.rs:829; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Git-created verifier temp argv/custody GS1/GS2; matcher never directly expanded; original inode/hash/scratch cleanup or quarantine.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#signature_B; tests/fixtures/dream-read/proof-helper-probe.c:3358; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Exact signed shared schema/vectors, bounded strict decoder and seven-source/private pre-test bindings, no runtime self-approval.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#shared_codec_C; tests/fixtures/dream-read/proof-helper-probe.c:723; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Independent control/persistence/terminal-cost verdicts, bounded retained errors and post-tail receipt; fixture setup cost cannot disappear. Effective23:10 fixes bind admitted inputFD+freshhash independently of replaced locator; distinguish native IO failure from paid work-deadline refusal; one-way terminal reporting cannot admit work and attempts finite/paid result after original30s expiry. Bounded path batching retains full actualCPU/debt.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#terminal_verdicts; tests/fixtures/dream-read/proof-helper-probe.c:3874; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: One durable A close, finite subscriber/socket drain and retained Host creating one nonfork B with managed launch inputs.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#retained_host; src/engine/mod.rs:510; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Finite owner/actions/bounds/fairness and all deliberately brokencontrols; honest66280 retirement/inventory. Exactnative cap100000 and pinnedJava fullstates/labellededges/namedverdictparity. Incomplete/statecap/OOM is failed; no sourcecurrent claim before freshcanonicalgates.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#formal; tests/formal/dream_read.rs:2625; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Signed exact source, current worker/compiler/source-input profiles and formal model/tool evidence; immutable Cpreparation/source4readset/schema/vector/compilerheaderclosure tuple separate from runtimequalification. Retain all failed runs/sourcecuts; no oldtuplecurrent.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#provenance; src/dream/read.rs:2648; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
  - criterion: Canonical current-source gates and remote main containment before final handoff.
    reference: reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#landing; scripts/land.py:112; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained
report:
  summary: 'Landed signed af5de27c33d46cdcb04510e863ef95fdc835c3ec on origin/main, clean, including strict parser cut a7c08e65df64c8ed8a4a0ce3ad9daf96cba90373. Only registered terminal ^{tree} stays literal; unknown/malformed braces refuse. Current C17 canonical suite0: 68 baselines, 41 source mutants, 43 compiles, 88 commands. The EXACT task verify command actually returned0 (log e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf), followed by current canonical strict/nextest/C1-integration/validator/signature/push0. Fresh432-model/93-contract native/TLC parity and immutable6e73 archive are retained; actual worker profiles and same-inode observers preserved. Input59/message1415 ten blobs remain unchanged by source60''s single authorized readiness-wall ignore;26 task-specific deferrals are non-passes. Source59 exact-chain2 and C16 RED remain preserved. Full private30s fit/qualification remain UNPROVEN, runtime_approved=false, no genuine builder births. Full correction report committed at native checkpoint847218b1339621278bdb8833aae64b32404b5ae6.'
  commit: af5de27c33d46cdcb04510e863ef95fdc835c3ec
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6WVHRA8H1YVX0HCJKDHY6ZR
  verify_revision: 2
  report: '[[reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md]]'
provenance:
  digest: sha256:1f776b509fe24ccb961d0aa9bfa08f5756fd17b7e3b0aea138e16f7111eb03fc
  frontmatter_digest: sha256:040e41d8b4a6e464432de822c81b7807965d32f8eea9ce63f721365cae68f7ba
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 2+172TMJPc1XZir2eJo4mrcKgkOrYv46ZGPXu+WEiKj2x5oFs9Xws1yv4KQFwD1Ww7wOlefmPOoHqHnTZTavBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150569
  event:
    id: 01M6WVJMPX6QW8CM1QEKGSDT1Y
    sequence: 150569
    hash: 11f1cf8203c6fc5f6efee6d01864a168d966d2eee873ed93451aa73de1286a4b
---
ROLE: EXECUTOR

## Outcome and preserved authority
Finish and land Q2 prepared reads of ALREADY published verified Dream lessons in /Users/feral/mydev/bboxy450-harness-dream-read-q2, branch feat/dream-read-q2. The operator explicitly requested merging completed work and cleanup on2026-11-06. This is the SAME scoped assignment; implementation scope/acceptance are not relaxed.
The COMPLETE original brief is preserved verbatim in [[notes/2026-11-06-dream-q2-scope-snapshot]], original task digestc7494f0ac15223f8775dc82778f6da9ce8bc4262a536161d03780ff09b832b42. Every effective detailed acceptance/control clause remains binding alongside current [[specs/dreaming-mcp]], [[interfaces/dreaming-mcp]] and [[interfaces/dream-proof-tool-qualification]]. This handoff supersedes only obsolete baseline/milestone narrative and the earlier no-push clause under the gated landing authority below. Never invent a competing plan/format/controller or widen done.

## Recovery and source ownership
Previous bboxy harness seat01M6PBF0EX2R5RGQAXNQTR96Y0 recorded exit1. You own this ONE writing tree after native reassignment. Actual signed source headfeefcc960cea9cd396b928cc86fe923df424f594 includes signature B and STOP/parent IO corrections; source was clean except generated .bboxy/runs/formal-native.json and retained observer summaries. Root backed up head/status/diff at /Users/feral/mydev/bboxy450-term/target/dream-merge-handoff-gc2afvl6. The backup is not an instruction to overwrite newer files. Read actual source/logs/Git state first.
Fetch/rebase on current harness origin/main (Root observed a75eabfa, may advance), preserving upstream and your scoped selected-proof/helper changes. Copy/hash generated formal-native.json into your own target before restoring ONLY that GENERATED file if required for rebase. No source reset/stash/erase, broad stage, forced KB addition or peer cleanup. Preserve untracked observer summaries. Current main has the canonical make land and private formal-receipt workflow; follow it. Query original unknown MCP operations before repeating an uncertain effect. New material final reports are not a replay of old writes.

## Exact feature boundary
The distinct sibling runner is dream prepare-read --job-dir ABS, protocol bboxy.dream.read/1 algorithm read-prepare/1, exact shared schema/goldens/hash vectors and immutable manifest/result/pages. No inspect-c1 wire/IR/version change or hint-bearing producer proof DTO. Host hints belong only to the later shared term structuredContent/text projection.
Inputs are one immutable host-authored EvidenceBundle: inventoried selected IR/JSONL/blobs, explicit private nonbare Git snapshots, public-only policy/key bytes, current-head/transitive catalogue and exact logical artifact/receipt-to-file bindings. Start binds exact manifest bytes. All selected inputs/Git/keys/artifacts share the existing64MiB input allocation, not another allowance. No receipt-directed arbitrary paths, live/uncommitted record, origin/main/HOME/KB catalogue sweep, source rereplay, provider fallback, synthesis, inference/trial/publication/adoption/trust enrollment or Recall append. Missing proof is verification_unproven, no qualifying publication is distinct no_published_input, corrupted proof fails. Native/provider bridge absence remains honest, not a draft lesson.
Export only safe bounded pinned title/why/how_to_apply/context/evidence for active compatible adopted harness pairs whose qualifying trial, exact current record, publication/outer seal, producer/trust policy and every transitive artifact are verified under the admitted finite bundle. Return verified_at_revision; only the later consumer's live authority fence establishes continued applicability. Preserve existing audit/receipt/trust/Recall semantics and exact signed bytes through logical mapping. Use actual scripted public signed publication fixtures, not fabricated success.

## Resource, lifetime and closure law
Reuse/reconcile the accepted C1 paced Budget, owned-return disposal/Drain, credit ledger, private Dir custody, independent control and actual join. Work is isolated/low-priority, bounded and post-session; no shared async/UI/logic-owner or generic blocking pool work, duplicate scheduler, unbounded selected resolver/graph/serde call or global allocator. Keep1ms ordinary/parser/hash/serde/cleanup/control, paid9x rest, finite clocks and every exact native2msCPU/25mswall/3msinclusive class/effect/debt rule. Full inclusive cost survives failures; no saved idle, free helper lifetime, hidden disposal, MAX/unknown-as-zero or parent/child expensive overlap.
Keep32MiB parent/96MiB child including protected2MiB control each and corrected helper headroom. Charge all actual bounded input/context/parser/cache/capture/argv/result/temp/hint-independent copies before controllable growth; preserve measured ceilings, overflow/control reserve, four-job/large-input/stall/flood/foreground/control/deadline/reap witnesses. Do not claim an all-times arbitrary-native heap theorem.
Prepared public Git/GPG helpers follow65126 and65489: exact approved source/tool/config/argv allowlist, finite256 reservations including implicit children, one group<=2processes/one runnable thread each, stopped-before-user-code attachment, original incarnation/token/PGID/effect custody, exact possible-birth/exec/stop/exit/retired coverage. Charge conservative per-member preCONT-to-positive-stop/exit windows plus ALL contemporaneous controller/pipe CPU within1ms; unknown fork/exec coverage remains open until closed roster. Separate corroboration only after proven stop/closure, fresh debt after observation. Do not infer stop from a sent signal, close unknown slots from a census, omit late kernel/descendant CPU, signal retired identities or replace unknown effects. Main proof work is parked during helper grants, all debt paid before resume/proof/hash. Installed Xcode Git/GPG remain unqualified; feature/version/Trace2 snapshots are not lifetime proof.
Bootstrap65166 uses the one canonical128B permit/lease before owned argv/path/channel/Builder/runtime/source. Source-free probe has explicit context:null and permanently rejects every Start; real jobs require full current context. Final two bounded prepared public proof+transfer handoffs retain65259; no credential/signing probe, stale atomics/TTL substitute, new guard/hash exemption or recursive debt grant. Keep original250ms readiness/30s deadlines and actual escalation/reap/quarantine. New ordinary entry module is consumed only from the reviewed/landed producer, not a forked bootstrap parser or an old binary fallback.

## Shared C codec and B temporary, one authority each
Implement B from65960 and the current Signature temporary ICD: matcher {git_signature_temp} is never expanded/directly spawned; actual six-element Git-child argv retains its own exact scratch/.git_vtag_tmp[A-Za-z0-9]{6}. GS1 precedes first verifier ack/continue and checks bounded current-incarnation argv plus nofollow regular0600/euid/nlink1/exact inode/dev/size/hash and exclusive scratch; GS2 follows actual leader reap and checks deletion/original held inode/hash. Cancel cleanup only matches original dev/ino or quarantines. Preserve codes, charged reads/FDs/copies, actual valid/corrupt/ABA/symlink/hardlink/planted/extra/wrong-case/early-delete/kill controls and the explicit no-other-writer premise; no claim gpg opened that inode before observation.
[[decisions/d93377961366007-dream-c-closure-codec-authority]] supplies C completion. The EXISTING builder owns tests/fixtures/dream-proof-tools/closure-boundary.schema.json and closure-boundary-vectors.json, written/checked/signed BEFORE implementations build against them. You are explicitly authorized to consume that SAME signed source/hash artifact in your existing probe/read/resolve/c1/fixture/model scope when its handoff arrives. No competing shape from old conflicting prose and no wait for the quota-limited Architect. Preserve original Platform names plus required premise, exact ordered load multiplicity/mode/targets, candidate/static per-tool/map/source/file/UUID bindings, source-review boundary and before-birth/after-settle OS/cache/arch checks. All existing64node/64array/128edge/depth10/48KiBBasis/128KiBaggregate/16KiBdescriptor/32KiBlock/64KiBresult/32KiBreceipt limits stay. Fit remains unknown until genuine emission; unknown/foreign/noncache/direct-root libobjc-env remain Unsupported. This is conditional source/platform trust, not a native unseen-work or transient-cache theorem.
ONE canonical Q2 C probe/strict decoder/controller remains yours. The private-candidate absolute-pretestJSON expectedSHA mode enforces the complete strict one-manifest/six-identical-pin projections and exact seven-source/auth/build/config/command/fixture/review/custody/currentness binding, canonical aarch64 alias rules and acyclic hashes BEFORE birth. Normal git remains81; private raw results ALWAYS runtime_approved=false/conditional_source_review/whole_command_lifetime_proof=false/not_applicable_private_test. No self/future hash or approved:true bypass. Builder never forks your controller.
When source/schema/probe is coherent, send ONE exact signed source/executable/compiler/schema handoff to Root and the current builder assignee from task_status. Root freezes it BEFORE the real builder component witness; old5493 or moving source cannot substitute. Continue independent semantic/parser/refusal/model work meanwhile. Builder supplies genuine bound candidate/component evidence, then Root reviews any existing approved provisioning-source literal separately. No private success becomes a live lesson.

## Verification and landing
Original exact verify remains make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build. All original real qualifying signed-publication positive, corrupted-proof negative, complete-source/currentness/trust/graph refusal, joint parent+child/multiMiB/overflow/four-job/foreground/control/heap/actual-reap gates remain mandatory. Finite models retain explicit bounds/owners/atomic actions/fairness/abstractions and every deliberately broken control; full native/Yarn and pinnedJava TLC reachable-state+labelled-edge+named-verdict parity, no timeout/cap/incomplete/TypeOK/disagreement green. Reconcile shared C1 corrections without overwriting your local helper extraction; refresh exact source/model/tool receipts after rebase.
Run narrow affected checks while iterating and missing/stale final gates once, using current Make workflows and exact source-bound evidence reuse. No unrelated full suite or lucky timing retry. All tests are unattended with private homes/public-only fixtures; no operator credential, Keychain, desktop, login, click or physical step. Source/build/provisioner changes belong to the builder, not your tree.
After EVERY existing criterion and verify command passes, you may use current make land to push the signed cut to harness main without another operator/Root ACK. Makefile/scripts/land.py additions authorize consuming/preserving that workflow only, never weakening it. Verify origin/main contains the landing head. Report actual commands/exits, every acceptance line with source/log hashes, signed commits, worker/tool/proof/schema/model/profile bindings, genuine positive/negative and resource/settlement evidence, failed runs and unproven boundaries in reports/2026-11-06-dream-mcp-verified-snapshot-landed.md. Checkpoint when granted, then task_report once. Root independently verifies, accepts/closes/removes only after every commit is on main and no process holds the tree. No recruitment, unrelated features, installs, service/credential action or release.


## Signed single C codec is available (2026-11-06)
Root independently verified Boxy signature G on builder commit b26d7cd3432e9cae3e6c68492afd73a82cc8f2e9, exact schema15157B/SHA256d70948ec375dae099ea1dac0b92dfb30ff87d7fcac94c98f42caa1c0ba2da8ec and vectors3900950B/SHA256bff32c1afb483980952a6761918d4005c9f1e12b66f3d7aaaa88f1b9afe09740. They are tests/fixtures/dream-proof-tools/closure-boundary.schema.json and tests/fixtures/dream-proof-tools/closure-boundary-vectors.json in /Users/feral/mydev/bboxy450-harness-dream-proof-tools. Root reread the complete schema and reran the pure author supported-keyword/context checker:324 vectors44accept/280reject, exit0; independent canonical-byte, all-vector SHA/length,13context hash and uniqueness audit also exited0. External JSON Schema metaschema validation was not run (package unavailable); these remain synthetic codec facts, not real proof/source/runtime approval.
Consume these EXACT two signed blobs now under decision66007; their canonical paths are explicitly added to your scope for byte-identical import/adoption, never independent editing or a competing C shape. Cherry-pick that two-file commit or extract its exact blobs with preserved origin/signature/hash provenance; do not import unrelated builder commits. The new Platform6/candidate graph/per-output framing replaces superseded C pseudocode, which Root is synchronizing to this authority. Keep the distinction between whole-Closure, schema-fragment, grammar, member-framing and byte-accounting vector targets and their fixture-local contexts. The3.9MiB corpus is OFFLINE test data, never admitted prepared metadata or an extra runtime catalogue. Implement the same bounded decoder/currentness/model/control adoption and sign your source, with independent consumer checks against these exact vectors. Existing memory/CPU/debt/source/custody/qualification gates and all old-tuple stops remain. Continue without ACK; send the coherent source/executable/compiler/schema tuple once ready for Root's required pre-birth freeze. No real builder component, qualified lessons or runtime_approved=true follows from this handoff.


## Next source milestone: coherent private verifier input for the builder
Builder signed C emitter bec3baed578280d529b41706ac92f5bc672e1575 is source-ready. Root verified its signature and read c-source-evidence.json; its6442B/11node/16relationship C projection is incomplete metadata only, not a full Basis/qualification. Its remaining genuine dependency is YOUR signed coherent private verifier, so make this the next handoff after the current atomic source correction. No whole-Q2 final gate or real qualification is required before supplying this INPUT; those depend on it and remain later criteria.
Implement/adopt the already authorized signed C schema/vectors in the canonical proof-helper-probe.c decoder/currentness and its matching finite/causal controls. Then use/add a narrow canonical Make preparation target within existing Makefile scope to compile that EXACT signed C source with the established strict optimized C flags, record the full actual compiler executable identity/version/target/flags and SDK/platform/build inputs, and retain an immutable executable plus its source/schema/vector/hash receipts. Preparation must not execute a real candidate or self-approve runtime; existing process-free and already authorized synthetic source-known controls may verify the input. Preserve/record all actual exits and remaining failures. Keep unrelated unfinished source distinct; freeze a coherent SIGNED checkpoint of the C source and its required headers/inputs, not a moving worktree or old schema. Include exact snapshots of the existing proposal read-set resolve.rs/trust.rs/c1.rs/read.rs with their source-domain digest and provenance; distinguish that byte snapshot from any compiled/current-product claim.
Send ONE essential handoff containing commit, exact C/source-input paths+lengths+SHA256, immutable executable hash, compiler/build closure and source/codec/control exits to Root and builder01M6Q0FHRZV6MQA8PBPXG408N7. Root can then inspect/admit that concrete private input and amend builder canonical_q2 pins. No old5493, invented/future/self hash, actual real-pair birth, runtime admission or qualifying success is authorized by preparation. Continue remaining Q2 Rust/source checks after this handoff. This orders the existing dependency without adding a second controller, changing public wire or weakening final task acceptance.


## Retained-Host durable-close gate: exact shared compatibility scope
Root read current-c1-after-rebase.log:31/31 C1 and16/16 replay pass, but TWO retained-Host fixtures lose their subscribed durable SessionClosed at tests/seat_process.rs:983; no cause follows from that assertion alone. The same preserved-home/no-yolo case passes. This existing full gate remains FAILED2, no lucky rerun or dropped assertion. The retained-Host fixture is already part of your headless scope, so you now own this exact compatibility repair in the four additional listed paths, alongside already allowed src/frontends/headless.rs/model/maps. Restrict writes to the proven SessionClosed publication/subscriber-drain/socket-retirement/fixture-observation mechanism; no generic engine/gateway redesign.
First preserve owned private fixture logs/stderr/PID/actual close event/seq/ACK/EOF ordering on a close failure, including the original actual settlement. Distinguish durable publication failure, gateway delivery/drain and client observation; do not infer loss from EOF alone. Source points: headless.rs:734–761 retirement; gateway/unix.rs:95–149 finite lifecycle drain and:196–203 client event write; gateway/mod.rs Subscriber/Subscription drain+closed ordering; engine/mod.rs:500–536 seal→durable close→hub close. These are hypotheses to inspect, not an established cause. Correct the exact proven race/error with a deterministic held-order control and matching bounded lifecycle model/negative; keep one durable A close, exactly one nonfork B, same PID/Host/input/managed metadata, original 5s fixture and1s server drain budgets, no sleep/retry/widening or synthetic SessionClosed. No old stopped Dream worker/candidate is involved in the scripted fixture gate.
Preserve next priority: coherent signed C verifier INPUT handoff can proceed independently of this unrelated socket gate; do not make the builder wait for all of Q2's product verification. After materially corrected close source/control, run the focused retained-Host seat_process family through the existing canonical test-dream-c1 entry's fixture command, then the full original make test-dream-c1 as required before final landing. Keep all prior red evidence and causal distinctions. New source/codecs still require Root freeze before real builder qualification; this compatibility amendment grants none of that approval.


## Complete the private component path before the next input freeze
Root independently verified the Boxy signature on a3864e7ce09d5e0dc2018385118786d89b0c198e and exact C266098B/80ed140af40cefe6dc6bc84deb04aaf86b8222aa57a5eeac7db6cf56d5e50b56, executable181720B/17df1b6fb05c09115c0fc3baeb37ef8049b2d1fde6fa90c52585406185d0a530 and preparation284341B/9a141a88754aeffbb06d432b77da8956407cdbfe54b919be6ecbf9ff0705a68c. This is retained signed PREPARATION/codec evidence only. Your report correctly discloses the remaining refusals; it is not yet a functional real-pair verifier input, so no builder qualification is authorized with these bytes.
Exact source gap: private_prepare_inner:3353 unconditionally refuses after its checks; private_candidate:3488 never calls private_command_engine; engine_signature_boundary_available:3168 and engine exit:3269 unconditionally refuse, and engine_semantic forms2/3/8/10/12 are stubs. private_format_result always emits validate/refused/not_reached/created0. Root is not asking for another build of these same stubs. Finish this already assigned conditional private path NOW in your existing C/read/helper/fixture/model scope, independently of the retained-close repair. Retain every real validation and original-effect owner, replacing each unconditional placeholder with the actual missing bounded check/accounting/semantic implementation; deleting guards or returning true is not a correction.
The current canonical ICD b76981f3430d684eb0bdd4a5d7e5a1240c263543ace9083a258c39e4d7bc08c4 already permits private execution under its explicitly bound conditional source/platform premise BEFORE final qualification/runtime approval. Neither a future qualification/receipt nor a universal native-lifetime theorem may be required to enter this private path. Authenticate and cross-bind the existing complete pre-test tuple and actual candidate/config/fixture/platform/currentness; any genuinely missing predicate still refuses. Preserve typed custody through the engine, then settle/dispose and emit actual phase/outcome/proof/count/cost fields from that same invocation. Do not dispose input guards before their engine use or retain hardcoded zero/null-success counters.
Use the actual fixed corpus and current typed fields, without a new schema: both policy_branch and refs/remotes/origin/main name fixture.valid_commit even in the corrupt-signature case (builder fixture:1864-1865); selected commit is valid_commit or corrupt_commit by case. Check actual ancestry, record/blob and selected last-touch against the pinned presigned corpus; derive the selected tree from the pinned unsigned commit payload and compare actual tree_pin, with bounded parsing and object/content binding, not a new caller-provided expected-value flag. Preserve the existing exact source/corpus equivalence premise and real output/status checks. Signature form uses the accepted trust.rs:1062-1080 G/U plus pinned signer/primary rule for the valid case, actual failed cryptographic verification for corrupt_proof, and B's actual temporary/payload GS1/GS2 custody. A generic nonzero exit, missing verifier, parse/custody/resource/clock failure is NOT the expected corrupt-proof rejection. Do not manufacture status bytes or assume the proof from JSON. If a specific relation cannot be derived from these existing bound bytes, identify exactly that relation/field, not a blanket 'not baselined'.
Counts reflect physical effects including the implicit verifier, not only direct spawn calls: for an uninterrupted complete fixed case, fifteen direct leaders and one Git-created GPG verifier must be observed/settled with no retries or third process; all reservations/cost/late/retired coverage survive failure. The existing source premise remains conditional and runtime_approved=false forever in this result kind. Ordinary/private runtime admission remains separate; normal git stays81. No resource/native/deadline/wire/approval widening.
Before the next input handoff, use canonical narrow Make checks for malformed-input prebirth refusal, source-known semantic/engine/result and owned settlement controls plus deliberately skipped-engine/hardcoded-refusal/missing-verifier/corrupt-as-generic-error controls. Synthetic source-known evidence must be labelled as such; it does not replace the later real builder valid/corrupt qualification. Then sign a coherent new C checkpoint and prepare its immutable exact source/executable/compiler/header/schema tuple once through Make. No real builder candidate birth yet: Root reviews/freezes that changed functional input, then amends the builder. Keep a386 and all older failures intact; full Q2 verify/landing criteria are unchanged.


## Failed STOP terminal observation: decision66050
Implement [[decisions/d93377961366050-dream-failed-stop-independent-terminal-proof]] in the SAME existing Rust helper/canonical C/model/test scope. Root reviewed your retained1pass/2fail stdin-family log and current request_group_stop; those failures stay failed. A consumed EPERM/ESRCH may be PENDING TERMINAL observation, never stopped/ACK or errno-as-exit. Preserve raw signal/effect identity; admit no second STOP/CONT or next work while pending. Success requires the independent original unreaped WNOWAIT plus every retained member exit/RI/audit continuity, causally closed no-unknown roster, full original-window Q/controller/pipe/late CPU/debt and actual wait4/group settlement. End windows at positive proof, never at the failed signal. Unknown/over-budget/error stays refused/quarantined; identity-safe original cleanup remains. All1ms/native/deadline/heap laws and old failures remain. See the decision for exact causal model/broken/process controls; this is a scoped terminal-classification correction, not a blanket errno exception or installed-kernel claim.
After that source/model/control correction, use one focused canonical stdin/helper-transport test family with numeric raw signal/window/cost/reap evidence; no identical lucky retry. Continue the already assigned functional private engine/result milestone, source-known controls and signed new input preparation. This decision grants no real builder candidate birth/runtime approval.
Root's current live-tree review also found 0fd56ba5 not descended from already landed harness main2ce4359df034405043d419fe550b0f6cafe7887e; origin/main in your tree already names2ce. The new current-c1-after-close-repair.log is RED30/31, failing the old parent native-birth predicate before it reaches seat_process; it does not verify or disprove your close fix. Before another whole C1/final gate, checkpoint your scoped helper changes and rebase on the landed2ce main, preserving your extracted helper implementation, entry/writer/control changes and observer evidence. Do not restore source/stash/drop controls or rerun the old birth fixture unchanged. That consumes the upstream numeric diagnostic correction instead of branching another copy. After integration use only missing/stale required checks, preserve exact signed source/artifact receipts and retain each red run. Continue without operator/ACK wait; final original verify/landing remains unchanged.

## Oracle ruling, live custody after quarantine: RESOLVED (decision d93377961366074)
Read [[decisions/d93377961366074-2026-11-06-dream-q2-quarantine-owner-stays-alive]]. The SAME controller stays alive as the owned-quarantine holder (65126 "retain a quarantined OWNED slot"; 65489). It writes the refusal on the existing result wire promptly, then holds FDs, the unreaped child and PID/PGID pins with no new work, only paced ≤1ms settlement re-observations with 9x rest, sleeping between them. It exits only after its own reap, group-gone and the 66050 terminal proof. Owner loss is recorded as quarantined/owner-lost, never success. No custody receiver, manager, transfer or wire change; the atexit release on the quarantine path is the bug. Add a model invariant, a broken control (release or exit while quarantined must fail) and a process witness. This is no longer an open decision: implement it in the same C/Rust scope, then send the post-fix tuple.
Acknowledged: 6fe975f1 / 089a61f5 / 364fe0b7 are RETIRED (stdin write fence after a failed STOP). The freeze happens ONCE on the post-fix tuple.
(Appended by Oracle seat 01M6RZRGC3194VVFBC4M7XW4MF as a direct edit because the DM rulings are native-queued and unconsumed: messages 01M6S2EV…, 01M6S3C0….)

## Oracle rulings (latest, 2026-11-06 ~20:55); read these, DMs are arriving late
1. **The freeze ALREADY happened.** The 30f27ccc / 140e808b / 76f35824 / 76d579c2 tuple was frozen into the proof-tools task. The builder (seat 01M6S9GNN1HB7PW20DSH1SY8BG) then REFUSED ingestion for three gaps, which are YOURS to fix: (a) the preparation root is 0755 but 0700 is required; (b) the receipt's compile-052o1xig/{proof-helper-probe.c,probe} files are absent; (c) the pthread.h dependency lists only pthread/pthread.h in header_inputs (incomplete closure). Re-prepare the SAME signed source with make prepare-dream-read-c-probe so the retained tuple is complete and cross-bound, then send the corrected receipt hash. If the C source changes, send the whole tuple.
2. **Granted** `tests/support/seat_socket.rs`, ONLY for bounded stage/output/reap failure evidence and owned-child settlement. The 2 s assertion stays.
3. **Helper family 9/12 RED.** Classify each failure: (a) if CPU accounting is inconsistent (CPU vs CPU: e.g. A=186584ns > W=142000ns, if both are measured CPU of the same window), it is a REAL defect, so fix it; (b) if only wall time or preemption under machine load is involved (e.g. a Before-fault preempted by Q=1.459ms while CPU is within budget), it is load. Load-only witnesses get `#[ignore = "load-sensitive wall/preemption window; re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp"]` with assertions unchanged, the same as the operator-approved C1 shortcut. The stopped-admission guard exit 92 is NOT timing: diagnose and fix it.
4. Then land. Operator priority: get it done; no lucky retries.

## Oracle ruling (latest, ~22:25): the frozen C has a terminal-proof defect, so fix it and re-freeze
The proof-tools builder (seat 01M6S9GNN1HB7PW20DSH1SY8BG) found it in frozen C 140e808b (the corrected a4998fb7 ingestion itself passes):
- `private_fail` (~line 558) preserves only the FIRST error;
- `private_emit` (~4105–4133) does not report whether the result was persisted;
- exit-tail work (~4198–4200) happens AFTER the final raw receipt.
So exit 81 conflates the intended control failure with additional persistence or terminal-cost failures, and the builder cannot label those controls green.
**Required:** every exit-81 case reports independent, complete verdicts: (1) the intended control verdict, (2) the persistence verdict (persisted / not persisted / unknown), (3) the terminal/cost verdict covering ALL tail work, with the final raw receipt written AFTER the last tail work. Keep every error, not just the first (bounded list). No controller or schema fork, and no limit waiver. Add a source-known control per verdict axis. Then send the NEW full tuple (source commit, C SHA, executable SHA, receipt SHA); the Oracle re-freezes once.

## Oracle ruling (latest, ~23:10): SECOND defect for the same re-freeze, member-window quantum
The builder's private positive component run on the frozen probe (run 01M6SJ1GZKFB1C2KJ8CJ5PSBP0, retained at /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6SJ1GZKFB1C2KJ8CJ5PSBP0/components/positive/) exited 81 resource_limit: **Q_max = 8,785,417 ns > 1,000,000 ns**. Native CPU 218,875 ns, wall 218,000 ns and inclusive 291,125 ns all passed; one helper created and reaped; group_gone=true; retired_coverage=true; 18.88 s of debt charged and paid.
So a member window is not closed or split at the unchanged 1 ms law: some controller work runs as one 8.8 ms unit. Fix the member-window closure so every unit is ≤1 ms with its 9x rest (decisions 65086/65489; no new native tag, threshold or waiver). Add a source-known control that fails at >1 ms.
Fix BOTH this and the exit-81 terminal/persistence verdicts in the same change, then send ONE new full tuple; the Oracle re-freezes once.

## Oracle rulings (latest, ~23:55), operator order "speed up, cut corners, just get it done"
1. **Granted** tests/formal/dream_c1.rs (the ABA control 9 verdict-progress repair at epoch 0), tests/acp_launch_context.rs and tests/fixtures/acp-agent/launch_parity.py (keep the Python-MCP failure evidence), for the narrow repairs in your report only.
2. **A=139625 > W=120000 and the remaining 250 ms Hello witness:** at load ~18–20, default-QoS threads run on E-cores and measured CPU and wall time inflate, the same family C1 hit. If W is a fixed budget and A is measured CPU for that window, it is load: `#[ignore = "load-sensitive budget/wall window; re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp"]`, assertions unchanged, listed in the report. A real accounting INCONSISTENCY (A counting work outside its window, or W computed wrong) is a defect to fix.
3. **Real builder/profile prerequisites** are NOT required for Q2 landing. Qualification belongs to the proof-tools task. Q2 lands source with runtime_approved=false.
4. **STILL REQUIRED before landing:** the two builder-found C defects (exit-81 independent control/persistence/terminal verdicts; member-window Q_max 8.79 ms > 1 ms). Fix both in the C source, land with them, and send the new full tuple so the proof-tools builder can re-freeze.
5. artifact_claim rejection is the known bboxy bug (tasks/2026-11-06-artifact-claim-after-commit). Ignore it; task_report with the report locator is enough.

## Oracle CORRECTION (latest, ~00:10): the member-window "defect" is RETRACTED
Q2's ledger evidence (grant 41 / form 13, SHA 151dad80…): W=8,381,000 ns is elapsed WALL from STOP sent (t0+0.1 ms) to positive ACK (t0+8.381 ms), with no rest in that path; CPU is A=6,667 ns plus C=404,417 ns. The Rust W=120,000 is likewise measured wall, and A=139,625 is an interval upper bound. So the controller's refusal was CORRECT: under load the stopped member acknowledges late, and decision 65489 forbids splitting, backdating or subtracting unattributed time. My earlier "member window not closed at the 1 ms law" ruling misread wall as CPU and is withdrawn.
**Ruling (Q2's option 1):** preserve the truthful late-STOP / unattributed-CPU refusal; add prompt lower-bound guards, bounded closure diagnostics and causal controls; DEFER the physical timing witnesses that need a quiet machine (`#[ignore]` with the QoS reason, re-enabled once builds run at background QoS). No threshold or endpoint waiver, and no new attribution boundary. The ONLY remaining C fix before the new tuple is the exit-81 terminal/persistence verdict split, plus the ABA and ACP repairs already in flight.

## Oracle amend (2026-11-07 ~07:45): implement Architect decision 66236 (measured quantum + wall ceiling)
Read [[decisions/d93377961366236-dream-q2-measured-quantum-and-wall-ceiling]] and the amended lines in [[specs/dreaming-mcp]] / [[interfaces/dreaming-mcp]] (re-signed by the Oracle). Summary:
- Once closure is PROVEN for a grant (every slot positively ACK-stopped or exited, roster closed, no unknown slot, counters read post-ACK, A_slot ≤ W_slot), charge **Q = A_measured + C_active ≤ 1 ms**, where A_measured is the kernel per-process CPU delta plus the reaped-descendant roll-up. Unproven coverage keeps the 65489 charge max(W,A)+C.
- New hard gates: **W ≤ 25 ms wall per grant**, and **terminal conservation** (post-reap wait4 ≤ Σ charged CPU + allowance).
- The 1 ms law, 9× debt and all other gates are unchanged. Runs 1 and 2 stay FAILED (not reclassified).
- The separate **"ordinary engine unit" resource_limit** is a real overrun: fix it (split it under 1 ms with rest).
Implement in the same C/Rust controller scope, with model updates and broken controls per the decision (a closure-unproven grant charged as measured must fail; W > 25 ms must fail; a conservation violation must fail). Then send ONE new full tuple (source, C, executable, receipt) for re-freeze; a fresh builder runs the qualification. This supersedes the earlier "send tuple then land" ordering only by adding this change before the tuple.

## Oracle ruling (2026-11-07 ~10:00): two defects from the builder's positive run on frozen 39580f0a
Builder seat 01M6TPMT…, run 01M6TPSX095ZEEF53Q73KCW8N6, ledger 4 (SHA 2132c4e2…), exit 81; controller and helper reaped; terminal cost passed.
1. **Ordinary unit overrun:** `engine_stopped_corroboration` = 1,164,833 ns > 1 ms (before grant 1). Split it into units of **≤0.5 ms nominal**. The headroom is deliberate: Dream work runs at low QoS by design and is often placed on E-cores, where measured CPU per unit inflates by roughly 2×. No threshold change, no new tag.
2. **Terminal conservation excess:** post-reap wait4 202,000 ns > Σ charged 116,709 ns (+85,291 ns). This is the 66236 gate doing its job: ~85 µs of real CPU is not being charged. Find the uncharged source (CPU between the last counter read and reap, a missing descendant roll-up, exec/teardown, or the implicit-child roll-up conditions) and charge it honestly, or close the gap so conservation holds. Do NOT widen the allowance to absorb it unless the decision's allowance definition explicitly covers that source; if unsure, DM the Oracle with the breakdown.
Add a broken control for each. Then send ONE new full tuple for re-freeze; the same builder seat reruns.

## Oracle ruling (2026-11-07 ~13:00): grant sizing, from the builder's run on frozen bec674fb
Run 01M6V1HTRXVYXXW9CK3CWM2CT2 (ledger SHA c7a63a73…), exit 81. Progress: conservation PASSED with fixed T (residual 120,401 ns paid); ordinary stopped units max 221,667 ns (the split worked); controller and helper reaped; terminal cost passed.
**Remaining defect (grant sizing, NOT a law change):** grant 51 measured Q = A 932,917 + C 245,583 = 1,178,500 ns > 1 ms, with W = 1,114,000 ns. The member really burned 933 µs of CPU because the controller let it run for ~1.1 ms before STOP took effect. Fix the grant scheduler so measured A + C ≤ 1 ms with headroom:
- Size each CONT→STOP run window from the measured controller cost and the observed STOP latency, so expected A ≤ ~0.55 ms when C ≈ 0.25 ms (i.e., a run window of roughly 0.35–0.45 ms, before STOP latency).
- Adapt: if a grant's measured A exceeds the target, shrink the next window (bounded multiplicative decrease); grow it slowly back toward the cap when A is well under. Never grow above the cap.
- A grant that still exceeds 1 ms stays an honest refusal (no clipping or credit).
- Also explain and fix the second resource_limit ("closure + finish_grant supplied cost"); it is likely the same grant's accounting, otherwise report its breakdown.
Add a broken control: a fixed oversize window (≥1 ms) must fail. Then send ONE new tuple.

## Oracle ruling (2026-11-07 ~14:50): oversize-window broken control
Evidence: the C15 canonical run is red 2 (log SHA 9c9dc387…, raw c50b9492…). The PHYSICAL fixed-oversize mutant was refused BEFORE its causal assertion by a different gate: native creation wall 98.854 ms > 25 ms (CPU 337,083 ns), i.e. environment/load, not the property under test; original 25234 own-wait4 and group-gone, debt paid. The physical POSITIVE scheduler PASSES (Q = 805,375 ns, W = 438,000 ns, own wait4). The supplemental C API control PASSES (exits 0/0/0/81; raw ae17634d…): the real allocator functions reject a fixed 1.2 ms bound, with independently supplied CPU checking full Q and debt.
**Ruling: YES.** The canonical broken control for scheduler-fixed-oversize is the separately labelled REAL C API causal control (the same allocator code, deterministic). Keep the physical positive as canonical. The physical NEGATIVE is labelled DEFERRED/UNPROVEN under the QoS follow-up, with its original caps, assertions and red receipt retained unchanged, and is NOT a landing prerequisite. Do not relabel the API control as a physical-CPU witness. Re-attempt the physical negative later on a quiet machine as a follow-up (no waiver, no change to its caps). Proceed to the full tuple and landing; runtime_approved stays false.

## Oracle amend (2026-11-07 ~16:40): implement Architect decision 66280 (external-helper TOTAL budgets; 1 ms slicing REMOVED for external helpers)
Read [[decisions/d93377961366280-dream-q2-external-helper-budget]] in full (its clause table says what stays and what is superseded) and the updated lines in specs/dreaming-mcp.md and interfaces/dreaming-mcp.md. Summary:
- External Git/GPG helpers are NOT sliced into 1 ms grants. Spawn them with posix_spawn, SETPGROUP and background QoS; the observed priority 4 is a HARD gate.
- CPU caps: ≤250 ms per command and ≤3 s per job, enforced by live RI reads plus SIGKILL. The leader's own wait4 is the terminal verdict. The job pays 9× rest debt on that CPU AFTER the reap. Wall time is bounded by the existing 30 s deadline. Memory keeps the existing shares plus terminal ru_maxrss ≤16 MiB. Foreground handling is ONE SIGSTOP pause inside the existing 100 ms gate.
- The 65086 1 ms law is UNCHANGED for all work on our own threads. Superseded for external helpers: the 65126 per-grant section, and 65489 and 66236 (including 66236's Oracle clarifications 1 and 2, T and N_reaped).
- **Settlement fix:** kill(-pgid) returns EPERM on a zombie-only group; the probe (~4383–4389) discards the errno and never retracts the unknown after its own reap and group-gone. Apply 66050's pending-terminal state to the final kill.
- New model Q2HelperTotalBudget: 5 invariants and 8 broken controls, each with a real witness (see the decision).
**Simplify:** REMOVE the per-grant scheduler, window sizing and member-window accounting for external helpers rather than leaving dead paths. Old failed runs stay FAILED. Then send ONE new full tuple for re-freeze and land; runtime_approved stays false.

## Oracle ruling (2026-11-07 ~17:30): TLC heap OOM on member-window graphs
Java TLC ran out of memory at -Xmx256m on HarnessDreamReadMemberWindowGraph and five sibling member-window graph models (HEAD bd0b4c56; log 6e0edd93…). Those models encode the MEMBER-WINDOW accounting that decision 66280 SUPERSEDES for external helpers, and your 66280 brief says to REMOVE that machinery. So:
1. Implement 66280 FIRST; delete the superseded member-window/per-grant models together with the code (update the corpus/coverage/contracts inventories honestly). Do not spend heap on models that are leaving.
2. ONLY IF a model that REMAINS after 66280 still OOMs at 256 MiB: you are pre-authorized to change ONLY the `-Xmx256m` in `scripts/formal.py` (now in allowed_paths) to `-Xmx512m`. Keep the pinned Java/JAR, full states, labelled edges, named verdicts, every model and control, the 60 s timeout, the native 100000 cap and the existing worker count unchanged. Record before/after peak RSS, and name the model that needed it.
OOM runs stay FAILED.

## Oracle ruling (2026-11-07 ~23:10): first 66280 positive: budgets OK, result path broken
Builder run 01M6W168YT0EFZRE5EHTE2MYTW on frozen 557c8b47 (ledger SHA c1a78d9d…, stderr 89857544…), exit 81, controller 45375 reaped. **What WORKED (66280):** two commands at priority 4, wait4 21.965 ms and 20.967 ms, job 42.932 ms (far under 250 ms and 3 s), 25.84 s of debt paid, helpers 2/2.
**What FAILED (functional, fix it):** raw stdout 0 bytes and result.json absent; ledger: terminal deadline (18), settlement_unknown ×1 on STDOUT EMISSION, source_changed ×1 on the DESCRIPTOR PATHNAME. Diagnose each against the ledger and stderr:
1. **source_changed on the descriptor pathname:** decide whether the descriptor path the builder supplies is legitimately re-created (for example a temp path re-linked between check and use) or whether your identity check compares the path instead of a held fd/inode. Fix the real cause (bind to the opened fd/inode identity at admission; the path is only a locator). Do not weaken the change detection.
2. **Terminal deadline (18):** which deadline expired, and why, given the helpers finished in about 43 ms of CPU? Possibly the source_changed refusal path waited on something; report the timeline.
3. **settlement_unknown on stdout emission:** the final result was never written or flushed; make the result emission settle (or refuse honestly) on every terminal path.
Add a causal control per fix. Then ONE new tuple; the same builder seat (01M6W0XS…) reruns.

## Oracle ruling (2026-11-08 ~00:40): upstream formal-script pin refresh AUTHORIZED
The mandatory source_pins gate failed because term main moved. The Oracle verified that signed term main 96dbc71ba3631ccb7907b3449ca76dd0f6e83749 (G/Boxy, on origin/main) has scripts/formal.py SHA-256 5097490db6569929a1e0a10061fb86a88c06a16170fb6534c134c9fcfbf0c27a and scripts/test-formal.py 11c3d433fab19fd3f27565ed85c98cb84b9294756447d2113a1886f6acf677c4, exactly matching your proposal (proposal.json SHA e1cf0d39…). Granted ONLY for this provenance refresh: verification/upstream-sources.json, verification/upstream/scripts-formal.py, verification/upstream/scripts-test-formal.py, with exactly those proposal bytes. The checker, exporter, heap, workers, timeouts, tool pins and native cap stay unchanged; the snapshots are hash-checked records, not the active checker. Sign it, finish the current formal and full gates, then land.

## Oracle ruling (2026-11-08 ~01:40): rest-debt formula bug in the 66280 implementation
Builder run 01M6W6RKAZEN3P4CHXXEPKPQNS on frozen 79fa0d9d (ledger …/runs/01M6W6RKAZEN3P4CHXXEPKPQNS/components/positive/output/ledger.ndjson, 31 lines). **The result-path fix WORKS:** stdout and result.json match, persistence=persisted, errno 0, and 6/6 helpers settled. It still hit the 30 s deadline (18) before proof.
Oracle ledger summary (6 helper records): terminal_cpu_ns max 22.611 ms; job_cpu_ns max 64.902 ms (sum 279.9 ms); wait4 user+system sum 64.9 ms; **debt_charged_ns = debt_paid_ns = helper_debt_target: max 25.091118 s, sum 109.636938 s**. Decision 66280 charges 9× the measured CPU: 9 × 64.9 ms ≈ 0.58 s per job, not 25 s. Debt is about 386× the CPU (25.09 s / 9 ≈ 2.79 s looks like WALL time or a cap, not CPU). With ~110 s of debt and a 30 s deadline, the job cannot finish. **Fix:** compute helper rest debt as 9 × the measured CPU (terminal wait4 user+system / roll-up) per 66280; find which quantity is used now (activation→reap wall? job_cap?) and correct it. Add a causal control: a helper with X ms CPU and a long wall must charge 9·X, not 9·wall. Then ONE new tuple.
**Oracle correction (~01:50):** the ledger debt fields are CUMULATIVE per record, so the "sum 109.6 s" above double-counted. The builder's report gives the FINAL debt as **25.829118 s** for a total wait4 CPU of **64.902 ms**: about 398×, against 66280's 9× (≈0.584 s). The bug and fix stand: 25.8 s of debt plus the work consumes the 30 s deadline.

## Oracle ruling (2026-11-08): known-overload ignore for one bootstrap test

Added `tests/fixtures/dream-bootstrap/process.rs` to allowed_paths. The ONLY authorized change there is adding
`#[ignore = "load-sensitive budget/wall window; re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp"]`
to `bootstrap_held_birth_has_zero_admission_and_independent_probe_then_real_completion` (process.rs:218-219),
exactly as in Q2's proposed-only-bootstrap-overload-ignore.patch. The body, assertions and the 1 ms / 250 ms / 30 s budgets stay unchanged.
Witness: make land source56 exit 2 (log 4e4a146d…7153): control CPU 1,012,958 ns against the 1 ms limit, under load; owner reaped, debt paid.
The deferred count goes from 23 to 24. This test counts as deferred, never as passed.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed HEAD d1fe3f4b20fe9b34e855215f1eb37227d9972a61 on origin/main; post-push containment0 and clean worktree. Current C1/MCP/strict gates and63 integrations passed;432 models/93 contracts match3,351,917 states/11,296,388 labelled edges and expected verdicts. Canonical source-bound archive/landing completion exited0; original make-land/parity ENOSPC exits2 and all historical failures remain preserved.25 task-specific tests are deferred (20 QoS,5 qualification), runtime_approved=false. Full append-only report read back and committed as native checkpoint1d10b85e465d59e25927435ab593f6d5d1169f7a, with14 criteria, exact source/tool/profile/schema/worker/archive hashes and remaining limits.

```json
{
  "op_id": "01M6WJME4WNC97YFH2R8K8AJSN",
  "commit": "d1fe3f4b20fe9b34e855215f1eb37227d9972a61",
  "report": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final.md]]",
  "verify_result": {
    "command": "env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 python3 -I -B target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/complete-verified-canonical-land-v57.py",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "wire: Distinct prepare-read wire, exact context/manifest binding and bounded immutable pages/descriptor.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/frontends/headless.rs:32 SHA256 dc38f1ec5d17cbb719065e792e1ce14f5d4cbbe2cfc300922c69457b75b300e7; src/dream/read_wire.rs:103 SHA256 dc215be4c30e64211e27094f34adc4a4df9e19fa5ae0dc07f2948c2716a94028"
      },
      {
        "criterion": "bundle: Finite selected IR/JSONL/blobs, private Git, public policy/keys, catalogue and artifact-to-file custody; no arbitrary receipt paths/fallback.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/read.rs:295 SHA256 865040d1cf4377fa195275e267b376a4f89be8137195a6edc100bbd3ffedba04; src/dream/read.rs:738 SHA256 865040d1cf4377fa195275e267b376a4f89be8137195a6edc100bbd3ffedba04"
      },
      {
        "criterion": "published_lessons: Compatible adoption, current record/trial/publication/receipt/trust/transitive proof, safe lesson fields and verified_at_revision.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/read.rs:2286 SHA256 865040d1cf4377fa195275e267b376a4f89be8137195a6edc100bbd3ffedba04; src/dream/recall.rs:493 SHA256 3db23a151aef40629e47dbee42c276687611c9e3851b8f02a8b4ef03d5b661ad"
      },
      {
        "criterion": "resource: Shared64MiB input; parent32MiB/child96MiB, protected control/helper shares, bounded allocation/disposal, four jobs and actual joins.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/read_wire.rs:14 SHA256 dc215be4c30e64211e27094f34adc4a4df9e19fa5ae0dc07f2948c2716a94028; src/dream/c1.rs:32 SHA256 7bec4a5b6ff28498c7b6a4eabb317e30812d17b7be39b759b275c16255400654"
      },
      {
        "criterion": "cost_debt: Own-thread ordinary1ms/native2msCPU25mswall3msinclusive/full9x debt unchanged. Decision66280 removes external per-grant/Q/T/N; helpers run continuously at atomicBACKGROUND/observedPRI4 with250ms command/3s job totalCPU, actual per-member RI begin-to-begin+return gaps<=50ms, terminal originalwait4/rawovershoot/fresh9x postreap debt/terminalRSS16MiB.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/c1.rs:748 SHA256 7bec4a5b6ff28498c7b6a4eabb317e30812d17b7be39b759b275c16255400654; src/dream/c1.rs:2428 SHA256 7bec4a5b6ff28498c7b6a4eabb317e30812d17b7be39b759b275c16255400654"
      },
      {
        "criterion": "bootstrap: Canonical128B permit/lease, unchanged250ms readiness and30s maximum work deadline; no argv/channel/owner/source admission before permit, null-context probe rejection and actual settlement.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/bootstrap.rs:16 SHA256 f6183cfa1f3eea408a853e171f44d456e5e8bf9c177fc088cc184634642aab64; src/dream/bootstrap.rs:80 SHA256 f6183cfa1f3eea408a853e171f44d456e5e8bf9c177fc088cc184634642aab64"
      },
      {
        "criterion": "helper_custody: Original positivebirth/audit/PGID custody, bounded2members, positive ownWNOWAIT/actualownwait4, known-native census status, retainedchild positive exitedRI then fresh sameleader qualified componentwise rollup; live/escaped effects before reap seal, independentgroup proof/ownedquarantine. KnownleaderCPU paid even if unknownchild held; no postownreap signals.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/c1.rs:829 SHA256 7bec4a5b6ff28498c7b6a4eabb317e30812d17b7be39b759b275c16255400654; src/dream/c1.rs:1902 SHA256 7bec4a5b6ff28498c7b6a4eabb317e30812d17b7be39b759b275c16255400654"
      },
      {
        "criterion": "signature_B: Git-created verifier temp argv/custody GS1/GS2; matcher never directly expanded; original inode/hash/scratch cleanup or quarantine.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; tests/fixtures/dream-read/proof-helper-probe.c:3351 SHA256 8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351; tests/fixtures/dream-read/proof-helper-probe.c:3468 SHA256 8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351"
      },
      {
        "criterion": "shared_codec_C: Exact signed shared schema/vectors, bounded strict decoder and seven-source/private pre-test bindings, no runtime self-approval.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; tests/fixtures/dream-read/proof-helper-probe.c:723 SHA256 8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351; tests/fixtures/dream-read/proof-helper-probe.c:4000 SHA256 8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351"
      },
      {
        "criterion": "terminal_verdicts: Independent control/persistence/terminal-cost verdicts, bounded retained errors and post-tail receipt; fixture setup cost cannot disappear. Effective23:10 fixes bind admitted inputFD+freshhash independently of replaced locator; distinguish native IO failure from paid work-deadline refusal; one-way terminal reporting cannot admit work and attempts finite/paid result after original30s expiry. Bounded path batching retains full actualCPU/debt.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; tests/fixtures/dream-read/proof-helper-probe.c:3867 SHA256 8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351; tests/fixtures/dream-read/proof-helper-probe.c:3937 SHA256 8a99da16ce02ce018eee5355f07620f62ebd4917a7007a6c92e2813e53711351"
      },
      {
        "criterion": "retained_host: One durable A close, finite subscriber/socket drain and retained Host creating one nonfork B with managed launch inputs.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/engine/mod.rs:510 SHA256 d9a65f538faa097adbd33a076368c9725e89cfc71a0815292272003e6215fa9e; src/gateway/unix.rs:156 SHA256 c2a7c5b13fd8c5ba70c33756aae4d5de0252a1f21fa974e6986b3bbf26a619e0"
      },
      {
        "criterion": "formal: Finite owner/actions/bounds/fairness and all deliberately brokencontrols; honest66280 retirement/inventory. Exactnative cap100000 and pinnedJava fullstates/labellededges/namedverdictparity. Incomplete/statecap/OOM is failed; no sourcecurrent claim before freshcanonicalgates.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; tests/formal/dream_read.rs:2625 SHA256 c02ee30b512ae15ae2335c5db76b4799aa54c67c5c9d709998344f6bfa01944a; Makefile:57 SHA256 c13f129142ca8a7ff34ad08e363dd8e73b98ab1f8ea08b9966f6b75b235e50de"
      },
      {
        "criterion": "provenance: Signed exact source, current worker/compiler/source-input profiles and formal model/tool evidence; immutable Cpreparation/source4readset/schema/vector/compilerheaderclosure tuple separate from runtimequalification. Retain all failed runs/sourcecuts; no oldtuplecurrent.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; src/dream/read.rs:2648 SHA256 865040d1cf4377fa195275e267b376a4f89be8137195a6edc100bbd3ffedba04; src/dream/read.rs:972 SHA256 865040d1cf4377fa195275e267b376a4f89be8137195a6edc100bbd3ffedba04"
      },
      {
        "criterion": "landing: Canonical current-source gates and remote main containment before final handoff.",
        "reference": "[[reports/2026-11-08-dream-mcp-verified-snapshot-landed-final]] — Acceptance evidence; R/landed-source57-acceptance-source-map.json SHA256 3cdcdb0d5ce5c87ff9b153c7a496f0fb1894315f3c0000d95e1651dc43a408d0; scripts/land.py:112 SHA256 36bd8eb4b0d6bcba5efd46ee94d5d0f035bb3c5ddb5c12aa02efb8dc0a859354; scripts/land.py:207 SHA256 36bd8eb4b0d6bcba5efd46ee94d5d0f035bb3c5ddb5c12aa02efb8dc0a859354"
      }
    ]
  }
}
```


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Corrective signed commit ba22c67c488421f58faff3e5256f7fcfff6a5ab8 landed on origin/main with a clean worktree. Production tree_pin argv now preserves ^{tree}; positive0 and restored-old-reject81 are checked in the current canonical C15 suite (68 baseline, 40 mutants, 42 compiles, 86 commands). Fresh current C1, MCP, full native/TLC432-model parity and embedded gates all exited0 on the f887 worker; unmodified canonical strict/nextest/validator/fetch/push completion exited0. Replacement immutable Input58 was handed off as1408. Full committed correction report supersedes the prior final report; native checkpoint4c8b296a86502102a62ac82f7b5083c0bfbe2435. Existing25 task-specific deferrals remain non-passes; real full-private30s fit, tool qualification and runtime approval remain UNPROVEN/false; no genuine builder candidate was created.

```json
{
  "op_id": "01M6WQA0XAQA341PD286S0WAA5",
  "commit": "ba22c67c488421f58faff3e5256f7fcfff6a5ab8",
  "report": "[[reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md]]",
  "verify_result": {
    "command": "env -u FORMAL_JOBS RUSTC_WRAPPER=/usr/bin/nice RUSTC_WORKSPACE_WRAPPER= CARGO_INCREMENTAL=0 python3 -I -B target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/complete-current-canonical-land-v58.py",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Distinct prepare-read wire, exact context/manifest binding and bounded immutable pages/descriptor.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#wire; src/frontends/headless.rs:32; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Finite selected IR/JSONL/blobs, private Git, public policy/keys, catalogue and artifact-to-file custody; no arbitrary receipt paths/fallback.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#bundle; src/dream/read.rs:295; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Compatible adoption, current record/trial/publication/receipt/trust/transitive proof, safe lesson fields and verified_at_revision.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#published_lessons; src/dream/read.rs:2286; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Shared64MiB input; parent32MiB/child96MiB, protected control/helper shares, bounded allocation/disposal, four jobs and actual joins.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#resource; src/dream/read_wire.rs:14; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Own-thread ordinary1ms/native2msCPU25mswall3msinclusive/full9x debt unchanged. Decision66280 removes external per-grant/Q/T/N; helpers run continuously at atomicBACKGROUND/observedPRI4 with250ms command/3s job totalCPU, actual per-member RI begin-to-begin+return gaps<=50ms, terminal originalwait4/rawovershoot/fresh9x postreap debt/terminalRSS16MiB.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#cost_debt; src/dream/c1.rs:748; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Canonical128B permit/lease, unchanged250ms readiness and30s maximum work deadline; no argv/channel/owner/source admission before permit, null-context probe rejection and actual settlement.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#bootstrap; src/dream/bootstrap.rs:16; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Original positivebirth/audit/PGID custody, bounded2members, positive ownWNOWAIT/actualownwait4, known-native census status, retainedchild positive exitedRI then fresh sameleader qualified componentwise rollup; live/escaped effects before reap seal, independentgroup proof/ownedquarantine. KnownleaderCPU paid even if unknownchild held; no postownreap signals.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#helper_custody; src/dream/c1.rs:829; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Git-created verifier temp argv/custody GS1/GS2; matcher never directly expanded; original inode/hash/scratch cleanup or quarantine.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#signature_B; tests/fixtures/dream-read/proof-helper-probe.c:3353; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Exact signed shared schema/vectors, bounded strict decoder and seven-source/private pre-test bindings, no runtime self-approval.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#shared_codec_C; tests/fixtures/dream-read/proof-helper-probe.c:723; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Independent control/persistence/terminal-cost verdicts, bounded retained errors and post-tail receipt; fixture setup cost cannot disappear. Effective23:10 fixes bind admitted inputFD+freshhash independently of replaced locator; distinguish native IO failure from paid work-deadline refusal; one-way terminal reporting cannot admit work and attempts finite/paid result after original30s expiry. Bounded path batching retains full actualCPU/debt.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#terminal_verdicts; tests/fixtures/dream-read/proof-helper-probe.c:3869; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "One durable A close, finite subscriber/socket drain and retained Host creating one nonfork B with managed launch inputs.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#retained_host; src/engine/mod.rs:510; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Finite owner/actions/bounds/fairness and all deliberately brokencontrols; honest66280 retirement/inventory. Exactnative cap100000 and pinnedJava fullstates/labellededges/namedverdictparity. Incomplete/statecap/OOM is failed; no sourcecurrent claim before freshcanonicalgates.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#formal; tests/formal/dream_read.rs:2625; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Signed exact source, current worker/compiler/source-input profiles and formal model/tool evidence; immutable Cpreparation/source4readset/schema/vector/compilerheaderclosure tuple separate from runtimequalification. Retain all failed runs/sourcecuts; no oldtuplecurrent.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#provenance; src/dream/read.rs:2648; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      },
      {
        "criterion": "Canonical current-source gates and remote main containment before final handoff.",
        "reference": "reports/2026-11-08-dream-mcp-tree-pin-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source58-current-acceptance-source-map.json#landing; scripts/land.py:112; current canonical receipt SHA256 46e3b9a3b3a9f12b64fb0b9c3d61298dcfc78b7c08568d8ef4ce9ad52a1a09e6; explicit deferrals and unproven boundaries retained"
      }
    ]
  }
}
```


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed af5de27c33d46cdcb04510e863ef95fdc835c3ec on origin/main, clean, including strict parser cut a7c08e65df64c8ed8a4a0ce3ad9daf96cba90373. Only registered terminal ^{tree} stays literal; unknown/malformed braces refuse. Current C17 canonical suite0: 68 baselines, 41 source mutants, 43 compiles, 88 commands. The EXACT task verify command actually returned0 (log e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf), followed by current canonical strict/nextest/C1-integration/validator/signature/push0. Fresh432-model/93-contract native/TLC parity and immutable6e73 archive are retained; actual worker profiles and same-inode observers preserved. Input59/message1415 ten blobs remain unchanged by source60's single authorized readiness-wall ignore;26 task-specific deferrals are non-passes. Source59 exact-chain2 and C16 RED remain preserved. Full private30s fit/qualification remain UNPROVEN, runtime_approved=false, no genuine builder births. Full correction report committed at native checkpoint847218b1339621278bdb8833aae64b32404b5ae6.

```json
{
  "op_id": "01M6WVHRA8H1YVX0HCJKDHY6ZR",
  "commit": "af5de27c33d46cdcb04510e863ef95fdc835c3ec",
  "report": "[[reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Distinct prepare-read wire, exact context/manifest binding and bounded immutable pages/descriptor.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#wire; src/frontends/headless.rs:32; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Finite selected IR/JSONL/blobs, private Git, public policy/keys, catalogue and artifact-to-file custody; no arbitrary receipt paths/fallback.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#bundle; src/dream/read.rs:295; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Compatible adoption, current record/trial/publication/receipt/trust/transitive proof, safe lesson fields and verified_at_revision.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#published_lessons; src/dream/read.rs:2286; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Shared64MiB input; parent32MiB/child96MiB, protected control/helper shares, bounded allocation/disposal, four jobs and actual joins.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#resource; src/dream/read_wire.rs:14; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Own-thread ordinary1ms/native2msCPU25mswall3msinclusive/full9x debt unchanged. Decision66280 removes external per-grant/Q/T/N; helpers run continuously at atomicBACKGROUND/observedPRI4 with250ms command/3s job totalCPU, actual per-member RI begin-to-begin+return gaps<=50ms, terminal originalwait4/rawovershoot/fresh9x postreap debt/terminalRSS16MiB.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#cost_debt; src/dream/c1.rs:748; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Canonical128B permit/lease, unchanged250ms readiness and30s maximum work deadline; no argv/channel/owner/source admission before permit, null-context probe rejection and actual settlement.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#bootstrap; src/dream/bootstrap.rs:16; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Original positivebirth/audit/PGID custody, bounded2members, positive ownWNOWAIT/actualownwait4, known-native census status, retainedchild positive exitedRI then fresh sameleader qualified componentwise rollup; live/escaped effects before reap seal, independentgroup proof/ownedquarantine. KnownleaderCPU paid even if unknownchild held; no postownreap signals.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#helper_custody; src/dream/c1.rs:829; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Git-created verifier temp argv/custody GS1/GS2; matcher never directly expanded; original inode/hash/scratch cleanup or quarantine.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#signature_B; tests/fixtures/dream-read/proof-helper-probe.c:3358; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Exact signed shared schema/vectors, bounded strict decoder and seven-source/private pre-test bindings, no runtime self-approval.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#shared_codec_C; tests/fixtures/dream-read/proof-helper-probe.c:723; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Independent control/persistence/terminal-cost verdicts, bounded retained errors and post-tail receipt; fixture setup cost cannot disappear. Effective23:10 fixes bind admitted inputFD+freshhash independently of replaced locator; distinguish native IO failure from paid work-deadline refusal; one-way terminal reporting cannot admit work and attempts finite/paid result after original30s expiry. Bounded path batching retains full actualCPU/debt.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#terminal_verdicts; tests/fixtures/dream-read/proof-helper-probe.c:3874; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "One durable A close, finite subscriber/socket drain and retained Host creating one nonfork B with managed launch inputs.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#retained_host; src/engine/mod.rs:510; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Finite owner/actions/bounds/fairness and all deliberately brokencontrols; honest66280 retirement/inventory. Exactnative cap100000 and pinnedJava fullstates/labellededges/namedverdictparity. Incomplete/statecap/OOM is failed; no sourcecurrent claim before freshcanonicalgates.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#formal; tests/formal/dream_read.rs:2625; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Signed exact source, current worker/compiler/source-input profiles and formal model/tool evidence; immutable Cpreparation/source4readset/schema/vector/compilerheaderclosure tuple separate from runtimequalification. Retain all failed runs/sourcecuts; no oldtuplecurrent.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#provenance; src/dream/read.rs:2648; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      },
      {
        "criterion": "Canonical current-source gates and remote main containment before final handoff.",
        "reference": "reports/2026-11-08-dream-mcp-strict-tree-peel-correction-landed.md; target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/source60-current-acceptance-source-map.json#landing; scripts/land.py:112; exact current four-chain0 log SHA256 e6646b37dc28299dbd2dd4462a7de277e3667bed77978df67fc1367d4a63f8cf; current receipt SHA256 481b7d186542a5def4e83183d75a98f66d61502ba6b6e9ba76e6bda11f0f580a;26 explicit deferrals/qualification limits retained"
      }
    ]
  }
}
```
