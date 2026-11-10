---
type: decision
title: Classify the fixed final public authorization check as bounded admission control
links:
- to: '[[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]]'
  rel: clarifies
- to: '[[tasks/2026-11-05-dream-c1-consumer]]'
  rel: implementation
id: 01M6PP86FJEPMRMM6VF2QBPSQB
updated: 2026-11-05T20:26:21.845Z
created: 2026-11-05T20:26:21.845Z
number: 93377961365259
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:10e71e4ddfcda720a2535552c75adc6f87d97cdd7168a50100366895829fbb48
  frontmatter_digest: sha256:2c52007c90f3c319d01da5a83d034949b1aab31cc8bcceac2cbd80d8e3fb4096
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: NdqmtqFs6Z3AONGPplk7vh/Gx95Nq2DYt8mM4B2Us2GtpVSBUS/80TZfnsZ3xLTRHZuizsPwRdIbb25IxHLUAQ==
  log: .bboxy/log/2026-11-05.jsonl#142002
---
Date2026-11-05. Oracle disposition for consumer01M6PNVKAQX699JM0YRJCD02TE. A fresh authorization recheck AFTER expensive-work debt must not itself recursively create another work/rest/recheck cycle. Current bounded public proof is retained; no atomic-only/TTL/custody substitute is authorized.

At the TWO final worker handoff sites only (single bootstrap-permit attempt and single authorized Start), classify the existing PREPARED public current-authority callback plus its bounded control transition/write as admission CONTROL, measured inclusive<=1ms on the existing isolated admission owner. It creates no expensive-work9x debt. Charge its full CPU/wall in control and complete owner/process usage, enforce the whole bounded admission step rather than split/hide >1ms. Status/Cancel/message/UI/shared reactor owners must not execute or wait on this SQLite/IO callback. Use the current independent lanes and protected budget, not new threads/pools or a global fence lock.

Exact allowed callback: current public account-file identity/inode, monotonic publication/revision/time/issuer/head/grant/device facts and current actor/seat/root/provider metadata, through the already prepared read-only connection/statement cache and existing nonblocking selection/session snapshot. identity/status.rs verified_public_rows has six fixed public keys plus at most one device key (<=7), PUBLIC_DIGEST_SQL bounds each cell to16KiB BEFORE returning/hashing its blob; total public digest input<=112KiB. registry/dream_c1.rs rows_match uses exact primary-key bounded actor scalar predicates/SQL CASE, not unbounded JSON/serde. Retain revision/file/publication checks after the transaction and all raw-row/inode/revoke/expiry/ABA controls. Missing/oversize/wrong type/count, held/busy dependency, clock failure or >1ms refuses/revokes before trusted handoff; never cache permission or silently return success. Prepared connection/key/snapshot construction and other setup must finish as ordinary work BEFORE its debt is paid.

Only those fixed public-proof digests belong to this admission bookkeeping, not source/payload/manifest/result/binary hash, generic SQLite/Git/KB/identity scan, signature/key/custody/provider process or arbitrary helper callback. All those keep existing work/native rules. The same guard used at ordinary work units keeps its existing accounting; this is not permission to move every guard or polling loop into a free control lane. No busy recheck loop: at most one such final check for each of the two actual handoff attempts. If currentness/foreground/deadline changes before effect, stop/refuse or return to the already bounded admission state without new birth/grant; do not reuse an authorization across an await or erase an unknown effect. Preserve the original readiness deadline/identity and uncertainty/settlement law. This does not promise instant knowledge of an arbitrary external write after a read; existing monotonic invalidation and fresh per-unit/terminal checks remain.

A source-free probe has no actor/root authority: it uses the explicit null-context rule and no fake final actor proof. Real jobs retain full public context. Permit binding preparation/hashing happens in ordinary work before this final callback; tiny permit/Start control write and ownership transfer are bounded by the same measured admission budget, with actual effect/lease retained on a later timing failure. No threshold, wire/serde/schema/IR/algorithm/control-version, protected memory entitlement or credential permission expansion. Existing sampled/declared memory limits cover guard state and native SQLite overhead; no universal allocator or OS latency claim.

Required source/model/process evidence: finite admission-check count/control cost, no source/payload work in this class, no stale/cache-only grant, real row/inode/head/grant/ABA tamper refusal after debt, held selection/SQLite fail-closed without blocking independent Status/Cancel, >1ms/clock-unknown final check revokes and retains real Child/pipe effect, no repeated permit/Start or debt reset. All inherited4-job/foreground/native-frame/heap/control/actual-reap gates remain. Observed445–906us supplied guard timings are baseline, not new final-source conformance. Oracle amends live briefs; Architect narrowly synchronizes the two handoff lines. No tests were rerun and no source/runtime acceptance follows from this decision.