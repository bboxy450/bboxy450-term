#!/bin/sh
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Fast landing (LAND.md): rebase, compile the workspace, test only the packages
# whose files changed, push. Runs niced on half the cores so live seats keep
# working. The full gate stays available as `make land-full`.
set -eu
export GIT_TERMINAL_PROMPT=0
cd "$(git rev-parse --show-toplevel)"
jobs=${CARGO_BUILD_JOBS:-$(( $(sysctl -n hw.ncpu 2>/dev/null || nproc) / 2 ))}
export CARGO_BUILD_JOBS=$jobs
run() { echo "land: $*"; nice -n 10 "$@"; }

test -z "$(git status --porcelain --untracked-files=no)" || { echo "land: commit or stash your changes first" >&2; exit 2; }
git fetch -q origin main
git rebase -q origin/main || { git rebase --abort; echo "land: rebase conflict; resolve it and run make land again" >&2; exit 78; }
test -n "$(git rev-list origin/main..HEAD)" || { echo "land: nothing to land"; exit 0; }

changed=$(git diff --name-only origin/main...HEAD)
packages=$(printf '%s\n' "$changed" | python3 -I -c '
import json, subprocess, sys
from pathlib import Path
meta = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"]))
root = Path(meta["workspace_root"])
dirs = {Path(p["manifest_path"]).parent.relative_to(root): p["name"] for p in meta["packages"]}
hit = set()
for line in sys.stdin.read().split():
    path = Path(line)
    owners = [d for d in dirs if d != Path(".") and d in path.parents]
    if owners:
        hit.add(dirs[max(owners, key=lambda d: len(d.parts))])
print(" ".join(sorted(hit)))
')

gate() {
    run cargo check --workspace --all-targets --locked -q
    for package in $packages; do
        run cargo clippy --locked -q -p "$package" --all-targets -- -D warnings
        run cargo test --locked -q -p "$package"
    done
    if printf '%s\n' "$changed" | grep -q '^\.bboxy/'; then
        run cargo test --locked -q -p bboxy450-doc --test corpus
    fi
    if printf '%s\n' "$changed" | grep -qE '(^|/)models(/|\.rs$)|^verification/'; then
        run make test-formal FORMAL_REUSE=1
    fi
}
gate || { echo "land: gate failed; fix and run make land again" >&2; exit 77; }

for attempt in 1 2 3 4 5; do
    if git push -q origin HEAD:main; then
        echo "LANDED $(git rev-parse HEAD)"; exit 0
    fi
    echo "land: main moved (attempt $attempt); rebasing and recompiling"
    git fetch -q origin main
    git rebase -q origin/main || { git rebase --abort; echo "land: rebase conflict after main moved" >&2; exit 78; }
    run cargo check --workspace --all-targets --locked -q || { echo "land: main moved and the merge no longer compiles" >&2; exit 77; }
done
echo "land: main kept moving; run make land again" >&2
exit 75
