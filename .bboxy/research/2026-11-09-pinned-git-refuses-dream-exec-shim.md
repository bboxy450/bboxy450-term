---
type: research
title: Pinned Git's existing hardcoded child guard rejects dream-exec-shim
derived_from:
- '[[tasks/2026-11-09-dream-exec-shim-enforcement]]'
- '[[decisions/d93377961366459-dream-external-helper-shim-enforcement]]'
id: 01M70GT8CSS91NVQ3TVDFSQQX8
updated: 2026-11-09T16:03:46.113Z
created: 2026-11-09T16:03:46.113Z
number: 93377961366466
created_by: seat:01M70FJGXV7W40W80200Z0TDN5
updated_by: seat:01M70FJGXV7W40W80200Z0TDN5
authorship:
  signer: seat:01M70FJGXV7W40W80200Z0TDN5
  kind: seat
  public_key: ZqTue5OR8Q7BNcheRMErqTQtqa+AL9dxIChIyiTZDGM=
  process_generation: '1'
provenance:
  digest: sha256:e9cf31424a40f1e1ad6417a15697e52087500f280f57f298bcc37a5e9b747b08
  frontmatter_digest: sha256:16389573f52902e714a8c56a5479a2541ff02af984234589fdcd3ff667ad5dab
  signer: seat:01M70FJGXV7W40W80200Z0TDN5
  sig: 2M8WxCtFiCDcoMKSRB7BVnGbmuRn6CPoojYCrEL2qPwf6OhgwbtlsD1nqQEF6l2zzpTyU0K0Dfilc8pw5PNMAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151044
  event:
    id: 01M70GT90XCQ23E5Y5V7NH902Z
    sequence: 151044
    hash: 28e400e65dc182f12e10c465bc143c0e17789a5d083d893d6c25948b047b8235
---
The next real-process witness cannot reach the shim through the current pinned Git: git2.54.0 exits128 with `fatal: Dream: unsupported process creation refused before effect` when gpg.program and gpg.openpgp.program name the new shim. This is a failed provisioned-tool assumption, not an argv relaxation request.

Evidence: /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/pinned-git-shim-refusal.json. The exact fixed-form argv, cleared allowlisted environment, original public presigned commit, stderr, status and tool/patch hashes are retained. The Python verifier of the expected refusal exited0. No builder or qualification workflow was run.

The pinned executable is /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M7035811Q2HTZ37K75WQ5K3C/bin/git, SHA256 b8a2dac0c19cb4b25df8497d7be0266ace9e53a991cc46346158b516cf3115cf, observed `git version 2.54.0`. The accompanying GPG reports2.5.24.

The bound existing patch tests/fixtures/dream-proof-tools/git-2.54.0.patch:16 requires `strcmp(cmd->args.v[0], DREAM_PROOF_GPG)==0` before any child creation. Its SHA256 a48b33776ae5a88308e096ea62640487706e360c374ec7eaf64d50adf47818b8 matches the pinned candidate's provenance blob. The same patch guards one permit, one attempt and exactly six arguments. Configuring the shim as the requested gpg.program deliberately fails its absolute real-GPG path check.

Required Oracle disposition: authorize the proof-tools builder to update this exact pinned child target to the shim and rebuild while preserving the remaining restrictions, then provide that frozen tool for the required real-process witnesses; or amend this Executor task's allowed paths and no-builder-run boundary to cover that minimal tool-patch/build work. The current allowed paths omit tests/fixtures/dream-proof-tools/ and scripts/dream-proof-tools.py. An alternate assurance/handoff order requires explicit task amendment. No bypass or external-tool mutation was attempted.

Partial implementation remains uncommitted in the assigned worktree: new C shim/header/process driver, controller receipt/SCM_RIGHTS custody path, preparation hash binding, best-effort live CPU path and HelperShimBudget replacement. Strict shim/controller/driver compilation exited0; the focused finite-model baseline and all nineteen gate controls passed (actual test process exit0). An intermediate focused Rust run had16 pass/2 fail (obsolete settlement/gap expectations); revisions are incomplete. No full verification, make land, INPUT70 or task completion is claimed. runtime_approved=false.