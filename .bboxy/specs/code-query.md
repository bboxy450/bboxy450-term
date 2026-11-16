---
type: spec
title: 'Code query: one MCP server for reading and searching code'
status: draft
id: 01M7B58R972307J7YT0GG738VY
created: 2026-11-13T19:13:37.126Z
number: 1794292861627504
intent: Agents read and search code through one deterministic MCP server, reachable as ordinary tools and from python_execute, that returns bounded range reads and searches instead of ad-hoc rg, grep and sed.
updated: 2026-11-16T19:44:25.861Z
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:5c76063c51c495422e57eb54af241e6b2eb21eb8507faf44b442839e92379ece
  frontmatter_digest: sha256:618b3dd84ef9a1e306552476e2ca7194fbbe1113475c8a26f67988c83ceb167a
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: UrfZ8IvPwyGAajQo0oCFQPQRuJ/PqNk2WHM1Xj5yWnqo/wwG1KrjdUNlhKwlXCnMJRnr7tDG+1JXDBzOZnL1CQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151716
  event:
    id: 01M7JY7BBRNZWTX4198BX44BZR
    sequence: 151716
    hash: 70beb234ea07c315e26ab023c937f2665b1d8c22e087ac1ef3e9768fcc403219
---
# Code query: one MCP server for reading and searching code

Asked for by the operator on 2026-11-13: agents answer code questions with ad-hoc `rg`, `grep`, `git grep`, `sed -n` and python; give them one deterministic MCP server, reachable as ordinary tools and from `python_execute`. Part of [[gaps-and-improvements]].

## What agents actually do (measured)

Every session log from the last 21 days with a cwd in this workspace or the harness: 162 Claude Code, 274 Codex and 84 harness sessions, 152,152 tool calls, 253,311 shell segments. Scripts and the machine-readable summary are in `/tmp/lumen-toolpatterns/` (`extract.py`, `analyze.py`, `summary.json`).

- Reading and searching code is 26.6% of all tool calls: 105,148 operations.
- **48% of those read a line range** (`sed -n 'A,Bp' file`). Search is second.
- 78.6% of range reads follow a search within three calls. Half start at the hit and read downward, a median of 51 lines.
- 30.8% of range reads re-read lines already read in the session.
- A search carries a median of three alternatives; 73.9% use alternation. 46.3% are piped to `head` to bound them by hand.
- 3.3% of search calls fail on a guessed path, 2.4% on an unquoted zsh glob.
- 3.5% of shell calls return more than 20 KB and carry 64.1% of all output bytes.
- `commands.rg`, `commands.git` and `commands.shell` inside `python_execute` were executed zero times.
- No transitive-caller question was observed. Go-to-definition by type resolution was not observed either; names are found by text.

| Intent | Share of code-query operations |
|---|---|
| Read a line range, whole file, head or tail, or a file at a revision | 58.0% |
| Search: uses of names, strings, tests, scripts, KB | 24.7% |
| Find a definition, or definition plus uses | 8.9% |
| List a directory, find files, count lines | 6.3% |
| Outline of a file, or a whole item body | 1.1% |
| Cargo members and dependencies | 0.7% |

## Decision

One server, `bboxy_code`, five tools. Text and syntax only.

**No rust-analyzer in the first version.** The measured questions are answered by text search plus the syntax tree. A language server costs minutes of cold start and gigabytes per worktree on this workspace, and nothing in 105,148 operations needed type resolution. It stays a later addition behind the same `symbol` tool if a need is measured.

## Tools

Every tool takes a batch, bounds its own output and never fails the whole call for one bad element.

| Tool | Input | Output |
|---|---|---|
| `read` | `requests[]` of `{path, start?, end?, around?, rev?}` | Numbered lines, at most 200 per request, clipped at 240 columns; the file's line count; `next_start` when cut. `around: N` returns the whole enclosing item at line N. `rev` reads the file at a git revision. |
| `search` | `patterns[]`, `paths[]`, `glob?`, `context?`, `limit?` (default 50), `mode: lines \| files \| count`, `fixed?`, `rev?` | Hits as `{path, line, text, item}` where `item` is the enclosing function, impl or type; the total count and a count per pattern; missing paths listed under `missing`, not fatal. |
| `symbol` | `names[]`, `kind?`, `paths?`, `refs?` | Definitions as `{path, start, end, kind, signature}`; with `refs`, a capped list of uses with their enclosing items. |
| `files` | `paths[]` or `glob`, `depth?` | Paths with line counts, capped at 200. |
| `outline` | `paths[]` | Each file's items with kind, name, signature and line range. |

Rules:

- Patterns are regular expressions in the Rust `regex` syntax; `fixed: true` takes them literally. Alternation is spelled `|`; a pattern containing `\|` is refused with a message that says so, because that grep spelling is the commonest silent mismatch.
- Paths are relative to the workspace root. A path outside it is refused.
- `.gitignore` is honoured. Binary files are skipped.
- Results are ordered by path, then line. The same call on the same tree returns the same bytes.
- A whole result stays under 16 KiB. When a cap cuts it, the result says what was cut and how to continue.

## Shape

| Piece | Where |
|---|---|
| Library and server | new crate `crates/bboxy450-code`, package `bboxy450-code`: the queries as plain functions, and a stdio MCP server binary over them |
| Syntax | `tree-sitter` with the Rust grammar first; other files get text tools with `item` left empty |
| Search and walk | the `grep-*` and `ignore` crates that ripgrep is built from, in-process; no `rg` child |
| Revisions | `git show REV:path` through one bounded child |
| Registration | the launcher adds `bboxy_code` beside `bboxy` in each seat's MCP configuration (`crates/bboxy450-harness-wake-up/src/launch.rs`), so `mcp.call("bboxy_code", "search", …)` works from `python_execute` with nothing new in the runtime |

The server holds no state between calls and writes nothing.

## Not built

Transitive callers, TODO counts, import lists, blame and `log -S`, log and temp-file reading, git state and diffs, scripted edits. Each was rare or belongs to a different tool.

## Acceptance criteria

- [ ] Each tool returns the documented shape for the workspace's own sources, checked against fixtures in the crate.
- [ ] A missing path, an invalid pattern and a path outside the root each return a per-element error while the other elements answer.
- [ ] No result exceeds 16 KiB; a cut result names its continuation.
- [ ] Two runs on the same tree are byte-identical.
- [ ] A seat's `python_execute` reaches `bboxy_code` through `mcp.servers()` and one `mcp.call`.
- [ ] Replaying the 100 most frequent sanitized search and read shapes from `summary.json` through the tools returns the lines the original commands returned.
