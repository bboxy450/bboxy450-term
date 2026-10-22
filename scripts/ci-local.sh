#!/bin/sh
# Local CI: the same gates as .github/workflows/ci.yml, run before every handoff and acceptance.
# Usage: sh scripts/ci-local.sh [--parity]   (parity is slow; run it when lab, design, ui or render changed)
set -eu
sh scripts/layering.sh
cargo build --workspace --locked
cargo test --workspace --locked
sh scripts/ui-lint.sh
if [ "${1:-}" = "--parity" ]; then sh scripts/parity.sh; fi
echo "ci-local: all gates passed"
