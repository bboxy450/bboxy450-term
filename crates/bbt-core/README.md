# bboxy450-term-core

## What it is

An embeddable terminal grid and byte parser with resumable checkpoints. It owns neither PTYs nor rendering nor child processes.

## Mental model

`Term` owns the parser and grid. `Line`, `Column` and `Point` address cells. A `Checkpoint` preserves terminal/parser state, while event listeners and side effects belong to the embedding application.

## Invariants

Restoring a checkpoint does not replay terminal effects. Screen and parser state travel together. Terminal modes govern the shared keyboard codec.

## Entry points

The package retains its Rust library name `bbt_core`; depend on it with
`bbt-core = { package = "bboxy450-term-core", version = "0.1.0" }`.

Construct `Term` with bounded history, feed bytes with `advance`, inspect the grid, and drain generated events. Use the checkpoint API for restart rather than replaying already-executed escape-sequence effects.

## Minimal example

```rust
// keeps: spec terminal-core §D1
use bbt_core::{Column, Line, Term};
let mut term = Term::new(80, 24, 0);
term.advance(b"hello");
assert_eq!(term.grid()[Line(0)][Column(0)].c, 'h');
```

## Recipes

The complete `grid.rs` program is in `examples/`. Build it with
`cargo build --example grid`.

Use `keyboard` to encode normalized key input against live terminal modes. Provide an unknown-OSC callback when a host needs an application-specific protocol. Checkpoint consumers must version their surrounding storage envelope.

## Pitfalls

Grid coordinates, capture cursors and application frame numbers are different domains. A terminal title is application-controlled input. Validate sizes and history bounds before allocating a model.

## Platform support

The grid/parser library has no native window or PTY dependency. The release is verified on macOS; other targets retain their dependency support limits.

## Feature flags

No package feature switches.

## License

Apache-2.0. The vendored parser also retains its MIT option; see NOTICE and the original licence files.

## Dependency name

The Rust library name remains `bbt_core`. Use the existing import name with a versioned package alias:

```toml
bbt-core = { package = "bboxy450-term-core", version = "0.1.0" }
```
