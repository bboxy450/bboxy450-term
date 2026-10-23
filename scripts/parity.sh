#!/bin/sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
# lab checks first (true-size type, no UA chrome; renderer.md §4.4b): fail fast before any capture
scripts/lab-check.sh
parity_out="${PARITY_OUT:-target/parity}"
mkdir -p "$parity_out"
cargo build -p bboxybook --locked
bboxybook_bin="${CARGO_TARGET_DIR:-target}/debug/bboxybook"
# The old registry has no complete geometry fixtures. Retain its pixel report
# alongside the current gate; never invent geometry or relax the current gate.
if [ -z "${BBOXYBOOK_GALLERY:-}" ]; then
    legacy_out="$parity_out/legacy"
    mkdir -p "$legacy_out"
    "$bboxybook_bin" manifest --legacy > "$legacy_out/requested.json"
    node crates/bboxybook/capture.mjs "$legacy_out" "$legacy_out/requested.json"
    "$bboxybook_bin" replay-images --out "$legacy_out/native" --manifest "$legacy_out/cases.json"
    "$bboxybook_bin" compare --legacy --out "$legacy_out" --manifest "$legacy_out/cases.json"
fi
"$bboxybook_bin" manifest > "$parity_out/requested.json"
capture_status=0
node crates/bboxybook/capture.mjs "$parity_out" "$parity_out/requested.json" || capture_status=$?
"$bboxybook_bin" render --all --out "$parity_out/native" --manifest "$parity_out/cases.json"
"$bboxybook_bin" compare --strict-geometry --out "$parity_out" --manifest "$parity_out/cases.json"
exit "$capture_status"
