---
type: interface
title: C1 Dream closed-IR inspection wire v1
derived_from:
- '[[specs/dream-term-integration]]'
links:
- to: '[[specs/dream-term-c1]]'
  rel: spec
id: 01M6NP15EEZYG86TF9Z6GDMER7
updated: 2026-11-06T16:07:58.701Z
created: 2026-11-05T11:03:16.536Z
number: 93377961364368
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:cafadb995c0fc2dd79cb22d6431fe929f6c7602dcf804ba6cdde691d94036a60
  frontmatter_digest: sha256:16003c038cab256842533d202edf42e36c1179fcd97e4ef1e10f4d90616ff809
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: on+7bYsROm4aDL+nAnwzRqALkOcXphdWDzXPy6k0jJaBZscW4yjdVXdjQ3od4T4Vean3cJGwfrCjrCp2hEDcDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149542
  event:
    id: 01M6RSVTJQMY31MJS3Y55Y198X
    sequence: 149542
    hash: db97776099308cb2c3b43dc63d03c0211985f6f8b8d83a50c5b6b4029f95837f
---
# C1 Dream inspection wire, version 1

Normative C1 boundary for [[specs/dream-term-c1]]; narrows [[interfaces/dream-term]]. Baseline: term ad243195daf74153756308346d01a56390c1261c, harness a2ca8f393630081b6be583151293ae286dc2419e. No inference/adoption API is part of v1.

## One schema owner and provisioning

Harness producer owns src/dream/c1_wire.rs (serde + schemars), generated schema/dream-c1-v1.schema.json, and tests/fixtures/dream-c1/{valid,invalid,vectors}. JSON Schema and fixture locks are generated/checked from that Rust definition. All structs deny unknown fields; enums are closed snake_case. No default silently changes a mode or version. Term mirrors only these transport/control DTOs in its leaf transport/proto crates; the schema is not independently edited there. Term's canonical test consumes the pinned harness corpus via DREAM_C1_FIXTURES, with Rust decoding/encoding and an independent Python stdlib canonical-JSON/hash vector check. It must fail on missing fixtures, changed schema/hash, unknown fields/enums or disagreement; no full harness library dependency in mux and no second fixture authority.

Launch the existing absolute sibling bboxy450-harness executable, with new arguments:

    dream inspect-c1 --job-dir ABSOLUTE_PRIVATE_JOB_DIR

This uses existing bundle-relative discovery (term adapters/mod.rs:72-98). Makefile:282-284 already builds app+harness together and :645-651 packages both; make build / make install-unsigned remain the provisioning path. Dev make build-harness (:2195-2196) is available. Missing sibling is runner_unavailable, never PATH fallback, runtime download or automatic build. This subcommand is proposed, not present in the baseline.

Term main.rs:10061-10063's harness-exec calls launch::exec_when_ready; wake-up launch.rs:810-834 waits for a native seat socket before external exec. A non-seat C1 pipe worker needs no seat-readiness wait or login shell, so launch the same provisioned executable directly through the existing owned-process mechanism. Harness main.rs:9-14 selects headless; headless.rs:81-84 dispatches dream. Producer adds inspect-c1 dispatch there, before profile/HOME/session discovery. Launch with a minimal cleared environment (LANG, private job TMPDIR and the single frozen BBOXY_DREAM_BOOTSTRAP entry specified below); inherit no HOME/profile, seat capability or provider variables. No model/profile/capability file is passed.

## Private bootstrap handoff (public C1 wire unchanged)

[[decisions/d93377961365166-dream-bootstrap-private-permit-handoff]] owns profile dream-bootstrap/1 and its exact128-byte binary permit table. Producer c1.rs owns its parser/fixture; consumer wake-up mirrors it with byte-exact conformance. Parent creates one anonymous pipe, inherits only read fd N (3..65535), and sets BBOXY_DREAM_BOOTSTRAP=1:<N>:<birth ULID>:<binding Hex> (≤128 ASCII bytes). Parent alone keeps the write end, retained Child/PID-start/group/original effect. No source path, credentials or capability bytes cross this gate; labels without the live inherited pipe/valid bound record cannot admit bootstrap.

binding hashes canonical profile,purpose,probe,original op/run/nonce,monotonic launch incarnation,captured public context and exact worker hash. Permit layout is fixed magic/purpose/probe/zero padding/birth/run/nonce/binding/incarnation/sequence1/absolute monotonic deadline, as tabulated in the decision. One128-byte write attempt; no partial/unknown retry or replacement birth. Gate state/scratch≤1KiB/side inside existing2MiB control shares. Keep the writer open as a revocation lease until matching Hello; EOF/extra bytes/mismatch/expiry prevents another controlled bootstrap unit and preserves any real effect for settlement.

Child entry waits before current early_worker_entry argv Vec/PathBuf/Arc work, not only before Builder. Source-free borrowed recognition/fd parsing/wait are bounded measured control. Parent pays full inclusive native/application debt before permit; it performs no expensive preparation while released child bootstraps. Child pays all bootstrap debt, retires its read end and sends existing Hello while parked before Start. Parent validates the approved profile/Hello, retires lease, performs/pays preparation, then existing Start transfers expensive ownership to child. Tiny permit/lease/Hello/Start I/O and owner transitions are≤1ms control bookkeeping without recursive work debt; all costs remain in control/full-process metrics. No loader/kernel preemption or all-times process-CPU claim.

The 2 s readiness window (`dream_bootstrap::WINDOW`; 250 ms until 2026-11-12, which a debug child on a loaded machine missed) is not reset at release; child entry wait≤WINDOW and the bound permit deadline is additionally limited by the original30s job deadline. Parent foreground/cancel/revoke/clock error closes lease and retains process custody; restart cannot reconstruct a private pipe, replay a permit or reset debt. No source/manifest read before matching Start; unknown effects/exit retain the slot/quarantine. Current public context/ABA, native2msCPU/25mswall/inclusive3ms, application/control/cleanup1ms, memory/UI/foreground/reap laws remain.

Approved source/binary/compiler/features receipt must establish bootstrap support BEFORE birth; an ungated old worker is not a safe feature probe. Same≤1/account source-free probe uses permit→paid Hello→EOF→actual reap, no Start. Missing/unsupported profile returns runner_unavailable for managed Dream while ordinary seat/Legacy behavior remains. Manual new inspect-c1 without gate refuses before controlled startup/source work; private launchers must implement it. Consumer/producer models and real held-parent-after-spawn/duplicate/stale/EOF/revoke/clock/probe controls are required; original schema/serde/IR/algorithm/control version and artifact hashes gain no host fields.

[[decisions/d93377961365255-dream-bootstrap-source-free-probe-context]]: source-free probe=true serializes context:null explicitly in C(binding), never omitted/empty/previous-seat/fixture context. Keep fresh original op/run/nonce/birth, nonzero monotonic host launch incarnation and approved source/binary/compiler/hash/profile; no actor/root authority is fabricated before Runtime::new's capability probe. Probe permanently rejects ALL Start, even matching IDs, without manifest/source/provider read; parent uses paid Hello→EOF→actual reap only. Real probe=false requires full captured current public context and closed-source authority before birth, with no null/missing-proof fallback. This changes only private canonical binding semantics, not public Context/worker/schema/IR fields.

[[decisions/d93377961365259-dream-bootstrap-final-public-admission-control]]: only the two final handoff attempts (permit and authorized Start) use the PREPARED fresh public-authority callback plus tiny transition/write as one inclusive≤1ms CONTROL step, without recursive9x work debt. Use the existing isolated admission owner/protected reserve and independent Status/Cancel; no SQLite/IO on hot/message/UI/reactor owners. Allow only≤7 fixed public rows, each≤16KiB bounded BEFORE fetch/hash (≤112KiB digest input), exact bounded actor scalar/primary-key predicates, account-file/inode/revision/time/issuer/head/grant/device and nonblocking provider/session currentness, including post-transaction checks. Connection/statement/key/snapshot and binding preparation remain ordinary work before debt payment. At most one final check per actual handoff attempt; no polling, cached/atomic-only permission substitute, stale reuse across await or changed accounting for other guard calls. Unknown/busy/oversize/clock/>1ms refuses/revokes and retains real Child/pipe/lease effects. Probe has no fake final actor proof. No source/payload/binary hash, generic SQLite/serde scan, credential/signature/provider process or blanket helper exception; all previous limits/settlement stay.

## Bootstrap entry record

Canonical law. Rationale: [[decisions/d93377961365960-dream-launch-entry-and-signature-boundaries]] A, as corrected.

1. **Producer.** `Gate::wait` makes one `write(2)` of exactly one line, immediately before the unchanged waiting line:
   `dream-bootstrap entry s=<u64> g=<u64> e=<u64> m=<u64> u=<u64> k=<u64> r=<u64> p=<u64> ok=<0|1>\n`
   - Key order is fixed; values are unpadded decimal u64 (≤20 digits); the line is ≤256 B including LF and built in the existing stack `Line`.
   - Sample order: e is the existing early_worker_entry CLOCK_MONOTONIC entry sample. Then m = CLOCK_MONOTONIC. Then `proc_pid_rusage(getpid(),RUSAGE_INFO_V4)` gives s=ri_proc_start_abstime, u=ri_user_time, k=ri_system_time, r=ri_runnable_time, p=ri_pageins. Then g=mach_absolute_time(). Then the write.
   - ok=0 if and only if rusage failed; s/u/k/r/p are then 0 and the launch continues. Existing clock failures still refuse.
   - Its CPU lies inside the existing pre-permit `usage.observe` and is counted once.
   - It adds no heap, fd, thread, env, argv, public wire or profile change, and leaves the 250 ms/30 s/1 ms and every other law unchanged.
   - This is ordinary behaviour, not diagnostic instrumentation.
2. **Primary evidence.** These are unit-safe comparisons on the same clock id. The parent records that it uses CLOCK_MONOTONIC via clock_gettime; any other clock means no comparison.
   - `spawn_return → e`: the child's own entry sample. It covers exec, loader, runtime and any scheduling delay, and is not decomposed.
   - `e → m`: borrowed recognition.
   - `m → parent first observation of the entry line`: m is a pre-write sample, so the true write time lies somewhere in [m, observation].

   No phase or OS cause is inferred beyond these bounds.
3. **Auxiliary raw counters.** s, g, u, k, r and p are retained verbatim. Their units, their inclusive or overlapping semantics, and the time domain of s are NOT established: only the SDK `rusage_info_v4` layout was read, not the exact XNU source for the running kernel. That covers, for example, whether runnable time overlaps CPU time. No conversion, subtraction or class is derived from them until a recorded, source-grounded premise exists.
4. **Consumer.** The consumer requires exactly one entry line before the waiting line, with fixed key order, numeric values and ≤256 B.
   - Missing, duplicate, misordered, oversize or non-numeric line, e>m, or a reversed clock → `entry_evidence=unknown`, never readiness success.
   - Waiting line present but entry line absent → producer defect.
   - Neither line by reap → unknown.
5. **Verification.**
   - Producer model invariants: at most one entry line; entry before waiting before any permit read; alloc=0; CPU inside pre-permit usage.
   - Producer broken controls that must fail: emit after the permit read; allocate in the formatter; emit twice; skip `observe`.
   - Producer witnesses: exact bytes for maximum u64; the ok=0 path.
   - Consumer: the negatives in rule 4.
   - Run: one ordinary joined canonical 250 ms gate on the new exact tuple, with no wrapper, warm-up or retry. A pass certifies that run only. A failure stays failed, and its primary evidence places the gap before entry, inside recognition, or in observation.

## Scalars and canonical bytes

Ulid = canonical 26-character uppercase ULID. Hex = 64 lowercase hex digits. Id = nonempty UTF-8 string ≤256 bytes, no NUL; Repo ≤4096 bytes. Integers are nonnegative JSON integers ≤2^53−1 (nonzero generations/seq); no floats. Option fields below may be absent or null; all other fields are required. Paths are never supplied by an agent.

Canonical JSON C(x): object keys sorted lexically (fixed ASCII keys), UTF-8 strings unnormalized, standard JSON escaping, no insignificant whitespace, integers in decimal; array order as specified below. Hash = lowercase SHA256(C(x)), without prefix. File/page hashes cover exact bytes without a trailing LF. Existing IR EventPin.digest alone retains the existing sha256: prefix and hashes the exact original line excluding LF.

## Public controls: same shapes for admitted MCP/ctl

~~~text
Inspect = {v:1, method:"inspect", op_id:Ulid,
           sources:[{session:Ulid, source_generation:u64}]}   // 1..32, distinct session IDs
Status  = {v:1, method:"status", run_id:Ulid, page:Option<u32>}
Cancel  = {v:1, method:"cancel", op_id:Ulid, run_id:Ulid}

ControlReply =
 {v:1, ok:true, run:RunView, page:Option<Page>}
 | {v:1, ok:false, error:Error}

RunView = {run_id:Ulid, revision:u64, state:State, settled:bool,
           request_sha256:Hex, manifest_sha256:Option<Hex>,
           result_sha256:Option<Hex>, pages:u32, error:Option<Error>}
State = deferred | preparing | running | paused | cancelling | settling
      | completed | failed | cancelled | interrupted | quarantined
Error = {code:Code, source:Option<u32>}  // index into sorted source list; no raw IO text
Code = bad_request | version_mismatch | not_found | stale_context | op_id_reused
     | busy | runner_unavailable | source_unavailable | source_open | source_changed
     | malformed_ir | unsupported_ir | source_mismatch | blob_mismatch
     | resource_limit | storage_unavailable | worker_lost | protocol_error
     | cancelled | deadline
~~~

An MCP caller can select only retained sessions of its own seat, with exact catalogue source_generation; historical generations require that retained ownership mapping. Current caller process generation/grant/account epoch must be valid. Operator ctl targets one explicit account-owned seat through routing, not an account-wide selector. Status/Cancel require that same live admitted actor context, or the actual account operator; foreign/unreadable run returns not_found without metadata. No Oracle-class exception expands C1 to peers or swarms. KB/provider content is never read, but captured root/provider identity is still fenced.

C1 obtains Context only from the host-authenticated public admission/incarnation/revocation fence specified in [[decisions/d93377961364581-dream-c1-enforceable-memory-contract]]. Host identity admission/public-proof owners publish it; actual account/device/grant/expiry/mute/root/provider changes invalidate monotonic revisions before subsequent work. Bounded off-owner public proof/current-revision checks and final presentation revalidation preserve revocation and A→B→A. No credential custody/key load, signing_hold/private-signing probe, Keychain/GPG prompt or status/admission fallback is permitted in Inspect/Status/Cancel or their monitor. ready_issuer/self-asserted cached readiness is insufficient. Missing/stale fence returns existing stale_context/no expensive admission or retains deferred work for a trusted refresh; it cannot trigger credential fallback. Internal fence publication adds no wire field or authority. Real private-route witnesses must assert zero custody/signing calls and no stale presentation during revocation/expiry/root/ABA changes.

Host supplies context, sorts sources by (source_generation, session), and rejects duplicates. request_sha256 = Hash({v:1, context, sources, algorithm:"d0-c1/1"}); op_id is excluded. The durable (actor incarnation, op_id) lookup precedes source IO. Same ID+digest returns the original RunView; different digest returns op_id_reused. Busy before admission creates no worker. Repeating an admitted operation never rereads/reexecutes it.

An authoritative close also proposes Inspect internally (not a public actor field). Durable unique SourceKey=(account,root revision,seat,source_generation,session,closed_seq,closed_line_sha256,algorithm) coalesces duplicate close/restart/manual selection of that exact single source to one run. Auto op_id is allocated once with that unique row. Explicit multi-source inspections use a sorted SourceKey-set. Deferred/busy handling and closure ownership are in the spec.

## Manifest and fixed custody paths

Consumer owns private per-account runtime dream-c1/<run_id>/, 0700; files 0600, checked same uid, regular/nofollow, no hardlinks to writable source. Host copies exact selected bytes to input/<session>.jsonl and input/<session>/blobs/<Hex>, then commits input/manifest.json last. Producer never opens original source paths, home directories, Git roots or knowledge providers.

Fixed v1 memory allocation ([[decisions/d93377961364581-dream-c1-enforceable-memory-contract]], superseding the universal rule): parent 32 MiB = 30 MiB work (including 8 MiB helper headroom) + 2 MiB protected control; child 96 MiB = 94 MiB work (including 16 MiB helper/startup headroom) + 2 MiB protected control. Reserve declared C1 source/line/parser-state/context/result/control storage before controllable growth, count concurrent capacities and use fallible bounded operations. Hidden serde/BTree/regex/SQLite/native/thread overhead is measured implementation coverage, not fictitious exact ledger bytes; no process-global SQLite/allocator limit or universal OS/native proof is required. Observed parent attributed live working heap ≤32 MiB and full child observed heap ≤96 MiB remain hard regression gates; breach fails. The 128 MiB declared credit sum is mechanically checked; an all-times whole-process/native cap is not established by samples. Separate live storage/helper heap, reservations, cumulative allocations, native overhead, stack/mapping/RSS and separate peak sums versus synchronized samples. Protected controls remain usable on reservation failure; private child exit is reaped/quarantined honestly. Fixed-schema/depth/container/count/thread bounds and real parent+child large-input/overflow/four-job/control/causal-reservation tests in the decision apply without changing wire or CPU/latency/source/settlement limits.

~~~text
Manifest = {schema:"bboxy.dream.c1.manifest/1", run_id:Ulid, nonce:Ulid,
 context:Context, trigger:"manual"|"session_closed", algorithm:"d0-c1/1",
 sources:[Source]}                              // sorted, 1..32
Context = {account:Id, account_epoch:u64, seat:Ulid, generation:u64, grant_epoch:u64,
 swarm:Ulid, swarm_revision:u64, workspace:Id, workspace_revision:u64,
 root:Id, root_revision:u64,
 provider:Id, provider_epoch:u64}
Source = {session:Ulid, source_generation:u64, repo:Repo, len:u64, sha256:Hex,
 closed_seq:u64, closed_line_sha256:Hex,
 blobs:[{sha256:Hex,len:u64,mime:String}]}         // blob order by sha256; mime ≤256 bytes
~~~

The catalogue must originate in exact process/socket/session discovery (harness_log.rs:59-119), retained generation ownership, and observed durable SessionClosed. Before/after source snapshot checks compare fd/path device+inode, size and change timestamps; growth/replacement/torn bytes fails source_changed. No newest-file guess. A closed IR file whose ownership/close event was not retained is source_unavailable, even if a caller knows its path. Idle, TurnEnded, UI detach, missing peer and native EOF do not establish closure. Imported-native source metadata and any SessionMeta.fork are unsupported in C1; reject either at SessionStarted as unsupported_ir before consuming inherited rows or admitting source work. No import is run. This is an explicit C1 subset, not a claim that canonical fork IR is malformed; see [[decisions/d93377961364460-dream-c1-fork-source-subset]].

Manifest bytes are bounded to 256 KiB before decoding. Producer independently validates fixed paths, total sizes/digests, complete UTF-8 JSONL ending in LF, sequence continuity/one session, SessionStarted metadata, terminal SessionClosed at closed_seq and exact final-line hash, transcript laws, canonical repo consistency with the retained source binding, and known IR major 1/minor ≤3. Unknown semantic variants fail unsupported_ir; additive unknown object fields remain in unchanged raw bytes. No line is reserialized into custody. Every typed BlobRef reachable in the IR must appear in that source's manifest inventory with identical len/mime/hash, with no invented/missing refs; validate each actual blob's bytes. Manifest metadata is not evidence authority.

All-or-error: absent/open/changed/malformed source or invalid blob fails the job; no partial corpus presented as complete. Current completion is an inspection result, not trust/audit/adoption.

## Worker protocol and immutable results

Local stdin/stdout use UTF-8 JSON object + LF, no blank lines, ≤256 KiB/frame including LF; reject oversize before allocation, invalid UTF-8, unknown/duplicate keys, wrong protocol/nonce or extra terminal frame. Stderr is bounded to 16 KiB and not echoed as raw source diagnostics. Worker stdout is protocol only. No corpus or result rows travel in an unbounded stream.

~~~text
WorkerOut Hello = {protocol:"bboxy.dream.c1/1", type:"hello",
 algorithm:"d0-c1/1", schema_sha256:Hex, ir_major:1, ir_minor_max:3}

WorkerIn Start = {protocol:"bboxy.dream.c1/1", type:"start", run_id:Ulid, nonce:Ulid,
 manifest_sha256:Hex, timeout_ms:u32, activity_revision:u64, foreground:bool}
                                                // 1..30000 remaining wall budget
WorkerIn Activity = {protocol:"bboxy.dream.c1/1",type:"activity",run_id:Ulid,nonce:Ulid,
 revision:u64,foreground:bool}                  // monotonic; old revisions ignored
WorkerIn Cancel = {protocol:"bboxy.dream.c1/1",type:"cancel",run_id:Ulid,nonce:Ulid}

WorkerOut Progress = {protocol:"bboxy.dream.c1/1",type:"progress",run_id:Ulid,nonce:Ulid,
 seq:u64,phase:"validate"|"replay"|"publish"|"paused"|"settling",
 bytes:u64,envelopes:u64,cpu_ns:u64}
WorkerOut Done = {protocol:"bboxy.dream.c1/1",type:"done",run_id:Ulid,nonce:Ulid,
 seq:u64,outcome:"completed"|"failed"|"cancelled",result_sha256:Option<Hex>,
 pages:u32,error:Option<Error>}
~~~

Start carries the current foreground latch/revision; the worker cannot assume quiescence before receiving it. Hello must match the pinned schema hash/algorithm before Start, otherwise version_mismatch. No filesystem hashing/parsing on the mux reactor to check it. Progress seq increases; identical duplicates ignored, conflicting duplicate or gap means protocol_error (job never certified completed). Progress lane ≤32, coalesce/drop intermediate counters before numbering so it cannot stall compute. Activity/Cancel and host Status use a separate bounded control lane/atomic flags; saturated stdout/result notification cannot block them. On stdin EOF, request cancellation, not a new run. Exactly one Done; exit 0 only for completed/cancelled after owned compute settles, 1 for failed, 2 for framing/usage failure. Host requires matching Done, validated output and actual successful process reap before completed; an exit alone is insufficient.

Producer writes only output/page-000000.json etc and output/result.json (index six decimal digits), via complete temporary file → sync → atomic no-replace publication; never replace differing bytes. result.json is published last and contains:

~~~text
Result = {schema:"bboxy.dream.c1.result/1",run_id:Ulid,nonce:Ulid,manifest_sha256:Hex,
 algorithm:"d0-c1/1",source_count:u32,episode_count:u32,cluster_count:u32,
 max_poll_cpu_ns:u64,knowledge:"not_requested",trust:"not_evaluated",
 pages:[{index:u32,len:u64,sha256:Hex}]}
Page = {schema:"bboxy.dream.c1.page/1",run_id:Ulid,manifest_sha256:Hex,index:u32,
 rows:[Row],next:Option<u32>}
Row = {kind:"episode",session:Ulid,turn:Ulid,
 origin:{session:Ulid,seq:u64,digest:String},fingerprints:[Hex],first_ms:u64,last_ms:u64}
 | {kind:"cluster",repo:Repo,fingerprint:Hex,count:u32,first_ms:u64,last_ms:u64}
~~~

Pages are canonical bytes, ≤16 KiB/page and ≤128 rows; next is exactly index+1 or null. Episodes sort by (session,origin.seq,origin.digest), then clusters by (repo,fingerprint); fingerprints sorted/unique. Cluster count is cardinality of unique episode origins, never sum of delivery counts. Empty result has one empty page. Host validates page sequence/digests/counts/budgets/manifest, then persists immutable result descriptor. Status without page returns only RunView; a completed requested page returns it plus that exact Page. Out-of-range page is bad_request; before completed it is not_found. Source IO, page read/hash and JSON conversion run off owners.

Result contains hashes/pins/counters, no transcript excerpts, tool args, raw notices, draft memories or policy claims. D0 fingerprint semantics are reused; no KB join, repo discovery subprocess, shared cache or audit/graph/why traversal. The spec defines the required selected-input D0 API gap.


## Managed post-close owner clarification
[[decisions/d93377961364464-dream-c1-sole-post-close-owner]] requires the explicit TermC1 launch mode to suppress both automatic legacy D0 replay and automatic IR advice scanning behind one typed close gate. The normal durable SessionClosed remains the notification; the consumer's bounded managed job is the sole post-close work owner in this mode. Standalone/manual Legacy behavior is preserved. C1 does not provide automatic schema advice; no parallel unbudgeted scanner is part of this contract. The large-blob close/foreground witness must cover this boundary. No wire/schema revision is introduced.


## Canonical C1 performance-fixture build
[[decisions/d93377961364684-dream-c1-optimized-private-fixture]] fixes measurement build settings without changing this interface's resource or authority law. make test-dream-c1 uses root Cargo profile dream-c1-fixture inheriting release with debug-assertions=true and overflow-checks=true for private parent/tests/native/retained-seat instrumentation. Keep the actual inspect-c1 production worker ordinary release without test-fixtures; any separately needed feature-gated seat executable is explicitly distinguished. The gate binds source/profile/compiler/features and both binary hashes, preserves cooperative1ms, debt-charged rest ([[decisions/d93377961365086-dream-native-quantum-and-debt]]),30s/heap/foreground/control/latency/frame/settlement limits and fresh authority checks, and retains baseline unoptimized failures separately. No claim of unoptimized performance, production-artifact equivalence for a fixture build or green runtime follows from selecting this profile. Existing ordinary build/install profiles and wire are unchanged.


## Structured MCP host hints
[[decisions/d93377961364692-dream-mcp-structured-host-hints]] applies to dream_c1 transport: use the shared post-journal/off-owner projection with essential guidance in structuredContent.hints and a matching text copy; _meta-only/second-text-only guidance is insufficient for a structured-only model client. These are host MCP metadata fields, never additions to canonical inspect-c1 worker/domain/artifact schemas or immutable result/page hashes. Preserve original operations and source authority, shared idempotence, normal frame/resource limits and strict typed-domain separation. Final integrated admitted MCP/schema/structured-only-client witnesses must consume the actual parallel shared-hints fix; a pasted patch is not completion evidence.

## Cooperative/native execution and metric interpretation

[[decisions/d93377961365086-dream-native-quantum-and-debt]] governs the fixed execution profile without a C1 field/schema/IR/algorithm change. Application, shared reactor/pipe-control polls and stepped cleanup remain≤1ms CPU. Only its named isolated process/thread creation and no-replace-publication calls receive observed native≤2ms CPU/25ms wall ceilings, fixed counts and original-effect custody; surrounding application remains≤1ms. Inclusive span≤3ms does not authorize3ms parser/SQLite/control wrappers. Rest after each expensive span is≥9*max(1ms,inclusive CPU), shared across that job's parent/child work admission; controls remain independent. Source, authority, slots, protected memory,30s deadline, all foreground/latency/frame/reap gates are unchanged.

max_poll_cpu_ns keeps the inclusive observed maximum at the exact field-capture cutoff **before final descriptor serialization/hash/write/publication**, including native intervals already observed; no clamping/subtraction or claim about its own future publication. Terminal disaggregated+inclusive metrics cover that final serialization/publication and later cleanup/control/settlement, with explicit cutoffs and real exit/reap evidence; validate captured maximum≤terminal inclusive maximum. A later violation fails despite an existing descriptor. No self-referential resealing/wire bump; retain post-reap complete process usage and label final telemetry cutoffs honestly. Under this explicit profile, a value>1ms is valid only with exact source/binary/profile-bound native and application/control metrics, never from the scalar alone; completed inclusive spans≤3ms, application/control/cleanup≤1ms, native≤2ms/25ms. Terminal private diagnostics retain complete later startup/cleanup/native CPU+wall/counts/effects/debt/rest and full process CPU. Missing measurement/profile support remains unproven/incompatible; old1ms-only consumers may refuse safely. New numeric diagnostics belong in bounded local metrics/receipts, not worker wire authority or a second protocol version. A post-syscall timing failure retains any actual PID/inode/effect and original op until real settlement; uncertain effects never become absence or an automatic retry.


## Bounded persistence and authority preparation implementation
[[decisions/d93377961366026-dream-c1-bounded-persistence-preparation]] governs the consumer correction after the measured26e5bb1f failures: private C1 catalogues may use supported rollback-journal DELETE with FULL durability, atomic schema/ready-marker and op/run/SourceKey/cursor transactions, busy0 and safe retained-state WAL transition. Account/provenance DB modes are unchanged. Public-authority preparation uses its existing real cooperative phases, separately charged cumulative CPU and maximum-unit evidence; the final permit/Start checks retain the original inclusive1ms CONTROL contract. Every SQLite/cleanup/application unit still obeys1ms and9x debt; no native exception, warm-up, hidden retry, new wire or success claim. Migration/crash/busy/ABA/control/process/formal and final integrated gates remain required; mode choice is not pre-certified by design.
