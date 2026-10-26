# MA-5: superseded, done and duplicate wakes

Signed source: `32a12e189bcd1af7cf9677f280f341bbacd9369e`, branch `velvet-ma5-wake-suppression`.
New cut range: `f206ee1dd76ceb5a2150bfb9bf117593ae5ef962..32a12e189bcd1af7cf9677f280f341bbacd9369e`; its base contains the separately handed-off refusal runtime plus accepted MA-6.
Rulings: #334 §4a at `21c9a802`, ownership clarification `b3d672b2`; fieldless Body marker confirmed by Architect `90ef88ee`.

A message can supersede only one of the same certified principal's retained messages; generations and leaf-key rotation preserve that ownership.
Replacement/withdrawal cancellation is committed with message admission, and chaining retains the old inbox rows and signatures.
An addressed recipient's in-chain answer cancels its pending question; another sender's reply does not. Task delegation waits for authoritative accepted/rejected status.
Task authority comes from the existing Index startup snapshot and status events; prose, reports, replies and literal done/closed are not task acceptance.
Receipt wake:none has the typed reason superseded/done/policy. Inbox rows and original signed content stay retained.
A cancelled digest cannot produce an empty summary, and an adapter's no-effect retry cannot rearm its cancelled member.
Native-written/queued/unknown attempt evidence is retained, never recalled or replayed. A repeated stored message id does not produce another native wake.
An unsigned local operator intent can be withdrawn while custody is unavailable; it gains no fake signer or signing counter.
Verification: **112 passed, 0 failed, 1 existing night-boundary test ignored; strict proto/mux/app all-target Clippy exit 0**.
The prior **53584 golden bytes are identical**, SHA256 `00a0c30deab04d4695ddb6e0690dc718d09558dc8371139888c79f45a9ebe4d5`; five additive rows cover new fields.
No full workspace gate, physical adapter probe, night or publication was run here; Oracle presubmit/landing remains required.

## Mechanism

- `crates/bbt-proto/src/messaging.rs:126` — `pub enum Body`.
- `crates/bbt-proto/src/messaging.rs:683` — `pub enum ReceiptWake`.
- `crates/bbt-proto/src/messaging.rs:696` — `pub enum WakeReason`.
- `crates/bbt-mux/src/messaging/ledger/suppression.rs:46` — `fn cancel(`.
- `crates/bbt-mux/src/messaging/ledger/suppression.rs:87` — `pub(super) fn admitted`.
- `crates/bbt-mux/src/messaging/ledger/suppression.rs:140` — `pub(crate) fn task_state`.
- `crates/bbt-mux/src/messaging/service.rs:236` — `fn supersedes_target`.
- `crates/bbt-mux/src/messaging/service.rs:271` — `pub(crate) fn task_state`.
- `crates/bbt-mux/src/messaging/service.rs:281` — `fn sync_task`.
- `crates/bbt-mux/src/logic.rs:2083` — `for (task, status) in worker.index.task_states()`.
- `crates/bbt-mux/src/logic.rs:3808` — `fn scan(&mut self)`.
- `crates/bbt-mux/src/messaging/wire.rs:106` — `let payload = match`.

## Actual commands and results

Environment: source cache helper; jobs2/incremental0; private memory test custody; dev/test opt-level1, debug0, debug assertions true.
Backend checks pin `7798045e`; final source differs only by the app constructor’s `supersedes: None` default. Backend/proto diff is empty; app checks and strict pin final source.

- `cargo test -p bbt-proto --locked --target-dir target/m2-reply-box-dev --test gui_wire_golden --test messaging --test messaging_pending --test messaging_suppression` — EXIT 0, 0.593s, source `7798045e`. Raw `target/ma5-suppression-proof/verified-contract.log`; SHA256 `8be7b68c3167baf8f9ad07f7016ccc7bc37a8ff0eab542f41f956237ae17d66d`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test messaging_suppression --test messaging_threads --test messaging --test messaging_flow --test identity` — EXIT 0, 10.178s, source `7798045e`. Raw `target/ma5-suppression-proof/verified-core.log`; SHA256 `0f790cfbd73af9567c09d52d179c77586fef2d0f9da218df3345ad36d2060281`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --lib messaging::service::` — EXIT 0, 70.308s, source `7798045e`. Raw `target/ma5-suppression-proof/verified-service.log`; SHA256 `290cb829ef6f2bcbfa6a358e3e7f657e27bc701103c1be44c2044159bc3c77c6`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp mcp_withdrawal_uses_the_signed_sender_and_preserves_its_cancelled_receipt -- --exact` — EXIT 0, 20.648s, source `7798045e`. Raw `target/ma5-suppression-proof/verified-mcp-withdrawal.log`; SHA256 `bc3dd8dcfabf0c591564ffde8f89754da2c59671e780890023e6e99f8b110538`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp indexed_task_acceptance_cancels_the_pending_delegation_with_a_done_reason -- --exact` — EXIT 0, 1.390s, source `7798045e`. Raw `target/ma5-suppression-proof/verified-mcp-task.log`; SHA256 `27c94372da8f7f647a7b606c23d09a32d678338ce31be71848b8b653ec530228`.
- `cargo test -p bbt-mux --locked --target-dir target/m2-reply-box-dev --test mcp unknown_thread_mcp_refusal_keeps_the_visible_hint_and_replays_without_a_send -- --exact` — EXIT 0, 0.825s, source `7798045e`. Raw `target/ma5-suppression-proof/verified-mcp-refusal.log`; SHA256 `05f0483d3bf1e8acc4b34ebfaca3d4153a282ae143f4ded36c77de212a7ec61c`.
- `cargo test -p bboxy450-term --locked --target-dir target/m2-reply-box-dev --test messaging_ui` — EXIT 0, 56.208s, source `32a12e18`. Raw `target/ma5-suppression-proof/verified-ui.log`; SHA256 `643f4b40e2600b6c4a72808ab2862e90fbb0036d7fba703ff659e6d0948e86c2`.
- `cargo clippy -p bbt-proto -p bbt-mux -p bboxy450-term --all-targets --locked --target-dir target/m2-reply-box-dev -- -D warnings` — EXIT 0, 58.529s, source `32a12e18`. Raw `target/ma5-suppression-proof/verified-strict.log`; SHA256 `dd896dd186d2f19751b7eb62a60af8026dd92d43c367b580c855d164e5eae272`.

## Intermediate failures retained

- `target/ma5-suppression-proof/first-check.log` — initial integration compile: admission hook/type/default corrections before source51f27300; SHA256 `163c28202bf82bdbef6edbe740a293efc34dc83d03e10e2aa292a66436e2570c`.
- `target/ma5-suppression-proof/suppression-first.log` — test recipient comparison needed its existing Ulid PartialEq, before source51f27300; SHA256 `4092fb4ae6a0b3670a9e7e513d4ce276d8c3328eacd58313824a1d9e1b983470`.
- `target/ma5-suppression-proof/contract-first.log` — strict withdrawal marker fixed; reason fixture compiled before additive golden rows were written; SHA256 `7dfbdbe6ff5ffc8bb87f12d6ed4bd73e8cbe745b3fe5cdbb0c6445cc3b20d420`.
- `target/ma5-suppression-proof/pre-fixture-fix-51f27300-verified-core.log` — rotation fixture inserted an existing registry row; changed to Registry::update; SHA256 `9fb0f79fc57bb0f1e216ee9899c175618fffeff17a3b7024be03b3d14e8545ad`.
- `target/ma5-suppression-proof/ui-before-default-7798045e.log` — remaining app SendRequest literal needed supersedes:None; backend/proto code unchanged; SHA256 `a844a26a6d146b155547851fe054aada6c3c45bce419eaf2f4a754e66cf243b0`.

## Cache retirement

Owned idle target `/Users/feral/mydev/bboxy450-term-exec6/target/m2-reply-box-dev`: lsof EXIT 1, stdout/stderr empty; retired=True, allocated bytes removed=2063839232. Raw `target/ma5-suppression-proof/retirement-lsof.log`, SHA256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Galleries/proof directories retained.
