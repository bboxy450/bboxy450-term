# Mermaid fixture provenance

`docs/` (20 files) and `state/` (20 files) contain examples copied from the
Mermaid documentation. The original fixture import revision and exact per-file
source URLs were not recorded; they remain unknown. No origin revision is
inferred from the license retrieval below. The `.mmd` bytes are retained.

The accompanying `LICENSE-MIT` is the exact upstream license text retrieved on
2026-10-28 using `gh api repos/mermaid-js/mermaid/license`. It identifies
Copyright (c) 2014-2022 Knut Sveidqvist. Its API Git blob SHA is
`2e5daebd2ad0fb200080a27d98aec5f04902acdd`.
Exact blob: <https://api.github.com/repos/mermaid-js/mermaid/git/blobs/2e5daebd2ad0fb200080a27d98aec5f04902acdd>.
Upstream project: <https://github.com/mermaid-js/mermaid>.
This retained current notice documents the upstream grant; it does not establish
which historical documentation revision supplied these fixtures.

`bboxy/` contains the project's own diagrams copied from repository-local `.bboxy`
records; the fixture test matches those source blocks. This directory is a mixed
provenance test corpus, not a blanket claim that the Mermaid examples are
bboxy450-owned code. Upstream notices and prior grants remain in force.

Project source: <https://github.com/bboxy450/bboxy450-term>.
Accurately credit copied, adapted or translated reuse; independent inspiration
credit is requested. Policy:
<https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md>.

Additional project blocks copied verbatim to restore the required corpus after
repository-local records became tracked (source: bboxy450-term):

- `2026-11-05-python-sdk-proposal-superseded-1.mmd` — Copied from [.bboxy/notes/2026-11-05-python-sdk-proposal-superseded.md](https://github.com/bboxy450/bboxy450-term/blob/main/.bboxy/notes/2026-11-05-python-sdk-proposal-superseded.md).
- `programmatic-execution-patterns-1.mmd` — Copied from [.bboxy/research/programmatic-execution-patterns.md](https://github.com/bboxy450/bboxy450-term/blob/main/.bboxy/research/programmatic-execution-patterns.md).
- `programmatic-tool-access-1.mmd` — Copied from [.bboxy/specs/programmatic-tool-access.md](https://github.com/bboxy450/bboxy450-term/blob/main/.bboxy/specs/programmatic-tool-access.md).
- `spec-first-os-1.mmd` — Copied from [.bboxy/specs/spec-first-os.md](https://github.com/bboxy450/bboxy450-term/blob/main/.bboxy/specs/spec-first-os.md).
- `workspace-provisioning-1.mmd` — Copied from [.bboxy/specs/workspace-provisioning.md](https://github.com/bboxy450/bboxy450-term/blob/main/.bboxy/specs/workspace-provisioning.md).
- `workspace-provisioning-2.mmd` — Copied from [.bboxy/specs/workspace-provisioning.md](https://github.com/bboxy450/bboxy450-term/blob/main/.bboxy/specs/workspace-provisioning.md).

`cards/RemoteDelivery.mmd` is generated from `RemoteDelivery` by
`bboxy450-term-remote/tests/delivery_card.rs`; the fixture test compares it with
the card and runs the same parse/layout checks as the rest of this corpus.
