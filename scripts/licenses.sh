#!/bin/sh
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Origin: bboxy450-term. Retain credit and scoped upstream terms on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
python3 -I "$dir/license-packages.py" "$@"
python3 -I "$dir/source-notices.py"
