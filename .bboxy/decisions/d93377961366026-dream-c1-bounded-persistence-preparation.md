---
type: decision
title: Bound private catalogue initialization and stage public authority preparation
status: decided
derived_from:
- '[[specs/dream-term-c1]]'
- '[[interfaces/dream-term-c1]]'
- '[[decisions/d93377961365086-dream-native-quantum-and-debt]]'
id: 01M6RSRNJ0R2HNQZWJ5G3D0PFG
updated: 2026-11-06T16:06:15.674Z
created: 2026-11-06T16:06:15.674Z
number: 93377961366026
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:d5b39338bc398dc70d7f59e5caee7520c3f42bc9380966897912873eb9f9f617
  frontmatter_digest: sha256:b7d5d0e26c21b557fe1aec90c899dcb71c390ca2a5085391aa6a22deed5cf78c
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: oZ/xdiipwKEm/pjszS+UkXUscSYqUfjuZhRxzGXafuvt7w3LLRJob3wRnGHlImEDKs4QSUtCHS/o/PctjTO8Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149533
  event:
    id: 01M6RSRP00D9HM6RDBX7BKZBQF
    sequence: 149533
    hash: 7e65929250df7450b396ec15887b89054499ccae3d7b997b080c130bcd6544d7
---
# Bounded C1 persistence and authority preparation

Root ruling,2026-11-06. Consumer source26e5bb1fbdbc16ecc317331ab56ca1be44d903b2 has actual optimized113-test run86pass/27fail. /tmp/dream-c1-finish-mux-api.log SHA9e360c5332a7d145b4c0341f8ce8315a5bf0c72b30e52290a7377d4e761b6acc records repeated registry/dream_c1.rs:313 first-step BEGIN IMMEDIATE(index7)/COMMIT(index8) overruns (including1556625ns), WAL conversion done-step and actual connection cleanup overruns. One authority guard preparation is1065000ns. These are failed evidence, not a proof of a scheduler/filesystem cause or universal SQLite bound.

Source inspection: Journal::initialize_schema converts to WAL before first writer/schema creation; its SQLite begin/commit/finalization can perform indivisible native setup/checkpoint work. Authority::guard_with_session calls prepare_session with a no-op checkpoint, aggregating connection opening/configuration and three statement preparations despite the existing prepare_session_cooperative/public_connection_cooperative hooks. No arithmetic/threshold change is needed to split that controllable preparation.

Alternatives: retaining the same failed spans and retrying unchanged source gives no correction. Exempting all SQLite or relabelling a wrapper as native violates65086. A new custom datastore/allocator/SQLite fork is disproportionate to this bounded private catalogue. Choose the following small supported-SQLite implementation correction and validate it before any success claim.

1. Keep EVERY ordinary application/parser/control/cleanup unit≤1ms CPU and charged9x rest. No new native class, larger threshold, warmed fixture, global setting, hidden retry or blocking shared-owner IO. Kernel/native duration remains observed acceptance, not a preemptive guarantee.
2. For the C1 PRIVATE per-seat catalogue, use supported rollback-journal DELETE mode instead of requiring WAL on newly initialized catalogues. C1's specified requirement is durable atomic original-op/run/SourceKey/close-cursor registration and recovery; WAL is not a public wire/schema requirement. Retain synchronous=FULL and the exact atomic schema+ready-marker transaction, CAS/dedupe/caps/restart/cancel semantics and filesystem custody. Keep busy0 for initialization/mode transition and independent controls. A busy/held reader or writer refuses/defers promptly; it cannot block the UI/control lane or masquerade as settled success. Do NOT change the account/provenance database, its journal mode, global SQLite state or credentials.
3. Existing C1 WAL state is not discarded. Any needed transition uses SQLite's supported journal-mode API on the exact owned connection outside an active transaction, with inode/identity checks and bounded busy0. Preserve every row/cursor/op/run and original effects; never delete/rename DB/WAL/SHM files manually, replace the catalogue, lose a predecessor or replay a source. A busy/unknown/over-budget transition remains honestly unavailable with retained state, not an implicit second attempt. Each actual SQLite call and close still meets the SAME1ms gate; this design does not certify that before measurements.
4. Stage public-authority PREPARATION through the existing cooperative hook: separately measure and repay file/open/configure/each statement preparation, bounded context/callback retention and final checks. Split only real source phases. Record cumulative CPU and maximum unit CPU distinctly, with unknown/reversed samples failing; do not rename a cumulative value to a maximum or erase cost. Native connection/statement disposal remains stepped/measured and all actual custody retained on refusal. The prepared permit/Start authority callbacks themselves retain their unchanged inclusive1ms CONTROL bound, exact bounded raw-row/head/grant/inode/revision/provider/expiry/ABA checks and zero signing/custody probes.
5. Verification first: deterministic migration/busy/crash-between-DDL-and-marker/rollback/reopen/late-failure/original-op/cancel/ABA controls and bounded cold real DELETE initialization/commit/close measurements; verify existing WAL rows survive supported transition and held readers fail without destructive change. Extend the finite transaction/recovery/mode+busy model and broken controls to those actual semantics; no new arbitrary-native theorem. Recheck independent status/cancel/foreground while private persistence is held. On materially corrected signed source run the focused catalogue/authority/API group once; over-budget/unknown remains a failure with numeric phase/effect evidence. Full final canonical gates and real joined worker/frame/heap/source/reap evidence still decide landing.

This authorizes an implementation correction, not a statement that DELETE mode is already within budget. Any remaining truly indivisible overrun is reported with exact source/call/CPU/wall/effect before another design change. Old failed113-test run, stopped12ba tuple and missing producer landing remain explicit. No public DTO/IR/wire/version or accepted task outcome is changed.
