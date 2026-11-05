---
type: report
title: Harness producer repair and complete unattended gate
task: '[[tasks/2026-11-04-hashbrown-search]]'
commits:
- afe92dae97b2a5f86209863fd1545d7208171b9b
verify:
  command: make -C crates/bboxy450-harness test TARGET_DIR="$PWD/target"
  exit_code: 0
author: seat:01M6KN12436S0N6CJDQ0RWBQNG
created: 2026-11-04T21:24:26.262Z
id: 01M6M75TFS8XE9G2RFBVEAZXN7
updated: 2026-11-04T21:24:26.262Z
number: 93377961363609
created_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
updated_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
authorship:
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  kind: seat
  public_key: 4FB10dq226S4+Z6CUKQrEvBUsQCbt4dAXA3Nd2bPO6o=
  process_generation: '1'
provenance:
  digest: sha256:ace17c1f63db2de2b0cd16d9a6ce48f884c4b024b1955f0b3e1304f91eadbdcd
  frontmatter_digest: sha256:36a29c00d9e82fd02a73bed15fe3a0c7494d866efe67720105024a68231c4658
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  sig: l8FpRqKtpswMS8jP6BszDhGUol7R808BcbvM/dDGKs1l4FVOIzkPEjMYVYAjSSYBuGKu+BwTZSih/TpFRzg5DA==
  log: .bboxy/log/2026-11-04.jsonl#130792
---
Canonical producer repair for [[tasks/2026-11-04-hashbrown-search]], before updating the parent harness pin.

Producer: https://github.com/bboxy450/bboxy450-harness, signed commit afe92dae97b2a5f86209863fd1545d7208171b9b, based on upstream f1d3a3f. The complete final canonical `make test` exited 0. No release, trusted installation, operator input or live operator UI was used. Parent adoption and its full gate remain pending in this producer report.

Files and mechanisms:

- src/frontends/gui/bridge.rs:214 — background QueryInputCapabilities admission responses do not enter user-intent errors. Current challenge/epoch/actor events still grant custody; twelve unchanged local/Unix/replay fixtures now agree.
- src/frontends/gui/view.rs — restore a bounded plain gray preview while pending Markdown worker data is absent. At most 160 Unicode scalars plus ellipsis are drawn; full source remains available, existing birth/session filters and bubble heights remain. Existing gap/history pixel assertions are unchanged; gui_render adds bounded Unicode/full-source coverage.
- src/protocol/mod.rs, src/backend/acp_agent.rs, schema and lock — additive absence defaults decode safely. SetAgentConfig.value is Option<String>: missing/null cannot select an advertised empty choice, while explicit empty remains valid. Existing identity, option and custody guards remain. Actual private child/API tests cover each missing field and the empty-choice edge. make lock-ir refreshed the unpublished baseline through unchanged additive validation; frozen IR fixtures are unchanged.
- tests/formal/models.rs and verification/{corpus,contracts,coverage}.toml — add discovery-error stuttering and independent BrokenHarnessAttachmentDiscovery, plus missing source/test/canonical-check declarations. Native and pinned Java compare 132 bounded model graphs/verdicts; 44 contracts are inventoried. This does not prove arbitrary Rust or unbounded TLA+ correctness.
- tests/gui_menu_motion.rs and gui_motion.rs — settled fixtures now supply prepared static content/matching SessionMeta. Existing menu motion/geometry timings stay asserted; chip assertions identify each intended item/action rather than also counting a new file picker.
- tests/acp_controls.rs and bridge's remote-profile fixture — isolate only private discovery PATH from installed operator adapters. Tests retain catalogue, selector, absent/unreadable-home and authority assertions.
- src/lib.rs, gui/zoom.rs, gui_render.rs and gui_rich.rs — canonical format/module placement/initializer/chunk-iterator repairs. scripts/test.py keeps strict -D warnings and gathers independent diagnostics with --keep-going.
- tests/app_packaging.py — default BUNDLE probe explicitly sets TARGET_DIR=target, with an additional exact path-with-spaces probe. Signing/install/launcher and retention assertions remain.
- Five legacy unsigned catalogue records — unique number-only migration, preserving IDs, paths, exact bodies, dates, lifecycle/verification fields and every signed counterpart. Old/new numbers and full/body hashes are in the linked JSON. No native historical task lifecycle was changed: d5974-log-ir-v1-only-add 5974→93377961361017; d5972-our-log-format-is-the-center 5972→93377961361018; 2026-11-02-log-ir-v1-compat 5975→93377961361019; d5978-unknown-is-preserved-and-promoted 5978→93377961361020; 2026-11-03-agent-initiated-turns 5981→93377961361021. Unchanged validator checks 2,925 records and its missing-number broken control.

Verification and evidence:

`. ./scripts/cache-env.sh; env CARGO_TARGET_DIR="$PWD/target" CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 RUST_TEST_THREADS=2 make -C crates/bboxy450-harness test TARGET_DIR="$PWD/target"` from the parent worktree — exit 0 on afe92dae97b2a5f86209863fd1545d7208171b9b. This is the final full run, not one of the earlier failed/interrupted diagnostics.

The canonical run includes formatting; embedded builds; default/all-feature strict Clippy; default/ACP unit and integration tests; isolated launcher/PTY tests; hardening/startup/heartbeat/approval controls; native/Java formal parity; secrets/header/KB/IR positive and negative controls; GUI tests, deliberate budget mutation and Metal screenshot; packaging; private ad-hoc hidden-window smoke with live frame, exit/reap and strict codesign. The expected IO-guard panic is an asserted negative control. Existing ignored tests retain their declared canonical treatment.

`make -C crates/bboxy450-harness check-formal-evidence TARGET_DIR="$PWD/target"` — exit 0, source afe92dae97b2a5f86209863fd1545d7208171b9b, dirty=false, 132 models, 44 contracts. Receipt SHA256 d1ac1f05bc7012e3edf33bd9e6fe40ebf25b2e5228e8ece5da3b8bca22e73221.

[Command/hash summary](../runs/2026-11-04-hashbrown-harness-gate.json) binds every canonical input hash and per-command exit. Raw final log SHA256 d525afd9da7006e30f6bb95034ba5fda2a14ce98a88113abbadc9e470b03ffbc. Full log, JSON, screenshots and model exports are retained outside temporary build cleanup at /Users/feral/.local/state/bboxy450-term/landing/harness/afe92dae97b2a5f86209863fd1545d7208171b9b/. Test-owned generated tracked files were individually restored from HEAD after archiving; the producer worktree is clean.

Earlier parent full-gate failure was strict Clippy in its upstream harness pin; subsequent producer diagnostics exposed the scoped compatibility, replay, presentation, fixture and catalogue issues above. All were repaired rather than bypassed. Old failed/interrupted logs are not final proof. The profile/search source remains unchanged by these producer repairs. Parent gate custody now also archives/restores the producer's newly tracked scrolled screenshot.
