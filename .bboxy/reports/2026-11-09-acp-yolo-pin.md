---
id: 01M70ZAHH7NJQT6DAC7Y0G6T9T
number: 205483610615674
type: report
title: Pin full-access ACP defaults for Claude, Codex, Pi and OMP
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits: []
verify:
  command: make test-acp-unrestricted TARGET_DIR=/Users/feral/mydev/bboxy450-term/target
  exit_code: 0
author: seat:01M70ZAHH7P4B75NWMQRJ3SGSR
created: '2026-11-09T20:51:12.133151+00:00'
---
Direct operator request: make ACP harnesses default to full YOLO permissions, prioritizing Claude/Codex and including Pi/OMP; bypass CI and restart only the UI. This standalone Codex execution uses its own worktree at Term d3f4e3c3a. The historical trusted-handoff task supplies delivery context; no native task or seat lifecycle is claimed by this execution identifier.

make bump-harness REV=d64b817091189f106c8bf245408a915683f754c4 moved the submodule through its canonical workflow. The producer is signed by the approved Boxy key and pushed to harness main. Its full mechanism/evidence report is crates/bboxy450-harness/.bboxy/reports/2026-11-09-acp-yolo-defaults.md. It supplies legacy-profile yolo defaults, provider-specific launch settings, acknowledged full-access ACP negotiation, automatic residual tool grants, and native OMP discovery. Existing explicit ask profiles remain an opt-in; outer --yolo overrides them. Pi uses pi-acp with its native unrestricted tools; OMP uses omp --mode acp --yolo.

The Makefile's focused test-acp-unrestricted target runs the three permission checks, the 26 ACP process cases serially, and exact private profile discovery. All 30 checks passed, Make exit 0. A private real Pi/OMP initialize probe also exited 0 with zero provider prompts/tools. The broader exploratory onboarding raster remains failed on unrelated conversation-scroll restoration (f32::MAX versus 123); it is reported and outside the delivery claim. Earlier failed runs and the fixture fixes are detailed in the producer report. No full CI, formal, lint, visual suite or make land was run, by explicit operator instruction.

Evidence stays private at /tmp/bboxy-acp-20261109-bicor4fc: acp-delivery-checks.log (SHA256 8d80ce06b2c902477f98997b5b78afcdd789098caf44ebcac1f057d0bc5a036a), pi-omp-initialize.json and delivery-inputs.json. git diff --check and Python/TOML syntax checks exited 0. Rust emitted existing nonfatal missing-libLLVM debug-stripping warnings.

The screenshot's Claude message also denotes an abandoned streamed draft before tool execution, including during steering; yolo cannot prevent every such message. Existing agent workers keep their executable/config until relaunched. The upcoming canonical signed build/install and GUI-only handoff will preserve Supervisor and Logic; their actual delivery results will be recorded separately. This signed containing commit identifies the source checkpoint without a self-referential hash.
