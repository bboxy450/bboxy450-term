#!/bin/sh
# Local CI: the same gates as .github/workflows/ci.yml, run before every handoff and acceptance.
# Run it in a clean worktree of the commit being handed off, never in the shared tree:
#   git worktree add --detach /tmp/ci-<name> <sha>; cd there; sh scripts/ci-local.sh
# Usage: sh scripts/ci-local.sh [--parity] [--slow]
# The watcher's --after-tests continuation reports post-test checks only.
# --parity runs visual parity; --slow also runs the scale gates.
set -eu
gate_clean() {
    [ -z "$(git status --porcelain --untracked-files=no)" ] &&
    [ -z "$(git ls-files --others --exclude-standard -- crates scripts lab design .cargo Cargo.toml Cargo.lock build.rs)" ]
}
if ! gate_clean; then echo 'ci-local: dirty source tree; no exact-SHA proof' >&2; exit 2; fi
if [ "${BBOXY_GATE_SLOT:-}" != none ]; then
    ci_entry=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
    exec python3 -I "$ci_entry/landing.py" ci-local "$@"
fi
after_tests=false
if [ "${1:-}" = --after-tests ]; then
    after_tests=true
    shift
fi
ci_source_root=$(git rev-parse --show-toplevel)
ci_source=$(git rev-parse HEAD)
cd "$ci_source_root"
ci_target=${CARGO_TARGET_DIR:-${BBOXY_CARGO_TARGET_DIR:-target}}
case "$ci_target" in /*) ;; *) ci_target="$ci_source_root/$ci_target" ;; esac
unset CARGO_TARGET_DIR
gate_sha=$ci_source
gate_mode=normal
[ "$#" -eq 0 ] || gate_mode=extended
[ "$after_tests" = false ] || gate_mode=after-tests
printf 'ci-local: source %s %s\n' "$gate_sha" "$gate_mode"
export BBOXY_CARGO_TARGET_DIR="$ci_target"
# shellcheck source=scripts/cache-env.sh
. ./scripts/cache-env.sh
# Keep profile environment identical across gates; it is part of the cache key.
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true
printf 'ci-local: checkout %s target %s\n' "$(pwd -P)" "$BBOXY_CARGO_TARGET_DIR"
# Resource observation has no cost threshold and is never a readiness gate.
ledger_helper="$(pwd)/scripts/resource-ledger.py"
BBOXY_LEDGER_AGENT=${BBOXY_LEDGER_AGENT:-worktree:$(pwd -P)}
BBOXY_LEDGER_BRANCH=${BBOXY_LEDGER_BRANCH:-$(git branch --show-current)}
BBOXY_LEDGER_SHA=$gate_sha
export BBOXY_LEDGER_AGENT BBOXY_LEDGER_BRANCH BBOXY_LEDGER_SHA
ledger() {
    ledger_stage=$1
    shift
    if [ -f "$ledger_helper" ]; then
        python3 -I "$ledger_helper" run "$ledger_stage" -- "$@"
    else
        "$@"
    fi
}
if [ -f "$ledger_helper" ]; then
    BBOXY_LEDGER_SAMPLER=$(python3 -I "$ledger_helper" start $$) || BBOXY_LEDGER_SAMPLER=''
    export BBOXY_LEDGER_SAMPLER
fi
if [ "$after_tests" = false ]; then
    ledger smoke sh scripts/layering.sh
    ledger integration python3 -I scripts/test-landing.py
    if [ -f "$ledger_helper" ]; then
        ledger integration python3 -I scripts/test-resource-ledger.py
    fi
    ledger build cargo build --workspace --locked --target-dir "$BBOXY_CARGO_TARGET_DIR"
    ledger build cargo clippy --workspace --all-targets --locked --target-dir "$BBOXY_CARGO_TARGET_DIR" -- -D warnings
    ledger unit sh scripts/quarantine-test.sh --workspace --locked
fi
ledger smoke sh scripts/ui-lint.sh
ledger build cargo build -p bboxy450-term --locked --target-dir "$BBOXY_CARGO_TARGET_DIR"
smoke_bin="$BBOXY_CARGO_TARGET_DIR/debug/bboxy450-term"
case "$smoke_bin" in
  /*) ;;
  *) smoke_bin="$(pwd)/$smoke_bin" ;;
esac
smoke_run=$(mktemp -d "${TMPDIR:-/tmp}/bboxy-smoke.XXXXXX")
# The real GUI has its own fixture inputs, as well as its own identity and sockets.
# Checkout/build provenance events are work, not an idle fixture.
mkdir "$smoke_run/repository"
if (cd "$smoke_run/repository" && BBOXY_TEST_CUSTODY=memory ledger smoke "$smoke_bin" --headless --smoke --run-dir "$smoke_run/run"); then
  rm -rf -- "$smoke_run"
else
  smoke_status=$?
  echo "smoke failed; private run retained at $smoke_run" >&2
  exit "$smoke_status"
fi
sh scripts/invariants.sh "$@"
if ! gate_clean || [ "$(git rev-parse HEAD)" != "$gate_sha" ]; then
    echo 'ci-local: source changed during gate; no exact-SHA proof' >&2; exit 2
fi
if [ "$after_tests" = true ]; then
    echo "ci-local: post-test checks passed"
else
    echo "ci-local: all gates passed"
fi
printf 'ci-local: result %s %s 0\n' "$gate_sha" "$gate_mode"
