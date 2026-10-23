#!/bin/sh
# Fail-closed push of .bboxy records (Architect, Creativity): record checks in a
# private target, commit the named paths, rebase, push, then report the hash
# that is actually on origin/main. Any non-zero step stops everything after it.
# Usage: sh scripts/records-push.sh "<commit message>" <path>...
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
message=$1; shift
[ "$#" -gt 0 ] || { echo "records-push: no paths" >&2; exit 2; }
target=$(mktemp -d "${TMPDIR:-/tmp}/records-push.XXXXXX")
trap 'rm -rf "$target"' EXIT
CARGO_TARGET_DIR="$target" CARGO_INCREMENTAL=0 cargo test -p bbt-doc --locked --test corpus -q
CARGO_TARGET_DIR="$target" CARGO_INCREMENTAL=0 cargo test -p bbt-mermaid --locked -q
sh scripts/layering.sh
git add -- "$@"
git diff --cached --quiet && { echo "records-push: nothing staged" >&2; exit 3; }
git -c commit.gpgsign=true commit -q -m "$message"
for attempt in 1 2 3; do
    git fetch -q origin main
    git rebase -q origin/main
    if git push -q origin HEAD:main; then break; fi
    [ "$attempt" -lt 3 ] || { echo "records-push: push rejected three times" >&2; exit 4; }
done
git fetch -q origin main
printf 'records-push: on origin/main at %s\n' "$(git rev-parse --short origin/main)"
