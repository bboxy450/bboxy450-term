---
acceptance:
- Read ID, original target mutation ID, replay/current-authority and unavailable-binding/store behavior are unambiguous; original mutation idempotency is unchanged.
- A minimal signed proposal includes Q/T, Q/T2, Q2/T, restart/unavailable examples and explicit owners/review state for any exception.
- Bounds and provider/bridge assertions cover chosen behavior without a new shared cache/queue/service or mandatory read-audit write dependency.
- Existing identities/links and actual focused/integrity results are preserved; no code or unproven API acceptance is claimed.
allowed_paths:
- .bboxy/specs/mcp-control-plane.md
- .bboxy/specs/w2-5-mcp-and-seat-wiring.md
- .bboxy/decisions
- .bboxy/runs/read-query-id-contract.json
artifacts:
- '[[01M64VF20STF4294067VCDHTF8]]'
- '[[01M64VF20STF4294067VCDHTF8]]'
assignee: seat:01M649QP6AMVQWCG28X5MQP3JR
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T22:07:33.436Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64V8FHJF3AQNAVJT3QVHH6T
number: 481
provenance:
  digest: sha256:453a21370abed2de02b6285280001acd85648de11e8ee7fcb33b4040fdb1d58e
  frontmatter_digest: sha256:1035712a26a4ed8687977aec3321c0170a524a2683db5c3ed4442b6d75f07e19
  log: .bboxy/log/2026-10-29.jsonl#2652
  sig: 7Z117Sekggq5APlXI7WHiigJwu49tI26as2DYVDmndratVlTucNqJLAFXKbagPzI9oSShy15zalFxdk1fFC6DA==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
report:
  commit: 023ce1c3d4554c6ab8d4d751bc6da8c9cce5ced0
  op_id: 01M64WRPDEJMB1VWA16TSVHTJW
  reported_by: seat:01M649QP6AMVQWCG28X5MQP3JR
  summary: 'Final owner-disposition source signed023ce1c3d4554c6ab8d4d751bc6da8c9cce5ced0, clean assigned worktree, exact four allowed paths. Recorded provider Rook agreement01M64W6HY3W3265XZRVG5XWQQJ (as cited by Oracle) and consumer ruling01M64WAQ77GFS12YBAZTCWE6M0 baseline of reviewed395f for CURRENT operation_get only. #87:294/372 and #135:200 explicitly scope266 exception: ownQ fresh per-call/currently authorized observation,Q!=T beforelookup,no Q binding/result/control-opwrites,no targetredispatch,unchanged everyothermutation args/tool/full-actor/root equality,zero newQstorage. Table says ''A current authorization denial returns forbidden''. errors_read remainsfuture/unimplemented;shared ownQ rule awaits its own scoped provider adoption/conformance;thiscut completesneither75contract/conformance norimplementation. #87:436 exact CURRENT operation_get catalogue/orient/bootstrap strings preserve every other available tool guidance;Rook existing487 adoption follows this final-source report,not a new grant. #87:460 minimum executable controls remain required/unproven. Decision483 updated through native writer WDT14 then wording update01M64WJY2E53B4WNMJYXY9X11B,lossyfalse,sameID01M64VF1N4PQ5FJSKD4WFDMHF1/number483/statusdecided,current-get-only title/scope;privatefile exactnativeSHA256f06f85c2fef6330b5b7e43d7c39649ac045c7c03c000d4874b5514ab811b7d7e. Artifact01M64VF20STF4294067VCDHTF8 updated reportedgrade. Exact signed023ce1c3 make test-licenses exit0 (6fixture+9notice tests,27crates,noticechanges0), target/read-query-id-baseline-licenses.log SHA256987a7e53131b615d5def55e344b812a3c75ad8fcf90c7a515a344986c1035052. Original87/135 frontmatter exceptupdated and unaffected oldbody/targetrules preserved (only explicit Ids/default-idempotency policy lines superseded), native483 stableidentity/byte equality,addedlinks,currentonlywording,allrecordhash/four-path checks0; target/read-query-id-baseline-integrity.log SHA2563a85d2ff8b76befd0c1a044f6eb38dc690dfeafe299946e59c0ec5ba7a0a2cb1;approved-key signature/diff0. No cold broad run under existing design/capacity exception;no current fullpass. This is a design baseline,not implementation/test pass;independent465/487 gates continue. No code/tests/Make/skills/runtime/credentials,old-operation reissue,peer cleanup,lifecyclefile edits,recruitment,merge/push orclosure;decision266 and otherdecisions untouched.'
role: architect
status: accepted
title: Resolve own-query ID retry semantics for read-only recovery/error tools
type: task
updated: 2026-10-29T22:40:21.638Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
verify: make test-licenses
verify_result:
  command: make test-licenses
  evidence:
  - criterion: Read ID, original target mutation ID, replay/current-authority and unavailable-binding/store behavior are unambiguous; original mutation idempotency is unchanged.
    reference: .bboxy/specs/mcp-control-plane.md:294/372 — current operation_get-only exception to266, Q/T,Q/T2,Q2/T,Q==T prelookup refusal and explicit 'A current authorization denial returns forbidden'; per-call authority/root/restart/unavailable grades; T and all other mutation equality/replay unchanged.
  - criterion: A minimal signed proposal includes Q/T, Q/T2, Q2/T, restart/unavailable examples and explicit owners/review state for any exception.
    reference: '.bboxy/decisions/d483-read-query-ids-are-fresh-observations.md:39 and #87/#135:200 — native483 ID/number preserved,statusdecided scoped tocurrentoperation_get; provider01M64W6HY3W3265XZRVG5XWQQJ plus consumer01M64WAQ77GFS12YBAZTCWE6M0 on reviewed395f. errors_read future/unimplemented and separate scoped provider adoption/conformance,not a completed75 contract/capability.'
  - criterion: Bounds and provider/bridge assertions cover chosen behavior without a new shared cache/queue/service or mandatory read-audit write dependency.
    reference: .bboxy/specs/mcp-control-plane.md:436/460 and w2-5-mcp-and-seat-wiring.md:200 — exact CURRENT operation_get catalogue/orient/bootstrap wording;other available tools unchanged;existing487 adoption after final source report;zero Q storage and unchanged IO/audit/communication bounds plus required implementation assertions.
  - criterion: Existing identities/links and actual focused/integrity results are preserved; no code or unproven API acceptance is claimed.
    reference: .bboxy/runs/read-query-id-contract.json — owner/native-update/source/command/hash scope record; exact023ce1c3 make test-licenses0,original IDs/frontmatter/unaffected-body/native483/newlinks/four-path hashes0,signature/diff0,clean. Current raw final gate hashes in report;no code/native retry acceptance or broad run.
  exit_code: 0
---
# Finish the reader's own ID agreement
ROLE: ARCHITECT — Nyx Copperwing. Same private writing worktree and branch. This follows your precise217 answer to Oracle question216. Existing266/87/135 default op_id first-result rule has no correlation-only exception;465 currently observes again on a reused read ID. Resolve it explicitly before calling retry semantics verified. Rook's independent lookup/projection/authority/causal gates continue.

Produce the smallest usable proposed amendment in existing87/135, with one proposed decision if a scoped exception is needed. Only that new decision may be created under decisions; never edit unrelated decisions or266 itself. Use native knowledge_write decision creation for global identity allocation; preserve returned ID/number in private source. No implementation/code/bootstrap edits. Code/catalogue adoption goes to Rook after a scoped grant.

Preserve target T and every existing mutation's args/actor/replay equality. Distinguish read Q from T. Define Q/T retry, Q/T2 reuse, fresh Q2/T, root/grant change, restart/eviction and unavailable/corrupt inspected ledger. Current authority precedes every returned payload. No cached foreign/stale data. Read audit is best effort; failure never means target rejection/no effect. Preserve one-second/bounded IO and independent communication.

Resolve the real conflict: strict first-result replay for Q needs provable own-query binding/result retention; coupling it to the unavailable ledger being inspected can make recovery unusable. A new durable global query service/cache is outside scope. A narrowly explicit fresh-observation exception for ONLY operation_get/errors_read may be simpler if consequences and exact catalogue/orient/bootstrap wording are honest; default guidance cannot silently promise first result under an exception. Do not rubber-stamp code. Recommend the option that preserves the product, exact bounds/storage/lifecycle consequences and minimum executable assertions. Send a concise signed proposal promptly for Oracle+Rook review before baseline; no broad cache framework.

No product code/tests/Make/skill edits, runtime/credentials, old-operation reissue, peer cleanup, lifecycle files, recruitment, merge/push or closure. make test-licenses plus original frontmatter/link/diff checks is the focused design gate; existing capacity exception means no unchanged cold build. Report signed source, actual exits/log hashes and remaining review/implementation gaps.

## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Signed proposed source395f0c50, clean assigned worktree; only87/135,one native-created decision483 and allowedJSON changed. Native knowledge_write allocated decision483/id01M64VF1N4PQ5FJSKD4WFDMHF1 at op sameID,lossyfalse; scoped native update01M64VV62FS696BRY7QTN7J0V2 incorporated Oracle selection224 Q==target rejection. Private decision is exact native bytes SHA256a9495c67b3de7d48bc506c37ea11f939e51b1fac4a33014ca581484d21c06c69,ID/number/proposed status preserved. Recommend explicit fresh-observation Q exception ONLY operation_get/errors_read: zero retained Q binding/results,each call fresh/currently authorized,Q/T maychange,Q/T2(T2!=Q)fresh,no op_id_reused solelyforQ, Q2/T independent,Q==T invalid_arguments beforelookup. No inspected-ledger writable dependency or new cache/service; target T and every other tool's args/tool/full-actor/root replay remain untouched. Root/grant/inflight/restart/eviction/unavailable/audit/cursor/concurrent-correlation cases explicit. #87:431 exact catalogue/orient/bootstrap text and :453 future executable assertions; #135:200 actual adoption seams, no code/skill changes. Initial signedafc proposal sent promptly; revised395f sent both owners. Oracle selection01M64VHD256NNCPYKB5CTAN2BJ chooses this shape but explicitly keeps it proposed; Rook review01M64W0AD07ANC53BQPGEQH7T5 supports operation_get direction on afc, not adoption/acceptance, with errors_read outside465. Corrected395f guard/final-source review and future scoped consistent adoption remain pending; do not claim retry conformance or baseline. Exact signed395f make test-licenses exit0 (6 fixture+9 notice tests,27 crates,notice changes0), target/read-query-id-licenses-final.log SHA256653bab800749871c8fa7ed4c1a3fa052cd5f31f5c053d66c266bcfe9f8c78772. Original identities/frontmatter exceptupdated/body/default mutation replay, added links, native483 equality and recordhash/four-path checks0, target/read-query-id-integrity-final.log SHA25601253ff46526d8917b9a66735ba4b4dbf4e461939d3a3f9d9410fc5ae7c7e3a7; approved-key signature/diff0. Initial integrity checker1 on an old escaped-link target corrected to added-link validation while preserving original links, actual result retained. Artifact01M64VF20STF4294067VCDHTF8 reported grade. No cold broad run under existing design/capacity exception; no current full pass. Independent465 lookup/projection/authority/causal gates continue. No code/tests/Make/skills/runtime/credentials,old-operation reissue,peer cleanup,lifecycle edits,recruitment,merge/push or closure; decision266 and other decisions untouched.

```json
{
  "commit": "395f0c5034f3727c179ac6e4780a435dc290fb8d",
  "op_id": "01M64W3CYPRNYPWN4P5XGDJ6E7",
  "verify_result": {
    "command": "make test-licenses",
    "evidence": [
      {
        "criterion": "Read ID, original target mutation ID, replay/current-authority and unavailable-binding/store behavior are unambiguous; original mutation idempotency is unchanged.",
        "reference": ".bboxy/specs/mcp-control-plane.md:371 — explicit proposed Q-only fresh rule, Q/T,Q/T2,Q2/T, Q==target prelookup refusal, current/in-flight authority, root/restart/inspected-ledger grades; original default294 and mutation T text preserved. Decision483:38 — no Q ledger/cache or target mutation rewrite."
      },
      {
        "criterion": "A minimal signed proposal includes Q/T, Q/T2, Q2/T, restart/unavailable examples and explicit owners/review state for any exception.",
        "reference": ".bboxy/decisions/d483-read-query-ids-are-fresh-observations.md:38/48 and .bboxy/specs/w2-5-mcp-and-seat-wiring.md:200 — provider Rook/consumer Vesper, proposed-not-baselined state, rejected strict/durable/ephemeral alternatives and all requested examples. Native creation483 ID01M64VF1N4PQ5FJSKD4WFDMHF1 preserved byte-exact after scoped native update."
      },
      {
        "criterion": "Bounds and provider/bridge assertions cover chosen behavior without a new shared cache/queue/service or mandatory read-audit write dependency.",
        "reference": ".bboxy/specs/mcp-control-plane.md:431/453 — exact two-tool catalogue/orient/bootstrap words, per-call unchanged bounds/zero Q storage/best-effort audit, actual provider/bridge/concurrency/authority/cursor/mutation causal assertions in existing Make targets; #135 names actual future adoption seams, no code or shared service."
      },
      {
        "criterion": "Existing identities/links and actual focused/integrity results are preserved; no code or unproven API acceptance is claimed.",
        "reference": ".bboxy/runs/read-query-id-contract.json — native allocation/update/source/command/exit/hash/review limits; exact395f0c50 license0, original frontmatter/body/new-link/native-ID/hash/four-path checks0, signature/diff0,clean tree. Raw final log hashes and partial provider direction review in report; no broad run or implemented API claim."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Final owner-disposition source signed023ce1c3d4554c6ab8d4d751bc6da8c9cce5ced0, clean assigned worktree, exact four allowed paths. Recorded provider Rook agreement01M64W6HY3W3265XZRVG5XWQQJ (as cited by Oracle) and consumer ruling01M64WAQ77GFS12YBAZTCWE6M0 baseline of reviewed395f for CURRENT operation_get only. #87:294/372 and #135:200 explicitly scope266 exception: ownQ fresh per-call/currently authorized observation,Q!=T beforelookup,no Q binding/result/control-opwrites,no targetredispatch,unchanged everyothermutation args/tool/full-actor/root equality,zero newQstorage. Table says 'A current authorization denial returns forbidden'. errors_read remainsfuture/unimplemented;shared ownQ rule awaits its own scoped provider adoption/conformance;thiscut completesneither75contract/conformance norimplementation. #87:436 exact CURRENT operation_get catalogue/orient/bootstrap strings preserve every other available tool guidance;Rook existing487 adoption follows this final-source report,not a new grant. #87:460 minimum executable controls remain required/unproven. Decision483 updated through native writer WDT14 then wording update01M64WJY2E53B4WNMJYXY9X11B,lossyfalse,sameID01M64VF1N4PQ5FJSKD4WFDMHF1/number483/statusdecided,current-get-only title/scope;privatefile exactnativeSHA256f06f85c2fef6330b5b7e43d7c39649ac045c7c03c000d4874b5514ab811b7d7e. Artifact01M64VF20STF4294067VCDHTF8 updated reportedgrade. Exact signed023ce1c3 make test-licenses exit0 (6fixture+9notice tests,27crates,noticechanges0), target/read-query-id-baseline-licenses.log SHA256987a7e53131b615d5def55e344b812a3c75ad8fcf90c7a515a344986c1035052. Original87/135 frontmatter exceptupdated and unaffected oldbody/targetrules preserved (only explicit Ids/default-idempotency policy lines superseded), native483 stableidentity/byte equality,addedlinks,currentonlywording,allrecordhash/four-path checks0; target/read-query-id-baseline-integrity.log SHA2563a85d2ff8b76befd0c1a044f6eb38dc690dfeafe299946e59c0ec5ba7a0a2cb1;approved-key signature/diff0. No cold broad run under existing design/capacity exception;no current fullpass. This is a design baseline,not implementation/test pass;independent465/487 gates continue. No code/tests/Make/skills/runtime/credentials,old-operation reissue,peer cleanup,lifecyclefile edits,recruitment,merge/push orclosure;decision266 and otherdecisions untouched.

```json
{
  "commit": "023ce1c3d4554c6ab8d4d751bc6da8c9cce5ced0",
  "op_id": "01M64WRPDEJMB1VWA16TSVHTJW",
  "verify_result": {
    "command": "make test-licenses",
    "evidence": [
      {
        "criterion": "Read ID, original target mutation ID, replay/current-authority and unavailable-binding/store behavior are unambiguous; original mutation idempotency is unchanged.",
        "reference": ".bboxy/specs/mcp-control-plane.md:294/372 — current operation_get-only exception to266, Q/T,Q/T2,Q2/T,Q==T prelookup refusal and explicit 'A current authorization denial returns forbidden'; per-call authority/root/restart/unavailable grades; T and all other mutation equality/replay unchanged."
      },
      {
        "criterion": "A minimal signed proposal includes Q/T, Q/T2, Q2/T, restart/unavailable examples and explicit owners/review state for any exception.",
        "reference": ".bboxy/decisions/d483-read-query-ids-are-fresh-observations.md:39 and #87/#135:200 — native483 ID/number preserved,statusdecided scoped tocurrentoperation_get; provider01M64W6HY3W3265XZRVG5XWQQJ plus consumer01M64WAQ77GFS12YBAZTCWE6M0 on reviewed395f. errors_read future/unimplemented and separate scoped provider adoption/conformance,not a completed75 contract/capability."
      },
      {
        "criterion": "Bounds and provider/bridge assertions cover chosen behavior without a new shared cache/queue/service or mandatory read-audit write dependency.",
        "reference": ".bboxy/specs/mcp-control-plane.md:436/460 and w2-5-mcp-and-seat-wiring.md:200 — exact CURRENT operation_get catalogue/orient/bootstrap wording;other available tools unchanged;existing487 adoption after final source report;zero Q storage and unchanged IO/audit/communication bounds plus required implementation assertions."
      },
      {
        "criterion": "Existing identities/links and actual focused/integrity results are preserved; no code or unproven API acceptance is claimed.",
        "reference": ".bboxy/runs/read-query-id-contract.json — owner/native-update/source/command/hash scope record; exact023ce1c3 make test-licenses0,original IDs/frontmatter/unaffected-body/native483/newlinks/four-path hashes0,signature/diff0,clean. Current raw final gate hashes in report;no code/native retry acceptance or broad run."
      }
    ],
    "exit_code": 0
  }
}
```
