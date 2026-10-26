# MA-5: current-main refusal and wake-suppression union

Signed source: `f9cca27a76d9b19231428be3f8fae7e20e874e23`, branch `velvet-ma5-current-union`, rebased onto `8755bd115a9700695dc70ceb07c269c5f8ba24b0`. This combines the frozen refusal runtime and §4a suppression cuts; their original branches and historical proof files remain intact. The already-landed cache-budget commit was skipped.

Result: **134 focused tests passed, 0 failed, 1 existing night-only GUI boundary ignored; proto/mux/app strict all-target Clippy EXIT 0**. Every check below pins this exact source. No full workspace gate, physical adapter probe or unattended night is claimed; the Oracle performs landing/presubmit.

## Union mechanisms

- `crates/bbt-mux/src/messaging/ledger.rs:142` — retain the MA-8 recipient digest index and add both MA-5 visible-thread-tip indexes; retain root-cache migration and atomic cancellation hooks.
- `crates/bbt-mux/src/messaging/ledger.rs:913` — preserve MA-7 admission-time wake members, with the mechanical `ReceiptWake::None { .. }` match for the optional reason DTO.
- `crates/bbt-mux/tests/messaging_suppression.rs:241` — cancellation clears the public MA-8 deferred count/deadline before and after reopen; original no-empty-summary checks remain.
- `crates/bbt-mux/tests/messaging_flow.rs` — six digest fixtures explicitly choose Digest, independent of the separate MA-2 direct-wake correction.
- `crates/bbt-mux/tests/mcp.rs` — keep accepted attention ownership/Orient tests together with typed thread refusal, signed withdrawal and authoritative task-cancellation checks.
- MA-7 `messaging/wake_cost.rs`, MA-8 `deferred_view`/`attention_view` and `Ledger::digest_summary` compare byte-identical to the pinned main base; `preservation.json` records the hashes. The source includes main's frontend-flock fix `8755bd11`.
- Wire golden: all **193 prior rows / 60705 bytes** retained exactly, SHA256 `9a03d46b42fe296a039c28e34ad1742e73e849d5a41a5871b5c0a2920e52c398`; five suppression cases appended. Contract roundtrips pass.

Environment: source `scripts/cache-env.sh`; jobs 2, incremental 0, debug memory test custody, dev/test OPT_LEVEL=1, DEBUG=0, DEBUG_ASSERTIONS=true. Cargo target is supplied by `--target-dir`, never `CARGO_TARGET_DIR`.

| Exact command | Exit | Seconds | Raw log SHA256 |
|---|---:|---:|---|
| `cargo clippy -p bbt-proto -p bbt-mux -p bboxy450-term --all-targets --locked --target-dir target/m2-reply-box-dev -- -D warnings` | 0 | 58.205 | `d4a888de3dc9375f4946a2c7fef573c2e8cec5ef56a83caff8e0f94edf459637` |
| `cargo test -p bbt-proto --locked --target-dir target/m2-reply-box-dev --test gui_wire_golden --test messaging --test messaging_pending --test messaging_suppression` | 0 | 64.658 | `2470f6f1e7446f0445f236cf3aad481f2f283cf5f808b0adac8daefe1d70edf8` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test attention_policy --test messaging_suppression --test messaging_threads --test messaging --test messaging_flow --test identity` | 0 | 40.02 | `85f569afbc34535219c94adc717be8320821b14f8f66344afeebbaf508092f1a` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --lib messaging::` | 0 | 76.411 | `f143b037b4c425eef1c32028df6dc461c6f3784152000f82a1e119936363b223` |
| `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp -- --exact unknown_thread_mcp_refusal_keeps_the_visible_hint_and_replays_without_a_send mcp_withdrawal_uses_the_signed_sender_and_preserves_its_cancelled_receipt indexed_task_acceptance_cancels_the_pending_delegation_with_a_done_reason mcp_attention_is_owned_by_the_admitted_receiver_and_never_grants_other_seats mcp_attention_replay_survives_restart_without_rolling_back_a_later_policy mcp_orient_reports_role_defaults_overrides_and_always_wake_exceptions` | 0 | 13.789 | `e1be61d34932c9243a1f6dab9f878642e281677bb8335e32f4f478cef9c99204` |
| `cargo test -p bboxy450-term --locked --target-dir target/m2-reply-box-dev --test ctl ctl_unknown_thread_keeps_the_typed_hint_and_exits_one -- --exact` | 0 | 38.385 | `88c6307779f85ec4d770d7d5168d1a883fbd4ec87db9d3bebfef767cc436a93e` |
| `cargo test -p bboxy450-term --locked --target-dir target/m2-reply-box-dev --test messaging_ui` | 0 | 4.261 | `67c0145d4150b303bf692c18e467c630a7163d063f9c5482a8c18ade1930010e` |

Raw logs, step manifests, source/environment, preservation hashes and signature checks remain in `target/ma5-current-union-proof/f9cca27a76d9/`. The earlier `8fadbec50edd/` check set also passed 134/0 before main's frontend-lock fix landed; it is retained and is not substituted for the final-source checks above.

Owned inactive target retirement: removed=True; lsof EXIT 1, stdout/stderr empty; allocated bytes removed=2,120,007,680. `retirement.json` and `retirement-lsof.log` retained. Existing proof binaries and galleries were untouched.
