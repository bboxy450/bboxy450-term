#!/bin/sh
# The unattended e2e night (decision #282, invariant #283): the app's own runner over one
# group of promises, stdin closed, no window. Each promise runs in
# its own throwaway app (file test account, temporary run directory) against
# scripts/e2e/<ID>.sh, whose exit code is the verdict.
# The whole-night mode adds a separately built scale run and signed evidence.
# Usage: sh scripts/e2e-night.sh [GROUP|--all]; BBT names the prebuilt app binary.
# --all pins main, runs every bounded group, then the scale measurement once, last.
set -eu
cd "$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
if [ "${1:-}" = --all ]; then
    shift
    # Observational, bounded, private metadata only; never decides the night.
    if [ -f scripts/inference-ledger.py ]; then
        python3 -I scripts/inference-ledger.py >&2 || :
    fi
    exec python3 -I scripts/landing_recovery.py night-all "$@"
fi
bin=${BBT:-target/debug/bboxy450-term}
[ -x "$bin" ] || { echo "e2e-night: no app binary at $bin; the night builds nothing (#282)" >&2; exit 2; }
exec "$bin" e2e night --group "${1:-terminal}" --scripts scripts/e2e </dev/null
