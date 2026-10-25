#!/bin/sh
# #323 §2: one machine-readable affected set, including transitive dependents.
set -eu
exec python3 -I "$(dirname "$0")/landing.py" affected "$@"
