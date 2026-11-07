---
type: report
title: Six-seat GUI idle CPU and retained attention frames
task: '[[tasks/2026-11-06-gui-process-cpu]]'
author: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
created: 2026-11-07T12:47:16.395Z
commits:
- 8d7c331a05e8a21388a88e9de54a4a659ac18c8e
verify:
  command: cargo nextest run -p bboxy450-term -p bboxy450-ui -E 'test(/idle|frame|redraw/)' && make test-formal-contracts
  exit_code: 0
id: 01M6V0S0K56S6TR4GE8MCW9ATF
updated: 2026-11-07T12:47:16.395Z
number: 93377961366263
created_by: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
updated_by: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
authorship:
  signer: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
  kind: seat
  public_key: y8Cwdp7Hb5QpbbNMa2wOXwhfpPdnzimP2mzXYL5W3Yg=
  process_generation: '1'
provenance:
  digest: sha256:b3fe891ac388f11d1a0bc999ab4e1916fd747bcd5d24579159c7852b8df7ccc1
  frontmatter_digest: sha256:d8a94ccf62d906a9757199f7771138e9e4e845e1566f9c708aace23f05def115
  signer: seat:01M6TQ312P1ZD2JPS1NQS3Y8JD
  sig: fdr9/6FZFDSvbjKsS2C0Qx/o4JQek8iKqWH6a/LhcL8vag7rs/iPwGvi1I6MIaTTzqZXNavDp41s+hoWUA/kDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150319
  event:
    id: 01M6V0S1GPZVTJ0PSVPW4H8V5E
    sequence: 150319
    hash: 7b8607221328dc430e06349cf01cc87e2b8f401fe08f471ba20eac2ee491b474
---
Six private real PTYs meet the idle budget: native quiet 0.10% of one core and zero frames; visible waiting 5.17% and 24.97 retained pulse frames/s; actually occluded waiting 0% and zero frames. The endless d44 attention behavior remains. No operator app, desktop capture, OS input injection, installation or Keychain prompt was used.

## Profile and acceptance evidence

The final rebased-source native gate used Metal display-link pacing on the Apple M4 / 75 Hz display and binary SHA256 cb2d0f5302b5914f5584d7a759e9320c793a7bf59dd794c2efc73520b5d18433. Each fixture creates its own account, HOME, repository, sockets and six zsh PTYs. Waiting fixtures release their OSC question only after all six real seat states exist; the occluded fixture hides its own window and requires the real occlusion event.

| Fixture / evidence report.json under /tmp | CPU % one core | Completed frames | Verdict |
| --- | ---: | --- | --- |
| bboxy-idle-cpu-hux_q2ti, headless quiet, 10 s | 0.00 | 0 | budget passes |
| bboxy-idle-cpu-vlhci48m, forced headless redraw, 3 s | 73.30 | 1883 full | negative refutes budget |
| bboxy-idle-cpu-og65p_im, native quiet, 10 s | 0.10 | 0 | budget passes |
| bboxy-idle-cpu-0ifcw_9_, forced native redraw, 3 s | 11.49 | 227 full, 74.54 fps | negative refutes budget |
| bboxy-idle-cpu-rbkzfngf, visible waiting, 10 s | 5.17 | 251 pulse, 0 full, 24.97 fps | budget passes; progress in both halves |
| bboxy-idle-cpu-7mz_i_st, full-scene waiting pulse, 3 s | 7.17 | 76 full, 0 pulse | negative refutes damage rule |
| bboxy-idle-cpu-wp676_sc, occluded waiting, 10 s | 0.00 | 0 | budget passes |

CPU alone would have accepted the 7.17% full-scene control. Requiring zero full frames during settled attention independently catches that defect. Controls must produce real completed frames; a startup/readiness failure cannot count as a successful negative.

Private profiling evidence is retained in /tmp/bboxy-idle-cpu-wp5oombf and /tmp/bboxy-idle-cpu-0dtepapd. All three sample commands and both summary plus the wide logic vmmap commands exited 0 in each fixture. These profiles used the earlier UI-identical binary 0cc7c67e8a41bab1fb9997cbf6cd4b5ebb765ac42a8da0ff2c0576fdf75de9ab, before the incoming C1-only logic changes were rebased.

The forced-continuous profile completed 74.70 full frames/s at 24.02% core. Its GUI sample at lines 55–69 shows App::redraw -> App::solve_ui -> Scene::layout_contents -> Sidebar::compose / component layout on unchanged frames. The retained profile completed 25.01 pulse frames/s, zero full frames and 7.37% core; gui.sample.txt:59 samples Gpu::draw_attention instead. The fresh quiet baseline was already quiet; the original production 70.8% incident was not reproduced or sampled. The demonstrated waste is the full layout/paint path at every attention/display tick, plus budget-tail rearming without actual rich-face dirty work.

## Changes and mechanisms

- crates/bboxy450-term/src/attention_frames.rs:27 — one owner maintains a >=33.333334 ms attention deadline, pauses the display link after quiet frames, and admits a pulse only while visible. Stable attention does not keep the full-rate link running.
- crates/bboxy450-term/src/main.rs:5560 — Metal data invalidations retain demand until a display tick; a tick combined with a retained model still schedules delivery. main.rs:5825/5836 invalidates the retained scene whenever a model is actually consumed, including the dirty-bit race.
- main.rs:8167 — an attention-only frame takes the retained GPU path before project/solve/paint. Full/pulse counters advance on actual completed draw admission. main.rs:9169 cancels pulse demand on occlusion. The existing notification delivery path remains active.
- crates/bboxy450-term/src/gpu.rs:555/806/1519 — retain the composition below the rims, restore disjoint rim border damage, update exact attention envelopes, and replay intersecting rims/static foreground in painter order. Unchanged interiors remain in the composition. Static frost dependencies expand damage to their sampling support. Resize, scale or unsupported cache geometry retires eligibility and uses normal rendering.
- gpu.rs:2549/2598 — real GPU readback checks disjoint damage and compares successive retained peak/trough/rise frames against fresh rendering, including overlapping draw/material/underlay/frost/group foreground. This checks both interior preservation and absence of pulse trails.
- crates/bboxy450-term/src/scene.rs:710/732/781 — separate transient animation from persistent attention; bind retained geometry to exact seat episodes, including multiple waiting episodes on collapsed tab headers; propagate scroll/box clipping so wholly hidden rows do not schedule pulses. Tests at scene.rs:3510/3556 check clipping and episode phase preservation.
- crates/bboxy450-term/src/rich_faces.rs:184–195 — rearm a spent rich-drain budget only when unvisited panes retain actual dirty work, rather than because elapsed time alone exceeded the budget.
- crates/bboxy450-term/src/headless.rs:375/413/432 — private diagnostic counters use completed renderer observations, and the debug negative control forces real dirty work.
- crates/bboxy450-term/tests/idle_cpu.rs and idle_cpu.py:61/197 — unattended six-seat process CPU/redraw gates, visibility assertions, recurrent pulse assertions, real negative controls, binary hash evidence, and exact PID/birth-fenced cleanup of only fixture-owned GUI/logic/supervisor processes. Debug controls require both --unattended and an explicit test account.
- Makefile:1074–1083, scripts/qos.sh, scripts/qos-policy.py and LAND.md — canonical test-gui-cpu, test-gui-idle and profile-gui-idle entries; measurements/orchestrators retain default QoS while compiler children retain background policy. The initial alias inherited utility and failed seven existing QoS assertions; the inventory fix makes all eighteen policy/cost tests and real private MCP probes pass.
- verification/contracts.toml, corpus.toml, coverage.toml; tests/formal_contracts/models.rs and idle_frame_model.rs — register IdleFrameDemand and its broken controls with source mappings. One UI owner, two revisions, visibility, coalesced drawable demand and one pulse token per period are finite state variables. Late ticks, output, waiting/reveal and occlusion interleave. Safety guards pause quiet work, preserve newest data, forbid full-scene pulses and allow at most one pulse per deadline. Each removed guard independently fails its named invariant at idle_frame_model.rs:162.
- .bboxy/specs/attention.md and rim-glow.md — record the Oracle's cap, damage and visibility rules without changing the endless attention behavior. The exported task is the exact committed KB snapshot, SHA256 2e7d11d4be54ad9da85d67414de7dcd29298b8abeaff59abfdd47db2f3bdcb29.
- crates/bbt-doc/tests/corpus/correction_fixtures.rs:243 — the authorized one-line repair updates the obsolete invalid-author rejection text to the current validator message; the invalid-author negative case and validator are unchanged.

## Logic RSS investigation

The original 1.2 GiB cannot be classified as a leak from this controlled run. The six-seat private logic process had ps RSS 45,616 KiB during the positive profile, while vmmap reported physical footprint 23.3M (peak 23.4M). Two snapshots, two seconds apart, were identical: 11,116 malloc allocations, 5,932K allocated, 19.5M allocator resident with 13.7M fragmentation, 560K resident stacks and 720K resident mapped files. Shared pages, allocator residency and actual allocated bytes are different measures; virtual address reservations are not resident memory. The earlier quiet six-seat baseline had 13.7M footprint and 5,324K allocated, also stable across its two snapshots.

The following is a source-backed structure/ownership inventory, not heap attribution of the production PID:

| Structure | Bound / retained work / source |
| --- | --- |
| Live terminal playback | bbt-supervisor/src/ring.rs:331 uses Term::new(cols, rows, 0): zero in-memory scrollback, main/alternate grids. bbt-mux/src/logic.rs:8083 caps cols at 512, rows at 256 and visible cells at 32768. Historical resize spare rows still matter. |
| Grid spare storage | bbt-core/src/vendor/alacritty/grid/storage.rs:13/91/168 retains up to 1000 spare rows per grid; byte cost depends on row width/cell contents. |
| Sequence-to-row index | bbt-supervisor/src/ring.rs:39/435 caps playback points at 4096. |
| Ring reads / pages | bbt-mux/src/logic.rs:8998/9022/9046 admits one outstanding read per seat with a retained page; supervisor protocol.rs:28 limits a batch to 768 KiB. |
| History reads | bbt-mux/src/logic/history.rs:61 retains one outstanding page request per seat. |
| Parser work | bbt-mux/src/logic/ingestion.rs:35–81 allows one parse job per seat, cloning its terminal state during parsing, with 1–4 CPU lanes. Peak overlap can exceed settled grids. |
| Adapter observations | bboxy450-harness-wake-up/src/observe.rs:371 caps history at 256 observations; :574 caps the OSC collector at 128 frames with 4096-byte parameter payloads. |
| SQLite | bbt-mux/src/store/sqlite.rs:67/119 caps idle connections at 64, not all simultaneously active leases. :153 requests a 256 MiB mmap ceiling per connection; this is not a resident allocation. |
| Workspace indexes | bbt-mux/src/knowledge/workspaces.rs:302–307/379–389 retains mounted roots without eviction. bbt-mux/src/index.rs:312–327 retains document maps, graphs and file metadata, with copy-on-write overlap during mutations. MAX_DOCUMENT at :23 limits one document to 2 MiB; aggregate root/document count is not capped. |
| Supervisor-only buffers | bbt-supervisor/src/ring/writer.rs:29–31 limits queued capture to 64 MiB/seat, 128 MiB total and 8192 jobs. The ordinary 4 MiB hot ring at ring.rs:113 is on disk. These are not logic RSS allocations. |

A read-only census of the production knowledge directory found 28,365 Markdown files / 78,046,274 bytes, versus 554 / 4,558,860 in this checkout. This was a filename/size census, not a load or profile of the live app. The native workspace index has no aggregate eviction bound, and SQLite mappings/allocator slack can also retain memory. Therefore the 1.2 GiB is not established as a six-seat ring allowance, and the short fresh-instance plateau does not rule out a long-lived leak or identify the original allocation owner.

.bboxy/invariants/seats-400-memory-idle.md budgets 420 MiB for 400 idle PTYs. Its witness in bbt-mux/tests/gates_seats.rs:93/271 measures a single combined test process using an empty temporary repository. That does not cover a mature multi-root knowledge workload. No logic allocator/cache code was changed and the 400-seat test was not rerun.

## Verification and handoff

Completed commands, with real exits:

- make test-gui-cpu — 0; /tmp/gui-cpu-verify-final.log: 49 selected idle/frame/redraw tests, 44 native contract tests, Clippy, eighteen QoS/cost tests, private MCP priority/latency probes and both probe controls pass.
- make test-gui-idle — 0; /tmp/gui-cpu-native-rebased.log: two tests / seven process scenarios, 112.855 s, including the normally ignored native witness. Actual final-source measurements are in the table above.
- make profile-gui-idle — 0; /tmp/gui-cpu-profile-final.log. make profile-gui-idle GUI_IDLE_PROFILE_ARGS='--control --seconds 3' — 0; /tmp/gui-cpu-profile-control.log; the measured negative itself fails the budget as required.
- make test-licenses — 0; /tmp/gui-cpu-licenses.log.
- make test-formal FORMAL_REUSE=1 — 0; /tmp/gui-cpu-formal.log: 681 models, 34 crates, 255 contracts, native/TLC full-state/edge/verdict parity. IdleFrameDemand passes with 239 states / 1029 edges; BrokenIdleFrameDemand fails safety with matching 526 states / 2314 edges. Identical archived models were reused only through the existing hash/graph/verdict checks.
- cargo test --locked -p bboxy450-doc --test corpus, through cache-env — 0; /tmp/gui-cpu-corpus-fixed.log: all fifteen tests pass after the authorized fixture/link/task-export repairs.
- Initial make land — exit 2 from Make (land script 77), /tmp/gui-cpu-land.log. Its Clippy and 460 app lib/bin/process tests passed; it stopped at missing same-tree report/task records and the obsolete corpus assertion. Those records are now prepared through the required KB workflow, not by weakening a gate.
- git diff --check — 0. The code/source checkpoint 8d7c331a05e8a21388a88e9de54a4a659ac18c8e is signed by Boxy with key 9B07E849480C68328CA20684844ECC0587DE7AB0; the worktree was clean for formal verification.

The clean checkpoint's retained formal receipt is /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/0439b0dc58aa18e9033b56d0d672dda9250c4c370b9cbe00d5f631e92157eb26/evidence.json, SHA256 0439b0dc58aa18e9033b56d0d672dda9250c4c370b9cbe00d5f631e92157eb26. Evidence survives target/worktree cleanup. The finite model proves the stated admission safety abstraction; real clock timing, CPU, GPU pixels and recurrent progress are implementation witnesses, not arbitrary Rust or scheduler proofs.

This append-only report is checkpointed in the native KB and exported byte-for-byte into the code tree in its own signed record commit. The final lean landing runs after that record exists; its push SHA, refreshed formal receipt and final worktree state are supplied by task_report. No install/reinstall, production-PID profiling, full make test, 400-seat run or cross-platform run was performed. Raw samples, vmmap data, fixture accounts and process logs remain outside Git.
