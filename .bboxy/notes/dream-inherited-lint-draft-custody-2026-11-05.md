---
type: note
title: 'Inherited lint scope: Copy cleanup and UI-owned draft handoff'
derived_from:
- '[[decisions/d93377961365940-rich-draft-ui-owner]]'
- '[[tasks/2026-11-05-dream-c1-consumer]]'
id: 01M6QA5557YB7YP640X17Y17QA
updated: 2026-11-06T02:14:13.362Z
created: 2026-11-06T02:14:13.362Z
number: 93377961365942
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:17cfd1bbbe4009eff6682f1f20969e7d8e3464677822d4ba14b085779937dbd6
  frontmatter_digest: sha256:f12ca41dbab37446cf93de80fdbbe3439efd189b0bd94dee6eb8e7c2a8a18674
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: sDLfOAhcGbu53ptL4EzXhCLQ0+t/FlwKiCXEVmVsYnbaUAgvVcgX39vF3z5oNS/xTuJnNYy9eqp3+QIJqptVBw==
  log: .bboxy/log/2026-11-06.jsonl#148721
---
# Narrow inherited lint correction authorized

Date2026-11-06UTC. Consumer message01M6Q9J3SF758B6X4J010PSGJ7. Root read actual /tmp/dream-c1-formal-contract-lint-corrected.log exit101: redundant clone of Copy Option<SwarmScope> at ctl/plugin.rs:137 and disallowed UI Mutex::lock at rich_pane.rs:171. No lint waiver accepted. Reported37 native contracts remain separate subresults, not Root-certified full formal/runtime success.

Root source inventory: Draft=Arc<Mutex<Option<oneshot::Receiver<ViewDraft>>>> at rich_pane.rs:125–127; synchronous replacement before worker spawn at169–171; worker awaits predecessor then restores full draft. Sole production caller rich_faces.rs:136–159 clones the UI App.rich_drafts per-seat slot; main.rs map stores Draft. No other production slot access found. Blame daa301bd7 introduced that custody. Existing queued-last-edit/drop/reopen/two-panel test is rich_pane.rs:527+, while current RichHost/RichCancelFence models cover other session/remote fences rather than this receiver chain.

Decision [[decisions/d93377961365940-rich-draft-ui-owner]] digest2ce9a503faf2e852ef88e675e851fcb244a4c5ffd05b99c1145688f9ec36fff1 chooses the actual exclusive owner: a UI-owned Option receiver slot borrowed mutably by connect_retained, synchronous reserve before scheduling, move predecessor receiver/new sender into worker, keep async await/drain/full restoration/panel/generation fences. Reject blind try_lock loss/panic, spin/block, asynchronous reservation reorder, unsafe pointer or new broker/pool. Copy-scope correction changes no plugin/workspace/op/approval behavior. Reuse a signed landed exact fix if available; otherwise narrowly implement. Focused draft-chain model/broken controls and real library witness required with existing current-source native/Java full parity.

Live [[tasks/2026-11-05-dream-c1-consumer]] amendment op01M6Q9YPFC9MHXPZ5R5QFZQW0X applied, digestbce4a7bf46b26b8588e474807277133b5885bb2169ad9a643553764437d7dd25. Added exactly ctl/plugin.rs, rich_pane.rs, sole caller rich_faces.rs and existing tests/formal_contracts/rich_host.rs under crates/bboxy450-term; other mappings/model registry/main were already allowed. Compact brief requires reading the complete decision, source correction, focused model/causal tests and unchanged required checks. Full task/decision readbacks exact; every other non-host frontmatter/verify/dependency/lifecycle preserved. No unrelated UI/plugin redesign/dependency/wire/version/budget change or Dream worker launch. Original12ba250ms failure/one-shot stop unchanged.

First larger knowledge_write was refused at parameter admission with Invalid or oversized arguments and NO op ID (67926argument characters); fresh fetch confirmed original task digest3099c0... unchanged. No effect was claimed/retried blindly. Corrected compact arguments65006UTF8bytes reference the complete separately written decision; new write applied and was fully verified. The live brief is near transport capacity: future amendments should consolidate superseded narrative into current authoritative constraints/references, not keep appending obsolete text or omit effective requirements. No shell KB bypass.

Root retained exact signed d9817f32 source for the four paths plus actual failed log in /Users/feral/mydev/bboxy450-term/target/dream-inherited-draft-lint-2_izu8na/preservation.json. All read/archive commands exited0. Failed log1717B SHA0402ba15298348b188f7e5b7180555077ee8817999be38954b391bc788419a61 matches prior Root tail; rich_pane baseline26131B SHA86c10091ba9fcb63e2fd31ef62bad6a7feb26e969d0c3ae84cb7e914245a8549; remaining source hashes in manifest. Incoming explicitly acknowledged01M6QA22YYT878D9F3H7QTGXNW. Native signed amendment is authority, not a model-receipt or implemented/tested claim. Consumer owns source/lint/draft/formal/boundary verification; Root ran no product test/build/worker or source landing in this slice.
