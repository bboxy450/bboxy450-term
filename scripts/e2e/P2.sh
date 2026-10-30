# coverage: full
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Prebuilt by make build-perf-acceptance; the night never compiles.
. "$(dirname "$0")/lib.sh"
set -e
gate="$(dirname "$BBT")/bboxy450-perf-gates"
[ -x "$gate" ] || fail "P2 fixture_unavailable: run make build-perf-acceptance before the night"
# One Oracle plus 1000 distinct recipients (the invariant's 1000-seat fixture had 999).
bus=0
GATE_SEATS=1001 GATE_BUDGET=13299 GATE_SECONDS=60 "$gate" gate_bus --exact --ignored --nocapture --test-threads=1 > "$E2E_WORK/p2-bus.log" 2>&1 || bus=$?
cat "$E2E_WORK/p2-bus.log"
memory=0
GATE_SEATS=400 GATE_RATE=idle GATE_BUDGET=420 GATE_SECONDS=20 "$gate" gate_memory --exact --ignored --nocapture --test-threads=1 > "$E2E_WORK/p2-memory.log" 2>&1 || memory=$?
cat "$E2E_WORK/p2-memory.log"
[ "$bus" -eq 0 ] && [ "$memory" -eq 0 ] || fail "P2 bus_exit=$bus memory_exit=$memory"
awk '/^GATE (bus|memory) / { print }' "$E2E_WORK/p2-bus.log" "$E2E_WORK/p2-memory.log"
echo 'P2: 1000 distinct recipients under 13299 ms p99; 400 real idle PTYs below 420 MiB peak RSS'
