#!/bin/sh
# Fail-closed push of .bboxy records (Architect, Creativity): record checks in a
# private target, every fast record-reading invariant, commit the named paths, rebase, push, then report the hash
# that is actually on origin/main. Any non-zero step stops everything after it.
# Usage: sh scripts/records-push.sh "<commit message>" <path>...
set -eu
export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=  # no machine-wide sccache, #287 sixth instance (a29f839d)
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
message=$1; shift
[ "$#" -gt 0 ] || { echo "records-push: no paths" >&2; exit 2; }
target=$(mktemp -d "${TMPDIR:-/tmp}/records-push.XXXXXX")
trap 'rm -rf "$target"' EXIT
CARGO_TARGET_DIR="$target" CARGO_INCREMENTAL=0 cargo test -p bbt-doc --locked --test corpus -q
CARGO_TARGET_DIR="$target" CARGO_INCREMENTAL=0 cargo test -p bbt-mermaid --locked -q
sh scripts/layering.sh
sh scripts/pending-flags.sh
sh scripts/kill-own-pids.sh
sh scripts/generated-sources.sh
# The e2e checklist page is generated from e2e-acceptance.md section 1 and a bbt-ui test asserts equality;
# run it whenever either side is in the push (Oracle, 2026-10-24: a records-only edit broke main once).
for p in "$@"; do
    case "$p" in
        .bboxy/specs/e2e-acceptance.md|lab/web/e2e.js)
            CARGO_TARGET_DIR="$target" CARGO_INCREMENTAL=0 cargo test -p bbt-ui --locked --test e2e_checklist -q
            break ;;
    esac
done
# Every fast invariant that reads records runs here too (Oracle, 2026-10-24: a
# records-only push once broke licenses.sh, which reads licensing.md).
sh scripts/licenses.sh
sh scripts/brand.sh
sh scripts/images.sh
sh scripts/cross-os.sh
sh scripts/never-block.sh
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
