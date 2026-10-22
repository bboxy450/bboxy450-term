#!/bin/sh
# C1d 1000-seat gates (.bboxy/specs/c1-live-seats.md §5, decision #10).
# Usage: [GATE_BUDGET=N] sh scripts/gates.sh memory|bus|poller
# Prints one "GATE <name> value=..." line; exits non-zero when value > GATE_BUDGET.
# Env: GATE_SEATS (default min(1000, kern.tty.ptmx_max - 32)), GATE_RATE (32kB/s per firehose seat; 'idle' runs /bin/sleep: the memory floor), GATE_SECONDS,
# GATE_RUN_DIR (/tmp; the run dir is deleted on exit). macOS caps PTYs at
# kern.tty.ptmx_max (default 511); every line reports the seat count used.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
case "${1:-}" in
  memory|bus|poller) ;;
  *) echo "usage: sh scripts/gates.sh memory|bus|poller" >&2; exit 2 ;;
esac
cargo build --release --locked -q -p bboxy-firehose
exec cargo test --release --locked -q -p bbt-mux --test gates_seats "gate_$1" -- --ignored --nocapture --test-threads=1
