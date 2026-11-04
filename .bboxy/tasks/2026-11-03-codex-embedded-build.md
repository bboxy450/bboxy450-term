---
type: task
title: Gate the ACP relay correctly in embedded-only harness consumers
status: accepted
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-land
allowed_paths:
- src/mcp.rs
- tests/gui_dependencies.py
- tests/gui_gate.py
- scripts/test.py
- Makefile
- verification/**
- .bboxy/reports/**
- .bboxy/runs/codex-embedded-build.json
verify: make check-embedded-build
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6J13RYDS0C43JRNWR7BFN9H
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T02:22:32.040Z
created: 2026-11-04T00:59:58.515Z
number: 19662
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
verify_revision: 1
assignee: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
verify_result:
  command: make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Embedded-only strict lint regression, default and ACP/standalone compile configurations
    reference: reports/2026-11-04-codex-embedded-build.md; /tmp/codex-embedded-build-red.log exit2 -> /tmp/codex-embedded-build-green.log exit0; .bboxy/runs/codex-embedded-build.json
  - criterion: Minimal feature boundary with no runtime/IR/permission changes; preserve prior evidence
    reference: 035e5b29e4ba02cbd2160fc48ad3ad3600d8c6e7 src/mcp.rs:61 Makefile:32 scripts/test.py:38; models/relay implementation unchanged from a231295; prior evidence explicitly not fresh
report:
  summary: 'Signed cfg implementation035e5b29 and clean report HEAD626fbcc select mcp::relay only with acp-agent; canonical check-embedded-build now covers embedded-only strict clippy, default lib and ACP/standalone lib+binary. Actual baseline make2/cargo101 (five dead-code errors) becomes clean committed-source make0; exact logs/hashes and unchanged input inventory retained in the indexed report #19676 and run JSON. Full/runtime/formal suites were intentionally omitted under the current operator speed amendment; prior119-model/39-contract/context/MCP evidence is labeled prior-source. Term2d164 remains frozen/preserved, awaiting Oracle publication of this successor. Original report write timed out but its indexed signed bytes subsequently appeared and are committed; no unknown write was redispatched. No publish/install or permission widening.'
  commit: 626fbcc59b1c17952f85f054c258e9bc39dd283f
  reported_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  op_id: 01M6J5NTA30NCR6NNZB9CBHMEC
  verify_revision: 1
  report: '[[reports/2026-11-04-codex-embedded-build.md]]'
provenance:
  digest: sha256:144c6a2f837a8012a66483638923f152ac35022def75f2eb3212c93abfd282bc
  frontmatter_digest: sha256:eea8658d394eb1f165f9af44aee0e69c40606ba05d1c72964d07520e919ecceb
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: FCtbBG0wRArzDgkZ2GNVnCCK0qmhzEccnpx6TNzwrpO5A0r2vorYU+ipjLwFrfxPTLqWcrQlmrr0eaPJrwCsCg==
  log: .bboxy/log/2026-11-04.jsonl#110012
---
ROLE: EXECUTOR. OPERATOR SPEED OVERRIDE: only needed tests, no entire suites. This current brief REPLACES the earlier full make test/full-model requirement below. Work in /Users/feral/mydev/bboxy450-harness-land, branch codex-embedded-cfg-20261103, published basea231295. This assignment goes to the existing rich-host seat01M6HCYVGE7CJ8KK6PB20SPF3D serially: preserve/freeze clean term2d164 first; write only harness-land until this tiny cfg task is reported. Oracle then publishes it and returns you to the amended term task. No new worker/spawn required, no concurrent writing worktrees.

Apply the minimal acp-agent cfg guard to mcp::relay; no runtime/IR/MCP/permission behavior change and no allow/expect dead_code or wider consumer features. Add Make target check-embedded-build that performs strict affected compile configuration checks: embedded-only no-default-features lib clippy -Dwarnings, default ACP-enabled lib compile, and ACP/standalone-enabled compile. Capture the existing embedded-only failure and fixed green; this is the needed regression. Do not run make test, all unit tests, full GUI tests, full Java/TLC/model corpus or another already-proven process proof. Reuse the prior119-model/39-contract/context/MCP evidence, explicitly recording that it is prior-source evidence and no fresh whole-suite claim. No new state model is needed for a compile-time module guard. If compilation exposes an actual additional error, fix only the narrow scoped cause.

Verify make check-embedded-build exit0 on clean signed source, retain exact command exits/hash log summary, and create one append-only indexed focused report/task_report with operator-authorized skipped full gates made explicit. Oracle will inspect/publish the signed fix. Do not install, publish, merge, recruit or touch credentials/global configs. Keep the earlier source/evidence scope and preserve warm target/cache. Original unknown spawn remains untouched/unassigned if it ever appears; do not repeat it. Return to term work only after the Oracle publishes the fixed pin.

Original brief below supplies source diagnosis/scope context; its full-suite requirements are superseded.

ROLE: EXECUTOR. Narrow prerequisite in /Users/feral/mydev/bboxy450-harness-land, branch codex-embedded-cfg-20261103, published basea2312959f4ddd4519db13f2194c167705056e532. Read project instructions, Make help and source. Reuse its warm target, two Cargo jobs. Exact scope/staging and approved Boxy signed commits. No push/merge/publication/trusted install/recruitment/credentials/global configuration/desktop input or changes in another worktree.

Term host task01M6HCYDWBAX2FDYGMSPB12HRN is blocked at check-rich-host because src/mcp.rs:61 unconditionally compiles pub(crate) mod relay although src/backend/acp_agent.rs is feature-gated. Actual --no-default-features --features embedded consumer clippy -D warnings reports Relay/start/remove/serve/line_value/next_line dead code. Oracle read /tmp/codex-rich-7c-lint-final.log and confirmed the source. Default/all-features harness make test was green but did not cover the embedded-only compile configuration.

Fix the feature boundary minimally (relay belongs to acp-agent), without blanket allow/expect dead_code, disabling warnings, widening term's features to hide it, or changing runtime/context/MCP/IR behavior. Add an unattended canonical regression for no-default-features embedded-only clippy, preserving default and ACP-agent/full GUI configurations and existing gates. Prefer the existing Make/GUI dependency gate rather than a new framework. Demonstrate the existing source fails the named embedded-only check and the cfg correction passes; avoid repeating expensive unchanged gates except the one required final canonical run. No live vendor/provider/inference requests.

Acceptance: embedded-only consumer clippy -D warnings exits0 with ACP implementation excluded; default and ACP/standalone builds retain actual launch-context/MCP behavior and all119 model variants/39 contracts. Canonical RUSTC_WRAPPER= make test exits0 on committed source, current-source native/TLC receipt/check-formal-evidence0; source/model/tool hashes retained. This is compile feature selection, so no new lifecycle semantics/model is required unless you change those semantics (do not). Do not weaken any gate. One full machine gate at a time; host proceeds on independent corpus imports. Raw logs outsideGit; compact command/exit/hash evidence only in the scoped run JSON. One append-only indexed report through knowledge tools, committed with Report trailer, then task_report with signed SHA/real exits/remaining limits. Oracle verifies and publishes the successor for host; send only a true blocker or final report. No source/test scope beyond the six cfg/build gate paths without an amendment.

## Report from seat:01M6HCYVGE7CJ8KK6PB20SPF3D

Signed cfg implementation035e5b29 and clean report HEAD626fbcc select mcp::relay only with acp-agent; canonical check-embedded-build now covers embedded-only strict clippy, default lib and ACP/standalone lib+binary. Actual baseline make2/cargo101 (five dead-code errors) becomes clean committed-source make0; exact logs/hashes and unchanged input inventory retained in the indexed report #19676 and run JSON. Full/runtime/formal suites were intentionally omitted under the current operator speed amendment; prior119-model/39-contract/context/MCP evidence is labeled prior-source. Term2d164 remains frozen/preserved, awaiting Oracle publication of this successor. Original report write timed out but its indexed signed bytes subsequently appeared and are committed; no unknown write was redispatched. No publish/install or permission widening.

```json
{
  "op_id": "01M6J5NTA30NCR6NNZB9CBHMEC",
  "commit": "626fbcc59b1c17952f85f054c258e9bc39dd283f",
  "report": "[[reports/2026-11-04-codex-embedded-build.md]]",
  "verify_result": {
    "command": "make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Embedded-only strict lint regression, default and ACP/standalone compile configurations",
        "reference": "reports/2026-11-04-codex-embedded-build.md; /tmp/codex-embedded-build-red.log exit2 -> /tmp/codex-embedded-build-green.log exit0; .bboxy/runs/codex-embedded-build.json"
      },
      {
        "criterion": "Minimal feature boundary with no runtime/IR/permission changes; preserve prior evidence",
        "reference": "035e5b29e4ba02cbd2160fc48ad3ad3600d8c6e7 src/mcp.rs:61 Makefile:32 scripts/test.py:38; models/relay implementation unchanged from a231295; prior evidence explicitly not fresh"
      }
    ]
  }
}
```
