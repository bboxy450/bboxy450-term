#!/bin/sh
# Compatibility entry point. The Makefile owns all build, metadata and signing rules.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
profile=${1:-debug}
target_root=${CARGO_TARGET_DIR:-${BBOXY_CARGO_TARGET_DIR:-target}}
bundle=${2:-"$target_root/$profile/bboxy450-term.app"}
exec make --no-print-directory bundle "PROFILE=$profile" "TARGET_DIR=$target_root" "BUNDLE=$bundle"
