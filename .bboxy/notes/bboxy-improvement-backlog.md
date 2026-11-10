---
type: note
title: bboxy harness + terminal continuous improvement backlog
id: 01M6S3QY6GCE4X5X8FGNQGVN2Z
updated: 2026-11-06T19:00:37.511Z
created: 2026-11-06T19:00:37.511Z
number: 93377961366083
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:290eed4cec74f6d8dcf700f3d77c6ee3d8b6070f06814bf4676f6c0889b398d1
  frontmatter_digest: sha256:35b6e2a1a50dd9869fb125da6ba6e1ff3a09db3017e825b2cced68e05f8f9ca4
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: HAzdvgrLrAvTp8/W4NN4uQxVXYHcpsO142VEa7aE5os91fQjWe5IFFMgRJVXe1TM1uT1b+9D06mlO9xpFAsFAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149733
  event:
    id: 01M6S3QZDG123S9KHRX6J3V83F
    sequence: 149733
    hash: 3f4bcd092d47bfaca743e93ac995d7c89f5799490a0714ed37d9aa379e6fabce
---
# bboxy improvement backlog (Oracle-maintained)

Standing operator order, 2026-11-06: improve bboxy harness and terminal continuously in the background. Ranked by machine pain: lag/MCP timeouts, then CPU, then disk, then blocked workflow, then ergonomics. Run at most 1–2 improvement executors next to product work, and check `df` and load before spawning.

| # | Task | Pain | State |
|---|---|---|---|
| 1 | [[tasks/2026-11-06-python-execute-nonblocking-and-legible]] | blocked workflow, lag | **landed** harness fe266d6a; needs a term harness bump plus the operator's reinstall |
| 2 | [[tasks/2026-11-06-mcp-live-catalogue-large-wake-dead-seat-close]] | blocked workflow | in progress (seat 01M6S0BP…) |
| 3 | [[tasks/2026-11-06-qos-builds-never-starve-mcp]] | **lag**, CPU | queued: waits for free disk (C1 worktree cleanup) |
| 4 | [[tasks/2026-11-06-one-test-binary-per-crate]] | CPU, disk | queued after #3 |
| 5 | [[tasks/2026-11-06-mux-preexisting-test-failures]] | gate noise | open |
| 6 | [[tasks/2026-11-06-dream-c1-cold-mcp-latency]] | lag | open |
| 7 | Harness: one test binary per crate (67 files) | CPU, disk | to file after #4 shows its numbers |
| 8 | Workspace prepare after a task amend: binding goes stale silently (`workspace_task_changed` only at spawn) | workflow | to file: prepare should report the contract digest, and amend should warn about bound workspaces |
| 9 | Python execute: emits before a crash are lost unless emitted early; `python_execution_get` record lags process exit | ergonomics | partly in #1; recheck after reinstall |
| 10 | A worker without git-worktree metadata (managed checkout) breaks the sibling-path assumptions in scripts | workflow | formal.py fixed in #1; sweep the other scripts |

Workarounds in use (each needs its fix to land): a fresh `bboxy450-term mcp` bridge from Python ([[decisions/d93377961366080-2026-11-06-dream-task-ownership-override]] context), and direct `assignee`/`delegated_by` frontmatter edits.

## Added 2026-11-06 (later)

| # | Item | Pain | State |
|---|---|---|---|
| 11 | [[tasks/2026-11-06-formal-evidence-retention]]: 22 GiB of evidence, never pruned | disk | open |
| 12 | [[tasks/2026-11-06-gui-process-cpu]]: GUI at 71% CPU with nobody typing, logic at 1.2 GiB | lag, CPU | open |
| 13 | [[tasks/2026-11-06-build-dirs-out-of-spotlight-fsevents]]: marker files placed by hand in 36 folders; make it automatic | CPU | open |
| 14 | [[tasks/2026-11-06-oracle-messages-reach-busy-executors]]: rulings stuck "native-queued" for an executor's whole turn | blocked workflow | **in progress** (seat 01M6S742…) |
| 15 | `land-fast.sh` does not refresh formal or verify gates after it rebases onto a moved main; it only runs `cargo check`. Reported by the executor that landed 24d03c76, which refreshed them by hand | correctness of land | to file |
| 16 | `artifact_claim` refused a report with `artifact_not_committed` even though its knowledge checkpoint (87c54975) was committed | workflow | to file |
| 17 | Python execute reports a clean `SystemExit(0)` as failed; the paged `knowledge_fetch` page 2 errors on externally edited records | ergonomics | to file |

#2 (live catalogue, 40 KiB delegation, dead-seat close, amend diagnostics) **landed as 24d03c76 and was accepted**. It reaches the running app only after a reinstall.
| 18 | `scripts/quarantine.py` resolves `--task` only against the code repo's tracked legacy `.bboxy/tasks` (149 files), so tasks in the native KB are invisible from worktrees (`quarantine_task_not_unique`); its isolated retry also inherits the caller environment instead of Make's test profile and corpora | blocked workflow | to file |
| 19 | Executor-less worktrees accumulated: 55 removed 2026-11-06 (28 with uncommitted changes saved as patches). Close-out should remove a worktree automatically once its seat is closed and its commits are on main | disk | to file |
| 20 | Formal receipts were 6.9 GiB each, uncompressed; the operator ordered all evidence removed (39 GiB) | disk | see #11 |
| 21 | TLC verdict ordering is nondeterministic under load: periodic liveness checks are wall-clock triggered, so a model violating both safety and liveness flips between them (seen live 2026-11-06). Term fix: `-lncheck final` (in C1). The harness's adapted `scripts/formal.py` needs the same flag | correctness of gates | term: landed in C1 (06dacc38); harness: in progress (QoS executor) |
| 22 | `verification/{contracts,corpus,coverage}.toml` are three shared append-only files that every model-touching task edits, so concurrent landings conflict on rebase every time (C1 vs d88df43b, 2026-11-06) and each conflict forces a full formal rerun (671 models). Fix: one inventory file per crate or model family (e.g. `verification/models/<family>.toml`), merged by the checker; plus per-model evidence reuse keyed by model hash (see #11), so a rebase reruns only changed models | CPU, landing speed | to file |

Update (~22:15): term origin/main pins harness 42e4c802 (Maksim Soltan), which contains fe266d6a, so the Python execute fix (#1) is in term main and the separate harness bump is no longer needed. #1, #2 (24d03c76) and #14 (d88df43b) are all on main and wait only for the operator's reinstall.
| 23 | Closing a Codex seat (`seat_close`) leaves its MCP servers running: `bboxy450-term mcp`, `paperhands mcp`, `rea mcp`, `node_repl`. The shared Codex app-server daemon (pid 91555) spawned them per thread, so they outlive the seat by hours, keep the worktree as cwd, and hold memory (seen for C1 seat 01M6RZZY… pids 46817–46852, and the dream-entry-c1 leftovers). Close-out should end the thread's MCP process group, or the daemon's thread/close should | memory, cleanup | to file |
| 24 | `task_accept` correctly refuses when verify_result ≠ task verify, but there is no lifecycle state for "landed under operator override, acceptance pending on follow-up X"; C1 sits in `reported` with a prose note | workflow honesty | to file |
| 25 | The QoS landing's `make land` took **5100 s wall / 4752 CPU-s / +8.4 GB disk** for one landing (full formal plus the wide mux/term suites after rebase). Landing cost is now the biggest CPU and disk sink: prioritize #4 one-test-binary-per-crate, #22 split verification inventories, and per-model formal reuse | CPU, disk | to file |
| 26 | The harness repo has no QoS build changes (term's landed QoS does not apply to harness `make` targets), so harness builds still run at default priority and compete with the UI. Port cache-env/taskpolicy to the harness Makefile | lag, CPU | to file |
| 27 | GUI process (old installed build) seen at **124% CPU** at 02:00, up from 71% earlier; raises #12's priority | lag | #12 open |
| 28 | `workspace_release` refuses any workspace with ignored files, including `target/` build output, so finished tasks' managed workspaces piled up to **~63 GiB** (6 workspaces; free space fell to 16 GiB and blocked Q2's formal gate). Release should remove regenerable build output itself (after verifying commits are on main, nothing uncommitted, nothing holding it), and close-out after `task_accept` should trigger it. Workaround used 2026-11-07: Oracle removed the `target/` dirs, then release succeeded | disk | to file |
| 29 | `knowledge_write` on update DROPS `created` / `created_by` unless the caller resupplies them (found by the Helper Budget Architect, who restored them on specs/dreaming-mcp.md). An update must preserve host-stamped origin fields; check other records updated today (Oracle amends re-sent the full frontmatter, so they are likely intact) | data integrity | to file |
| 30 | Per-run formal caches fill the disk: Q2's `target/` grew to **109 GiB**, because every failed formal run and every landing keeps a full writable TLC/cargo cache (1–16 GiB each) under a new directory. Free space fell below the builder's 8 GiB gate twice on 2026-11-08, and the Oracle deleted ~60 GiB by hand. Fix: keep only hash/summary JSON for failed runs, reuse ONE warm cache per worktree, and `make land` refuses below a free-space floor with a clear message | disk | to file |
| 31 | `seat_close` returns a result over 64 KiB, so it can't be `emit()`ted from python_execute (output_limit). The close appears to happen, but the caller gets no readable receipt. It should return a compact receipt (seat, generation, closed, pid reaped) and keep the bulk behind a reference. Also: its required `expected_process_generation` is easy to miss; the refusal text should name the missing field | ergonomics | to file |
| 32 | Under load (avg 6+), every bridge MCP call (knowledge_write amend, task_accept, seat_spawn) takes 40-90 s. That makes every Oracle ruling a multi-minute wait and is the MCP lag the operator reported. The mux/MCP threads must keep interactive QoS while builds run at background QoS (re-check after the QoS task) | lag | to file |
| 33 | Harness lib tests do not compile with `--no-default-features --features embedded` (the feature set term uses): `src/python_execute/tests.rs:860/908` reference `mcp::relay` unconditionally, and that needs `acp-agent`. Gate those tests on `acp-agent`, and add a CI check that builds the lib tests with term's exact feature set | build | to file |
| 34 | Fresh evidence for #23 (2026-11-08): after `seat_close` of Dream Q3 (seat 01M6YC2FT3YFH5DH6NJX7N8VJX, close status applied), two `bboxy450-term mcp` processes (PIDs 40302 and 58097, alive 5.5 h and 7.5 h) still have the closed seat's worktree as their cwd. That blocks the close-out worktree removal (lsof rule). seat_close must reap the seat's MCP children | lag/disk | to file |
| 35 | `seat_close` refuses stale seats whose pane is gone ("target viewport unavailable"): 12 executor seats from earlier sessions stay `open` forever in seat_list. Closing a seat must not need its viewport; a dead seat should close by record (reap whatever process remains, mark closed) | clutter | to file |

36. **task_close unavailable to the Oracle seat** (2026-11-09): task_accept hints `task_close`, but the call returns "tool is unavailable for this actor and catalogue", even through a fresh bridge. Accepted tasks cannot reach done over MCP. Also: seat_close leaves bboxy450-term processes holding the worktree (see #34), so it cannot be released right away.
