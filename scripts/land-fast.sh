#!/bin/sh
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Lean landing (LAND.md), the default: rebase, lint and test the crates the diff
# touches (their lib/bin tests, plus each changed integration-test file), compile-
# check every crate that depends on them, push. LAND_MODE=wide also tests every
# dependent (scripts/affected.sh); Cargo.toml/Cargo.lock/toolchain changes select
# all. Niced, shared sccache. The full gate stays available as `make land-full`.
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
LAND_MODE=${LAND_MODE:-lean}
case "$LAND_MODE" in lean|wide) ;; *) echo "land: LAND_MODE must be lean or wide" >&2; exit 2 ;; esac

# `gate || ...` suspends `set -e` inside the function: every check returns explicitly.
lean_gate() {
    plan=$(python3 -I scripts/landing.py lean origin/main..HEAD) || return 1
    echo "land: lean plan:"; printf '%s\n' "$plan" | sed 's/^/  /'
    touched=$(printf '%s\n' "$plan" | awk '$1=="test"{printf " -p %s", $2}')
    if [ -n "$touched" ]; then
        # shellcheck disable=SC2086
        run cargo clippy --locked -q $touched --all-targets -- -D warnings || return 1
    fi
    printf '%s\n' "$plan" | while read -r kind name selection; do
        # shellcheck disable=SC2086
        case "$kind" in
            test)
                # Real-process tests run the crate's executable; build it as wide runs do.
                case " $selection " in
                    *" --bins "*) run cargo build --locked -q -p "$name" --bins || exit 1 ;;
                esac
                run sh scripts/quarantine-test.sh --allow-ignored-only --locked -p "$name" $selection || exit 1 ;;
            check) run cargo check --locked -q $(printf -- '-p %s ' $name $selection) || exit 1 ;;
        esac
    done || return 1
}

gate() {
    if [ "$LAND_MODE" = lean ]; then
        lean_gate || return 1
    elif [ -n "$packages" ]; then
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
