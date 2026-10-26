#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ -n "${BBOXY_CARGO_TARGET_DIR:-}" ]; then
    set -- --target-dir "$BBOXY_CARGO_TARGET_DIR"
else
    set --
fi
exec cargo clippy "$@" -p bboxy450-ui-core -p bboxy450-ui -p bboxy450-render -p bboxy450-term --locked -- -D clippy::disallowed_methods
