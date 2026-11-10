---
type: research
title: 'Dream and swarm Dream integration: current source evidence'
derived_from:
- '[[tasks/2026-11-05-dream-term-integration-architecture]]'
id: 01M6NK5HB07H1A1GZSJSTV0JD5
updated: 2026-11-05T10:13:14.143Z
created: 2026-11-05T10:13:14.143Z
number: 93377961364356
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:1463f51ca97d927133bac5eed657f695bf9d10d97e9bbdc1a36a6a3043b04d6f
  frontmatter_digest: sha256:09c615727a8da118bf76087c1f1db596bab12e80a5096906b49b6512325a9a9a
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: k+InBJK64gJ2QSjlHNEAq3dyVErv7w3tNMNBlzoAfUaczkaEaeRndkwVkjsnulBvi+Kv5Y6vkvjcSK2z2vT+BA==
  log: .bboxy/log/2026-11-05.jsonl#133951
---
# Dream and swarm Dream: current source map

Task: [[tasks/2026-11-05-dream-term-integration-architecture]]. Design: [[specs/dream-term-integration]]. Boundary: [[interfaces/dream-term]].

## Finding

The premise is partly true. This checkout contains substantial Dream implementation: retained IR ingestion, deterministic replay, strict proposal synthesis, a confined scripted engine trial adapter, trust/audit/projection/graph machinery, confirmed-flush and immutable-publication APIs, and explicit Recall seams. It does not yet contain a production personal or swarm run coordinator. The terminal already embeds the harness face, but does not expose Dream controls. A Trigger::Schedule variant is a wire shape, not a scheduler.

“Current” below means statically reachable in these source trees, not a verified installed executable or a successful live run. No product command, inference, build or formal suite was executed for this audit.

## Revisions and method

At 2026-11-05 10:02:32 UTC:

| Tree | HEAD | Observed state |
|---|---|---|
| bboxy450-term | 59b06c304510fa4a75b7673bdadb0a88fd16722b | One untracked delegated task record; no tracked diff in the initial short status |
| embedded crates/bboxy450-harness | a2ca8f393630081b6be583151293ae286dc2419e | Clean short status |

Harness paths in the next two tables are relative to crates/bboxy450-harness. Term paths are relative to the term root. No sibling checkout was needed. The final report records the closing revisions/status because other seats can change this shared checkout.

Read through the authenticated task/provider workflow and read-only rg, nl/sed and Git commands. Relevant intent: term [[vision]], [[specs/harness-adapters]], [[specs/mux-daemon]], [[specs/knowledge-base]], [[specs/work-items]], [[specs/budgets]]; harness .bboxy/specs/{harness-zoomed-out,dream-log,memory-graph}.md, .bboxy/interfaces/dream-sinks.md and decision d20261103136206-dream-contract-refinement. The latter explicitly supersedes older sink, prompt-role and quota-unit descriptions. The term decision d365 lookup returned number_conflict; verification/README.md independently describes the formal stack.

The amended brief explicitly permits Git-backed knowledge records and requires provider-dependent custody. Earlier private/uncommitted-only launcher wording is not the design baseline.

## Actual harness producers, consumers and limits

| Boundary | File:line — mechanism | Reachability and consequence |
|---|---|---|
| Durable IR | src/protocol/mod.rs:237,818 — protocol 1.3 and SessionMeta; src/store.rs:176,447,519,597 — owned Store open/append/sync and log path | Ordinary session JSONL plus referenced session blobs are the evidence substrate. Raw PTY history is a different format. |
| Event/record identity | src/protocol/mod.rs:17-25,91-128 — Event pins hash exact stored envelope bytes excluding LF; Record pins include repository root/path/commit/digest | Preserve stored bytes. Re-serializing a copied event, or treating a latest record/etag as a committed pin, changes the evidence. |
| Native ingestion | src/import/mod.rs:993,1139-1141,1165,1187,1439,1522,1585 — ordinary import plus bounded snapshot/stage/no-clobber publication APIs | Dream replay can retain deterministic Claude/Codex imports. Snapshot bounds are 16 MiB/10,000 native records; custody validation has a 128 MiB bound. Native EOF does not independently prove native process completion. |
| D0 admission | src/dream/d0.rs:207-242 — parse durable IR, require terminal SessionClosed, project/read-check transcript | Open sessions are omitted from D0. Imported snapshot completion must keep its weaker provenance label. |
| D0 inputs/cache | src/dream/d0.rs:372-434,630-654 — recursive explicit inputs, native materialization, deduplicated episode members; :69-101 — nonblocking per-fingerprint flock | Replay creates derived observations/cache, not trusted memories. Passing an account-wide sessions directory would broaden the scan; term must supply a selected manifest. |
| Close hooks | src/engine/mod.rs:427-445; src/backend/acp_agent.rs:415; src/import/watch.rs:82; src/dream/d0.rs:656-671 | Ordinary harness/ACP/import-close paths launch detached D0. Trial closes suppress unrelated post-close work. Errors in the detached D0 helper are discarded; terminal integration needs an observable result channel. |
| D0 CPU | src/dream/d0.rs:682-722 — maximum CPU between await points checked against 1 ms | Existing per-poll check is not a wall-clock, heap, corpus-size or aggregate machine allocation. |
| Task joins | src/dream/join.rs:32-98,101-134,138-188 — parse committed task verdicts; associate only task_report/task_accept/task_reject arguments; resolve origin/main once and read exact Git objects | A tool-call mention associates an episode; it does not prove acceptance. Live task status alone cannot satisfy the committed-record/trust contract. |
| D1 inputs | src/dream/prompt.rs:19-45,137 — prompt/threshold revision, conservative 60,000-byte input bound, 4 KiB block clipping, at most 32 episodes | The implementation uses bytes as a conservative token bound; do not describe this as an exact tokenizer count. Source gate requires repeated evidence and an independently trusted verdict anchor. |
| D1 execution | src/dream/synth.rs:523-590,637-691 — caller-owned synthesis Store, no tools, bounded Provider stream, Usage retention, strict parse, Pending/Proposed | Library can accept a Provider. It neither runs trials nor authorizes adoption. Success explicitly reports semantic audit UNPROVEN without the enclosing final receipt. |
| D1 CLI | src/dream/synth.rs:734-818,870-942 — select input/root/trust, noop without eligible evidence; skip without --scripted; construct ScriptedProvider | --profile is not a working production provider/quota scheduler here. There is no automatic real inference behind this CLI. |
| Trials | src/dream/trial.rs:675-684,819-831 — harness must equal this crate version; non-scripted source profiles omitted | The current adapter is confined scripted engine replay. Native/ACP trial isolation is explicitly unavailable. A real engine profile needs a deliberate adapter extension and its own witnesses. |
| Trial ownership/results | src/dream/trial.rs:1014-1095,1788-1819,1942-1955,2080 — caller-tightenable limits, detached owner through cancellation, confined continuation, ordinary retained arms and Trial/Tested appends | Drop requests cancellation, not immediate cleanup. Unsupported/busy/ineligible work can become Inconclusive. Stage results remain unproven for complete audit without the enclosing run receipt. |
| Lifecycle | src/dream/projection.rs:441-508,559,658 — per-harness pairs, global terminal state, qualifying-trial checks | A pass is bound to entry revision and harness. Terminal revisions cannot be reopened by delayed completion; a revival needs a new EntryId. |
| Flush | src/dream/flush.rs:19-46,56-117,140-170,212-269 — already-authenticated Route, affine ticket, original write/commit IDs, status query after ambiguity, exact committed record verification | Library-only route seam. Restart retention is explicitly a caller responsibility. Created is emitted only after confirmed commit and matching record bytes/signature; Pending is not a materialized memory. |
| Publication | src/dream/lower.rs:29-95,244-270,299-349 — current pins/approval, scoped selection, owned job and settlement; :80-84 rejects Grok/Proposal loaders | Existing stage requires independently approved executable capabilities. Stage publication alone is UNPROVEN until outer run R4. Do not turn a skipped native loader into Adoption. |
| Constructive run evidence | src/dream/seal.rs:192-281,340-375; src/dream/receipt.rs:840,848,1094,1131,1542,1658 — owner tracks effects/children, freezes/pins a prefix, validates canonical publication/run receipts | The caller must observe all relevant effects and join children before final seal/SessionClosed. A list of child rows, successful command, hash or stage return is not completeness evidence. |
| Trust | src/dream/trust.rs:113-175,465-487,660-709,713-756 — external owned policy, independently bound roots/signers, KB signature and signed last-touch ancestry, measured executable approval | Event evidence is untrusted even with a correct digest. No current live policy or producer approval was inspected or installed. Embedding the library in a different executable does not inherit the harness executable's approval. |
| Scope | src/protocol/mod.rs:82-88,180 — Repo/System only; src/dream/resolve.rs:116-149,758-804 — canonical repo keys and committed cross_repo opt-in | No seat/swarm/account authorization field exists in Dream Scope. A same-repository swarm filter must be a term-owned admission/audience boundary, not an invented ScopeLevel::Swarm. |
| Recall preparation | src/dream/recall.rs:7-18,28-125 — explicit current audited publication, active pair/current record, exact regenerated bytes and drift checks | Immutable caller-prepared Context; no automatic production loader/discovery. Current check covers harness/repo, so term must separately enforce seat/swarm audience. |
| Recall insertion | src/dream/recall.rs:127-195 — suppress trial, validate target harness/repo, append pinned reported Recall and sync before returning block | Failed logging returns no context bytes. Recall proves insertion, not use, truth, or permission. |
| Native exposure | src/dream/recall.rs:202-213; src/import/mod.rs:207-224; src/import/edge.rs:236-297 — explicit validated publication timeline/citation projection | No timeline means no invented inferred Recall. All exposure bases conservatively exclude contaminated evidence/held-out sessions. |
| Instruction barrier | src/dream/deny.rs:46-91,95-119 — redaction/deny scan and instruction-sink detection/neutralization | Memories remain quoted low-trust data. AGENTS/CLAUDE/rules/skills promotion is a separate proposal workflow. |
| Query/graph | src/dream/audit.rs:928-1003 — audit/graph/why CLI with explicit roots and sessions; src/dream/graph.rs:18-60,141-149,421-437 — typed graph and provenance closure | Read-only derived views must retain scope and distinguish missing/unproven evidence from acceptance. |

## Source reachability versus promised workflow

src/frontends/headless.rs:84 dispatches dream to src/dream/mod.rs:32-45. That dispatcher admits synthesize, audit, graph, why and replay only. dream trial, flush, lower and run described in .bboxy/specs/dream-log.md:141-157 are not dispatched in this revision.

A production-source search for trial::, lower::, flush::, recall::Context::prepare, recall::Exposure::prepare and new_with_recall outside src/dream returned no matches (rg exit 1). Separate search found the explicit open_with_recall/load_with_recall definitions and tests, not a live loader. src/gateway/host.rs:297-331 starts the ordinary engine/ACP load methods. Thus “already implemented” must not be read as “the complete user workflow is exposed.”

The current trial source restriction is stricter than the desired multi-adapter workflow in dream-log.md:187-205. Treat the implementation as the current limit and stage the missing adapters. Existing fixtures and reports describe substantial tests; this audit did not rerun or accept those reports.

## Terminal seams and missing ownership

| Term evidence | Meaning for integration |
|---|---|
| crates/bboxy450-term/src/rich_pane.rs:10-13,185-216 | Existing worker-side Bridge/HarnessView embedding. Reuse the face; do not run synthesis/audit in paint. |
| Harness src/frontends/gui/face_rows.rs:732-740 | Dream items already have generic notice rendering, not a Dream control panel. |
| crates/bboxy450-term/Cargo.toml:20; harness Cargo.toml:19-26 | Term's embedded library disables defaults and enables embedded; ACP agent support is a separate feature. A bundled worker's capabilities cannot be inferred from the GUI dependency's features. |
| crates/bbt-mux/src/adapters/launch.rs:194-220; crates/bbt-proto/src/registry.rs:18-23,96-110 | Launch captures seat generation/session/role and swarm identity/workdir revision. There is no Dream run record or durable seat-to-IR provenance catalogue yet. |
| crates/bboxy450-harness-wake-up/src/harness_log.rs:22-41,59-119,145-159 | Harness session discovery uses the exact socket, kernel peer PID/uid and session, then an owned log; never “newest file.” Reuse that identity evidence for a catalogue, not the observer's activity label as task truth. |
| crates/bbt-mux/src/logic/history.rs:21-47,52-73 | Terminal ring history is generation-scoped captured bytes. It is not harness JSONL and cannot be substituted for IR. |
| crates/bboxy450-harness-wake-up/src/rich.rs:72-115,138-164; crates/bbt-mux/src/rich.rs:43-78 | Existing per-generation transport, bounded lane and retained original-session operation identity illustrate the required fences; they are not a Dream runner. |
| crates/bbt-mux/src/knowledge/routing.rs:214-227 | Root binding is revalidated after workspace revision changes. Dream completion must carry the same captured context, including A→B→A epochs. |
| crates/bbt-mux/src/mcp/knowledge.rs:48-58,85-88,168-192 | Commit is Oracle-class, with explicit locators/message. Flush currently sends record instead; an adapter must translate and validate both request and result schemas. A worker role cannot self-upgrade to commit. |
| crates/bbt-plugin/src/data_source.rs:329-358 | Write returns record/lossy, commit returns commit/committed refs. A generic commit string does not promise Git ancestry, exact historical reads or independent signer verification. |
| crates/bboxy450-term/src/usage_strip.rs:43,64-98; crates/bboxy450-usage/src/lib.rs:121-131 | usage.json exists; used is integer percent. sampled_at advances on any update, so it is not proof of a fresh observation for the chosen provider. |
| rg over mux/app/wake-up production sources for Dream/Recall/harness Dream import returned exit 1 | No direct terminal Dream surface was found in the searched production source. This is bounded negative evidence, not a claim about every external script or installed binary. |

## Personal, swarm and provider compatibility

Personal and swarm are proposed term source selectors, not two proven shipped schedulers. Personal selects explicitly authorized owned seat-session history. Swarm selects a frozen roster/catalogue for one admitted swarm and partitions by canonical repo; it must neither sweep the account home nor treat “Oracle” as permission for another swarm. Publication audience is a separate explicit decision: selecting private evidence does not authorize publishing its contents to every reader of a repository KB or native account sink.

Bundled Git records can satisfy current Dream pins when the exact record bytes are committed, reachable from the independently configured allowed main, correctly signed, and independently trusted. A newly accepted live task is not necessarily committed or main-reachable. knowledge_commit never pushes. Pending/main-unavailable is an honest usable outcome.

An external provider may support the same Git semantics, another immutable revision system, or no commits. Its live etag and host signature are not automatically the current Git RecordPin/Trust proof. First support exact Git-backed providers through an explicit capability adapter; otherwise allow bounded inspection/proposals and label anchor/materialization unavailable. A future provider-neutral immutable-evidence/trust adapter needs its own written schema and conformance tests. Never silently read a local .bboxy fallback or manufacture a Git commit for external state.

## Existing verification substrate and gaps

Harness verification/corpus.toml:670-759,806-955 and verification/contracts.toml:706-733 register Dream lifecycle/merge/sink/D0/native-custody/seal/trial/flush/publication/Recall models and controls. tests/dream_{replay,pipeline,trust,seal,sinks,recall,graph,audit_broken}.rs are real implementation witness locations, not results run here.

Harness Makefile:49-57 distinguishes make test-formal (native) from make test-formal-parity (native plus Java TLC). IR additive checks are make test-ir (:209-211). Term verification/README.md requires full native/Java reachable state/transition/verdict parity, source-bound receipts and explicit implementation-only gaps. Both sides need gates after the future integration changes.

Remaining uncertainties: live producer/policy enrollment; supported installed binary/features; real engine/native/ACP trial adapters; durable restartable flush ticket reconstruction; provider immutable history; fresh per-provider quota observation; host aggregate resource allocation; native loader/forgetting semantics. None blocks the bounded inspection cut. These are dependencies for later stages, not claims that those stages already work.

Origin and credit: bboxy450-term (https://github.com/bboxy450/bboxy450-term) and bboxy450-harness (https://github.com/bboxy450/bboxy450-harness). Preserve source notices and accurate copied/adapted/translated credit in later implementation: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md and https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md.
