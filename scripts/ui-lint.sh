#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ -n "${BBOXY_CARGO_TARGET_DIR:-}" ]; then
    set -- --target-dir "$BBOXY_CARGO_TARGET_DIR"
else
    set --
fi
exec cargo clippy "$@" -p bbt-ui-model -p bbt-ui -p bbt-render -p bboxy450-term --locked -- -D clippy::disallowed_methods
