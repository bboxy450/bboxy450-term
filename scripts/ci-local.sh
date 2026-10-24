#!/bin/sh
# Local CI: the same gates as .github/workflows/ci.yml, run before every handoff and acceptance.
# Run it in a clean worktree of the commit being handed off, never in the shared tree:
#   git worktree add --detach /tmp/ci-<name> <sha>; cd there; CARGO_TARGET_DIR=<own dir> sh scripts/ci-local.sh
# Usage: sh scripts/ci-local.sh [--parity] [--slow]
# --parity runs visual parity; --slow also runs the scale gates.
set -eu
# The machine-wide sccache server stalls agent builds (clients idle at 0% CPU for minutes,
# ~7% hits across private targets): a chokepoint under no-chokepoints (#287). Call rustc directly.
export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=
# Disk is tight: gates need no incremental cache and no debug info (about half the target size).
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}" CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}" CARGO_PROFILE_TEST_DEBUG="${CARGO_PROFILE_TEST_DEBUG:-0}"
# Bulk replay/render checks need O1 inside the startup budget; keep debug guards on.
export CARGO_PROFILE_DEV_OPT_LEVEL="${CARGO_PROFILE_DEV_OPT_LEVEL:-1}" CARGO_PROFILE_TEST_OPT_LEVEL="${CARGO_PROFILE_TEST_OPT_LEVEL:-1}" CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true
# Gate slots are claimed atomically here (mkdir), so two agents starting at once cannot overrun
# the limit: 3 while ~ has at least 25 GiB free, else 2. A slot whose owner pid is gone is reclaimed.
# BBOXY_GATE_SLOT=none skips the claim (scoped runs that are not full gates).
if [ "${BBOXY_GATE_SLOT:-}" != none ]; then
    free=$(df -g "$HOME" | awk 'NR==2 {print $4}')
    limit=2; [ "${free:-0}" -ge 25 ] && limit=3
    slot=""
    for n in 1 2 3; do
        [ "$n" -gt "$limit" ] && break
        d="/tmp/bboxy450-gate-slot-$n"
        if [ -d "$d" ] && ! kill -0 "$(cat "$d/pid" 2>/dev/null || echo 0)" 2>/dev/null; then rm -rf "$d"; fi
        if mkdir "$d" 2>/dev/null; then echo $$ > "$d/pid"; slot=$n; break; fi
    done
    if [ -z "$slot" ]; then echo "ci-local: all $limit gate slots are taken; retry when one is released" >&2; exit 75; fi
    trap 'rm -rf "/tmp/bboxy450-gate-slot-$slot"' EXIT INT TERM
    echo "ci-local: gate slot $slot of $limit"
fi
sh scripts/layering.sh
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
sh scripts/ui-lint.sh
smoke_run=$(mktemp -d "${TMPDIR:-/tmp}/bboxy-smoke.XXXXXX")
if BBOXY_TEST_CUSTODY=memory cargo run -p bboxy450-term --locked -- --headless --smoke --run-dir "$smoke_run"; then
  rm -rf -- "$smoke_run"
else
  smoke_status=$?
  echo "smoke failed; private run retained at $smoke_run" >&2
  exit "$smoke_status"
fi
sh scripts/invariants.sh "$@"
echo "ci-local: all gates passed"
