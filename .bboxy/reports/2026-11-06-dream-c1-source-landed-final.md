---
type: report
title: Dream C1 source landed; final-main formal verified; runtime unapproved
task: '#93377961364376'
author: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
created: 2026-11-07T01:31:40.343Z
commits:
- 06dacc38da2488e845bc25acebbb3855ed2c64b6
verify:
  command: FORMAL_REUSE=1 make test-formal && make check-formal-evidence
  exit_code: 0
supersedes: '[[reports/2026-11-06-dream-c1-help-passed-formal-verdict-blocked]]'
id: 01M6ST3Y5X5S8BVQH63EX3QPEK
updated: 2026-11-07T01:31:40.343Z
number: 93377961366193
created_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
updated_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
authorship:
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  kind: seat
  public_key: vTy8kxzALPVGVJxPquLZelvl5D6TNzgsirkLE/DA5ek=
  process_generation: '1'
provenance:
  digest: sha256:32f0b88a2c4fc4f547b5abc880634eba6c53bae60b4e1a96b65d1ef63f9cf2a2
  frontmatter_digest: sha256:39d13fdf400b070b05cc6c652e3140b24646a16f04a11a26b819c112d1dab3a4
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  sig: ATvK+NBnWrG32jS3pHiZzWLGx5ZRZxYCv1zUGBR1Rph0ycJnwzcsklmcSmccaRKMzlD/GDF0v+MaoHNcLJbPBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150081
  event:
    id: 01M6ST3ZH810CKEYPQ614ZKZ0Q
    sequence: 150081
    hash: 4616725f784bf60c8e8e79d34c5c0a9d20ac4fda303b5020bb8b9395963ec4c2
---
Source-only landing COMPLETE: signed commit 06dacc38da2488e845bc25acebbb3855ed2c64b6 is on origin/main. The final detached main checkout I verified is that SAME SHA; harness checkout and gitlink are both f6a247b2be2ef21bb4d38ef805d404876d9e4adc, on harness main and containing the historical 2ce4359 producer. Worktree status is clean. No installation, live supervisor/app change, bundle recipe change or bundle sidecar was performed.

This append-only final addendum supersedes the blocked status in [[reports/2026-11-06-dream-c1-help-passed-formal-verdict-blocked]] and supplements the full mechanism/evidence record [[reports/2026-11-06-dream-c1-persist-entry-landed]]. Earlier failures and dated-source evidence remain historical. Oracle source-only exceptions and the final verification disposition are messages 01M6S3848WM40WWQ3MFXN1189Y, 01M6S8S6276F1C2HJ7H13YQYMR and 01M6SSK9BFQ29M1GPQRBRS3VMX. Runtime remains FAILED / UNAPPROVED / not enabled; this report does not claim full original joined-runtime acceptance.

Verification and exact evidence:
- make land — exit0, LANDED 06dacc38da2488e845bc25acebbb3855ed2c64b6; /tmp/dream-c1-protocol-wall-pin-make-land.log and its -result.json. Signed-commit gate, affected all-target Clippy, changed lib/bin and integration-test selections passed. The authorized exclusions below are real omissions, not passed assertions.
- Automatic push-race rebases left an old formal receipt and an older submodule checkout. Initial post-land make check-formal-evidence exited2 (source differs), /tmp/dream-c1-landed-check-formal-evidence.log. I disclosed that immediately; Oracle explicitly directed final-main verification instead of another landing loop.
- git fetch; git checkout --detach origin/main; git submodule update --init crates/bboxy450-harness aligned the final checkout. FORMAL_REUSE=1 make test-formal — exit0 on exact 06dacc38da2488e845bc25acebbb3855ed2c64b6, final pin f6a247b2be2ef21bb4d38ef805d404876d9e4adc. make check-formal-evidence — exit0 on the same clean source. Logs /tmp/dream-c1-verified-main-formal.log and /tmp/dream-c1-verified-main-check-evidence.log; each has a separate -result.json.
- CURRENT immutable formal receipt: f71ba0486b94f751fe2ca7e12ef2896fa0ced67f7a49f6d4edd3e8a79a8a2442, /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/f71ba0486b94f751fe2ca7e12ef2896fa0ced67f7a49f6d4edd3e8a79a8a2442/evidence.json. 679 models / 34 crates / 254 contracts; complete bounded native/TLC state, labelled-edge and named-verdict parity with canonical hash-validated unchanged-model reuse. Caps, expected verdicts and broken models were not weakened. Finite model parity does not establish arbitrary Rust/OS correctness or erase the storage defect below.
- make test-rich-pane — exit0 on merged893734b8ac2118e5e802a1942a1d8b50d1b2d413: 4 rich_pane tests, 2 real private rich-client tests, 4 rich_faces verification tests, all passed. /tmp/dream-c1-parked-rich-pane-tests.log. Offscreen row_structure_mismatch=0; composition microbenchmark is explicitly not a loaded-frame or measured-heap proof. This dated check used then-main harness13a2038; later final-pin verification follows Oracle's exact disposition above.
- Prior make test-formal-contracts — exit0,43passed plus strict lint on8d9cb55f, /tmp/dream-c1-help-formal-contracts.log. Later differences were upstream reconciliation, explicit test routing/omissions and the parked-view payload merge; this is dated evidence, not a falsely claimed final-source rerun. Native provisioning7/7, profile controls, licenses and boundaries remain dated in the linked reports.
- The prescribed original AND-chain never passed: the consumed make test-dream-c1 exited2. Its failure is preserved under the source-only override, not relabelled green.

Final changes and reconciliation:
- Private C1 catalogue stays PERSIST, synchronous FULL, busy0; supported WAL migration remains SQLite-owned. Focused persistence COMMIT observations were under1ms; no durable_commit class was introduced. All production 1ms units,9x cuts, custody checks and size limits remain unchanged. A later two-owner failure is separately disclosed below.
- Exact-file admission uses side-effect-free --help, private first-launch/2 schema/argument key and durable before-exec claim; known failed --version history remains byte-identical, unknown history refuses. Native receipt validation and admission-before-Start model/broken control remain. Public protocol/profile versions and250ms Hello are unchanged.
- Parent verification TOMLs retain the union of main and C1 entries. Final local inventory validation preserved176 contract tables/398 model tables/34 crate tables; /tmp/dream-c1-parked-union-validation.json. Harness pins follow main, never regress to the historical build pin.
- scripts/formal.py equals main plus strict explicit multi-control family/verdict validation and TLC -lncheck final. The latter fixes periodic safety/liveness race classification after complete exploration; BrokenHarnessDreamC1QuantumDebtAbaEffect now consistently reports safety with13902states/36975edges. The failed periodic artifacts remain /tmp/dream-c1-tlc-periodic-failure-b3cfcx7q. The obsolete _HarnessContractLayout shim and its four shim-only tests were removed per Oracle; upstream HarnessDraftHandoff and its broken control remain checked.
- rich_pane.rs carries main's WHOLE Parked{view,retained,started} through the exclusive UI-owned Option<oneshot::Receiver<Parked>> reservation. Restore reattaches the retained bridge and reuses the view/clock; save parks the whole view. No Mutex returned. DraftControl positive, early-restore, lost-tail and off-owner-order assertions retain their meaning with Parked fixtures. Oracle disposition01M6SQRZX1QB6XF4J1T71GF7AT; merged tests passed as above.
- The standing empty-target grant added real pure tests to keychain, keychain-file, keychain-windows (no Windows API), term-desktop, plugin-fixture lib/bin, and the default Yarn library. Yarn checks persistent set membership/removal/idempotence at domain ordinals0/62. No dependency, production behavior or generic empty-target planner exemption was added.

KNOWN DEFECTS (not load flakes):
- `dream_c1_two_real_sqlite_start_owners_dispatch_exactly_one_private_child` — crates/bbt-mux/src/registry/dream_c1.rs:4036.
The failed witness returned StorageUnavailable after a real child dispatched; child38076 was actually reaped exit0, then the owner thread failed its post-dispatch record. Oracle classifies this as a likely real busy0 concurrency defect, filed HIGH at [[tasks/2026-11-06-dream-c1-two-owner-storage-race]], and explicitly authorized the ignored test/source-only landing. Assertions remain unchanged. Evidence /tmp/dream-c1-load-dispositions-make-land.log; runtime stays unapproved.
capture() also maps inner preparation/current-proof errors to stale_context, masking their real category. Oracle filed [[tasks/2026-11-06-dream-c1-honest-capture-errors]]; no masking fix was made here. Its positive witness is listed with the load exclusions and also needs that follow-up.
The canonical landing retry did not refresh the receipt/checkout after push-race rebases. Final explicit main verification above repairs this task's evidence; I did not change landing tooling.

LOAD-FAMILY EXCLUSIONS (10; assertions and production thresholds unchanged):
Oracle widened rule01M6SME1D97D59Q9G4Y3Z9T8RZ permits only correct-behavior wall-window failures or CPU-frame ResourceLimit exclusions in Dream tests. Re-enable through [[tasks/2026-11-06-qos-builds-never-starve-mcp]] (#93377961366081); capture also requires its error-masking fix. Classification is Oracle's disposition, not an independently proved OS cause.
- `actual_private_process_clears_authority_environment_and_uses_fixed_argv` — crates/bboxy450-harness-wake-up/tests/dream_c1.rs:469.
- `actual_private_prepared_start_is_one_write_and_duplicate_closes_without_replay` — crates/bboxy450-harness-wake-up/tests/dream_c1.rs:522.
- `actual_private_control_invalid_ulids_and_unsafe_revision_emit_no_new_frame` — crates/bboxy450-harness-wake-up/tests/dream_c1.rs:634.
- `actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames` — crates/bboxy450-harness-wake-up/tests/dream_c1.rs:696.
- `eof_and_drop_retain_exact_child_custody_through_actual_reap` — crates/bboxy450-harness-wake-up/tests/dream_c1.rs:744.
- `custody_copies_exact_ir_and_only_typed_blob_carriers_into_private_new_inodes` — crates/bbt-mux/src/logic/dream_c1.rs:7407.
- `dream_c1_source_failure_is_bounded_immutable_and_fenced_before_any_replay` — crates/bbt-mux/src/registry/dream_c1.rs:3735.
- `dream_c1_source_error_capacity_is_atomic_across_real_sqlite_owners` — crates/bbt-mux/src/registry/dream_c1.rs:3802.
- `dream_c1_admitted_mcp_is_exact_own_seat_and_reports_honest_missing_runs` — crates/bbt-mux/tests/dream_c1.rs:243.
- `dream_c1_held_private_delete_catalogue_keeps_mcp_gui_control_owner_independent` — crates/bbt-mux/src/logic.rs:14803.
Retained observations include registry CPU1138250/1066000ns>1ms, private native CPU414958/458000ns with wall47.497125/138.216458ms>25ms, and a correct busy control reply at259091us>250ms. Readiness failures can precede later assertions; omitted assertions are not called passed. Failed logs remain /tmp/dream-c1-joined-target-make-land.log, /tmp/dream-c1-storage-defect-make-land.log and /tmp/dream-c1-parked-make-land.log. The ineffective private-script admission experiment is NOT applied (saved patch/hash in the previous addendum); no quarantine row/task copy was added.

RUNNER-REQUIRED ROUTING (12, unchanged assertions):
These require deliberately pinned DREAM_C1_RUNNER/seat artifacts. Plain source tests skip them; scripts/test-dream-c1.py explicitly passes --run-ignored all for EXACTLY bboxy450-term --test dream_c1, so the canonical runtime gate still selects them. This selector change was not used to consume another ordinary run.
- `dream_c1_real_closed_job_owns_controls_dedup_and_native_frame_overlap` — crates/bboxy450-term/tests/dream_c1.rs:1464.
- `dream_c1_real_completed_status_rejects_valid_json_page_tamper` — crates/bboxy450-term/tests/dream_c1.rs:1799.
- `dream_c1_real_daemon_loss_quarantines_predecessor_without_replay_or_adoption` — crates/bboxy450-term/tests/dream_c1.rs:1909.
- `dream_c1_four_live_real_jobs_keep_independent_controls_and_exact_settlement` — crates/bboxy450-term/tests/dream_c1.rs:2069.
- `dream_c1_actual_source_read_hold_keeps_controls_and_cancel_owner_until_release` — crates/bboxy450-term/tests/dream_c1.rs:2267.
- `dream_c1_real_omit_rest_control_is_rejected_by_measured_schedule_invariant` — crates/bboxy450-term/tests/dream_c1.rs:2359.
- `dream_c1_production_cpu_and_progress_keep_actual_controls_responsive` — crates/bboxy450-term/tests/dream_c1.rs:2401.
- `dream_c1_protocol_fake_progress_flood_keeps_controls_and_cancel_independent` — crates/bboxy450-term/tests/dream_c1.rs:2484.
- `dream_c1_hot_owner_mutant_refuses_snapshot_custody_before_source_effect` — crates/bboxy450-term/tests/dream_c1.rs:2571.
- `dream_c1_real_owned_blob_over_limit_refuses_before_manifest_or_worker_start` — crates/bboxy450-term/tests/dream_c1.rs:2629.
- `dream_c1_real_multimib_store_blob_completes_and_retains_exact_private_input` — crates/bboxy450-term/tests/dream_c1.rs:2715.
- `dream_c1_retained_same_generation_turn_pauses_real_worker_with_legacy_read_held` — crates/bboxy450-term/tests/dream_c1.rs:2782.

The ONE corrected ordinary run remains FAILED:
make test-dream-c1 on8d9cb55f exited2; /tmp/dream-c1-help-one-joined-run.log and /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-3tubkf94/receipt.json.
--help admission PID42152 exit0/actual_reap=true,295519000ns wall. Ordinary source-free Hello PASSED original250ms: PID42175 exit0/reaped/settled, one128B permit, null context, entry bounds4293000..6881000ns, uncertainty2588000ns, entry-to-prewrite50000ns. No Start/source job was performed by this probe.
The next producer Store/export fixture failed before its job/child: approved_runner() required bootstrap-support.txt and hashes.txt, while term supplies its .dream-c1.json layout; command-02.log, producer tests/dream_c1.rs:1218, test exit100. Oracle forbade an artifact bridge or continuation here; neither occurred.
Historical tuple: source2ce4359df034405043d419fe550b0f6cafe7887e; rustc1.99.1-vV SHAe003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5; ordinary release/default acp-agent/no test-fixtures SHA2b85c7114545f35a62842115ae91e08e1a22fbfcd24dfd27e3aa45fa1d3837cb. This is historical evidence, not approval of a worker built from final f6a247b2. Older failed Hello and failed --version claims stay failed and retained; unknown predecessor report operation01M6RYQ9VK9JSRHVTNMYH7PW0P remains unknown/unretried.

Other known limits:
- Full four-job readiness, real-worker memory/RSS,240frames/input/message latency,200-call controls and complete real-worker retirement/foreground pipeline remain UNPROVEN/not reached.
- Cold/warm MCP Status wall1.7532575s then1.458710541/1.490842375s; page3.1525905s then2.800326916/3.027203167s. Exact phase CPU/wall samples and aggregate table remain /tmp/dream-c1-cold-warm-phase-summary.json (SHAe88803869fc0f964b01af16b00ebb668cdb50594ac398261788d765108c8cc62). Warm2 domain+retirement1744.176458ms wall/5990833ns cumulativeCPU; these phase totals include many repaid units. Diagnostic binary hash was not retained, so this is not binary-bound acceptance. Follow-up [[tasks/2026-11-06-dream-c1-cold-mcp-latency]] owns snapshot reads.
- Installed-bundle admission still needs a runtime Application Support record keyed by final path/inode/ctime/SHA. A copied build receipt is provenance only. No bundle sidecar or installation recipe workaround was added (Oracle01M6S5792H2KG3BNGR3J5VADT8).
- Owner-safe cold compiler-output cleanup receipts, failed logs, workers and formal evidence were preserved as recorded in the earlier reports. No peer worktree/target cleanup, reset/stash/drop, unsigned commit, compiler/binary fallback or live install was used.

Full report/checkpoint storage is native private KB, separate from the signed source push; no .bboxy record was force-added to source Git. Oracle owns acceptance, seat closure and worktree removal. This is the final report for the requested source-only landing.
