---
acceptance:
- GitKB commit persists only explicitly named slugs through native backend
- Two swarm roots with colliding slugs remain isolated
- Private fixture and Harmony read-only evidence provided
allowed_paths:
- plugins
- scripts
- Makefile
assignee: seat:01M62ZC87EV8MWGA4FS7Y7VFYA
authorship:
  kind: seat
  process_generation: '1'
  public_key: PtawKUprAOBmHUgC+Qpt9hciMQWgq9SCP/8ID3uAPx4=
  signer: seat:01M62ZBZ8MX6Y5HPE662JVDABC
created: 2026-10-29T04:58:37.229Z
created_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
delegated_by: seat:01M62ZBZ8MX6Y5HPE662JVDABC
id: 01M630CE8WXXZK8W17871TM16V
number: 372
role: executor
status: done
title: 'Harmony provider: scoped KB commits, slug identity and swarm reality'
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: make test
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/50a849eb", "https://github.com/bboxy450/bboxy450-term/commit/4bd4654b"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "scoped GitKB provider commits and signed_record landed. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR

Addendum to delegated tasks/harmony-gitkb-adapter.md, same assigned writer/worktree /Users/feral/mydev/bboxy450-term-harmony-adapter and allowed paths plugins/, scripts/, Makefile, adapter documentation. All original boundaries apply: no recruiting/merge/push, no writes/test records in real Harmony KB, install executable only Harmony/.bboxy/plugins. Origin/main 618df56f is corrected base.

Operator explicitly expands outcome: add explicit knowledge-base commit support. Adapter optional data_source_commit takes bounded explicit locators/slugs and supplied message; invokes native git-kb commit only for named documents; no auto commit or push. Coordinate schema with Vesper. Add meaningful private real-backend test verifying only selected document is committed and unrelated pending document remains pending. Add canonical Make test/build/install commands; no parallel recipe.

Operator also requires each swarm own complete view: cwd/root/plugins/provider/documents/tasks/messages isolated, same full slug can exist in two roots. Natural GitKB slugs are first-class document/task identity and delegation addresses; native IDs preserve rename continuity. Adapter fixture uses two roots with colliding slugs and proves no cross writes/commit. Host worker owns UI context fences; coordinate boundary.

Signed writer mapping is decided: top-level signed_record optional opaque host-signed portable JSON intent, adapter persists byte-exact native bboxy_signed_record and echoes; actual native UUID/status retained, outward portable ID reversible from UUID, native external_id preserved; bboxy_status host lifecycle, no next_number/global ledger. Strip prior proof from outward FM to prevent recursion. Optional private fixture bootstrap only from disposable checker, never production root. Confirm source-aware paging on actual Harmony meets 2 second contract. Final combined make test/boundaries/license gate run by Oracle in review worktree; report focused checkpoint results as provisional until that gate, with actual exits/pointers. Request PR now authorized; Oracle opens, no merge.