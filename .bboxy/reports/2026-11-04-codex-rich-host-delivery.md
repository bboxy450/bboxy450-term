---
type: report
title: Focused integrated rich harness delivery and GUI activation
task: '[[01M6HCYDWBAX2FDYGMSPB12HRN]]'
supersedes: '[[reports/2026-11-03-codex-rich-host-budgets]]'
commits:
- a531338d930f6bed73747047e06c56883b5ce5ea
- 2d1642d320397c16a9bcbf1cb7c8629fb093ca1c
verify:
  command: make verify-rich-delivery
  exit_code: 0
author: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
created: 2026-11-04T02:41:53.106Z
id: 01M6J6X5Z0R18GKDPKA77H5NS6
updated: 2026-11-04T02:41:53.106Z
number: 19886
created_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
updated_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
authorship:
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  kind: seat
  public_key: YKv54MXoFa4rl83lF7g/hJHRLfs8fsSGu2+nb6F1XqY=
  process_generation: '1'
provenance:
  digest: sha256:9027438badf6a140e02973b8ccbf57be0372d254d79f0ff11b665b272477a62f
  frontmatter_digest: sha256:2369ffaf2e0b4a5f5ed05c9652d018e82554838529b3a18a5171225b5793fc55
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  sig: qPHPmGYtpfrnqQmbYqwOdznjU7epGDgV5xJAibyjLMXHKglau1DFmmHvjYP0Pwbz+1I64LCAUQ2uGWUx19U+AA==
  log: .bboxy/log/2026-11-04.jsonl#112109
---
Supersedes [[reports/2026-11-03-codex-rich-host-budgets]] (#18874). Historical [[reports/2026-11-03-codex-rich-host]] (#17675) and [[reports/2026-11-03-codex-rich-host-cancel-fence]] (#18711), their signed commits, raw logs and formal archives remain preserved.

Final application source: a531338d930f6bed73747047e06c56883b5ce5ea, branch codex-rich-pane-20261103 in /Users/feral/mydev/bboxy450-term-codex-rich. The original signed2d1642d320397c16a9bcbf1cb7c8629fb093ca1c is retained at preserve/rich-host-2d1642d3. Rebased onto published term main f01428b8867ddfeee33105022710ffb92920b101, retaining both upstream and rich behavior/models. All18 resulting branch commits have signature verdict G; the compact run JSON records their exact SHAs. This final report/run-only commit follows the tested application source; its SHA is supplied by task_report.

Harness gitlink: **5c288d0bcbdaebc7249740220158829e87c85e5a**, published by Oracle at https://github.com/bboxy450/bboxy450-harness, ref oracle/embedded-cfg-20261104. Canonical make bump-harness REV=5c288d0bcbdaebc7249740220158829e87c85e5a exited0 (/tmp/codex-rich-final-cfg-pin.log). This published cut contains the accepted minimal acp-agent relay guard, prior socket/recovery/IR1.2/Quiet/context/MCP work and signed report19676. There is no private dependency override, widened consumer feature or warning suppression.

The upstream d32803e2c51f0aec03c4044c9020d77118e2ee25 correction was reused verbatim: startup-modal report, CORRECTIONS.toml and decision16393 match that published commit byte for byte. No other report was rewritten. Exact authenticated own task01M6HCYDWBAX2FDYGMSPB12HRN and parent01M6HCH1SJHDSB6SDAC2ZF1BXX were fetched by ID and copied with IDs/numbers/provenance intact. Their etags/locators/file hashes are in .bboxy/runs/codex-rich-host.json. The malformed duplicate-locator coordination tasks were excluded. Rebase conflict resolutions retained upstream startup input ownership before rich dispatch and the union of existing model/contract mappings; static inventories confirm both prior sets remain present. Shared rich actor/view, GUI handoff source and four own formal source files remain byte-identical to2d164; hashes are recorded.

**Fresh focused verification**, on clean signed application source a531338d:

| Command in make verify-rich-delivery | Actual result |
|---|---|
| make check-rich-host | 0; strict affected all-target clippy, including the actual embedded-only harness consumer |
| cargo nextest run -p bboxy450-doc --test corpus, selecting only real_bboxy_frontmatter_parses and real_corpus_links_numeric_handles_and_legacy_ledger_anchors_resolve | 0; both pass,13 unselected cases |
| make prepare-rich-native-fixture | 0; current native fixture built |
| make test-rich-gui-handoff | 0; one real App handoff/rollback case,76.37s |
| make test-rich-app-detach | 0; one real App unexpected-detach/replay case,37.65s |
| make test-rich-launch-context | 0; one private launch/MCP case,7.11s |

Canonical **make verify-rich-delivery exited0**. Raw combined log: /tmp/codex-rich-verify-delivery-final.log; SHA256 **40859d172cf878d046e15334589f81a030adfd6331942540ceda050ae18ec9ef**. Two Cargo jobs, worktree-local target, private test identities/repositories/sockets, no operator input. Upstream changed GUI/backend ownership and launch inputs, so these three existing runtime cases were necessary. The Make target adds only these focused checks and retains enforced warnings/errors.

A preliminary clean17871c3d attempt exited2: its premature future Report trailer did not resolve in that committed tree. The unearned trailer was removed by signed amend before reporting an actual result; the failed source/ref/log/hash remain retained at preserve/rich-host-17871c3d and in the run JSON. Parser/tests were unchanged. The final canonical rerun passed. The earlier full make test-parity failure remains historical evidence; it is not relabeled green.

Fresh process evidence: GUI source/instance/PID and installed source all attest a531338d. GUI replacement retained supervisor54491, harnessPID54512, generation1 and session01M6J6JZHHEWTNPT0X9187A083. The explicit stored-version migration replaced logic54492→57557 while retaining supervisor/seat ownership. Injected post-detach activation failure exited1 as intended; rollback restored prior Contents inode and a ready GUI, and the enclosing test passed. Unexpected GUI SIGKILL targeted only private PID60990; supervisor61065/logic61067/harness61087 and session01M6J6NB5WQQ3ZJHB9KQEDCG40 survived. Both cases resumed cursor14→19 and assert durable1..19 with no gap/duplicate, no Close and an alive owned process. The schema fixture downgrades stored version metadata; it does not execute a historical release binary.

Launch evidence: real private Codex and Claude compatibility children call their own admitted MCP at generations1,2 with quoted names, spaced cwd, task/role/workflow and resume identity preserved. The actual managed native harness completes real scoped MCP under deny-default profile using production --yolo; its wrapper adds no approval flag. Three loopback model requests use private-context-model; there is no vendor traffic. These are process/API compatibility witnesses, not commercial vendor or ACP system-role conformance.

ICD mechanisms and retained coverage:

| Clause | Mechanism and evidence disposition |
|---|---|
| §2 socket/identity | wake-up/src/launch.rs:520 chooses launch socket; wake-up/src/rich.rs owns bounded asynchronous connection/Hello/session/epoch/head and original-generation lifetime. Prior socket/process cases cover <=96-byte paths, permissions, fork/reconnect and real interrupted/resumed seats. |
| §3 shared face | rich_pane.rs:106 hosts Bridge/HarnessView, rich_pane.rs:320 adapts its Frame to the existing renderer; rich_faces.rs:74 creates only visible actors and retains default announced rich face, explicit persisted Terminal choice and replay cursor. Fresh App cases show harness_rich, ready=true and actual paints. |
| §4 routing/fences | rich_pane.rs:76 requires captured session/epoch/generation, ready and nonretired identity before remote input, including Cancel. rich_faces.rs:238/261 supplies native key/IME routing. Prior real stale-A/new-B and A→B→A cases emit zero ResolveApproval; current identity denies exactly once. Prior source mutation produces named red101 and exact restored0. Composer/stop/approval and zero PTY byte assertions remain preserved. |
| §5 delivery/status | mux/src/rich.rs:30 selects idempotent socket delivery before PTY arbitration; Unknown is not retried through PTY. wake-up/src/rich.rs:280 uses the actual ephemeral:false subscription. Prior actual consumer remains live beyond21s with unchanged semantic activity; Ack/duplicate/Busy/Closed/unknown, original operation identity and zero PTY cases remain preserved. Recovered harness_log.rs:178 bounds off-thread log reads; :232 retains original ts_ms. Current-generation Reported dominates cadence; EOF/Hello do not fabricate activity, only an open turn becomes stale after60s, and close clears errors. |
| §6 lifecycle | Fresh real App detach/handoff/rollback and launch generation/resume checks above; GUI-only public owner probe and lifecycle action in cockpit/gui_handoff.rs:97/128 and scripts/reopen-gui.py. No backend stop is used for ordinary GUI replacement. |
| §7 budgets | rich_faces.rs:150 retains aggregate300us drain; composition allocation remains600us. Projection/layout/Markdown/IO stay on workers; bounded/coalesced work and hidden/occluded actors0 remain in the preserved source and prior scheduling witness. No allocation was raised. |
| §8 failure | Missing socket retains honest starting/unavailable state and Terminal toggle; prior client/refusal, reconnect and no-PTY-fallback controls are retained. Fresh activation failure restores the prior installed fixture and ready GUI. |
| §9 verification | Prior component controls and native/TLC evidence are retained with source/tool/model hashes; their broader results are explicitly prior-source. This final acceptance command is the operator-authorized focused target above. |

Prior-source measurements are retained, not rerun: native standalone/shared-pane fixture row mismatch **0.0** (required <=0.01). Actual four-view App p99 drain **0.041792ms**, composition **0.083792ms**, joint rich UI slice **0.120167ms**. Concurrent off-thread projection elapsed sum p99 **1.303208ms**; the earlier4.018416ms sum remains in #18874. These sums measure elapsed worker latency, without establishing CPU use or a whole-frame budget verdict. Whole-frame120Hz/4ms remains **UNPROVEN**; loaded/background CPU and heap are **UNMEASURED**;32MiB/view is an unmeasured allocation. No fresh performance claim is made for the rebased cut.

Reused logs and their SHA256 hashes are enumerated in the run JSON: prior native, boundaries, status cadence red101→restored0, licenses/toolkit/layering and formal checks all recorded exit0 on their stated old inputs. The prior clean2d164 receipt contains316 models/127 contracts/34 crates with native/Java graph/verdict parity, archive SHA256 **b19641f1c5dbed75b14eb7f9eda21c7d31d9dd28ccbd04ac35c962170dee31d4**. Current model/tool hashes and static preservation audit are recorded; this is **not a source-current formal receipt for a531338d**, and finite model parity is not arbitrary Rust correctness.

The authenticated speed amendment43af34434f82824a804505277c4016314080444080d2b009fd7f2c62ea46b267 expressly omits full make test/test-parity, whole runtime/visual suites and the full native/Java TLC/model corpus. No such suite was rerun or claimed fresh. Causal delivery/Cancel/status/launch/GUI controls and earlier actual input/IME checks are retained as historical component evidence. Optional presentation work and baseline adoption remain deferred.

Oracle deployment handoff: run the existing canonical **make install-reopen GUI_REPOSITORY=/Users/feral/mydev/bboxy450-term** from the approved candidate checkout after independent verification. Makefile's install-reopen builds/verifies the trusted bundle and uses the same tested installer/GUI-only handoff; it checks installed revision, fresh rendered GUI source/instance/PID and exact public owner identity, preserving previous Contents for rollback. The private equivalent install-unsigned-reopen was exercised above. This Executor performed no trusted installation, push, merge or publication. Oracle must still observe the actual installed/running operator GUI outcome; local product delivery and any later broader publication gate remain Oracle responsibilities.