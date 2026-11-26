# bboxy450-code

Bounded, deterministic code reading and search: seven queries as plain functions
and as a stdio MCP server.

Part of [bboxy450-term](https://github.com/bboxy450/bboxy450-term). The shapes
come from measured agent use, recorded in that repository's
`.bboxy/specs/code-query.md`: 105,148 code-reading operations over three weeks,
58% reads of a line range and 25% searches.

| Tool | Answers |
|---|---|
| `read` | numbered lines of files: a range, the whole item around a line, or a file at a git revision |
| `search` | ripgrep matches as `path:line: text`, with enclosing items on hits and shared context shown once; with `ast`, ast-grep matches of a code shape |
| `symbol` | where Rust names are defined, and optionally where they are used |
| `files` | files under paths or matching a glob, with line counts |
| `outline` | a Rust file's items in order |
| `den_check` | Den's verdict on `*.den` files: sound or not, records with states, transitions and theorems, diagnostics with repairs |
| `contracts` | what specifies, models or proves a Rust path (contracts, models, broken controls, tests, crate coverage, Den specs), or that nothing does; and the code a contract, model or spec name speaks for |

Every tool takes a batch, keeps its result under 16 KiB, says what it cut and how
to continue, and reports a bad element beside the good ones. The same call on the
same tree returns the same bytes. Nothing is cached and nothing is written.

```sh
bboxy450-code                          # serve MCP on stdio; root is BBOXY_CODE_ROOT or the cwd
bboxy450-code search '{"patterns":["fn main"],"paths":["crates"]}'
```

```rust
let root = bboxy450_code::Root::new(".")?;
let hits = bboxy450_code::search(&root, &serde_json::json!({"patterns": ["fn main"]}))?;
```

`search` uses a sibling `rg` executable when bundled, or the system path
`/opt/homebrew/bin/rg` (Apple Silicon), `/usr/local/bin/rg` (Intel Mac), or
`/usr/bin/rg` (other Unix). Set `BBOXY_CODE_RG` to an absolute path to use a
different installation. Search ignores user ripgrep configuration. Library and
CLI line results carry the listing in `text`. MCP displays the listing directly,
followed by one JSON summary line with counts, missing paths and any cut.
The `files` and `count` modes return JSON summaries.

With `"ast": true` the patterns are [ast-grep](https://github.com/ast-grep/ast-grep)
code patterns (`Command::new($P)`, `fn $F($$$A) -> Result<$T> { $$$B }`), matched
on the syntax tree, so never inside a comment or a string. The two tools work
together: ripgrep first keeps the files that contain every word the pattern spells
out, ast-grep parses only those, and each match is listed as a hit on the line it
starts on, through the same listing, context, modes and bounds as a text search.
`ast-grep` is found like `rg` (a sibling executable, then the system path), or
through `BBOXY_CODE_AST_GREP`.

`contracts` reads `verification/{contracts,coverage,corpus}.toml` under the root
and answers `unavailable` where there is none.

Syntax comes from [tree-sitter](https://tree-sitter.github.io/) with the Rust
grammar; file walking honours `.gitignore` through the `ignore` crate.
