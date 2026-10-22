#!/bin/sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
# lab checks first (true-size type, no UA chrome; renderer.md §4.4b): fail fast before any capture
scripts/lab-check.sh
parity_out="${PARITY_OUT:-target/parity}"
mkdir -p "$parity_out"
cargo build -p bboxybook --locked
bboxybook_bin="${CARGO_TARGET_DIR:-target}/debug/bboxybook"
"$bboxybook_bin" manifest > "$parity_out/requested.json"
capture_status=0
node crates/bboxybook/capture.mjs "$parity_out" "$parity_out/requested.json" || capture_status=$?
"$bboxybook_bin" render --all --out "$parity_out/native" --manifest "$parity_out/cases.json"
"$bboxybook_bin" compare --strict-geometry --out "$parity_out" --manifest "$parity_out/cases.json"
exit "$capture_status"
