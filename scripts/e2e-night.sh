#!/bin/sh
# The unattended e2e night (decision #282, invariant #283): the app's own runner over one
# group of promises, stdin closed, no window, nothing built or signed. Each promise runs in
# its own throwaway app (debug, memory custody, temporary run directory) against
# scripts/e2e/<ID>.sh, whose exit code is the verdict.
# Usage: sh scripts/e2e-night.sh [GROUP]   default terminal; BBT names the app binary.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
bin=${BBT:-target/debug/bboxy450-term}
[ -x "$bin" ] || { echo "e2e-night: no app binary at $bin; the night builds nothing (#282)" >&2; exit 2; }
exec "$bin" e2e night --group "${1:-terminal}" --scripts scripts/e2e </dev/null
