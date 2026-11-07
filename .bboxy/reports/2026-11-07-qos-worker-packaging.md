---
id: 01M6W2VK223ERXEGMZN51RE8VF
number: 93377961366301
type: report
title: Recognize the canonical QoS compiler wrapper during worker packaging
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits: []
verify:
  command: make test-dream-c1-profile and make prepare-dream-c1-worker TARGET_DIR=/Users/feral/mydev/bboxy450-term/target
  exit_code: 0
author: seat:01M6W2VK23VPPDMTR5N114NK3R
created: '2026-11-07T22:42:51.587236+00:00'
---
Direct operator request: pull Term and harness main, update/reinstall both and restart the UI, preserving the running backend. The historical trusted-handoff task supplies related delivery context; no native task lifecycle is changed. This author identifier labels the standalone Codex execution, not a registered native-seat signature.

The canonical release compilation completed on 261086dbb, but bundle-files failed with unknown_RUSTC_WRAPPER: cache-env.sh now selects the repository's rustc-qos shim, while the worker input validator understood only an empty wrapper or sccache. Cache-off also selects that shim, so switching cache modes cannot resolve the mismatch.

Changes are limited to scripts/test-dream-c1.py:
- Admit only RUSTC_WRAPPER named rustc-qos resolving to this checkout's scripts/qos.sh, with the canonical QoS root. Capture its opened file identity and hash.
- Record and validate BBOXY_INNER_RUSTC_WRAPPER separately; retain the existing empty/sccache allowlist. The postbuild tool-change fence now covers both files through the existing flattened wrapper inventory.
- Include qos.sh and qos-policy.py in source/configuration hashes and label caching enabled only when an actual sccache wrapper is present.
- Extend private controls for QoS with/without cache, foreign scripts/roots, unknown inner wrappers, and mutations of each wrapper between observations. No admission guard, runtime approval tuple, profile or resource limit is relaxed.

Evidence:
- Initial make -o check-licenses bundle: Make exit 2 after Cargo release compilation exit 0 (12m58s); packaging rejected the unknown wrapper. Rust emitted nonfatal debug-stripping warnings for missing libLLVM.dylib; no global toolchain changes were made.
- make test-dream-c1-profile: exit 0, including the new positive and negative controls. Receipt SHA256 b05aee22392839fee9dee562db5e53885af104cdb0c65c1325ec94419849a0f5 at /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-c8y9tb1e/receipt.json.
- BBOXY_CARGO_TARGET_DIR=/Users/feral/mydev/bboxy450-term/target make prepare-dream-c1-worker TARGET_DIR=/Users/feral/mydev/bboxy450-term/target: exit 0; real ordinary release worker built and both before/after closure verdicts are admissible. Receipt SHA256 55cd0558f79b91120966cd014a1ca51eba8a21a8f62b5f33e2f93c4fe76d8053 at /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-kb65fphl/receipt.json. This does not assert native Dream runtime readiness or consumer conformance.
- git diff --check: exit 0. Formal models are unchanged: this extends input parsing/provenance accounting, not a runtime concurrency/lifecycle contract. Full CI, Rust suites, smoke and visual parity were not run for this operator update workflow; focused packaging controls were necessary for this repair.

Harness main is already pinned to 38d99096f4f9c475a5547cf2192982af6ff7a5c0; Cargo.lock is unchanged. Local KB updates that blocked the primary pull were snapshotted and preserved. Supervisor 24351 and Logic 24354 have not been restarted. The final clean-source rebuild, signed installation and UI-only activation follow this source checkpoint; their results are not asserted in advance. The containing signed commit identifies the change without a self-referential hash.
