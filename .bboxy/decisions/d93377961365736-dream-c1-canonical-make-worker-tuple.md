---
type: decision
title: Use the exact canonical Make-built C1 worker and record its build closure
links:
- to: '[[decisions/d93377961365724-dream-c1-term-1991-producer-profile]]'
  rel: supersedes
- to: '[[tasks/2026-11-05-dream-c1-consumer]]'
  rel: implementation
- to: '[[interfaces/dream-term-c1]]'
  rel: contract
id: 01M6PZQ2RN63C3K9AAFRT6C411
updated: 2026-11-05T23:11:46.408Z
created: 2026-11-05T23:11:46.408Z
number: 93377961365736
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:1de996ad9700ee335e8b652d169018431704151d7fe2149434c250e7f870cc03
  frontmatter_digest: sha256:96becdda5e2654f9a04f76752a537dd8b4866e35ef450469518872fce6e32afe
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: mMiyX2tZnEbrhIGZMcGIjpLxXqjTMxJHS8bjkcbvC9eGRvXV4g7GPZCH3d+wpircMINDp6CnjG6MwHPfEPBXAA==
  log: .bboxy/log/2026-11-05.jsonl#146313
---
Date2026-11-05. Supersedes65724's primary/canonical selection of1b577. It corrects a workflow-premise error, without changing runtime law, wire, budget or final acceptance gates.

## Evidence and correction
1b577d80759bec976c2c780d0ba24233a29d4f83d6777a69124ca9e8ffeb15c0 was built by direct worker-only script invocation. Root incorrectly described that as the canonical Make/cache workflow in65724. That candidate remains retained historical inspected bytes, not the primary canonical selection; do not keep it as an automatic fallback.

Actual make prepare-dream-c1 produced different ordinary bytes12ba4006b694bdce3c44d199de460d09f0f2414fafb7fd17880921a7d397a3b4 and refused the old approval BEFORE sidecar/probe/execution. Receipt /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-l8jrpmwj/receipt.json, actual command-00.log cargo build exit0/46.664s; overall preparation is incomplete/refused, not green. Both use d06 source and rustc1.99.1/e003fd3d..., normalrelease/defaultacp-agent/NOtest-fixtures, same Cargo.lock and no conflicting flags. The Make receipt records four standard dev/test defaults; Make sources scripts/cache-env.sh, which sets wrapper/build-dir/cache variables. Different bytes demonstrate that direct and Make contexts cannot be called equivalent; root has NOT established which input caused the byte difference or reconstructed unrecorded parent environment.

Root read Make exports and actual prepare/test/worker targets, cache-env source, both receipts/logs and Cargo fingerprints (default/acp-agent, rustflags[], release profile). Independently rehashed actual12ba, sampled current rustc-vV and matched exact bytes/hash, and rechecked102 harness-source/lock/config/cache/toolchain inputs against the receipt. Harness source is clean atd06. Current Make acquired a canonical worker-only target after the recorded build (recorded e01f618f..., current d8d1f959...); the original prepare/cache recipe is unchanged. Initial whole-current-Make hash assertion exited1 on this active source race, not on source/binary/protocol failure; root read the exact additive diff before continuing. No claim that today's whole Make equals the earlier snapshot.

Frozen copy/receipt/log/compiler and explicit post-build input snapshots: /Users/feral/mydev/bboxy450-term/target/dream-c1-term-make-1991-z6rqolzf. Exact candidate26,276,128bytes; root did not execute it. No test-fixture bypass marker in ordinary bytes is supplemental, not a general binary theorem.

## Exact primary candidate authorized before birth
- Source/gitlink: reviewed/landed d06e696f99472ed754e84353baf11a2974589d6d.
- Compiler: existing pinned rustc1.99.1, commit58cf8f9c35b355199727627d07ae34a4e2846d67, aarch64-apple-darwin/LLVM23.1.1; exact rustc-vV SHAe003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5.
- Ordinary canonical Make worker SHA:12ba4006b694bdce3c44d199de460d09f0f2414fafb7fd17880921a7d397a3b4.
- Release/defaultacp-agent, explicitfeatures[], NOtest-fixtures, existing Make/cache closure; no compiler/profile/feature override or toolchain change.
- Existing dream-bootstrap/1 and c1-native-debt/1 profiles, fixed source bootstrap14b3155d23b47e1ac45b9d023bbea00cab730c3043fbf954dc95402d9ae63b05, c1 63b703f3ddaf693f23f3598c6d121945d1333c098af773db1d53feb0fd97d471, vectorsde919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307.

Within existing scoped Make/script/provision source, replace primary tuple data with EXACTd06/e003fd.../12ba... and bind the existing bounded sidecar; proceed through canonical Make preparation/source-free gated probe/integration. No unknown binary ungated discovery or arbitrary version/hash equivalence. No need to reproduce1b577 by bypassing Make, and no third identical build just to chase a previous hash. The newly added canonical worker-only Make target is within existing allowed paths and may prepare bytes under the same closure; its result still must match before birth.

## Build evidence and runtime limits
Before next canonical invocation's candidate admission, capture a bounded safe build-input record in existing private receipt: actual Make/script entry and cargo argv; resolved compiler/cargo/wrapper path+version/hash where used; known CARGO_PROFILE_ defaults, RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS presence/values if allowed, CARGO_INCREMENTAL, build/target directory and selected cache mode/wrapper disposition; Cargo config/lock/source hashes and actual feature/fingerprint/profile. Use an explicit allowlist, never a whole environment dump or capability/credential contents. Keep generated evidence private/off owner and out of Git. Do not claim cache/profile inputs unrelated unless actually demonstrated. This adds receipt evidence, not public worker fields or a new provisioning service.

If a later apparently identical canonical build gives another hash, refuse before birth and compare actual build inputs/metadata first; do not silently bless by source/version, retry toward a lucky hash, or retag old measurements. A deliberately changed reviewed build needs an explicit source/compiler/artifact/closure disposition. Runtime source-free probe remains same approved pipe/paid-debt/explicitnullcontext/permanentALLStart rejection; real work still needs full current closed-source authority. Preserve all250ms/30s/native/app/control/cleanup/debt/heap/foreground/control/frame/hints/ABA/source/reap/four-job gates. Old cold-copy/61.6ms/publication/build failures remain failed evidence. Approval is known-source pre-birth eligibility ONLY; exact12ba runtime/combined conformance and final source-bound formal/tests remain unproven. Standalone414be successes and direct1b577 approval are separate historical evidence, not12ba measurements.