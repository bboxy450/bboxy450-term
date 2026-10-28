# Respectful Slopping — Source Attribution and Reuse

Project: **bboxy450-term**, including the bboxy450 crate suite in this repository.
Canonical source: https://github.com/bboxy450/bboxy450-term
Copyright 2026 Boxy (bboxy450) and contributors, for their respective contributions.

Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE

**Slop respectfully. Keep the credit attached.**

Read this policy when studying, copying, adapting, translating or deliberately
borrowing a project-specific approach from this code, whether you are a human
or a coding agent. Keep origin information with the excerpt, not only in agent
instructions that the next reader may never load.

## Terms and adoption

On 2026-10-28 the operator adopted **Respectful Slop License 1.0-Draft** for
bboxy450-owned code and subsequent contributions in this repository. The complete
supplied draft, including its explanatory appendix, is in [LICENSE](LICENSE).
Its local source-header identifier is
`LicenseRef-Respectful-Slop-1.0-Draft`; this is a custom draft, not an SPDX-listed
license or a claim of OSI approval. Cargo packages use `license-file = "LICENSE"`.

Sections 10(a)–(e) make accurate attribution a condition when reusing protected
material under this license, including in internal projects and with automated
tools. Section 10(f) requests credit for known conceptual inspiration and is
expressly nonbinding. This policy explains that distinction; it adds no license
conditions of its own.

Earlier MIT/Apache grants remain available for the material and revisions to
which they applied. Adoption does not withdraw them. `LICENSE-MIT` and
`LICENSE-APACHE` remain for those grants and for upstream material; their presence
does not offer all subsequent project contributions under those alternatives.
Existing upstream terms, notices and copyrights remain authoritative for their
respective material. Do not relabel upstream code as our original work.

## Keep credit beside the material

For copied, adapted or translated protected material, include a comment naming
**bboxy450-term**, the canonical source URL, and the affected file, symbol or block.
Include the original file/revision when supplied and reasonably available;
describe modifications honestly. Preserve applicable copyright and license
notices. A rewrite, renaming or translation may still be an adaptation.

```rust
// Adapted from bboxy450-term: https://github.com/bboxy450/bboxy450-term
// Original source: crates/bboxy450-term/src/spawn_params.rs, resolve_with_catalog
// Applies to: this launch-default resolver. Changes: describe actual changes.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
```

Use **Copied from** for a copy, **Adapted from** for an adaptation and
**Translated from** for a language port. These labels describe reuse; they do not
replace the full license and upstream notices distributed with it. Do not call
copied or adapted code merely **Inspired by**, invent sources, or imply endorsement.

For a project-specific idea that knowingly and materially informs independent
work, please identify the particular approach:

```rust
// Inspired by bboxy450-term: https://github.com/bboxy450/bboxy450-term
// Borrowed approach: directional cross-focus between sidebar and terminal pane.
// Implemented independently.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
```

Use “Implemented independently” only when true. Inspiration credit is a request;
unprotected ideas and independent creation do not acquire new copyright
conditions through this policy.

A format without comments uses a companion attribution file identifying the
affected files/material. Compiled or minified distributions carry corresponding
credit in `NOTICE` or accompanying documentation. An unchanged separate upstream
package may keep its own package notice; extracts need credit where they move.

## Scope and upstream inventory

Paths below are relative to the canonical repository. Each crate ships this same
inventory so the source identity remains available outside the monorepo. Only
rows actually present in a distribution apply to it; linked dependencies retain
their own licenses and are not relicensed by this document.

| Material | Origin and retained terms |
|---|---|
| Project-authored source, tests, examples, manifests and design data in `crates/`, except the boundaries below | bboxy450-term; Respectful Slop License 1.0-Draft for bboxy450-owned contributions, with prior grants preserved. Headers identify source files; this row also identifies commentless JSON schemas, goldens and configuration data. |
| Project-authored launcher/design reference in `lab/web/` and repository scripts | bboxy450-term, same owned-contribution terms. Generated/mirrored files retain their generator origin; regenerate/synchronize rather than changing their content independently. |
| `crates/bbt-core/src/vendor/alacritty/` and `crates/bbt-core/src/keyboard.rs` | Alacritty, Apache-2.0; revisions, original notices and local changes in `bbt-core/VENDOR.md` and source headers. |
| Other pre-adoption `crates/bbt-core/` source | Retained Apache-2.0 fork terms; project attribution requests do not amend that grant. New separately identified bboxy450 contributions may use the new license. |
| `crates/bbt-core/src/vendor/vte/` | vte, MIT OR Apache-2.0; retained original license files. |
| Pre-adoption source in `crates/bbt-yarn/`, `crates/bbt-yarn-macros/` | kittens-old/Yarn, MIT OR Apache-2.0; pinned revision and Kittens contributor notices in the crate READMEs and original license files. New separately identified bboxy450 contributions may use the new license. |
| `crates/bbt-render/tests/fonts/` | JetBrains Mono, OFL-1.1, including the accompanying `OFL.txt`. |
| `crates/bbt-mermaid/tests/fixtures/docs/`, `state/` | Mermaid documentation examples, MIT; attribution and license in the fixture directory. The original import revision was not recorded; do not invent it. |
| `crates/bboxy-firehose/reference/vendor/` | Pinned xterm.js assets, MIT; accompanying notices/provenance apply. Excluded from its Cargo package. |
| Names, logos and identity images | Identity/trademark rights stay with their respective owners. Existing project-brand exclusions and upstream logo rights remain unchanged. No blanket ownership claim over harness logos. |

For schemas, examples or design documents reused separately, identify the actual
file/section and its canonical URL in the accompanying attribution. Preserve
third-party fixture bytes and notices; a project source header never overrides
their terms. Full factual notices are in [NOTICE](NOTICE).

## Repository workflow

Read this file from `AGENTS.md`, but carry compact identity, license and absolute
policy links in source headers and beside distinctive implementations as well.
New source must retain any upstream attribution and accurately identify its own
scope. Never put a custom-license header over someone else's retained code.

Use `make sync-licenses` for canonical crate notice copies, `make test-licenses`
for policy/source checks and unattended negative cases, and
`make check-license-packages` for Cargo package-content checks. The install and
release workflows include the license and attribution documents with binaries.
