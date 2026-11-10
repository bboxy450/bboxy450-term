---
type: decision
title: Keep rich-pane draft reservation on its exclusive UI owner
derived_from:
- '[[tasks/2026-11-05-dream-c1-consumer]]'
id: 01M6Q9TWWPFFEG5HT6Y1YFBC55
updated: 2026-11-06T02:08:37.496Z
created: 2026-11-06T02:08:37.496Z
number: 93377961365940
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:de0daba1c3acec55490c1e906cdb08b3a179334b31e5802c75cc125af07f5cdd
  frontmatter_digest: sha256:dbb45dfdec1c99202a34edb5d14d8bd961c95d918fba9c115c7c7235d209740b
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: oNMm3KCeQoEvXHM/m2CA/cuAKEmMP/ZDmAEzuNbadj/j2CtUCS7/Gg92y5GOIWZBMNPbjNCPmgyZWWuauOlqAQ==
  log: .bboxy/log/2026-11-06.jsonl#148689
---
# Replace the shared draft mutex with its actual exclusive owner

Date2026-11-06UTC. Root source read-set: consumer rich_pane.rs:125–171 defines Draft as Arc<Mutex<Option<oneshot::Receiver<ViewDraft>>>> and synchronously replaces its receiver before spawning the next worker. rich_faces.rs:136–159 supplies a clone from App.rich_drafts; main.rs:2257 stores that per-seat map. Targeted source inventory finds no other production draft-slot access. Current strict lint rejects this UI Mutex::lock; the failure is inherited from daa301bd7, not a reason to waive it. Actual /tmp/dream-c1-formal-contract-lint-corrected.log exits101 and also reports Option<SwarmScope>.clone at ctl/plugin.rs:137.

## Choice and mechanism

The UI already owns the reservation order. Use an exclusive UI-owned draft slot (Option<oneshot::Receiver<ViewDraft>> or equivalently small local owner), borrowed mutably by connect_retained. Reserve/replace the next receiver synchronously BEFORE scheduling either worker; move the predecessor receiver and new sender to the worker. Preserve asynchronous predecessor await, accepted local input drain, full ViewDraft restoration, panel isolation and terminal draft handoff. Update the sole rich_faces caller to pass its map entry mutably rather than cloning shared mutex custody. Keep ordinary connect convenience behavior and generation/remote-input fences.

Reject a blind try_lock fallback/panic/drop, spinning/retry/contended lock, worker-scheduled reservation reordering, unsafe atomic-pointer ownership or a new queue/broker/pool. No new dependency, UI redesign, layout/renderer budget, source/session operation or public Dream wire/profile change. Receiver/draft ownership is moved, not copied or silently discarded. This is the same abstract atomic reservation order with an explicit Rust owner.

## Verification

Preserve and extend rich_pane's real draft_handoff_drains_last_edit_before_reopening_and_isolates_panels witness: queued last edits survive immediate drop/reopen, rapid repeated same-seat reopen has correct predecessor order, distinct panels never mix, and existing identity/remote admission tests still hold. No sleeps for composer readiness/operator action. Formal coverage must explicitly represent two panels, bounded accepted edits/reservations and old/new workers whose execution can reorder, with weak fairness only for internal drain/complete. Safety: no lost accepted edit/cross-panel adoption/duplicate predecessor ownership; next restoration waits for predecessor drain. Broken off-owner reservation/early restore/drop controls must fail; implementation witness must expose the removed guard. Existing RichHost/RichCancelFence models do not explicitly cover this draft chain; add a small focused case in the existing rich_host.rs formal module and source/contract/corpus mappings. Full current-source native/Java state+edge+verdict parity remains required, with honest abstractions and no arbitrary Rust refinement claim.

ctl/plugin.rs correction removes the redundant clone on the Copy scope only; workspace/plugin/original-op/approval behavior stays. Reuse an already-landed signed exact fix if available rather than creating a competing behavioral change, otherwise this is the narrow authorized correction. Existing strict lint, affected lib/draft/formal/boundary checks and source-bound receipts decide. Root acceptance of this decision is implementation scope, not a passed test or runtime readiness claim. Consumer Dream12ba launch stop and all250ms/CPU/duty/heap/authority/reap gates stay.
