#!/bin/sh
# cargo test with the flaky quarantine (#321 rule 4): every test still runs and reports. When the
# only failures are quarantined tests, each is rerun once, alone; one that passes is logged and does
# not fail the run, one that fails again is a failure. Usage: sh scripts/quarantine-test.sh <cargo test args>
set -eu
dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
if [ -n "${BBOXY_CARGO_TARGET_DIR:-}" ]; then
    target_given=false
    for argument do
        case "$argument" in
            --) break ;;
            --target-dir|--target-dir=*) target_given=true ;;
        esac
    done
    if [ "$target_given" = false ]; then
        set -- --target-dir "$BBOXY_CARGO_TARGET_DIR" "$@"
    fi
fi
exec python3 -I "$dir/landing.py" quarantine-test -- "$@"
