# Landing on main

One command, from your own branch in your own worktree:

```sh
make land
```

It runs `scripts/land-fast.sh` (`make push` is the same command):

1. Rebase on `origin/main` (exit 78 on conflict); refuse unsigned commits and
   initialize the newly pinned harness before computing or compiling the plan.
2. Compute the plan with `scripts/landing.py lean`: the crates whose files changed
   ("touched") and every crate that depends on them. `Cargo.toml`, `Cargo.lock`, `.cargo/`
   and `rust-toolchain.toml` touch every crate; docs/lab-only diffs touch none.
3. **Lean gate (default):** `cargo clippy -D warnings` over the touched crates; for each
   touched crate, nextest over its lib and bin tests (when its source changed) plus each
   changed integration-test file; `cargo check` of the dependents. A test-only change runs
   only that test. `.bboxy/` changes add the doc corpus test; model or `verification/`
   changes add `make test-formal`. A nonempty integration target skipped entirely by
   `#[ignore]` is metadata-checked and reported as compiled only; its native Make check
   remains explicit. Empty filters and build/test failures still fail the gate.
   `LAND_MODE=wide make land` instead lints and tests the
   whole affected set, doctests included.
4. Push. If `main` moved, rebase, initialize the current harness pin, rerun the
   current-source gate and push again (up to 5 times).

On macOS, Cargo builds/checks/clippy and compiler/linker children run under
`taskpolicy -b`; test trees use `taskpolicy -c utility`. `cache-env.sh` installs
private Cargo/compiler shims in the selected target, inherited through scripts.
Test selections prebuild with `--no-run` as a background tree (including native
build scripts), then execute against that warm selection. Nextest's controller stays unclamped;
a command-scoped stable Cargo target runner assigns utility to each ordinary
test and default to the named timing witnesses below. The pathless private shim
works in checkout paths containing spaces, preserves an existing environment
runner, restores nextest's Darwin library paths after taskpolicy, and restores
the previous runner configuration before executing the test. Cargo libtest
cannot split one process by test: an unfiltered mux, wake-up, harness-host or app
batch runs at default because it contains these witnesses. Filtered ordinary
libtest selections stay utility. No tests or assertions are dropped.
Rustdoc compiles doctests at background policy and uses its stable test runtool
to execute each doctest at utility; `--doc` first prebuilds library dependencies.
The sccache daemon is started at background policy as well. Linux retains
`nice -n 10`. Land defaults to two Cargo jobs (`BBOXY_CARGO_JOBS`) and two test
threads, reusing the worktree target and shared cache.

Each gate command prints `land-cost` with wall seconds, child CPU seconds,
allocated target-directory growth in bytes and exit status, including failures.
The final `total` line accounts for the entire landing process tree (including
rebases and pushes). CPU uses `getrusage(RUSAGE_CHILDREN)`, so a separate already
running sccache server's CPU is outside this measurement. Disk growth is the
target's `du -sk` delta; shared cache growth is outside it.

### Explicit default-QoS measurements

Headless measurement/tool entries retain default; the app claims interactive
when it enters the actual window thread. Supervisor, logic, desktop and MCP
claim their explicit roles at startup.

Only these measurement entries opt out. Their compiler/linker children still
run at background policy. Test execution outside this list stays utility.

| Entry | Measured work |
| --- | --- |
| `test-dream-c1`, `test-dream-c1-integration`, `test-dream-c1-qos`; Cargo selections `dream_c1`, `dream_replay`, `dream::c1::` | Dream C1 CPU/latency bounds; the root native workflow, harness integration workflow and thirteen private QoS re-enable witnesses retain their own entry points |
| `test-perf-p1`, `test-perf-p2`; Cargo selection `gates_seats` | P1 frame/hitch and P2 bus/PTY budgets |
| `bench-budgets`; Cargo selections `gui_budget`, `rich_faces::budget_verification::rich_four_view_aggregate_budget` | UI drain/composition and GUI frame budgets |
| Cargo `bench` and `test --benches` | Benchmark execution (compiler children remain background) |
| `test-input-latency`, `test-input-backend`; Cargo selections `input_latency`, `logic::idle_command_tests`, `seat_socket::input_check_tests`, `seat_socket::gui_tests::busy_gui_poll_and_locked_cursor_keep_the_session_and_queued_events` | Input/echo latency |
| `test-terminal-streaming`, `test-codex-streaming`, `test-native-strip`; Cargo selection `gpu::streaming_tests::` | Frame latency and streamed rendering |
| `test-seat-ingestion`, `profile-ingestion`; Cargo selections `client::tests`, `transport::tests`, `logic::ring_batch_tests`, `capture_fairness` | Watched-seat read/parse/publication latency |
| `profile-input-ui`, `profile-render-input`, `profile-power`; Cargo selections `lazy_projection_tests::input_frame_cpu_profile`, `repeated_text_frame_cpu_profile`, `cpu_profile`, `rich_native_parity_and_composition_microbench` | UI layout/glyph CPU, idle-power CPU and native composition timings |
| `test-formal`, `test-formal-contracts`; Cargo selection `formal_contracts` | Real-time API witnesses paired with the contract corpus |
| `test-qos`, `test-qos-app`, `scripts/qos-probe.py` | Private MCP latency/policy controls and app startup class witnesses |
| `test-landing`, `test-landing-extended` | Actual default/background/utility process-priority witnesses |
| `test-parity` | Aggregate parent of the full CI timing gates |
| `test-harmony-host`, `test-harness-startup`, `test-harness-native` | Explicit mixed-library parents and parent of `bench-budgets`; ordinary filtered Cargo selections and individual nextest tests remain utility |
| Invariant leaves `seats-1000-bus`, `seats-400-memory-idle`, `seats-967-frame` (`scripts/gates.sh`); `presubmit-tests-fast` (`scripts/fast-tests.sh`) | Bus/CPU, memory/idle, frame and test-runtime budgets; Cargo/Make invariant commands orchestrate their own policies |
| `test-terminal-replies`, `test-task-pages`, `test-task-panel-formal`, `test-jsonl-recovery`, `test-capture-storage-model`, `test-watcher-models`, `test-toolkit-models`, `test-knowledge-orientation-formal`, `test-provenance-background-formal`, `test-provenance-mcp-formal` | Explicit parents of the `formal_contracts` measurement executable; ordinary Cargo selections remain utility and model-checker Python leaves use utility |

The thirteen original C1 load-family witnesses also opt out individually in mixed nextest
suites: `actual_private_process_clears_authority_environment_and_uses_fixed_argv`,
`actual_private_prepared_start_is_one_write_and_duplicate_closes_without_replay`,
`actual_private_control_invalid_ulids_and_unsafe_revision_emit_no_new_frame`,
`actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames`,
`eof_and_drop_retain_exact_child_custody_through_actual_reap`,
`custody_copies_exact_ir_and_only_typed_blob_carriers_into_private_new_inodes`,
`dream_c1_source_failure_is_bounded_immutable_and_fenced_before_any_replay`,
`dream_c1_source_error_capacity_is_atomic_across_real_sqlite_owners`,
`dream_c1_admitted_mcp_is_exact_own_seat_and_reports_honest_missing_runs`, and
`dream_c1_held_private_delete_catalogue_keeps_mcp_gui_control_owner_independent`.
The inventory is shared by Cargo filter classification and the per-test runner
in `scripts/qos-policy.py`.

The subsequently observed
`dream_c1_actual_control_routes_reject_expired_public_grant_without_credentials`
and `dream_c1_schema_bootstrap_busy_writer_refuses_without_waiting_or_partial_publication`
and `dream_c1_concurrent_register_original_operation_wins_before_candidate_source_key`
also retain default for their native CPU units, bringing the inventory to sixteen.

The three journal/authority witnesses listed first below complete this thirteen
test re-enable inventory. `make test-dream-c1-qos` runs each private component/API
witness separately and refuses empty or ignored-only selections. It does not
launch the one-shot C1 runtime or require producer provisioning. All thirteen
load-family ignores are removed after the primary QoS landing. Six witnesses
remain enabled. Per the Oracle's final ruling, these seven original defects and
the subsequent expired-grant admission, busy-writer schema and concurrent-registration failures (ten
witnesses in total)
retain known-defect ignores and their owning tasks assume re-enabling them:

- admitted-MCP stale-context mapping: `tasks/2026-11-06-dream-c1-honest-capture-errors`;
- held-catalogue control latency: `tasks/2026-11-06-dream-c1-cold-mcp-latency`;
- source-error capacity / busy0 contention: `tasks/2026-11-06-dream-c1-two-owner-storage-race`;
- both final-authority setup witnesses, immutable source-error replay, expired-grant admission, busy-writer schema and journal-registration CPU units: `tasks/2026-11-07-dream-c1-native-unit-budget-overruns`.

The target prints all ten deferrals and runs the other six; it does not claim
a thirteen-test pass. Each owner removes its ignore and the corresponding
`C1_DEFERRED_TESTS` entry on repair. No assertion or budget changes.

The following C1 unit witnesses also retain default for their measured 1 ms
native/cleanup CPU maxima and 100/250 ms API bounds. Their fixture requirements
and existing ignored status, when applicable, are independent of QoS:

- `dream_c1_final_actual_authority_held_clock_cpu_refusals_keep_owned_child`
- `dream_c1_registration_commits_close_cursor_original_op_and_source_key_once_after_reopen`
- `dream_c1_final_actual_authority_rechecks_row_head_grant_inode_aba_after_debt`
- `dream_c1_held_provider_selection_refuses_real_mcp_gui_controls_within_250ms`
- `dream_c1_held_provider_selection_refuses_actual_page_owner_before_next_unit`
- `dream_c1_held_provider_refuses_ordinary_mcp_and_gui_read_admission`
- `dream_c1_held_provider_suppresses_actual_successful_ordinary_gui_completion`
- `dream_c1_schema_bootstrap_busy_writer_refuses_without_waiting_or_partial_publication`
- `dream_c1_delete_catalogue_cold_init_commit_close_preserves_original_operation`
- `dream_c1_legacy_wal_transition_preserves_rows_and_busy_reader_never_changes_custody`
- `dream_public_prepared_session_releases_transaction_and_lock_between_fresh_units`
- `dream_c1_tiny_json_combines_scan_decode_work_and_reuses_complex_fallback`
- `dream_c1_tiny_json_error_and_cpu_verdict_keep_owned_cleanup`
- `dream_c1_collection_growth_reserves_old_overlap_and_preserves_resource_limit`
- `dream_c1_native_buffer_free_is_measured_and_retains_reservation_through_cancel`
- `dream_c1_rejected_owned_results_drain_before_credit_for_cpu_clock_cancel_deadline`
- `dream_c1_dense_value_success_and_later_error_scope_retirement_are_paced`
- `dream_c1_partial_serde_value_after_mid_reader_stop_has_owned_paced_collections`
- `dream_c1_typed_descriptor_and_nested_episode_partial_collections_retire_before_error`
- `dream_c1_cooperative_preparation_preserves_cumulative_cost_and_paid_phase_maxima`
- `dream_c1_withheld_returned_effect_retains_lease_and_independent_cached_controls`
- `dream_c1_final_public_control_is_once_after_debt_and_retains_component_child_on_refusal`
- `dream_c1_private_signed_gated_worker_held_parent_paid_probe_and_refusals`

These existing opt-in performance witnesses also run at default when selected;
their ignores are retained. A Cargo libtest `logic`, `reaping` or `runtime`
integration batch that includes these ignored measurements stays default;
nextest continues to classify each selected test separately:

- `ring_seals_twenty_mib_with_logic_stopped_and_a_stalled_notification_queue_then_replays_without_gap`
- `ring_capture_throughput_with_logic_running_and_stopped`
- `ring_cold_replay_twenty_mib_profile`
- `ring_capture_twenty_mib_profile_running_and_stopped`
- `final_output_and_exit_reach_the_owner`
- `exit_events_at_machine_scale`
- `million_messages_per_second_never_wait_for_a_consumer`

Aggregate `test`/`check` orchestration stays at default; ordinary CI leaf drivers
run at utility. The macOS main watcher also stays unclamped, letting its CI
leaves select their policies; non-macOS watchers retain `nice`. A Darwin utility clamp is inherited and cannot be cleared by
the background-policy reset: timed Make parents must stay unclamped, then their
explicit Cargo selections choose default independently of ordinary utility tests.

The probe compares default versus background logical-CPU saturation. Its actual
native bridge forwards `self_status` through a private mux/socket and identity.
Default load is comparative evidence: it need not exceed 250 ms. The deterministic
controls leave load at the wrong QoS (policy failure) and delay the reply past
`max(4 * unloaded p95, 250 ms)` (latency failure). `make test-qos` requires both
specific failures as well as the restored pass. Requested thread class and
effective process priority are reported separately; neither promises core affinity.

`make land-full` is the full signed gate (`scripts/land.sh --full`): `make test` plus a
formal receipt. Gate slots keep their checkout and target warm between landings, so only
changed crates rebuild; one slot runs at a time (`BBOXY_GATE_SLOTS`, at most 3) and other
landings queue for it instead of failing (`BBOXY_GATE_WAIT=0` restores exit 75); it uses 4
Cargo jobs (`BBOXY_GATE_JOBS`). Use it for
a release, a toolchain change or when a brief asks for the full suite.

## Why diff-scoped by default

This laptop runs many seats at once (16 cores, load 7-36). The full gate took a median
of 24 min (p90 30, max 51) while `main` moved every ~33 min, so agents sat in the gate and
then raced. Measured cost was mostly compilation in a fresh, empty gate target (10-25 min)
and test binaries run one after another (8-15 min). The diff-scoped gate builds warm and
tests the affected crates in parallel. A red `main` is fixed forward by whoever broke it.

## Rules

1. **Stay off shared hot lines.** Every agent edits these, so touching them conflicts with
   every landing: the `families` array in
   `crates/bboxy450-term/tests/formal_contracts/models.rs` and the `bboxy450-term-mux`
   entry in `verification/coverage.toml`. Put new model families in their own module
   appended with one `cases.extend(...)` line (see `plugin_models.rs`), and register them
   under your own crate's coverage entry.
2. **New crate:** in the same commit, add it to `scripts/layering.sh` `ORDER`, classify it
   in `.bboxy/specs/licensing.md` (decision 362), and add a `verification/coverage.toml` entry.
3. **Installed binaries equal the package name** (decision 362, `scripts/brand.sh`). Make
   helper tools examples, not binaries.
4. **Links in committed `.bboxy` records must resolve.** Never link to an untracked local
   record.
5. **Keep build headroom and bounded fixture space.** Recovery reserves 128 MiB plus
   1 MiB per seat (132 MiB for the default four-seat run), checks retained file bytes
   against that budget, and reports usage. It does not require a blanket 10 GiB free.
   Prune your own build dirs only.
6. **Every commit compiles on its own.** Run `cargo check --workspace --all-targets` per
   commit when you rebase several.
7. If your default `rustc` is older than the workspace `rust-version`, export
   `RUSTUP_TOOLCHAIN` for the run. Never change global Cargo configuration.

## When it stops

- **Exit 78, conflict:** resolve the rebase by keeping both sides (rule 1 makes this
  rare), then `make land` again.
- **Exit 77, gate failed:** the output names the failing check. A real failure is yours to
  fix. A test that fails only under load and passes alone may be quarantined only per
  `.bboxy/quarantine.md` (owner, task, at most 7 days).
- **Exit 75:** `main` kept moving; run `make land` again.
- `make land-full` keeps its logs in `target/land-attempt-<n>.log`.

## Formal evidence retention

After a successful `make land` or `land.sh`, evidence larger than 5 GiB schedules
maintenance detached at background QoS (`taskpolicy -b`, nice 19 on macOS).
`FORMAL_EVIDENCE_BUDGET_BYTES` changes the trigger. Maintenance failure is logged
in the private evidence store's `prune.log` and does not change a landing verdict.

`make prune-formal-evidence` keeps the newest `KEEP=10` receipts, all receipts
for `origin/main` and extant worktree HEADs, and evidence/source hashes cited by
reports whose tasks remain unaccepted (unknown tasks are conservatively pending).
`DRY_RUN=1` lists the counts without writing. Other receipts go to one monthly
`bboxy-formal-evidence-archive/receipts-YYYY-MM.tar.zst` beside the Git evidence
store. Tar hardlinks also deduplicate identical compressed graphs across monthly
appends. Its `.tar.sha256.json` manifest binds the archive and every member; frame
integrity, exact file count and all hashes must pass before any original is
removed. Appends verify the previous archive first. Broken custody retains the
originals. Pins are refreshed after archive IO, before removal.

New receipts store `.dot`, `.states` and `.edges` as zstd files hardlinked to
`by-model/graphs/<uncompressed-sha256>.zst`. Hashing and reuse decode the original
bytes. Each receipt owns a hardlink, so index cleanup cannot invalidate readers.
Legacy raw receipts remain readable and follow the same retention policy. The
zstd executable is required for writing or reading compressed evidence; no
product-runtime dependency is added. Concurrent maintenance skips its busy lock;
formal producers and receipt readers never acquire that lock.
