#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
exec cargo clippy -p bbt-ui-model -p bbt-ui -p bbt-render -p bboxy450-term --locked -- -D clippy::disallowed_methods
