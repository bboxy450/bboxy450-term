#!/bin/sh
# Local CI (operator, 2026-10-22: no cloud CI for now). Green means exit 0.
# Record the result in the kb as: `sh scripts/ci.sh — exit 0 at <commit>`.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
sh scripts/layering.sh
cargo build --workspace --locked
cargo test --workspace --locked
sh scripts/ui-lint.sh
sh scripts/parity.sh
echo "ci: green at $(git rev-parse --short HEAD)"
