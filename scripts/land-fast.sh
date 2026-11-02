#!/bin/sh
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Fast landing (LAND.md): rebase, test only what the diff can break, push.
# The affected set is the changed crates plus every crate that depends on them
# (scripts/affected.sh); Cargo.toml/Cargo.lock/toolchain changes select all.
# One clippy and one nextest run cover the whole set, niced on half the cores
# with the shared sccache. The full gate stays available as `make land-full`.
set -eu
export GIT_TERMINAL_PROMPT=0
cd "$(git rev-parse --show-toplevel)"
BBOXY_CARGO_JOBS=${BBOXY_CARGO_JOBS:-4}
export BBOXY_CARGO_JOBS
# shellcheck source=scripts/cache-env.sh
. ./scripts/cache-env.sh
run() { echo "land: $*"; nice -n 10 "$@"; }

test -z "$(git status --porcelain --untracked-files=no)" || { echo "land: commit or stash your changes first" >&2; exit 2; }
git fetch -q origin main
git rebase -q origin/main || { git rebase --abort; echo "land: rebase conflict; resolve it and run make land again" >&2; exit 78; }
test -n "$(git rev-list origin/main..HEAD)" || { echo "land: nothing to land"; exit 0; }
for revision in $(git rev-list origin/main..HEAD); do
    git verify-commit "$revision" 2>/dev/null || { echo "land: unsigned commit $revision" >&2; exit 2; }
done

changed=$(git diff --name-only origin/main...HEAD)
crates=$(sh scripts/affected.sh origin/main..HEAD --crates)
packages=''
for crate in $crates; do packages="$packages -p $crate"; done
echo "land: affected: ${crates:-none}"

# `gate || ...` suspends `set -e` inside the function: every check returns explicitly.
gate() {
    if [ -n "$packages" ]; then
        # shellcheck disable=SC2086
        run cargo clippy --locked -q $packages --all-targets -- -D warnings || return 1
        # nextest runs every affected test binary concurrently, then doctests.
        # shellcheck disable=SC2086
        run sh scripts/quarantine-test.sh --locked $packages || return 1
    fi
    if printf '%s\n' "$changed" | grep -q '^\.bboxy/'; then
        run cargo test --locked -q -p bboxy450-doc --test corpus || return 1
    fi
    if printf '%s\n' "$changed" | grep -qE '(^|/)models(/|\.rs$)|^verification/'; then
        run make test-formal FORMAL_REUSE=1 || return 1
    fi
}
gate || { echo "land: gate failed; fix and run make land again" >&2; exit 77; }

for attempt in 1 2 3 4 5; do
    if git push -q origin HEAD:main; then
        echo "LANDED $(git rev-parse HEAD)"; exit 0
    fi
    echo "land: main moved (attempt $attempt); rebasing and rechecking the affected crates"
    git fetch -q origin main
    git rebase -q origin/main || { git rebase --abort; echo "land: rebase conflict after main moved" >&2; exit 78; }
    crates=$(sh scripts/affected.sh origin/main..HEAD --crates)
    packages=''
    for crate in $crates; do packages="$packages -p $crate"; done
    if [ -n "$packages" ]; then
        # shellcheck disable=SC2086
        run cargo check --locked -q $packages --all-targets || { echo "land: main moved and the merge no longer compiles" >&2; exit 77; }
    fi
done
echo "land: main kept moving; run make land again" >&2
exit 75
