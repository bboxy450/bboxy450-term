#!/bin/sh
# cargo test with the flaky quarantine (#321 rule 4): every test still runs and reports. When the
# only failures are quarantined tests, each is rerun once, alone; one that passes is logged and does
# not fail the run, one that fails again is a failure. Usage: sh scripts/quarantine-test.sh <cargo test args>
set -eu
# Tests are hermetic: a gate run from inside a bboxy seat must not hand that seat's live
# identity to the code under test (bugs/harness-python-execute-tests-leak-seat-context).
# These are exactly the variables the launcher sets for a seat (bboxy450-harness-wake-up
# src/launch.rs, bbt-supervisor); fixtures that need them set their own.
unset BBOXY_CAPABILITY BBOXY_CAPABILITY_FILE BBOXY_GENERATION BBOXY_HOOK_SOCKET_ONLY \
    BBOXY_PROCESS_GENERATION BBOXY_ROLE BBOXY_RUN_DIR BBOXY_SEAT_ID BBOXY_SOCK BBOXY_USER_ZDOTDIR
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
