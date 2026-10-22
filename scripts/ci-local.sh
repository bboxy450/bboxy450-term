#!/bin/sh
# Local CI: the same gates as .github/workflows/ci.yml, run before every handoff and acceptance.
# Run it in a clean worktree of the commit being handed off, never in the shared tree:
#   git worktree add --detach /tmp/ci-<name> <sha>; cd there; CARGO_TARGET_DIR=<own dir> sh scripts/ci-local.sh
# Usage: sh scripts/ci-local.sh [--parity]   (parity is slow; run it when lab, design, ui or render changed)
set -eu
sh scripts/layering.sh
cargo build --workspace --locked
cargo test --workspace --locked
sh scripts/ui-lint.sh
sh scripts/invariants.sh "${1:-}"
echo "ci-local: all gates passed"
