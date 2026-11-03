---
type: decision
title: Hash-pinned corrections for malformed immutable reports
status: decided
decided: 2026-11-03
id: 01M6HG2MQ9WW1QB8ZPD6QRXMAS
updated: 2026-11-03T20:03:10.492Z
created: 2026-11-03T20:02:15.592Z
number: 16393
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
provenance:
  digest: sha256:c882008528a51c04201d856545c583c3f519c3cc82fbc37f0e3549eb9a11f459
  frontmatter_digest: sha256:d61b7925607e9adf1a8546635556f6bb4ac13186f6611522dccd409f23887ba9
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: yQg5kN8DaBcyRef6R9mtHkVo1qQQWXjVHr6e+0pQNnB7MbotzHvE4YmC2kBPow+D3OlQWUvod5xjSmBT5fiwAA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#95035
---
# Hash-pinned report corrections

Decided 2026-11-03 by Oracle Vesper Greywind; recorded by Codex Oracle under [[tasks/2026-11-03-oracle-handoff-codex]].

The published startup-modal report is malformed (duplicate number, invalid task locator, invalid author and absent commits). The corpus validates superseded records and rejects every report modification/deletion, so a superseding record cannot repair the current corpus. Evidence: crates/bbt-doc/tests/corpus.rs:71-92 and crates/bbt-doc/tests/corpus/reports.rs:22-56,154-170.

Use an append-only `.bboxy/reports/CORRECTIONS.toml` ledger. Each correction records `path`, `from_blob`, `to_blob`, `reason`, and `decision`. A report modification is allowed only when a newly appended ledger entry in that same commit exactly matches the path and both Git blob IDs. Deletions, renames and unlisted modifications remain errors. Ledger removal, mutation and reuse of an old entry remain errors. Every corrected report must still pass ordinary metadata, task resolution and signature checks.

First use repairs only `.bboxy/reports/2026-11-03-startup-modal.md`: allocate a unique number, record the correcting seat as correction author (preserving the original Boxy attribution and explicitly stating that an original seat identity is unproven), and restore implementation commit evidence, and resolve its task to a new record retaining the 2026-11-03 operator brief and signed implementation 1affb986. This is historical evidence, not fresh verification of the waived formal/parity checks.

Required negative controls: unlisted modification, wrong destination blob and deletion fail; additionally retain immutable ledger and same-commit checks. A valid exact correction passes. Do not quarantine gates, rewrite published history or allow arbitrary report edits. This is a Git migration gate for legacy malformed records; it does not grant native writers general permission to overwrite reports.
