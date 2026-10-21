#!/bin/sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
parity_out="${PARITY_OUT:-target/parity}"
mkdir -p "$parity_out"
cargo build -p bboxybook --locked
target/debug/bboxybook manifest > "$parity_out/requested.json"
capture_status=0
node crates/bboxybook/capture.mjs "$parity_out" "$parity_out/requested.json" || capture_status=$?
target/debug/bboxybook render --all --out "$parity_out/native" --manifest "$parity_out/cases.json"
target/debug/bboxybook compare --out "$parity_out" --manifest "$parity_out/cases.json"
exit "$capture_status"
