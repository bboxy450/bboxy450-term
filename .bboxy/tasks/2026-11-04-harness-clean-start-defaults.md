---
acceptance:
- Canonical Codex profile identity is codex, its generated file is codex.md and its ACP executable is codex-acp without duplicate profile generation
- A private empty home with fake installed Codex/ACP tools starts through normal GUI and CLI or seat entrypoints without relying on legacy nemotron or pre-existing profiles
- Explicit profile, environment and saved-default precedence is consistent across frontends and existing edited profiles/default files remain preserved
- A private machine with no agent installed reaches usable GUI setup and headless startup gives an actionable configuration error without vendor calls or implicit package installation
- Named clean-start process and GUI checks plus affected compilation pass without touching real profiles or restarting the shared backend
allowed_paths:
- crates/bboxy450-harness/src/onboarding.rs
- crates/bboxy450-harness/src/provider/profile.rs
- crates/bboxy450-harness/src/frontends/headless.rs
- crates/bboxy450-harness/src/frontends/gui/options.rs
- crates/bboxy450-harness/src/frontends/gui/window.rs
- crates/bboxy450-harness/src/frontends/gui/bridge.rs
- crates/bboxy450-harness/src/lib.rs
- crates/bboxy450-harness/src/main.rs
- crates/bboxy450-harness/tests/**
- crates/bboxy450-harness/verification/**
- crates/bboxy450-harness/scripts/verify-clean-start.py
- crates/bboxy450-harness/Makefile
- crates/bboxy450-harness/ATTRIBUTION.md
- Makefile
- scripts/verify-clean-start.py
- ATTRIBUTION.md
- crates/**/ATTRIBUTION.md
- plugins/**/ATTRIBUTION.md
- .bboxy/tasks/2026-11-04-harness-clean-start-defaults.md
- .bboxy/reports/**
- .bboxy/runs/codex-clean-start.json
- crates/bboxy450-harness/src/backend/acp_agent.rs
artifacts:
- '[[01M6M7JX25B2499PGBV6T38275]]'
assignee: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
authorship:
  kind: seat
  process_generation: '1'
  public_key: qxL/3a47UEqef+6VNGve7zFPxAJ4V2xtFwav0ldZ+l0=
  signer: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
closed:
  by: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
  op_id: 01M85CDV204VCQNM3PMGMHAY41
created: 2026-11-04T21:06:40.639Z
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
id: 01M6M65A1VYVMQNPSTJ46YD5S2
number: 93377961363555
report:
  commit: aa740e8fa4e0ed73b72088571020a7c8ad86ccaa
  op_id: 01M6M7KGZ6HV9X30YJ8RWVA04X
  report: '[[reports/2026-11-04-harness-clean-start-correction.md]]'
  reported_by: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
  summary: 'Implemented shared Codex clean-start bootstrap and consistent explicit/environment/saved/default precedence, retaining codex.md with [codex-acp], edited profiles and saved preferences. Corrected the Oracle''s login-shell PATH regression using the existing bounded ACP resolver, retained original profile diagnostics and verified crashed-seat identity recovery. Signed producer checkpoints fdaaf876 and final 87f0a3e7; signed parent handoff aa740e8f. make verify-clean-start exited 0: seven fresh private process/native-GUI checks, strict standalone lint, embedded compilation and license checks; unchanged GUI options/Bridge evidence reused with source/log hashes and no installer rerun. Exact indexed task/report copies and tested source hashes verified; producer clean, parent gitlink deliberately unstaged. No push/merge/install/backend restart or real profile/vendor calls.'
  verify_revision: 0
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Unify profile identity and make empty-home Codex startup work
type: task
updated: 2026-11-23T23:38:59.000Z
updated_by: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
verify: make verify-clean-start
verify_result:
  command: make verify-clean-start
  evidence:
  - criterion: Canonical Codex profile identity is codex, its generated file is codex.md and its ACP executable is codex-acp without duplicate profile generation
    reference: crates/bboxy450-harness/tests/clean_start.rs:45 and :155 — canonical codex.md/[codex-acp], actual argv, regeneration, preserved bytes and six concurrent create-new publishers.
  - criterion: A private empty home with fake installed Codex/ACP tools starts through normal GUI and CLI or seat entrypoints without relying on legacy nemotron or pre-existing profiles
    reference: crates/bboxy450-harness/tests/clean_start.rs:45, :126 and :181 — private normal CLI, hidden native GUI and seat/socket startup.
  - criterion: Explicit profile, environment and saved-default precedence is consistent across frontends and existing edited profiles/default files remain preserved
    reference: crates/bboxy450-harness/src/onboarding.rs:333; tests/clean_start.rs:233 and :277; src/frontends/gui/window.rs:1161 — precedence, custom login-PATH launch and actual crashed-seat recovery.
  - criterion: A private machine with no agent installed reaches usable GUI setup and headless startup gives an actionable configuration error without vendor calls or implicit package installation
    reference: crates/bboxy450-harness/src/frontends/headless.rs:306 and :312; tests/clean_start.rs:101 and :233 — same bounded ACP resolution and original parse/config errors plus actionable guidance.
  - criterion: Named clean-start process and GUI checks plus affected compilation pass without touching real profiles or restarting the shared backend
    reference: .bboxy/runs/codex-clean-start.json — make verify-clean-start exit 0; seven fresh process checks, strict standalone/embedded compile/license checks; unchanged options/Bridge checks reused with source/log hashes.
  exit_code: 0
worktree: /Users/feral/mydev/bboxy450-term-onboarding
provenance:
  digest: sha256:c906abac03b822076b5fe384a449f3f683405f30819d557c27f77739724b6a62
  frontmatter_digest: sha256:dbc6e5f0959605b431595cc4238040a7abb5ff4afc20f2a10a389895bb09f28a
  signer: seat:01M85C5BC8Y3ETPJDDSZRJN6EB
  sig: 5wHwak4xeZkG/vO+BNM8bpNKVbZVgScqwFafgMPhIUNYehCVTphdXo6fovhazCfvwP2ENF9pBbw8i0m3I4O0AQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1352
  event:
    id: 01M85CDVPXMAW2T7NHEP78505F
    sequence: 1352
    hash: 7bbcdfccef570514985ac728c2c898aff9e7e0800376867926d2dd56c5e2da3c
---
ROLE: EXECUTOR.

Direct operator: trace whether default is codex or codex-acp, and ensure a clean machine works. Oracle verified generated canonical key/file is codex / profiles/codex.md; agent.command is [codex-acp]. Current BUGS: headless.rs:70-82 falls back to nemotron with no saved default, and GUI options.rs:42 independently hard-codes nemotron and ignores saved default/env. Discovery currently runs through GUI Bridge; clean CLI/seat startup can try loading a profile before it exists.

Own warm checkout /Users/feral/mydev/bboxy450-term-onboarding, producer branch codex-harness-onboarding-20261104 at f1d3a3f; Oracle fast-forwarded parent to current main2f4d2f13. Single writing executor. Use new bboxy450-harness profile codex (old codex-acp/exe profile files were deliberately archived). Oracle owns ALL Git fetch/pull/rebase/merge/pins/push/build/install. Do not do them.

Implement the smallest shared default/bootstrap path so normal clean startup works, rather than fixing only this operator's default-profile file. Canonical generated filename remains codex.md and command codex-acp, never create legacy aliases. With a private empty home and fake codex/codex-acp installed, normal GUI plus CLI/seat startup must discover/create the right profile and select codex without nemotron, user edits, login or network. Honor explicit --profile above nonblank BBOXY450_HARNESS_PROFILE above saved default above canonical first-run Codex behavior, consistently across frontends. Preserve existing valid/custom profiles and saved preferences; no silent overwrite or unrelated agent fallback. If needed initialize a missing saved default atomically only when a usable canonical Codex profile exists; never overwrite an existing user choice. A machine without an installed agent must get the existing working welcome/setup UI; non-GUI must fail with actionable setup guidance rather than an unexplained missing nemotron file. No automatic package installs or paid/live vendor calls.

Keep profile discovery/filesystem work off GUI callbacks; reuse the current bounded discovery/create-new machinery, existing launcher/context/MCP/yolo semantics and restored loading priority. Do not redesign welcome, rename profiles to adapter executable names, touch real profiles/credentials, switch running agents, or restart backend. Current real home already has ONLY generated claude/codex/goose/grok/opencode/pi and saved default codex; leave it untouched.

Add make verify-clean-start with only named private temp-home/PATH/fake-adapter process and actual GUI/Bridge checks, plus affected strict compilation/license checks. Cover missing root/profile/default file, no installed CLI, native CLI without wrapper, proper argv/profile identity, env/saved/explicit precedence, preserved edits, repeated startup and concurrent create-new if relevant. Parsing/layout keep specialized checks; reuse unchanged protocol/runtime/formal evidence and add only a minimal affected model/control if you introduce a new ownership contract. No full CI/parity/formal/runtime suites. Two Cargo jobs, local target; disk92%, Codex96% weekly—keep this bounded and deliver promptly.

Commit a signed producer source checkpoint immediately when the focused gate passes; do not wait for report indexing. Parent holds only Make wrapper, exact indexed task/report and compact .bboxy/runs/codex-clean-start.json; leave its gitlink unstaged. Register append-only report reports/2026-11-04-harness-clean-start.md via knowledge tools, copy exact indexed bytes, signed report commit with Report trailer, task_report using acceptance strings VERBATIM. Save raw logs/fixtures outside Git, only command/exits/hashes in run JSON. Unknown write: retain original op ID and query it, never redispatch an unknown effect. Quiet: no plan/progress/ack DMs, only genuine all-work blocker and final task_report. No root or peer worktree edits, broad staging, reset/stash, OS desktop input or test needing the operator.


## Oracle source-review correction — same bounded startup scope

The current fdaaf876 candidate fails a private existing-behavior witness: an explicit valid custom ACP profile whose executable exists only in the login-shell PATH succeeds with installed f1d3a3f (exit 0 and PRIVATE-CLEAN-START), but candidate exits 1 before launch. Exact private instrument/results: /tmp/bboxy-oracle-recovery-01M6KDR7TE4H4B4AF0PZYK0Q1T/clean-start-login-path-review.json and clean-start-login-path-{installed-baseline,candidate}.log. The process owns a private HOME, fake login shell and fake adapter; no vendor or credentials.

Remove the new narrower inherited-PATH rejection or reuse production ACP resolution for any early availability check. backend/acp_agent.rs is now allowed solely to expose/reuse that existing bounded resolver if needed; preserve its launch semantics. Preserve the actual profile parse/config error when adding setup guidance. Add this regression to the existing focused fixture, run only the changed named checks/affected compile, and refresh the compact exact-source receipt. No full suite, new formal model, backend action or installer run. New-session default discovery must not change saved/environment selection during resume; retain existing Host identity handling. This source correction must be complete before final handoff; the old focused green alone is insufficient.


## Report from seat:01M6M69Y5FVMKEHTCVGWKY8EZH

Implemented shared Codex clean-start bootstrap and consistent explicit/environment/saved/default precedence, retaining codex.md with [codex-acp], edited profiles and saved preferences. Corrected the Oracle's login-shell PATH regression using the existing bounded ACP resolver, retained original profile diagnostics and verified crashed-seat identity recovery. Signed producer checkpoints fdaaf876 and final 87f0a3e7; signed parent handoff aa740e8f. make verify-clean-start exited 0: seven fresh private process/native-GUI checks, strict standalone lint, embedded compilation and license checks; unchanged GUI options/Bridge evidence reused with source/log hashes and no installer rerun. Exact indexed task/report copies and tested source hashes verified; producer clean, parent gitlink deliberately unstaged. No push/merge/install/backend restart or real profile/vendor calls.

```json
{
  "op_id": "01M6M7KGZ6HV9X30YJ8RWVA04X",
  "commit": "aa740e8fa4e0ed73b72088571020a7c8ad86ccaa",
  "report": "[[reports/2026-11-04-harness-clean-start-correction.md]]",
  "verify_result": {
    "command": "make verify-clean-start",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Canonical Codex profile identity is codex, its generated file is codex.md and its ACP executable is codex-acp without duplicate profile generation",
        "reference": "crates/bboxy450-harness/tests/clean_start.rs:45 and :155 — canonical codex.md/[codex-acp], actual argv, regeneration, preserved bytes and six concurrent create-new publishers."
      },
      {
        "criterion": "A private empty home with fake installed Codex/ACP tools starts through normal GUI and CLI or seat entrypoints without relying on legacy nemotron or pre-existing profiles",
        "reference": "crates/bboxy450-harness/tests/clean_start.rs:45, :126 and :181 — private normal CLI, hidden native GUI and seat/socket startup."
      },
      {
        "criterion": "Explicit profile, environment and saved-default precedence is consistent across frontends and existing edited profiles/default files remain preserved",
        "reference": "crates/bboxy450-harness/src/onboarding.rs:333; tests/clean_start.rs:233 and :277; src/frontends/gui/window.rs:1161 — precedence, custom login-PATH launch and actual crashed-seat recovery."
      },
      {
        "criterion": "A private machine with no agent installed reaches usable GUI setup and headless startup gives an actionable configuration error without vendor calls or implicit package installation",
        "reference": "crates/bboxy450-harness/src/frontends/headless.rs:306 and :312; tests/clean_start.rs:101 and :233 — same bounded ACP resolution and original parse/config errors plus actionable guidance."
      },
      {
        "criterion": "Named clean-start process and GUI checks plus affected compilation pass without touching real profiles or restarting the shared backend",
        "reference": ".bboxy/runs/codex-clean-start.json — make verify-clean-start exit 0; seven fresh process checks, strict standalone/embedded compile/license checks; unchanged options/Bridge checks reused with source/log hashes."
      }
    ]
  }
}
```
