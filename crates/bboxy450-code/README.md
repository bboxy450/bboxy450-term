# bboxy450-code

Bounded, deterministic code reading and search: five queries as plain functions
and as a stdio MCP server.

Part of [bboxy450-term](https://github.com/bboxy450/bboxy450-term). The shapes
come from measured agent use, recorded in that repository's
`.bboxy/specs/code-query.md`: 105,148 code-reading operations over three weeks,
58% reads of a line range and 25% searches.

| Tool | Answers |
|---|---|
| `read` | numbered lines of files: a range, the whole item around a line, or a file at a git revision |
| `search` | lines matching any of several patterns, each with the function, impl or type that contains it |
| `symbol` | where Rust names are defined, and optionally where they are used |
| `files` | files under paths or matching a glob, with line counts |
| `outline` | a Rust file's items in order |

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

Syntax comes from [tree-sitter](https://tree-sitter.github.io/) with the Rust
grammar; file walking honours `.gitignore` through the `ignore` crate.
