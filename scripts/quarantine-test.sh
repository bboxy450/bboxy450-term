#!/bin/sh
# cargo test with the flaky quarantine (#321 rule 4): every test still runs and reports. When the
# only failures are quarantined tests, each is rerun once, alone; one that passes is logged and does
# not fail the run, one that fails again is a failure. Usage: sh scripts/quarantine-test.sh <cargo test args>
set -u
list="$(dirname "$0")/../.bboxy/quarantine.md"
log=$(mktemp); trap 'rm -f "$log"' EXIT
cargo test "$@" > "$log" 2>&1
status=$?
cat "$log"
[ "$status" -eq 0 ] && exit 0
failed=$(sed -n 's/^test \(.*\) \.\.\. FAILED$/\1/p' "$log" | sort -u)
[ -n "$failed" ] || exit "$status"
for t in $failed; do
    grep -q "^| \`$t\` |" "$list" 2>/dev/null || { echo "quarantine: $t is not quarantined; failing" >&2; exit "$status"; }
done
# Keep the package/feature args, drop any test-name filter and trailing harness args.
args=""; for a in "$@"; do [ "$a" = "--" ] && break; args="$args $a"; done
for t in $failed; do
    echo "quarantine: rerunning $t alone"
    # shellcheck disable=SC2086
    cargo test $args -- --exact "$t" || { echo "quarantine: $t failed again alone; failing" >&2; exit 1; }
    echo "quarantine: $t passed alone (recorded as a flake)" >&2
done
exit 0
