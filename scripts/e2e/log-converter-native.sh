#!/bin/sh
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Real clients are isolated by the P0 fixtures. Missing clients emit SKIP,
# which records absent evidence and must never be treated as acceptance proof.
set -eu
cd "$(dirname "$0")/../.."
. ./scripts/cache-env.sh
export BBOXY_CARGO_TARGET_DIR="${BBOXY_CARGO_TARGET_DIR:-$(pwd)/target}"
python3 -I scripts/zstd-tool.py
BXL_ZSTD="$BBOXY_CARGO_TARGET_DIR/tools/zstd-1.5.7/programs/zstd" cargo test --locked \
  -p bboxy450-log-converter --target-dir "$BBOXY_CARGO_TARGET_DIR" --test compat --test size -- --ignored
exec sh crates/bboxy450-log-converter/probes/resume.sh
