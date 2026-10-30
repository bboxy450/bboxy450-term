#!/bin/sh
# coverage: full
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Credit copied/adapted/translated reuse accurately; keep origin attached.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# F1: production event publication, runtime wake and native dock projection.
set -eu
cd "$(dirname "$0")/../.."
exec make test-live-feed
