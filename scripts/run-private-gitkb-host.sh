#!/bin/sh
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Cargo builds normally; only this test executable receives the private account.
set -eu
umask 077
: "${BBOXY_GITKB_ADAPTER:?Make must supply the adapter}"
: "${BBOXY_GITKB_BIN:?Make must supply the native backend}"
harmony_root=$(mktemp -d "${TMPDIR:-/tmp}/bboxy-gitkb-host.XXXXXX")
harmony_home="$harmony_root/run/account/plugin-home"
mkdir -p "$harmony_home"
harmony_status=0
env -i PATH="$PATH" HOME="$harmony_home" TMPDIR="${TMPDIR:-/tmp}" \
    LANG="${LANG:-en_US.UTF-8}" TERM="${TERM:-xterm-256color}" \
    RUST_TEST_THREADS="${RUST_TEST_THREADS:-2}" \
    BBOXY_GITKB_TEST_ROOT="$harmony_root" \
    BBOXY_GITKB_ADAPTER="$BBOXY_GITKB_ADAPTER" BBOXY_GITKB_BIN="$BBOXY_GITKB_BIN" \
    "$@" || harmony_status=$?
if [ "$harmony_status" -eq 0 ]; then
    rm -rf -- "$harmony_root"
else
    printf 'Private GitKB host fixture retained: %s\n' "$harmony_root" >&2
fi
exit "$harmony_status"
