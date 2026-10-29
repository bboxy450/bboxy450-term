# coverage: full
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
set -eu
exec python3 -I -B "$(dirname "$0")/settings.py" G1
