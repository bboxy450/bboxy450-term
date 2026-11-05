---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:56:42.327Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GB0THEWJ6MDHMCHD71E7S
links:
- rel: context
  to: '[[vision]]'
number: 437
provenance:
  digest: sha256:56eb1349662c657ee8bf1d0a6dcf8f56c2620f90d4fe274d6705984cc53490c2
  frontmatter_digest: sha256:6cbd64159a28294f4512dc509efd0af9c459369a055ee3ae986b2d656f88a0c8
  log: .bboxy/log/2026-10-29.jsonl#2719
  sig: UAZnsNTh6JHayoZB1xZHNzQdnSWDzUlQzrsmOzxQZxHV1KuH4dyh48mcF/I85tFgBgrVrEAxFNLs+ES5Dee+CQ==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
title: 'Journey: usable terminals and honest recovery evidence'
type: note
updated: 2026-10-29T22:47:08.729Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
<!-- bboxy450-term: https://github.com/bboxy450/bboxy450-term
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Usable terminals and honest recovery evidence

Back to [[implementation-journey]]. Recent input, replay, font, selection and lifecycle improvements are useful landed work. They do not establish the following still-open acceptance lines on main `014e555c`.

| Issue / next task | Remaining requirement | Evidence / next verification |
|---|---|---|
| [#11 · S1](https://github.com/bboxy450/bboxy450-term/issues/11) | Byte-identical historical cell/style data and bounded lazy paging | Read S1's current partial header and [[supervisor-logic]]; prove the retained cells and real paging boundary, not only visible text. |
| [#12 · S2](https://github.com/bboxy450/bboxy450-term/issues/12) | Real ring-gap production, historical cells and rendered gap placement | Preserve actual ring/cursor endpoints and assert the produced frame. Existing partial S2 is not whole acceptance. |
| [#13 · S3](https://github.com/bboxy450/bboxy450-term/issues/13) | Fifty-watched history memory and hitch budgets | Keep the full count and existing limits; account for actual history memory and hitches. A reduced proxy is diagnostic only. |
| [#14 · S4](https://github.com/bboxy450/bboxy450-term/issues/14) | Valid over-budget history viewport plus heap accounting | Repair the fixture without lowering its assertion, then run the unchanged night promise. |
| [#15 · K1](https://github.com/bboxy450/bboxy450-term/issues/15) | Native Kitty keys/modifiers, IME and reserved navigation | App-layout shortcut fixes already shipped (#55/#67). Finish the remaining K1 assertions through structured private app controls, never OS input. |
| [#16 · K2](https://github.com/bboxy450/bboxy450-term/issues/16) | Amber pixels, actual operator DM native delivery, delayed peer release exactly once, signed mute audit | make test-seat-mute covers useful controls; canonical K2 still declares these missing assertions. |
| [#17 · K3](https://github.com/bboxy450/bboxy450-term/issues/17) | Cmd-click a Markdown path or wikilink into the document modal | Graph shortcut partial proof exists; D3/D4 and native path-hit testing remain separate. Preserve current K3 missing list. |
| [#38 · fifty-seat ENOENT](https://github.com/bboxy450/bboxy450-term/issues/38) | Capture original failing stage/path and repair its mechanism | make test-fifty-seat-startup now retains bounded post-cap witnesses. The probe saw errno -6 at posix_openpt, **not** the original ENOENT. No causal fix; quarantine expires 2026-11-01. |
| [#39 · restart EOF](https://github.com/bboxy450/bboxy450-term/issues/39) | Causally reproduce the well-formed restart EOF and repair it without losing cursors/unread/identity/PTY assertions | `e2278cf3` fixes swallowed read-error diagnostics and its mutation test. make test-gui-events remains useful, but does not prove that historical cause. Quarantine expires 2026-11-01. |
| [#40 · zsh process lookup](https://github.com/bboxy450/bboxy450-term/issues/40) | Retain the real child/process-lookup witness and deterministic causal regression | make test-adapter-startup proves controlled exit 41 diagnostics, not the old InvalidData cause. Keep separate from MA-7 accounting; quarantine expires 2026-11-02. |
| [#56 · original hook failure](https://github.com/bboxy450/bboxy450-term/issues/56) | Original exit-1 stderr/cause and supported observation limits | Journal/observer cuts `4d0e66e4`/`6b4d574f` shipped. Parent-session isolation proves one bug class; the original failure had no retained stderr. Preserve that uncertainty and unavailable historical/async observations. |
| [#72 · Codex right-edge strip](https://github.com/bboxy450/bboxy450-term/issues/72) | P1: reproduce during normal real-Codex streaming/resize, inject/regress it, then pass with the repair | Native489 [[2026-10-29-issue-72-codex-rendering]] delegates Lumen on a distinct branch from signed d70. Existing make test-codex-rendering proxy/idle/recovery does not yet prove per-present pixels; capture-induced redraws cannot disprove the defect. The new cut must prove normal produced streaming/resize frames plus a causal negative, preserving colors/geometry. |

[Quarantine](../quarantine.md) remains executable policy, with exact names/owners/expiry. Diagnostic improvements do not remove its rows. Evidence: [startup accounting](../runs/night-fifty-seat-startup-accounting.log), [adapter diagnostics](../runs/night-adapter-startup.log), [GUI EOF diagnostics](../runs/night-gui-events-diagnostics.json).

## Unresolved incident: GitHub #64

[#64](https://github.com/bboxy450/bboxy450-term/issues/64) is the 2026-10-26 typing/paste-dialog freeze reported during shutdown. The owning application, dialog, root cause and a bboxy450-term reproduction were never established. Later image/clipboard/selection fixes do not establish the cause of that incident.

Current disposition after the operator selected verified fixes before closure: **reopened in GitHub**, unresolved. The earlier administrative archive established no cause or fix. Establish the owning process/dialog and timestamp, then use a private harmless clipboard/dialog reproduction with bounded redacted diagnostics to demonstrate and repair the app-owned input failure, or verify an external owning cause. Existing clipboard/input component passes do not resolve this report. Never use the operator clipboard, desktop capture or injected OS input.

The next code assignments remain #72 and the causally unresolved boundaries above; no peer worktree or retained runtime was stopped or reset by this triage.



Admission note, 2026-10-29: this dated triage was first prepared in signed private commits de793c11b7dd3bfd17ac453d8c47c98dc91adbae and 0efe0d917de225fa702667dc646b7981ae7edecc. The authenticated host now assigns this local copy a fresh identity and global number; the private draft numbers are not imported as current authority. Follow [[2026-10-29-zero-issues-cohort]] for subsequent assignments and verified dispositions. Unresolved promises remain unresolved.
