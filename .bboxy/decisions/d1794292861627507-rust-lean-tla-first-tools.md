---
type: decision
title: 'Tools are opinionated: Rust hardcore, Den for knowledge, other languages as sources to port from'
status: decided
date: 2026-11-13
id: 01M7B7V31DW55KQM4R51G8V28D
updated: 2026-11-13T20:01:00.456Z
number: 1794292861627507
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:c6a2d1695df7e3640ce3e41f6cdb90f0bd86e8a747808f99a30bedda8db4299b
  frontmatter_digest: sha256:ae96268036746719b7423e5ac5b6c2aa9183ff9a1059913ccba58d3dfddf681a
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: nzq9t3gmnAWiOKlO/uCpTJm0GGnJPhaEAMv6+WAN2u0WhYahvWStUNeUDhPc40TSfXc6ltzPi031If9Ms4t7Cg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151678
  event:
    id: 01M7B7ZHN113HNQC1VCM49VTFP
    sequence: 151678
    hash: 883057fdc768e10b64120fcb1bd60ac062da0f686ac4972b93e776b0320f0e5f
---
# Tools are opinionated: Rust hardcore, Den for knowledge, other languages as sources to port from

Operator, 2026-11-13:

- "we are going to be very opinionated, we focus on building rust code (until something new, rust for LLMs shows up), we want to focus on LEAN language and TLA+ integration into codebases, that how we should structure our tools, with those directions in mind."
- "and we do all of that through .den files, everything in KB should slowly move into .den"
- "people will be taking python, go, typescript projects and rebuilding them into rust, then whatever comes after rust - people will be rewriting into this, but we basically on lowest level support python/typescript/go projects but we support RUST HARDCORE"

This applies [[spec-language]] to the tools. That spec already rules that TLA+ and Lean are engines of Den, not second languages: an agent writes Den, TLA+ text and every Lean statement are generated, and the only Lean anyone writes is the proof beside the spec.

## Decision

Three tiers. A tool states which tier each of its features serves.

| Tier | Languages | Role | Support |
|---|---|---|---|
| Target | Rust | the code we build | everything: items, signatures, uses, crates, the link to its specs and proofs, the formal loop, and whatever deeper analysis is measured to pay |
| Knowledge | Den (`*.den`, with `name.lean` proofs beside it) | specs, machines, properties, theorems, and in time every structured record | read through Den's own model; checked, viewed and lowered by Den's engines |
| Source | Python, TypeScript, Go | projects being rebuilt into Rust | the lowest level: read, search, list, and a syntax outline so a port can enumerate what there is to rebuild. No build, no semantics, no formal link |

| Generated, never hand-written | From | By |
|---|---|---|
| TLA+ text and the TLC parity check | a Den spec or a Rust `protocol!` | `den check`, `make test-formal` |
| Lean statements and the check file | a Den `theorem` | `bboxy450-den lean`, `make test-lean` |

**The target tier is replaceable.** When something better than Rust for model-written code arrives, it becomes the target and Rust joins the source tier. So the tools keep the tiers apart: the source-tier surface (text plus outline) is language-neutral, and Rust-only depth sits behind tools that say so.

The link between code and knowledge is a first-class question: "what specifies, models or proves this code" and "what code does this spec speak for". Today it lives in `verification/contracts.toml`, `coverage.toml` and `corpus.toml`; under [[spec-language]] it moves into Den links.

## What it changes

| Tool | Today | Under this decision |
|---|---|---|
| `bboxy_code` `read`, `search`, `files` | any text | unchanged: the floor for every language |
| `bboxy_code` `outline`, `symbol`, `read around`, enclosing item on a `search` hit | Rust only ([[code-query]]) | Rust in full; `.den` through the Den model (`bboxy450-den`'s `load`), never a second parser; Python, TypeScript and Go by syntax tree only (functions, classes, methods, types) |
| `bboxy_code` new `contracts` | none | target tier only: for Rust paths, the Den specs, contracts, models, broken controls and theorems that name them, and the reverse |
| `den_check`, `den_view`, `den_render` | specified in [[spec-language]], not in the catalogue | built as tools |
| `python_execute` | `sh`, `fs`, tools as functions | the formal loop as functions: check a Den file, generate and run its Lean, replay a recorded trace, each returning the verdict as data |
| New KB records | Markdown with a YAML header | Den for the kinds [[spec-language]] lists as ready; Markdown stays for notes, research, skills and prose |
| Language servers | none | none for the source tier, ever; rust-analyzer only if a need is measured |

## Not doing

- Hand-written TLA+ or Lean statements, or tools that encourage them.
- Anything beyond read, search and outline for source-tier languages: no test runners, linters, type checkers or dependency tools for Python, TypeScript or Go.
- A bulk conversion of the KB now. Records move kind by kind in the order [[spec-language]] gives (reports first, tasks last), with its round-trip check.

## Open

A port needs to know what has been rebuilt: source item to Rust item, with status. Whether that is a Den record kind or a projection over existing links is not decided.

## Order

1. `.den` in `bboxy_code`: outline, symbol, enclosing item.
2. Python, TypeScript and Go outlines in `bboxy_code`.
3. `den_check` (then `den_view`, `den_render`) in the tool catalogue.
4. `contracts`.
5. The formal loop as `python_execute` functions.

This record, [[code-query]], [[gaps-and-improvements]] and the intent decision are Markdown because Den has no prose-spec or decision kind built yet; they convert when their kinds land.
