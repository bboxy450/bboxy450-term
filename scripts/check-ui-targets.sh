#!/bin/sh
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
set -eu
targets=$(sed -n '/^targets = \[/,/^\]$/s/^[[:space:]]*"\([^"]*\)",$/\1/p' rust-toolchain.toml)
test -n "$targets"
installed=$(rustup target list --installed)
for target in $targets; do
    printf 'check-ui-targets: %s\n' "$target"
    if ! printf '%s\n' "$installed" | grep -qx "$target"; then
        rustup target add "$target"
    fi
    cargo check --locked --target-dir "$1" --target "$target" \
        -p bboxy450-ui-core -p bboxy450-ui -p bboxy450-render
done
