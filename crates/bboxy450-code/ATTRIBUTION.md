# Respectful Slopping — Source Attribution and Reuse

Project: **bboxy450-term**, including the bboxy450 crate suite in this repository.
Canonical source: https://github.com/bboxy450/bboxy450-term
Copyright 2026 Boxy (bboxy450) and contributors, for their respective contributions.

Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE

Runtime app-menu naming is a project-owned contribution; its source header carries
this policy and origin with extracts.

Event-log checkpoint verification and its bounded formal model are project-owned
contributions; their source headers carry this policy and origin with extracts.

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
| `verification/`, `scripts/formal.py`, and the new Yarn native model checker/exporter | bboxy450-term, project-owned contributions under Respectful Slop License 1.0-Draft. Independently implemented using TLA state-machine/fairness concepts (Leslie Lamport, https://lamport.azurewebsites.net/tla/high-level-view.html). No TLC implementation source copied. Downloaded TLC/Temurin tools retain their own licenses and stay outside source/binary packages. |
| `crates/bboxy450-tlc/src/lean.rs`, `scripts/lean.py`, the Den and Yarn Lean printers and the `.lean` proofs | bboxy450-term, project-owned contributions under Respectful Slop License 1.0-Draft. Inspired by Veil's embedding of transition systems in Lean 4 (https://github.com/verse-lab/veil) and by TLA+ reachability; independently implemented, no source copied, and the output needs only Lean's core library. The downloaded Lean 4 toolchain (Apache-2.0, https://github.com/leanprover/lean4) keeps its own license and stays outside source and binary packages. |
| Byte-bounded task summary paging in `crates/bbt-mux/src/mcp/work_items.rs` and its `TaskBytePage` contract/witnesses | bboxy450-term, project-owned implementation under Respectful Slop License 1.0-Draft; canonical source and reuse policy above apply. |
| Project-authored launcher/design reference in `lab/web/` and repository scripts | bboxy450-term, same owned-contribution terms. Generated/mirrored files retain their generator origin; regenerate/synchronize rather than changing their content independently. |
| `crates/bbt-core/src/vendor/alacritty/` and `crates/bbt-core/src/keyboard.rs` | Alacritty, Apache-2.0; revisions, original notices and local changes in `bbt-core/VENDOR.md` and source headers. |
| Other pre-adoption `crates/bbt-core/` source | Retained Apache-2.0 fork terms; project attribution requests do not amend that grant. New separately identified bboxy450 contributions may use the new license. |
| `crates/bbt-core/src/vendor/vte/` | vte, MIT OR Apache-2.0; retained original license files. |
| Pre-adoption source in `crates/bbt-yarn/`, `crates/bbt-yarn-macros/` | kittens-old/Yarn, MIT OR Apache-2.0; pinned revision and Kittens contributor notices in the crate READMEs and original license files. New separately identified bboxy450 contributions may use the new license. |
| `crates/bbt-render/tests/fonts/` | JetBrains Mono, OFL-1.1, including the accompanying `OFL.txt`. |
| `crates/bbt-mermaid/tests/fixtures/docs/`, `state/` | Mermaid documentation examples, MIT; attribution and license in the fixture directory. The original import revision was not recorded; do not invent it. |
| `crates/bboxy-firehose/reference/vendor/` | Pinned xterm.js assets, MIT; accompanying notices/provenance apply. Excluded from its Cargo package. |
| `crates/bboxy450-usage/src/{claude,codex,grok}.rs`, provider abstraction and quota selection in `src/lib.rs`, and `tests/fixtures/*.json` | Translated from Tachyon (MIT), https://github.com/Gonzih/tachyon, `Sources/Tachyon/Providers/{Provider,ClaudeProvider,CodexProvider,GrokProvider}.swift`, `Model/UsageModel.swift`; JSON copied from `Tests/TachyonTests/ProviderDecodeTests.swift`. Rust workers/OS sampling and integration additions are bboxy450-owned. Original notice retained below and in `crates/bboxy450-usage/LICENSE-TACHYON`. |
| Names, logos and identity images | Identity/trademark rights stay with their respective owners. Existing project-brand exclusions and upstream logo rights remain unchanged. No blanket ownership claim over harness logos. |

For schemas, examples or design documents reused separately, identify the actual
file/section and its canonical URL in the accompanying attribution. Preserve
third-party fixture bytes and notices; a project source header never overrides
their terms. Full factual notices are in [NOTICE](NOTICE).

## Image composer interaction

The atomic `[Image #N]` marker interaction is independently implemented, inspired
by the Codex CLI image composer. No Codex source code was copied or adapted.
Origin: https://github.com/openai/codex (interaction reference only).

## Repository workflow

Read this file from `AGENTS.md`, but carry compact identity, license and absolute
policy links in source headers and beside distinctive implementations as well.
New source must retain any upstream attribution and accurately identify its own
scope. Never put a custom-license header over someone else's retained code.

Use `make sync-licenses` for canonical crate notice copies, `make test-licenses`
for policy/source checks and unattended negative cases, and
`make check-license-packages` for Cargo package-content checks. The install and
release workflows include the license and attribution documents with binaries.

## Prepared rich toolkit content

The cockpit startup modal, lifecycle observations and additive history-verification
progress records in `crates/bboxy450-term/src/{startup_modal.rs,cockpit/startup.rs}`
and their mux integration are project-authored; keep the canonical source and
absolute policy URL above attached to reused extracts.

Streaming Markdown, transcript/diff/tool/approval controls and editor extensions
are project-authored source under the policy above. Syntax highlighting uses
[syntect](https://github.com/trishume/syntect), MIT, as an unchanged dependency
with its pure Rust `fancy-regex` backend; no highlighter source was copied.
Its embedded syntax and theme assets retain the dependency's upstream notices.

## Tachyon translated provider portions and fixtures

The following MIT notice applies to those translated portions and copied fixtures,
not to independently written bboxy450 integration code. No endorsement is implied.

MIT License

Copyright (c) 2026 Maksim Soltan / Wild Honey on the Porch, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

- Shared composer attachment custody, workspace completion, async ownership fences and private native/ACP/formal witnesses are project-owned; retain this policy with extracted implementations.

Harness onboarding verification in `scripts/verify-harness-onboarding.py` is a
project-owned focused gate; the harness producer retains its own canonical
origin, factual adapter sources and attribution in its submodule.

Dream C1 public and worker-wire mirrors are adapted from
[bboxy450-harness src/dream/c1_wire.rs](https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/c1_wire.rs).
The harness owns their sole schema/corpus; term owns its private source custody,
control routing and bounded subprocess integration. Keep both origin notices and
absolute policy URLs with extracts.

The private Dream bootstrap record and its consumer mirrors are adapted from
[bboxy450-harness src/dream/bootstrap.rs](https://github.com/bboxy450/bboxy450-harness/blob/main/src/dream/bootstrap.rs).
The harness owns the byte layout and sole `tests/fixtures/dream-bootstrap`
conformance corpus; keep its origin and policy with copied protocol code.
