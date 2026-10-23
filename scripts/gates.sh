#!/bin/sh
# C1d 1000-seat gates (.bboxy/specs/c1-live-seats.md §5, decision #10).
# Usage: [GATE_BUDGET=N] sh scripts/gates.sh memory|bus|poller|frame
# Prints one "GATE <name> value=..." line; exits non-zero when value > GATE_BUDGET.
# Env: GATE_SEATS (default min(1000, kern.tty.ptmx_max - 32)), GATE_RATE (32kB/s per firehose seat; 'idle' runs /bin/sleep: the memory floor), GATE_SECONDS,
# GATE_RUN_DIR (/tmp; the run dir is deleted on exit). macOS caps PTYs at
# kern.tty.ptmx_max (default 511); every line reports the seat count used.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
case "${1:-}" in
  frame)
    # The app custody selector exists only with debug assertions. Keep the
    # optimized profile, but never let this gate use the operator's keychain.
    export CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true BBOXY_TEST_CUSTODY=memory CARGO_INCREMENTAL=0
    unset BBOXY_VERIFY_INPUT
    cargo build --release --locked -q -p bboxy450-term
    limit=$(sysctl -n kern.tty.ptmx_max)
    count=${GATE_SEATS:-$((limit - 32))}
    parent=${GATE_RUN_DIR:-/tmp}
    output=$(mktemp -d "$parent/r5-frame.XXXXXX")
    echo "R5 frame artifacts: $output; ptmx_max=$limit requested=$count" >&2
    ulimit -n 8192
    target=${CARGO_TARGET_DIR:-target}
    exec "$target/release/bboxy450-term" --stall-test --stall-real --stall-frame-gate \
      --stall-seats "$count" --stall-watched "${GATE_WATCHED:-50}" \
      --stall-seconds "${GATE_SECONDS:-10}" --stall-idle 10 \
      --stall-run-dir "$output/run" --stall-report "$output/report.json" \
      --capture "$output/window.png" --capture-after 5
    ;;
  memory|bus|poller) ;;
  *) echo "usage: sh scripts/gates.sh memory|bus|poller|frame" >&2; exit 2 ;;
esac
cargo build --release --locked -q -p bboxy-firehose
exec cargo test --release --locked -q -p bbt-mux --test gates_seats "gate_$1" -- --ignored --nocapture --test-threads=1
