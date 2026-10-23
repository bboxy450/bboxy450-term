#!/bin/sh
# Local CI: the same gates as .github/workflows/ci.yml, run before every handoff and acceptance.
# Run it in a clean worktree of the commit being handed off, never in the shared tree:
#   git worktree add --detach /tmp/ci-<name> <sha>; cd there; CARGO_TARGET_DIR=<own dir> sh scripts/ci-local.sh
# Usage: sh scripts/ci-local.sh [--parity] [--slow]
# --parity runs visual parity; --slow also runs the scale gates.
set -eu
# Disk is tight: gates need no incremental cache and no debug info (about half the target size).
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}" CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}" CARGO_PROFILE_TEST_DEBUG="${CARGO_PROFILE_TEST_DEBUG:-0}"
# Bulk replay/render checks need O1 inside the startup budget; keep debug guards on.
export CARGO_PROFILE_DEV_OPT_LEVEL="${CARGO_PROFILE_DEV_OPT_LEVEL:-1}" CARGO_PROFILE_TEST_OPT_LEVEL="${CARGO_PROFILE_TEST_OPT_LEVEL:-1}" CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true
sh scripts/layering.sh
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
sh scripts/ui-lint.sh
cargo run -p bboxy450-term --locked -- --smoke
sh scripts/invariants.sh "$@"
echo "ci-local: all gates passed"
