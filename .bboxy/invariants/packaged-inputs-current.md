---
id: 01M5WZX468FCVNRG4Q5265SEN1
number: 361
type: invariant
title: packaged build inputs match their canonical repository sources
status: active
check: python3 -I scripts/release-inputs.py
slow: false
cost: stale bundled inputs make the published crate differ from the tested workspace
links:
  - { to: "[[landing]]", rel: constrains }
created: 2026-10-26T20:54:48Z
updated: 2026-10-26T20:54:48Z
---
# Packaged build inputs match their canonical repository sources

A crate's `release-inputs/` directory mirrors repository-relative paths.
Every file there is a regular byte-identical copy of the canonical source.
The workspace uses the canonical files; Cargo's isolated package build uses
these copies. `scripts/landing.py::native_input_crates` derives the same mapping,
so a source edit still selects the crate and its reverse dependencies.

`python3 -I scripts/release-inputs.py` returns 0 when every copy agrees and 1
with the mismatched paths otherwise. It runs in the default invariant set.
This check implements the Oracle's release ruling 20143 (2026-10-26).
