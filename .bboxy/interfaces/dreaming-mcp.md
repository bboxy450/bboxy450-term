---
type: interface
title: Bounded Dream search/read/feedback MCP boundary
derived_from:
- '[[research/dreaming-services-mcp]]'
links:
- to: '[[specs/dreaming-mcp]]'
- to: '[[interfaces/dream-term-c1]]'
created: 2026-11-05T14:22:23.464Z
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
id: 01M6P1DRCHV10ZBJRVR3NF6PGH
updated: 2026-11-07T15:52:38.926Z
number: 93377961364656
updated_by: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
authorship:
  signer: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  kind: seat
  public_key: gX8ozXnq5IspMTpqtE9rHRFSbgoDxsJh5dGOJgtj2o8=
  process_generation: '1'
provenance:
  digest: sha256:3d13ec8001dc92434e9592a05fc97419bb93a31cee4626b21873a8c0cc251224
  frontmatter_digest: sha256:b3147fd53eec18eb7540d1f3e2b541f02b5f554d6ce0cfb318d5ad5670d15a4c
  signer: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  sig: klGcEWf/MKW1a5wn0eGeRlja3VUtyztVTUG0/ChUgB5oLMyeZmdywwHjDsiKEhZNU62TaAz7z9BK8IidxoeiAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150379
  event:
    id: 01M6VBCFC0WXEFME0WKR3EHG72
    sequence: 150379
    hash: 664ce6304531143569b437983413fb8f027435c93aca87fa0423b9e2357f4506
---
# Dreaming MCP read and feedback boundary v1

Derived from [[research/dreaming-services-mcp]]; behavioral contract [[specs/dreaming-mcp]]. This is a proposed interface, not an assertion of shipped tools. Keep [[interfaces/dream-term-c1]] and [[decisions/d93377961364581-dream-c1-enforceable-memory-contract]] intact. Origin: bboxy450-term / bboxy450-harness, https://github.com/bboxy450/bboxy450-term and https://github.com/bboxy450/bboxy450-harness. Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md. Source-derived proof/rendering code retains its origin notices; external research is cited inspiration, not copied implementation.

## Participants and ordering

| Boundary / owner | Shape and ordering | Lifecycle / errors / compatibility |
|---|---|---|
| Agent → outer bboxy MCP, term consumer | dream_search, dream_read; later dream_feedback. Existing dream_c1 remains Inspect/Status/Cancel. Exact own admitted seat/root/repo/harness context injected by host | Ordinary CallToolResult, structuredContent plus serialized text fallback. Closed JSON schema, unknown fields rejected. Version v=1; incompatible change needs version/fixtures on both owners. |
| Managed engine/native/ACP MCP client → same bridge | Engine tool binding → launcher-owned bboxy server; ACP provider via existing uid-private relay; native Claude/Codex via private launch overlay | Existing process/generation/cancellation fences. No recursive mux→bboxy MCP→mux request. No private global config or second server installation. |
| Completed C1 → consumer prepared observation projection | Digest-validated C1 completed/settled run, manifest, ordered pages and closed-source pins | C1 completion only; event-driven bounded projection update, no account corpus sweep or read-triggered work. Observation is not adopted knowledge. |
| Producer → consumer prepared verified snapshot | Internal bboxy.dream.read-snapshot/1 described below; producer owns proof-bearing entry/manifest schema + corpus | Explicit closed-input preparation, fixed approved runner, immutable output then descriptor last. Consumer validates bytes, owner/process settlement and current public fence before exposing. Not a new publication sink. |
| Feedback → host own-seat event journal → optional IR writer | Durable original-op claim; bounded reported observation. Engine/ACP actual exposure recorder is producer-owned | Transport offer, model citation and successful application are different events. Native adapter may report exposure only when it observes the citation; no invented Recall on a tool fetch. |
| Selected KB provider → proof resolver | Host root/provider pin and authorized immutable evidence snapshot | Git-backed committed pins are supported primitives; native draft/acceptance alone is not a trusted pin. Missing provider proof bridge is unavailable/unproven, not Git fallback or retired provider invocation. |

Read service owner: term crates/bbt-mux/src/logic/dream_read.rs; public DTO/schema owner: crates/bbt-proto/src/dream_read.rs, mirrored nowhere independently. Harness producer owns src/dream/read_wire.rs for prepared snapshot/entry proof DTOs and canonical fixtures; consumer's narrow mirror consumes those fixtures. No full harness dependency in mux. Search/read envelopes and prepared artifact schemas each have one owner.

## Common rules and scope

All Dream domain and internal worker/artifact structs deny unknown fields. The public MCP envelope additionally declares the host op_id/hints fields below; it does not admit arbitrary unknown domain fields. Integers are nonnegative JSON integers ≤2^53−1; booleans exact; UTF-8 text. Id is a nonempty ≤256-byte opaque identifier; Hex is 64 lowercase hex digits; Ulid is canonical uppercase. Digest refers to SHA256 of exact immutable bytes; existing IR/RecordPin digests retain their sha256: prefix. Canonical snapshot/fixture JSON sorts object keys and has no trailing LF; IR is never reserialized as evidence.

Caller supplies no account, actor, seat, swarm, root, repo path, provider, policy or credentials. Host derives the exact current account/daemon/seat generation/grant/workspace/root/provider public fence from the authenticated route. No keys.signing_hold, identity.status/admission fallback, private-key/custody/Keychain/GPG probe. Missing/stale public admission fails before content. Revalidate before every page presentation; changed generation/incarnation, revocation, expiry, mute or binding invalidates handles/cursors. A→B→A cannot revive old access.

Q1 observations are own-seat historical closed sessions, exact current repo/root only. Historical generations require retained ownership. Q2/Q3 lessons additionally require an explicit existing read grant to that selected published repo/root and compatible harness pair/sink audience; no Oracle-role exception expands to peers. First implementation gives no account-wide or arbitrary swarm search. A swarm partitions routing/cache; it does not fabricate a Swarm variant in IR Scope (currently repo/system). System-scope records and cross-repo sharing are unavailable until an explicit separately authorized adapter/grant exists. Operator uses its current authenticated target-seat route; no unscoped selector.

## Public request DTOs

~~~text
Search = {v:1, kind:"observation"|"lesson",
 query?:String, fingerprint?:Hex, limit?:u32, budget_tokens?:u32, cursor?:String}
Read = {v:1, handle:String, view:"summary"|"evidence"|"context",
 budget_tokens?:u32, cursor?:String}
Feedback = {v:1, delivery:Hex, outcome:"used"|"not_applicable"|"contradicted",
 evidence:[Pin], note?:String}
~~~

Tool names: **dream_search**, **dream_read**, **dream_feedback**. Feedback is not advertised until Q3 is implemented. Original op_id is host Call metadata for Feedback; never accept a second agent authority/op field. Read calls have no mutating op requirement.

Search query defaults ""; ≤256 UTF-8 bytes, ≤16 lexical terms. fingerprint is optional and observation-only. limit defaults 3, range 1..8. Observation search matches only literal repo/session/run identifiers, exact fingerprint and recency metadata; title/preview are deterministic metadata renderings. Natural-language symptoms are not matched to opaque hashes or used to synthesize advice. Q2/Q3 lesson text supplies reusable guidance. Lesson search indexes title/keywords/why/how_to_apply. Deterministic lexical ranking: exact fingerprint match first (observation only), then matched title terms×3 + keyword terms×2 + body terms; tie-break by entry/observation ID. For observation query="" without a fingerprint, order by descending observed last_ms then ID (recency ranks observations, never establishes authority). No embedding, LLM, freshness score used as authority or paid call.

cursor ≤512 bytes; handle ≤256 bytes. Search cursor is exclusive of query/fingerprint; kind/limit/budget must equal its captured request. Read cursor must match handle/view. Handles/cursors bind admitted principal/context, snapshot digest, item or sorted result position, query digest and expiry (60 seconds for cursors); validate through host-held bounded state or authenticated MAC, never accept a client-authored proof flag. Snapshot rollover/invalidation returns stale_cursor, not a mixture of revisions. No generic URL or filesystem fetch.

budget_tokens defaults 2048, range 1024..4096. Charge the combined emitted text+structured representation using the matching tokenizer when available, otherwise conservative UTF-8 byte count, including metadata. If the next whole hit/block does not fit, return a bounded next cursor/truncated marker; never silently omit provenance or slice an instruction into a changed meaning. A single over-budget entry returns budget_exceeded with a summary handle, not unbounded content. Output ≤16 KiB serialized total per call. No hidden full transcript in _meta or text fallback.

## Result DTOs

Optional fields below are explicitly null when absent; fixed schema definitions use these exact variants.

~~~text
Page = {v:1,status:"available",snapshot:Hex,
 items:[Hit],next:String|null,truncated:bool,coverage:Coverage,
 offered:Hex|null}
Unavailable = {v:1,status:"unavailable"|"pending"|"inconclusive",
 reason:Reason,run_id:Ulid|null,retry_after_ms:u32|null}
Failure = {v:1,status:"error",error:{code:Code,retryable:bool}}
Coverage = {kinds:["observation"|"lesson"],indexed:u32,omitted:u32,
 complete:bool,currentness:"historical_observation"|"verified_at_revision"}
Hit = {handle:String,id:Id,kind:"observation"|"lesson",title:String,
 preview:String,repo:Id,harness:Id,proof:ObservationProof|LessonProof}
ReadPage = {v:1,status:"available",snapshot:Hex,item:Hit,
 view:"summary"|"evidence"|"context",payload:Payload,
 next:String|null,truncated:bool,offered:Hex|null}
Payload = {kind:"observation",episode:Episode|null,cluster:Cluster|null}
 | {kind:"observation_evidence",proof:ObservationProof}
 | {kind:"lesson_summary",why:String,how_to_apply:String}
 | {kind:"evidence",nodes:[EvidenceNode],edges:[EvidenceEdge]}
 | {kind:"context",text:String,citation:String}
FeedbackResult = {v:1,status:"recorded",report:Hex,
 assessment:"reported_only",duplicate:bool}
~~~

Title ≤160 bytes; preview ≤256 bytes. Pin is the existing typed EventPin (session,seq,digest) or RecordPin (root,id,path,commit,digest); any additional provider revision is carried in the trusted manifest, not smuggled into legacy pins. Node={id:Id,kind:Id,pin:Pin|null}; Edge={from:Id,to:Id,relation:Id}. At most 16 nodes/32 edges and depth 2 per evidence page, clipped by output/token budget. No raw graph data Value or transcript excerpt. A referenced source outside read scope becomes a redacted evidence-count stub, never a foreign locator.

Every observation is bound to a **result row**, not necessarily an IR event. Existing C1 wire is unchanged: producer src/dream/c1_wire.rs:319-375,413-416 defines descriptor/page/rows; Episode has Origin and Cluster does not. src/dream/d0.rs:940-968,986-999 aggregates matching episodes across distinct selected sessions.

~~~text
ResultRowPin = {run_id:Ulid,manifest:Hex,result:Hex,
 page_index:u32,page_sha256:Hex,row_index:u32}
ObservationProof =
 {kind:"closed_ir_episode",row:ResultRowPin,origin:EventPin,trust:"not_evaluated"}
 | {kind:"closed_ir_cluster",row:ResultRowPin,
    members:ClusterMembers,trust:"not_evaluated"}
ClusterMembers = {basis:"result_rows",links:[ResultRowPin],
 total:u32,omitted:u32,complete:bool}
~~~

manifest/result are the SHA256 of exact retained manifest.json/result.json bytes, not a reserialized approximation. Resolve only a completed/settled authorized C1 run with matching run_id/current public fence. Validate result.manifest_sha256 and run/nonce against retained run custody; locate exactly one descriptor PageRef with index=page_index; verify its len and sha256=page_sha256 against exact retained page bytes. Page schema/run_id/manifest/index/order must match the descriptor; row_index is zero-based within page.rows and must be <rows.len() (C1 maximum128 rows/page). The selected row kind and every exposed Episode/Cluster field must equal that row. The immutable observation id is SHA256 of canonical {schema:"bboxy.dream.observation-id/1",row:ResultRowPin}. Neither a client flag nor an arbitrary event pin can substitute. Old descriptor/page hashes, kind substitution, out-of-range indices or altered row bytes refuse with protocol_error; a no-longer-current authorized context uses stale_context/not_found as above.

For Episode, origin is copied exactly from that row's Origin, mapped to the existing EventPin; retain its session/seq/digest. It identifies the episode's recorded origin, not every contributing event. For Cluster there is **no origin field or synthetic cluster event**. Its proof establishes the exact aggregate emitted by this C1 run over the manifest-selected sources; trust remains not_evaluated and it does not independently certify the D0 algorithm or imply causal/lesson truth. Same fingerprint in different runs is not the same provenance.

Cluster member links are optional evidence, bounded to8 distinct episode rows from this exact run/manifest/result. Q1 may emit no links: total=cluster.count, omitted=total, complete=false. total is the producer-reported episode count; links.len()+omitted=total. Validate each offered link's descriptor/page/row chain, Episode kind, fingerprint inclusion, and session→manifest-source repo equal to cluster.repo; no foreign run/repo, duplicate episode or representative-as-all-members. Links are ordered by page_index,row_index. complete=true additionally requires the off-owner bounded completion projection to have checked **all descriptor pages** and established that these are exactly every matching unique episode, with count/min(first_ms)/max(last_ms) equal to the cluster. If not all pages/rows are available or linked, complete=false; contradictory fully checked output refuses instead of silently certifying it. No completeness inference from one representative or from a matching count alone. Omitted links, skipped pages/index entries and search Coverage are separate coverage dimensions. Do not reread/replay source IR, export transcripts or start work on a tool read. Member preparation, if offered, consumes the existing bounded completed-result projection budget; the zero-link mode is sufficient for Q1.

Episodes/clusters reuse exact C1 Row fields. Observation context returns unavailable/not_a_lesson; its evidence view returns Payload {kind:"observation_evidence",proof:ObservationProof}. Generic EventPin/RecordPin graph nodes are for lesson evidence, not a fabricated observation source. Omitting optional member links for budget never removes the primary row pin or claims complete membership.

LessonProof={kind:"verified_lesson",entry:Id,revival:u32,record:RecordPin,
 trial:EventPin,publication:EventPin,outer:EventPin,
 projection:Hex,trust_policy:Hex,producer:Hex,currentness:Hex}.
These are producer-validated pins for an already-adopted compatible pair and current record/publication/outer seal, not request assertions. Full evidence need not fit a hit; immutable compact proof digests stay with it. title/why/how_to_apply come from the pinned record and rendering rules, scrubbed/denied as existing code requires. Never use arbitrary KB status text or C1 trust:not_evaluated to construct this variant.

Reason = not_prepared|not_a_lesson|no_published_input|provider_proof_unavailable|
 producer_unapproved|harness_incompatible|stale_source|verification_inconclusive|
 storage_unavailable|busy.
pending means an already-admitted known preparation run exists (include run_id); no job is secretly started by a read. An available empty page means the prepared authorized snapshot has no match; unavailable is not disguised as empty success. Validation Fail is not Pending; audit Unproven maps to unavailable/inconclusive with its bounded category.

Code = bad_request|version_mismatch|not_found|stale_context|stale_cursor|
 budget_exceeded|op_id_reused|resource_limit|protocol_error.
Foreign/unreadable handles yield not_found without metadata. Dependency absence is a domain Unavailable, not a fabricated zero-hit success. Unexpected protocol/IO details stay in private bounded diagnostics.

## Model-visible host hints and closed output schemas

[[decisions/d93377961364692-dream-mcp-structured-host-hints]] governs all Dream MCP tools. Essential guidance lives in structuredContent.hints and has a matching serialized text copy for content-based clients. _meta is an optional UI duplicate. Use the shared host hint schema/generator/projection; hints are advisory and cannot admit work, change authority or become lesson content. Current built-in structured replies are the domain fields plus host op_id and hints. The Page/ReadPage/Unavailable/Failure/FeedbackResult DTOs above describe the STRICT DOMAIN portion; their complete MCP outputSchema explicitly extends each variant with known host op_id and shared-schema hints. Construct a compatible closed outer schema rather than an allOf that leaves a closed domain variant rejecting host fields; do not globally enable unknown fields. Validate/separate those known host fields before strict typed-domain parsing. Internal inspect-c1/read-prepare wire, artifact rows, pins and proof hashes receive no hints.

Reuse the common post-journal/off-owner projection, idempotent when seat socket and bridge both encounter a reply. Preserve original op_id, nested provider result, signed receipts and immutable source/result/page bytes. All emitted hint copies count toward the existing serialized16KiB/context budget; reserve bounded guidance space before selecting whole hits/pages. Keep compact required recovery guidance and exact pins, with no hidden metadata overflow. Shared generic frame safety does not replace this tool-specific budget.

Acceptance uses ACTUAL admitted Dream replies and a structured-only client projection that discards content/_meta. Essential guidance must survive success/page, empty/unavailable/pending/domain-error and original-op recovery cases supported by the slice. Validate the final hinted result against advertised outputSchema, compare its text hint mirror, reject unknown domain fields, and cover double projection, near-budget results and poisoned historical/provider text. A visible second text block alone cannot pass. No live Codex/provider account or UI interaction is required by that compatibility fixture.

MCP wrappers: reads use readOnlyHint=true, destructiveHint=false, idempotentHint=true, openWorldHint=false; feedback readOnlyHint=false with other values unchanged. Hints do not confer authorization. Domain available/unavailable/pending are ordinary bounded results; Failure sets isError=true with the same structured shape and safe text. Exact request/result goldens and rejection fixtures are part of the public schema owner.

## Prepared snapshot producer contract

Proposed new internal algorithm read-prepare/1, distinct from inspect-c1 and its wire. Launch only through the already provisioned sibling runner as dream prepare-read --job-dir ABS. Missing/protocol-mismatched runner is unavailable; no PATH fallback/install recipe. No public new “run consolidation” tool.

Job input is host-authored in private C1-style custody: {schema:"bboxy.dream.read-input/1",nonce:Ulid,
 context:Context,closed_sources:[ClosedSource],records:[RecordPin],
 publications:[EventPin],trust_policy:Hex,provider_revision:Id,evidence:EvidenceBundle}.
ClosedSource reuses exact C1 manifest source evidence and limits. Host supplies authorized immutable provider bytes/proofs; agents never select path/policy. ≤32 explicitly selected sources, ≤64 MiB source/blob bytes and ≤512 selected records; ≤4 MiB final prepared snapshot. No default Resolver::open directory sweep, HOME or GitKB invocation. Add a selected-input bounded resolver path; existing audit/projection/receipt/Graph::why primitives are source mechanisms, not presumed bounded as a whole.

### Q2 exact selected evidence packaging (accepted baseline47182320)

Record/publication identifiers alone never authorize a read. Host prepares the immutable job input before Start; the worker receives only the exact input manifest hash and opens no ambient repo/provider/HOME. This is the missing physical binding for the proposed read-input/1 schema, not a C1 schema change.

~~~text
EvidenceBundle = {files:[BundleFile],roots:[RootBinding],policy:u32,
 public_keys:[KeyBinding],artifacts:[ArtifactBinding],catalogue:[RecordPin],
 selected_at_ms:u64}
BundleFile = {slot:u32,path:String,len:u64,sha256:Hex}
RootBinding = {root:Id,repo:Id,main:String,dir:String}
KeyBinding = {fingerprint:Id,file:u32}
ArtifactBinding = {logical:String,file:u32}
~~~

files is a sorted unique inventory of ALL read-input files except manifest.json: exact closed JSONL/blob paths from ClosedSource plus selected evidence under input/evidence/. Paths are job-relative lexical paths (no absolute/dot/dotdot/NUL/empty component); open descriptor-relative nofollow, private regular single-link owned files, verify length/hash and stable fd/path identity before/after use. Slots are contiguous zero-based. Path/logical/dir strings≤4096 UTF-8 bytes; main uses the existing valid_commit grammar. roots≤32; each dir is an inventoried private **nonbare Git snapshot** under input/evidence/roots/<index>, with origin/main exactly main and repo binding matching the independently admitted root. Inventory every Git object/ref/config/working file actually used. Reject alternates, include/includeIf, worktree indirection, hooks/filter/external helpers, symlinks and network/ambient config; no clone/fetch/init or discovery in the worker. Host makes this bounded snapshot; Git ancestry/signature checks reuse existing trust semantics on its exact local objects. Ref/config changes fail source_changed. Public verification may use only existing explicitly provisioned Git/GPG tools, owned/bounded helpers and a private temporary public-key-only keyring; no operator keyring, private keys, signing or enrollment.

policy is a file slot whose exact bytes hash equals trust_policy and parse as existing bboxy.dream.trust/2. It is chosen by host admission, located outside every packaged Git root, never inferred from a record or loaded by HOME. Each policy root/branch must map uniquely to an admitted RootBinding and resolve exactly to its main; no extra policy root or mutable ref may enlarge the read set. public_keys binds only declared policy fingerprints to exact inventoried public key bytes; there is no credential material. Root/policy key/path values remain logical evidence: the selected read view maps them only through these admitted bindings; do not rewrite signed record/receipt/policy bytes to relocate them. artifacts maps exact logical receipt/sink/generated paths to inventoried immutable bytes for current_labels/render checks. Duplicate/conflicting mappings, outside-scope paths or arbitrary receipt-named file access refuse. Missing mapping/proof yields verification_unproven, never a lookup outside the bundle.

records is the candidate allowlist; catalogue contains exact current-head memory records plus selected transitive task/gate/evidence records needed by those candidates, with explicit commit/id/digest. Both together have≤512 distinct pins. Resolve locators/accepted-task evidence only against this finite catalogue; no Root::paths, accepted_tasks directory catalogue, default Resolver::open, or live/uncommitted record fallback. Prove each candidate's record bytes at the bound root.main, ancestry/signature, current projection/revival, qualifying held-out trial, historical producer approval, full accepted publication selection/adoption and outer run seal using existing verifier semantics. All referenced sessions/receipts/blobs must be selected; missing closure is Unproven, not partial certification. Do not call Context::append or publish/adopt anything. Reuse the read-only validation/rendering half, not a Recall effect.

selected_at_ms is authenticated host snapshot time (safe integer); evaluate temporal/expiry rules at it, retaining it in the prepared source revision. Bound source_revision=SHA256(canonical {context,provider_revision,closed_sources,evidence,trust_policy}); input_digest additionally binds the entire manifest. This certifies the admitted finite evidence at that revision, not an account-wide completeness claim. Host must supply current root/provider/public-fence and required lifecycle closure; unavailable provider bridge stays Unproven. Q3 revalidates original live public/root/provider revisions before presentation; immutable job copies cannot establish continued live currentness.

Input manifest≤256KiB; files≤2048, total original JSONL+blob+all evidence/Git/key/artifact bytes≤64MiB (no extra allowance); each record/policy≤2MiB and source line≤256KiB, existing depth/count/owner/CPU/credit/disk/output caps remain. Every retained decoded graph/catalogue and overlap is admitted within child78/16/2MiB. Excess/missing evidence returns the existing resource/unproven errors, not an expanded scan. One exact real scripted publication can fit this subset; an incomplete proof set cannot produce an available lesson. An admitted empty candidate set may produce the existing verified empty page.

Accepted source points: resolve.rs:115-150,323-388,414-444 and audit.rs:198-229 show why new selected constructors/catalogue traversal are necessary; trust.rs:465-591,660-692 supplies public policy/signature/anchor checks; recall.rs:28-125 and receipt.rs::check_outer/check_publication/check_adoption supply read validation. Existing async whole-audit/whole-log wrappers are not presumed paced. Reuse accepted c1.rs::Budget/unit_clock, Paced/disposal::Drain, storage and Dir plus independent control/actual-join primitives through a narrow crate-private facade or extraction; do not weaken inspect-c1 or create a new resource scheduler.

Output: {schema:"bboxy.dream.read-snapshot/1",nonce:Ulid,context:Context,
 input_digest:Hex,source_revision:Hex,producer:Hex,trust_policy:Hex,
 entry_ids:[Id],coverage:Coverage,pages:[PageRef]}.
PreparedEntry carries Hit plus pinned safe text and bounded evidence, with LessonProof only after successful audit, exact record currentness, qualifying trial, active harness pair, accepted publication and outer seal. No new adoption, synthesis, trial execution, trust enrollment or sink publication. No Recall event is emitted merely by building an index. Missing native/provider evidence bridge leaves that source unavailable; legacy Git trust remains explicitly selected and signed, not a fallback for drafts. Consumer checks private custody, process exit, descriptor/page digests and live fence; bytes and a “verified” field alone confer nothing.

Preparation shares C1 job/root/account slots and its fixed memory/foreground/control/settlement/ABA law, including measured opaque overhead and the cooperative/native/debt distinction [[decisions/d93377961365086-dream-native-quantum-and-debt]]. Application/control/cleanup stay1ms; only enumerated isolated native calls have2ms CPU/25ms wall measured gates with full inclusive charging/rest and retained effects; no blanket Git/GPG/SQLite/serde/helper exception or new wire field; heavy work requires authoritative source closure. Its protocol/schema fixtures and capability probe are separate from C1 v1, so existing inspection wire is not silently expanded. Source/currentness changes atomically invalidate the prepared read epoch before further presentation. No opportunistic tool-call audit, provider scan, network or inference.

### Q2 public-proof helper purpose admission

[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]] extends only process_create admission for exact prepared public-proof Git/GPG creation, with unchanged2ms caller CPU/25ms native wall/3ms inclusive caller gate,1ms application/control/cleanup and9*max(1ms,C) stopped rest. No whole-command native exception. Use the existing isolated proof owner and single per-job controller; no shared reactor launch or extra pool. One helper group/job, at most two processes (Git and one admitted GPG) and256 consumed creation reservations/job including implicit children; reserve two before signature Git, one otherwise, no refunded retries. This is a bounded supported subset, not a guarantee all512 selected pins finish in30s.

The decision's exact form table is the allowlist: selected root rev-parse/config checks; cat-file -t; merge-base --is-ancestor; exact ls-tree -z/show record blobs; rev-list -1 and rev-parse blob/tree anchors; log -1 fixed signature format; public-only GPG show-keys/import from bounded stdin and Git's captured verify-child argv (its signature operand is Git's own temporary bound by [[interfaces/dream-proof-tool-qualification]] Signature temporary, never a fixture path). No generic command string/subcommand allowance. Host locally provisions absolute executable paths+hash/build/capability receipt, including Git's exact GPG child behavior; no PATH/shell/agent/network/operator-keyring fallback. Effective argv/env/config is fixed, env_clear plus explicit private job values;≤64 argv elements and32KiB total argv+env, existing4096-byte paths. Selected key bytes/stdin and stdout each≤2MiB;4KiB IO chunks; existing8MiB/256-entry pure-read cache charged in WORK. Every uncached read/import/inspection/signature counts; successful cache bindings include immutable bundle/root/policy/tool/incarnation. Missing tools/coverage yields verification_unproven.

Creation binds PID/start/PGID/effect and controller custody before any verdict. The prepared posix_spawn sets SETPGROUP and background QoS (posix_spawnattr_set_qos_class_np QOS_CLASS_BACKGROUND) atomically; START_SUSPENDED is permitted, no longer required. Helper LIFETIME is governed by [[decisions/d93377961366280-dream-q2-external-helper-budget]], which supersedes 65126's per-grant slicing, all member-window accounting of [[decisions/d93377961365489-dream-q2-member-window-accounting]] and the per-grant basis/W≤25ms/conservation/T/N_reaped law of [[decisions/d93377961366236-dream-q2-measured-quantum-and-wall-ceiling]] for external helper processes. The65086 cooperative1ms law is unchanged for every unit on our own threads. No CONT/STOP grants, per-grant Q, windows or per-grant roster closure.

QoS gate: before first pipe service the controller observes each live member's base priority; it must equal the probe-recorded throttled band (Darwin4), including Git's GPG child when observed; mismatch kills the group, resource_limit. CPU: ≤250ms per command and ≤3s per job across helpers, user+system including reaped descendants. Live RI user+system reads inside ordinary≤1ms control units at least every50ms wall SIGKILL the group on breach; the leader's own wait4 (subtree roll-up, each µs field rounded up by1µs) is the terminal verdict, so wait4>cap is resource_limit even if no live read saw it; excess retained, never clamped. After the leader is reaped the job pays rest≥9*max(1ms,wait4 CPU) before its next helper or main expensive span; no banking/reset. Wall: existing30s job deadline only. Memory: existing shares below plus terminal wait4 ru_maxrss≤16MiB per process the controller reaps. Foreground: one group SIGSTOP; positive identity-bound stopped ACK (same PID/birth/start/incarnation with zero running threads, or retained exit) inside the existing100ms pause gate; RI user must not grow until one SIGCONT after current activity/authority admission; no CPU charge for the paused interval. Controller stays nice+10/utility, no elevation; the proof owner runs no concurrent expensive job work while a helper lives. A final group-kill EPERM/ESRCH (Darwin returns EPERM for a zombie-only group) enters the pending-terminal state of [[decisions/d93377961366050-dream-failed-stop-independent-terminal-proof]], retaining raw errno; own wait4 plus group-gone resolves it to settled with a group_kill_raced_exit observation, otherwise settlement_unknown with [[decisions/d93377961366074-2026-11-06-dream-q2-quarantine-owner-stays-alive]] owned quarantine. Missing/reversed clocks or counters refuse and preserve custody, never zero.

Retain caller app/native/inclusive CPU+wall, controller CPU, grant/debt, direct+descendant usage and coverage, group RSS samples/high waters and actual exit/group-gone. A direct-child wait4 number or group-gone is not full descendant CPU proof. Unsupported/extra/escaped child behavior refuses trusted output; exact provisioned Git→GPG roll-up needs a real witness. The16MiB opaque/helper share covers ALL helpers plus existing opaque overhead, never per child. Preserve main observed heap≤96MiB, observed group RSS≤16MiB and additionally main observed heap+group RSS≤96MiB, parent≤32MiB; label the mixed admission metric and sampled coverage, never call it whole-tree heap or an all-times proof. All capture/cache/control metadata remains pre-reserved in78/2MiB.

Keep original operation/effect/incarnation through failure/ABA/restart; retire signal custody with leader unreaped, terminate/reap and establish real group settlement before detach or trust. Unknown exit/accounting remains owned quarantine; no PID-reuse signaling or duplicate restart. Corrupt proof maps verification_failed; cap/timing/output/memory breach resource_limit; unavailable supported accounting verification_unproven. Existing closure/currentness/30s/slot/cancellation laws and wire/schema/artifact pins stay unchanged. Model Q2HelperTotalBudget (BackgroundOnly, TrustWithinCap, NoBurnWhilePaused, DebtBeforeNext, SettlementHonest) plus its causal controls; actual signed publication must complete, corrupt proof fail, and four independent qualifying jobs meet unchanged control/duty/memory gates under make test-dream-mcp and source-bound formal parity. Current failed log is evidence of the gap, not successful preparation.

Lifetime capability additionally requires [[decisions/d93377961365654-dream-q2-tool-lifetime-basis]]. Existing group-members-2/single-threaded-helpers claims are NOT established by sampled TASKALLINFO/RI/Trace2. Current Apple Git-157/2.54.0 + Homebrew GPG2.5.24_1 is verification_unproven; no qualifying retry or available lesson. Selected prerequisite: one private source-built real Git/GPG pair, preserving ALL65126 command/signature semantics, with reviewed finite lifetime assumptions and authenticated immutable source/build/config/recursive-library/platform bindings. One-thread/two-process behavior of those exact reachable paths is an explicit trusted source/build premise, not a theorem derived from runtime snapshots. Source-built or NO_PTHREADS alone is insufficient (Git async can fork); GPG common.conf/keyboxd/agent/helper and transitive paths need an exact basis.

Use existing32KiB proof-tools/1 receipt fields and128KiB capability-evidence budget: git/gpg hashes/builds bind artifacts; source_sha256 binds actual provisioner/guard; capability.evidence_sha256 binds host-approved bounded source/build/command/dependency/assumption/check+witness evidence; OS/arch plus evidence bind exact platform. Provisioner/guard must actually admit that matching qualification BEFORE birth, not merely check hash syntax or append feature strings. No new strict DTO/wire/schema/version field or second authority store is selected. Missing/stale qualification or tool/library/OS/config/loader mismatch returns verification_unproven; changed input/custody, invalid proof and measured cap breaches retain existing source_changed/verification_failed/resource_limit semantics and original-effect settlement.

Runtime QoS/CPU-cap/wait4/debt/foreground/control/heap/reap checks stay mandatory and may refuse a qualified profile. Preserve unknown fork/exec/retired coverage; no source premise waives runtime evidence. Causal verification includes hidden transient process/thread, extra/escaped helper, library/OS/tool mismatch, forged/missing/stale receipt and debt/accounting/settlement failures; altered mutant tools fail static approval before birth rather than pretending snapshots always detect them. Real qualified Git→GPG positive, corrupted proof, four jobs/foreground/heap/reap and source-bound formal gates remain. No NOTE_TRACK/new tracer/controller/entitlement or runtime launch is authorized by the design; Oracle must amend implementation scope separately.

[[decisions/d93377961365875-dream-ksba-private-probe-boundary]] adds exact source7 Libksba1.8.1/API1, authenticated under the existing release roots; fixed source-lock/qualification counts are7, not variable, with unchanged32KiB/128KiB limits and worker proof-tools/1 fields. Actual configure/link output determines runtime/build-only closure. No required-library-check bypass or ambient Homebrew borrowing.

Only canonical Q2-owned private-candidate component mode may test the real source-built unapproved pair under the explicit builder test brief. [[interfaces/dream-proof-tool-qualification]] owns exact pre-test input/CLI/size/identity/hash/phase/raw result contract; builder consumes Root's new frozen probe, never forks/edits it. Normal git mode remains fail-closed. Expected input hash binds integrity, not runtime authority; no generic approved/true flag, future receipt/report hash or new controller. Same canonical QoS/CPU-cap/debt/control/foreground/clock/ABA/reap rules apply. Private result is always runtime_approved=false, has conditional source-review basis and measured outcomes separately, and is rejected as runtime/lesson evidence by itself. Root alone amends live consumption and admits the later exact runtime premise; no public protocol/schema/IR/version change. Private pre-test uses ONE strict private-pretest-basis/1 file with fixed build/review/closure/config/commands/fixture projections, all six outer pins identical; its exact required fields/limits/canonical bytes and pre-birth cross-binding/coverage checks are solely owned by that builder ICD. Outer-only inputs remain review_unbound. Canonical architecture is aarch64, with only named Darwin arm64 capture normalization and exact tool/compiler/SDK/OS/loaded-image matching. No same-version, hashed-prose, missing coverage or self/future/current-probe-source hash bypass; original16KiB/128KiB/32KiB/64KiB and runtime budget/custody laws remain.

### Worker birth ownership (inherited private profile)

Q2 c1.rs::run<J>/ParkedCompute, worker.rs/read.rs entry and its future term launch use the ONE producer-owned dream-bootstrap/1 permit/lease from [[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]] and [[interfaces/dream-term-c1]]. Same fixed128-byte layout/conformance, inherited fd/birth/run/nonce/context/incarnation/tool binding, first-controlled-argv-span cutoff, parent paid grant→child paid Hello→parent paid Start, source-free≤1/account probe and original-effect/EOF/restart settlement. A separate read capability does not waive birth fencing. No ungated old-binary probe, second helper/controller, wire/schema/control-version change or aggregate duty claim from independent clocks. Tiny bounded grant/control I/O is explicitly measured bookkeeping; source/proof/setup/native work retains existing budgets. The Git/GPG helper admission in65126 remains separate. Held-parent-after-spawn and bypass/debt/revoke/unknown-effect causal model/process controls accompany existing Q2 qualification.

[[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]]: source-free probe=true serializes context:null explicitly in C(binding), never omitted/empty/previous-seat/fixture context. Keep fresh original op/run/nonce/birth, nonzero monotonic host launch incarnation and approved source/binary/compiler/hash/profile; no actor/root authority is fabricated before Runtime::new's capability probe. Probe permanently rejects ALL Start, even matching IDs, without manifest/source/provider read; parent uses paid Hello→EOF→actual reap only. Real probe=false requires full captured current public context and closed-source authority before birth, with no null/missing-proof fallback. This changes only private canonical binding semantics, not public Context/worker/schema/IR fields.

[[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]]: only the two final handoff attempts (permit and authorized Start) use the PREPARED fresh public-authority callback plus tiny transition/write as one inclusive≤1ms CONTROL step, without recursive9x work debt. Use the existing isolated admission owner/protected reserve and independent Status/Cancel; no SQLite/IO on hot/message/UI/reactor owners. Allow only≤7 fixed public rows, each≤16KiB bounded BEFORE fetch/hash (≤112KiB digest input), exact bounded actor scalar/primary-key predicates, account-file/inode/revision/time/issuer/head/grant/device and nonblocking provider/session currentness, including post-transaction checks. Connection/statement/key/snapshot and binding preparation remain ordinary work before debt payment. At most one final check per actual handoff attempt; no polling, cached/atomic-only permission substitute, stale reuse across await or changed accounting for other guard calls. Unknown/busy/oversize/clock/>1ms refuses/revokes and retains real Child/pipe/lease effects. Probe has no fake final actor proof. No source/payload/binary hash, generic SQLite/serde scan, credential/signature/provider process or blanket helper exception; all previous limits/settlement stay.

## Read offers, feedback and Recall

Successful read output creates at most one bounded offer digest over actor incarnation, request, snapshot/item/proof pins and exact emitted content. Store it through the existing own-seat host journal, with idempotent original operation identity where effects exist. “Offered” is transport provenance, not evidence that the model read/used it. Cache contents and offers are projections/observations, not a second knowledge authority.

Feedback requires a prior offer to this actor, ≤8 already-readable pins and note ≤512 bytes. Enforce (actor incarnation,op_id,request digest) before effects; retry returns the same report, changed request returns op_id_reused. Append a bounded reported-only event to the own-seat journal, not KB memory text. Stop acceptance after scope revocation. Feedback can be submitted during live work because it performs no consolidation/inference. Saturation returns busy; reserve remains available for C1 Cancel/Status. No feedback alone changes rank authority, trust, adoption, expiry or revival.

For an actual harness IR writer, mirror the report as existing Item::Native{provider:"bboxy",record_type:"dream_feedback/1",raw:bounded_report,blob:null}; do not mutate closed source IR. For an actual validated lesson citation, the producer exposure recorder may emit existing Item::Recall with basis=Cited and exact record/sink/block pins after durable append. Reported is reserved for existing Context::append's durable injection path. Native tool fetch alone is never Inferred/Reported/Cited Recall; actual client transcript observation and supported publication audience are required, otherwise retain a reported-only host offer. This needs a typed live-writer bridge, not reading arbitrary native logs. Trials keep automatic Recall suppressed. All memory content remains quoted historical data beneath user/repository instructions.

## Negotiation and standalone behavior

Main bridge revisions stop at2025-11-25; engine dispatch handles CallToolResult. Q1–Q3 use ordinary tools and dream_c1 handles. A future2026-07-28 adapter requires server/discover capabilities.extensions["io.modelcontextprotocol/tasks"] and the client's request-local params._meta["io.modelcontextprotocol/clientCapabilities"].extensions["io.modelcontextprotocol/tasks"]={}. This declares support, not a preference: the server chooses CallToolResult or CreateTaskResult; the client handles either. Prior declarations do not authorize a task response. Without the request capability, a server unable to return an ordinary result uses -32021. See [official Tasks capability negotiation](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks), fetched2026-11-05, publication date not stated. Preserve original job identity; do not mix redesigned Tasks with experimental2025 tasks/result. No progress model wakes or elicitation. Resources remain skills until separately implemented; no duplicate index/prompts.

Standalone harness with no term admission does not invent a seat/account. It may use an explicitly configured local prepared publication via the same proof rules, or return unavailable. Existing CLI audit/graph/why remains manual inspection, not a synchronous live MCP fallback. Provider-pluggable KB proof support is explicit capability negotiation; no unsupported provider is treated as empty or trusted.


## Internal runner conformance and examples

The new runner uses UTF-8 JSON+LF frames ≤256 KiB and the C1 sequencing/cancellation/settlement rules, **not** a changed inspect-c1 protocol. Hello={type:"hello",protocol:"bboxy.dream.read/1",algorithm:"read-prepare/1",schema_sha256:Hex,ir_major:1,ir_minor_max:3}. Start={type:"start",protocol,run_id:Ulid,nonce:Ulid,input_sha256:Hex,timeout_ms:u32,activity_revision:u64,foreground:bool}. Activity={type:"activity",protocol,run_id,nonce,revision:u64,foreground}; Cancel={type:"cancel",protocol,run_id,nonce}. Progress={type:"progress",protocol,run_id,nonce,seq:u64,phase:"validate"|"prepare"|"paused"|"settling",bytes:u64,envelopes:u64,cpu_ns:u64}. Done={type:"done",protocol,run_id,nonce,seq,outcome:"completed"|"failed"|"cancelled",result_sha256:Hex|null,pages:u32,error:PrepareError|null}. Timeout is 1..30000 ms; exact field/protocol/type requirements, monotonic sequence, nonce/context and all terminal restrictions mirror C1. PrepareError={code:"bad_request"|"source_mismatch"|"source_changed"|"unsupported_ir"|"blob_mismatch"|"producer_unapproved"|"verification_unproven"|"verification_failed"|"harness_incompatible"|"resource_limit"|"storage_unavailable"|"protocol_error"|"cancelled"|"deadline"|"worker_lost",source:u32|null}. verification_unproven maps to bounded Unavailable/Inconclusive; verification_failed maps to public protocol_error, never a verified empty success. No task-input request or inference instruction exists.

Fixed input/manifest/output directories and exclusive/private inode rules mirror C1 custody. Input bytes at input/manifest.json are hashed by Start; output pages are page-NNNNNN.json, containing {schema:"bboxy.dream.read-page/1",run_id,nonce,index:u32,entries:[PreparedEntry],next:u32|null}. PageRef={index:u32,len:u64,sha256:Hex}. PreparedEntry={hit:Hit,why:String,how_to_apply:String,context:String,citation:String,nodes:[EvidenceNode],edges:[EvidenceEdge]}; hit.kind is lesson and its proof is LessonProof. why/how_to_apply each ≤2048 bytes, context ≤8192, citation ≤256; whole entry ≤12 KiB. Pages ≤16 KiB, complete descriptor+pages ≤4 MiB. Entries that cannot be wholly represented are omitted with explicit coverage, never partially certified. Snapshot descriptor schema above also carries run_id,nonce; entry_ids is the ordered [Id] index, while payloads live in pages. Its result_sha256 in Done is the exact output/result.json descriptor hash. One empty page represents a verified empty prepared selection. Publish pages before result.json; consumer validates total/index/digests before installing the immutable index.

Illustrative first-cut calls (not observed results):
~~~json
{"v":1,"kind":"observation","query":"","limit":3,"budget_tokens":2048}
{"v":1,"handle":"opaque-offer-bound-handle","view":"evidence","budget_tokens":2048}
~~~
With Q2/Q3 enabled an agent asks dream_search with kind:"lesson", query:"retry cancelled subprocess"; then dream_read context/evidence for one returned handle. A missing publication returns status:"unavailable",reason:"no_published_input",run_id:null,retry_after_ms:null. An observation context request returns unavailable/not_a_lesson, never a generated workaround. Feedback with outcome:"contradicted" references that read's offered digest and a retained same-scope failing-test pin; it records reported_only and does not retire the lesson.


## Two-session aggregate conformance case (Q1)

This is a concrete fixture prescription, not an observed run. Use the canonical existing C1 writer and private closed-source fixtures; compute real digests, do not substitute the symbolic names below into JSON. Select two distinct sessions S_A=01ARZ3NDEKTSV4RRFFQ69G5FAV and S_B=01ARZ3NDEKTSV4RRFFQ69G5FAW in repo R, both closed and admitted to run U. F is the same exact64-hex fingerprint in each episode. M=SHA256(exact manifest bytes), P=SHA256(exact page0 bytes), D=SHA256(exact descriptor referencing PageRef{index:0,len:page0.len,sha256:P}). Source closures/blob evidence stays in the unchanged C1 manifest.

| page0.rows index | Exact selected row metadata |
|---|---|
| 0 | Episode session S_A, turn T_A, origin {session:S_A,seq:3,digest:E_A}, fingerprints:[F], first_ms:10,last_ms:20 |
| 1 | Episode session S_B, turn T_B, origin {session:S_B,seq:7,digest:E_B}, fingerprints:[F], first_ms:30,last_ms:40 |
| 2 | Cluster repo:R,fingerprint:F,count:2,first_ms:10,last_ms:40; **no origin** |

Let B(i)={run_id:U,manifest:M,result:D,page_index:0,page_sha256:P,row_index:i}. Cluster hit.proof={kind:"closed_ir_cluster",row:B(2),members:{basis:"result_rows",links:[],total:2,omitted:2,complete:false},trust:"not_evaluated"} is the sufficient minimum. If optional membership was fully checked, links:[B(0),B(1)],omitted:0,complete:true is valid. If only B(0) is offered, omitted:1,complete:false is mandatory; its EventPin E_A does not prove S_B. Episode0 proof instead has kind:"closed_ir_episode",row:B(0),origin:{session:S_A,seq:3,digest:E_A},trust:"not_evaluated". Evidence exposes these pins/metadata, never transcript bytes. A query for F investigates recurrence; “how do I fix timeout” cannot be matched from F and observation context remains unavailable/not_a_lesson.

Golden/negative controls must: change cluster.count while retaining P (reject page hash); recompute P but retain D (reject descriptor binding); change row_index from2 to0 while claiming cluster (reject kind); supply a member from another run, repo or a nonmatching fingerprint (reject membership); duplicate B(0) (reject); omit B(1) while complete=true or omitted=0 (reject); and retain the valid zero-link/one-link incomplete cases. A clipped search index/page may reduce displayed rows without changing cluster.count or falsely claiming complete coverage. All witnesses use existing result pages, without source rereplay, raw export or a changed C1 schema.
