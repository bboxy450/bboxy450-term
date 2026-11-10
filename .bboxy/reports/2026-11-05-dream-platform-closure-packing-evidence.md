---
type: report
title: 'Offline platform closure packing: lossless roundtrips and explicit non-fit evidence'
author: seat:01M6Q0FHRZV6MQA8PBPXG408N7
created: 2026-11-06T02:16:22.761Z
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
commits:
- 78391c4c82e94fc9f10bad2998c700c6a9278bb7
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
derived_from:
- '[[reports/2026-11-05-dream-proof-tools-provisioner-source7-blocked]]'
id: 01M6QA939TRTV5Y89TKJSJNQVS
updated: 2026-11-06T02:16:22.761Z
number: 93377961365944
created_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
updated_by: seat:01M6Q0FHRZV6MQA8PBPXG408N7
authorship:
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  kind: seat
  public_key: PcKFbgDesoAlYaICRDceGYv79hGHd1L0ZA+qVT0MKXU=
  process_generation: '1'
provenance:
  digest: sha256:5e1c0cdc2b11cd124bb02dbfaf288c7f26b444e2abf02ca0ed3227b51783458e
  frontmatter_digest: sha256:f1e96e0f428ac905776aad18c37267f6872eaadad46ef7cccffa0731ec308ca9
  signer: seat:01M6Q0FHRZV6MQA8PBPXG408N7
  sig: cKPXzZYiVPcqMiOl0a3vPutk4ZulVcSaIsadLsSV8N+Vg/YOHbjViH2CXknCHbyFhxpUtobUjLAlrSviiGKGAA==
  log: .bboxy/log/2026-11-06.jsonl#148736
---
The authorized single bounded OFFLINE representability exercise is complete. It produced mathematical evidence only. No source implementation, normative codec/version/API, pretest Basis, descriptor, qualification, candidate receipt or runtime approval was emitted. No archive/network/OS probe, real Git/GPG/helper/component, build, formal-parity or timing gate was launched. The original full task remains FAILED/UNAPPROVED under its unchanged verify command.

Authority: [[tasks/2026-11-05-dream-proof-tools-provisioner]], Root amendment updated2026-11-06T01:58:27.243Z, full task body SHA df6944701b331663d4f4c015aadace733d69b1358dc49d0d7ae5d56b6d11c672, message79Y1NRG52TAJMFH04BHWGVKBBJ. This report supplements [[reports/2026-11-05-dream-proof-tools-provisioner-source7-blocked]]. Source implementation/artifact tuple remains signed78391c4c82e94fc9f10bad2998c700c6a9278bb7. No source Git command or new canonical gate was run during this exercise.

All generated exercise files are private under:
P=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/closure-packing-79Y1NRG52TAJMFH04BHWGVKBBJ
Directories0700, data0600; files are exclusive, no replacements or cleanup. The generated Python files are local arithmetic artifacts with project/origin-policy headers, not implementation recipes.

Exact input and context:
- R=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Q76F3VZQAXQPYTGXNC2V3S
- R/closure-bound-analysis.json length50,295/SHA23f4f6557bb80f8ee13a494b4e34889eb9c40caab2d37b8643de1fd3812987fb.
- Original build commit9e48c55ddc6e829d148f2ea096191eaae2139ac9, incarnation1397137652045125; intent and identity agree. Original compile recipe SHA c55580d5c355bf275fa4830f14fb1f928c04fed35bc35942e7de92a93d12601d. Later metadata recipe SHA df85a94d191442509bb1337728eccdda444e25d4058c07113a3244730bbb81b1. Both retained recipe files match those hashes.
- Earlier-pair Git SHA b0bf1849928dda4441250a4e00832e4452737f0a91fb44f95a3411200a7621ec; GPG SHA c9ea697144692242077d7bc73c17b9f439768f714de1d0d715041c4aa6ebe90a. These are NOT the final78391 build's tool hashes or identity.
- R/own-platform-images.json length373,800/SHA1586c7d7abbce6099608828dba9e7b56298ea26acb499ea98682ae23c5bd3c17; all390 original JSON records are preserved in the broad round-trip view. The relevant graph uses49 of them;341 unrelated developer-process image entries are explicitly excluded from narrower views.
- Exact retained platform: Darwin27.0.0, raw arm64/canonical aarch64, OS26A434, cache UUID c284a291b68b3d0da1a03f866f87efdc, slide289718272. These are retained sibling-provenance facts, not new current-OS measurements. The original analysis file alone has no complete build/incarnation/OS/cache binding; the study preserves those sibling bindings explicitly.
- All84 distinct input files/16,707,275 raw bytes read by the main script are listed with exact paths/lengths/hashes in P/summary.json. They include retained images, archive/member indexes, captured maps/metadata and recipe bytes; none is executed. File handles check owned regular one-link custody, lengths and before/after read identities. Inputs were rehashed after arithmetic and stayed unchanged.

Coverage and missing metadata:
There are467 distinct observed typed edges:458 load and9 static. They name61 referenced identities, of which60 are known:2 binary roots,9 static archives,49 platform images. /usr/lib/libobjc-env.dylib is an explicit unknown node with metadata=null/outgoing=null. Its outgoing edges remain unknown. The earlier-pair graph is INCOMPLETE lower-bound evidence; it is not complete final78391 closure.
All467 edges reconcile with the recorded real root/platform load lists and selected archives. All49 referenced platform header blobs reparse to the recorded UUID, subtype, raw load paths, rpaths and install names, with address=text+slide; their retained bytes total124,624. Static member selections reconcile exactly with both raw link-map object tables and real member indexes: Git289 selected members from1 archive, GPG215 from8. Every original selected-member name/order is preserved. Repeated destination appearances are408 across distinct load relationships; removing repeated destination strings as edges would lose facts.
Raw Git map1,694,076 bytes/SHA4206e0b86f368017f52148c8fa7110db2b703817358cd09eb83e983679e87b96; GPG835,208/SHAa6fbbdcd261da0619cd4974b8f23dd520375b9d9c357a694433cb95cabc8d8ea. Binary literal strings outside the object tables remain unaltered at their original raw-file refs. No UTF8 replacement, edge deletion or invented missing-node facts were used.

Mathematical encodings tested, not a proposed shared contract:
1. Intern the61 exact node identities and group edges by source. Each row retains original global ordinal, destination index and original mode. Decode sorts by ordinal and checks complete/unique ordinal ownership, preserving original edge order and multiplicity, not just graph set equality. Source groups number61; the largest group has39 rows.
2. Apply deterministic string interning to all keys/values of the whole selected input object. Dictionary and array chunks contain at most64 entries; numeric indices, booleans, nulls and all other fields decode exactly. This is a local arithmetic function, not a new producer/consumer schema. Its actual depth12 exceeds the current private depth10: it cannot be adopted directly even where individual arrays are bounded.
All three explicitly scoped input views round-trip through BOTH transformations with equality of every input field and the exact canonical serialized bytes. No value is silently removed within a view. The raw captured map/header bytes stay at verified immutable refs rather than being normalized or rewritten.

Exact byte results:
| View | Input canonical bytes | Indexed graph bytes | Indexed + string-interned bytes |
|---|---:|---:|---:|
| All observed semantic fields, including390 image records and verbose diagnostic/member/table evidence |617,563|576,255|367,838|
|49 relevant image records, preserving the same verbose diagnostic evidence |283,890|242,582|188,004|
| Closure-fact-only subset:60 complete known node fact records, explicit unknown, all467 edges,9 static records/all504 selected members, original context and immutable link-map refs |118,370|77,062|58,784|

The third view removes inline raw object-table diagnostic copies; every selected static fact, platform load/rpath/install/UUID, unknown marker, raw-map length/hash reference and original context remains. The full original JSON input fields remain available and round-trip in the broader first view. None of these views is an actual full Basis, and the larger views are deliberately verbose evidence containers rather than claimed minimum encodings.
Edge objects ALONE are49,670 bytes; the standalone identity/ordinal adjacency arithmetic is8,359 bytes. That savings does NOT demonstrate complete Basis/aggregate fit.
The indexed broad views retain diagnostic arrays up to430 items; the closure-fact view retains the289-member array. Chunked string interning brings arrays to64 but depth to12. Both failures are explicit; the current128-edge/array/depth caps were not modified.

Corruption controls:
P/negative-controls.json contains20 actual rejected mutations with hashes. They cover duplicate node identity; unknown endpoint; missing edge; truncated row/serialized bytes; valid-but-changed mode; unknown mode; duplicate ordinal; dropped unknown marker; changed order; changed UUID/rpath/install path/link-map hash; missing node facts; duplicate/missing string table entry; out-of-range string reference; changed dictionary order and truncated interned pair. Each must fail strict arithmetic framing or exact expected all-field canonical-byte equality. This is offline integrity evidence, NOT canonical Q2 decoder conformance or before-helper-birth process evidence. A separate deliberately duplicated-edge sample preserves both equal tuples and their order; the algorithm does not silently set-deduplicate multiplicity. All6 complete view/transform round-trips passed. No negative was repaired by dropping data.

Independent current typed-JSON lower bound:
Using actual60 known nodes, ALL467 edges and raw map refs, shortest one-character nonroot mathematical ID aliases, and EMPTY link-input arrays that intentionally undercount required facts, the current typed Closure alone occupies54,813 bytes. Nodes37,169; edges16,863. The missing platform node is not fabricated. This deliberately incomplete lower bound already exceeds the entire49,152-byte Basis allowance by5,661, before Build/Auth/Patch/compiler/SDK/config/commands/fixture/review projections. It is an in-memory arithmetic lower bound, not an emitted valid Closure or fictional Basis.
The independent read-only reviewer reproduced the same values and map/member reconciliation with standard-library Python commands, actual exits0; the retained main study independently recomputes and asserts54,813.

Budgets and reserved overlap:
- Actual retained source lock7,175 bytes. Bounds remain Basis49,152, aggregate131,072, descriptor16,384, raw result65,536, separate receipt32,768.
- In a FULL raw-result-ceiling reservation scenario, the incomplete54,813 Closure lower bound +7,175 lock +65,536 result leaves only3,548 bytes for every other Basis field, descriptor and qualification envelope. Adding a full16,384 descriptor reservation makes the remainder -12,836. These ceilings are NOT additional grants or observed actual result/descriptor lengths.
- The58,784-byte string-interned closure-fact view alone exceeds Basis by9,632. With lock and full result reservation it totals131,495, exceeding the aggregate by423 before the remaining Basis/descriptor/envelope. This is a conservative experiment, not a theorem that every future lossless representation is impossible.
- Complete final closure, actual complete Basis, actual descriptor, raw private result and qualification-envelope byte lengths are unavailable; all actual full-fit values are null/UNKNOWN. Missing source/probe/publication facts are not filled with examples.
- Exact serialized-byte-buffer overlap lower bounds are in artifact-inventory.json. For the closure-fact view, byte-equality comparison holds at least236,740 bytes (two118,370 byte buffers); input+indexed serialization at least195,432; input+interned serialization at least177,154. Broad-view comparison at least1,235,126. Python object/allocator peak heap was NOT measured and is unknown, not declared to meet any runtime heap law.
-19 generated physical study files total2,610,728 bytes, including all retained variants, scripts/logs/control results/inventory. Those are OFFLINE review outputs, not prepared runtime metadata. All physical copies are explicitly counted; counting only a small edge projection or one shared logical table would be false. Existing84 distinct raw inputs are reused by reference and not rewritten. No product working-storage credit or native/runtime grant was exercised.

Commands and actual exits, unattended:
| Command | Actual exit |
|---|---:|
| python3 -B -I P/study.py > P/study.log 2>&1 |0|
| python3 -B -I P/fact_view.py > P/fact-view.log 2>&1 |0|
| Standard-library input/readback/hash/inventory arithmetic shown in the retained evidence |0|
No full make verify/build/parity/native/timing run occurred. Existing exact task verify remains prior actual exit2 at recursive edge129 refusal. The code/source/formal state is unchanged; no new signed commit was needed or created.

Exact output hashes:
| P-relative artifact | Bytes / SHA256 |
|---|---|
| study.py |24,099 /444f97f514e60ec11279d0216778aff986935bfd3cd64f9b604916efaa8a368b|
| study.log |3,826 /91800d81ac6e5cfff2586b8178d85b2534ed509a66ba64e73233029e860269d6|
| summary.json |30,557 /d60186118036bb701d023ef13b488514b4290d6c0eb95aa048e7b84713f2ff4f|
| all-observed.input.json |617,563 /43321267d2208dc7e325a9807c2aaac146cdebf372710f7637c7c82247f80207|
| all-observed.indexed.json |576,255 /86b812abbe1900285991c2ca783a40211635584b3690ab1e7b46470bacfaceb0|
| all-observed.interned.json |367,838 /35e5a43ea174b4aca3a2c31f5c6b75accdc54e86e0809db9d826861ffce2091d|
| relevant-known-closure.input.json |283,890 /e90366efd3b5b01f8860012150f72fc417d9372dbb64f21931fdd86c361a2121|
| relevant-known-closure.indexed.json |242,582 /885c7c186b877ed3cf79a4578d94c795934085b477208d85c10612c9d3b13021|
| relevant-known-closure.interned.json |188,004 /44db3eb8c75c8adfbf774f731460d049bf5ba34e685c945f91f9ee6b8038d91a|
| edge-only-arithmetic.json |8,359 /3ecb1382b618200663707a78477260e4c2b306f1d525e314f3298cbdd85fb2e6|
| negative-controls.json |2,711 /ae9dec75ac1724760244736817524ca6841eb0ccfad9340f99a1ad121ca87cb2|
| fact_view.py |4,195 /7305f78cdeda0bdbe4f9aa5de2f562afaffa6bc5a7413e7958c8d052f81ca0ec|
| closure-facts.input.json |118,370 /ce3ffecd0300818bc1421aeebbf26320a22fdb19e368103aad5eee980769b7df|
| closure-facts.indexed.json |77,062 /e1dba403e1a317dd9954d76ed0ee8817c8da7c94a0097da5d74d74794c9c7b96|
| closure-facts.interned.json |58,784 /eb856b5e9212a13a0d5650c33aa18ade0c1eb3311040dc6add32ee76289b78e8|
| closure-facts.summary.json |1,267 /c0c88874ff17be4bbac26cfb47e046c3249eb48a6d686c566b86e3e136367709|
| artifact-inventory.json |3,758 /bcc521453ef2d53a472729bb65adbb7a2f73db19611d2aed4de2e60cee9cb2ad|

artifact-inventory.json inventories all18 other files and deliberately excludes its own future/self hash. It also records exact input physical lengths and serialized-buffer overlaps. The report/inventory are not authority stores.

Unchanged failure/limits:
Earlier R/terminal.json still contains unsupported, effect_unknown=false, original dyld_info UUID ambiguity; SHA84bd43367632859d08ff11f8d9c98d3585040586cc8899dab22e3da81955db91. Its later explicit offline metadata-bound refusal and original identity are preserved. Final78391 build's separate recursive edge129 failure remains unchanged. No prefix was cleaned, rebuilt, relabelled, sealed or approved.

Only the Architect's SINGLE frozen lossless bounded representation and Root's coherent changed canonical Q2 source/executable handoff can clear the remaining actual interface dependency. This evidence supplies no new normative codec, native admission, complete closure/currentness proof, runtime qualification, available lesson or task-completion claim. All 1ms ordinary/control/cleanup, native2ms/25ms/3ms inclusive,9x debt,256 attempts,member/thread,30s/four-job,heap/foreground/bootstrap/actual-reap and all original byte laws remain.
