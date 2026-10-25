#!/bin/sh
# Admission executes and records an exact isolated retry, with a seven-day expiry.
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec python3 -I "$dir/landing.py" quarantine "$@"
