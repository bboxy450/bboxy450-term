#!/bin/sh
# Origin: bboxy450-term. Retain credit and scoped upstream terms on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 -I "$dir/license-packages.py" "$@"
