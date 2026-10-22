#!/bin/sh
# Run the one-line shell checks in invariant frontmatter. README/body text is not executable.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

frontmatter_field() {
    awk -v key="$1" '
        NR == 1 { if ($0 != "---") exit; next }
        $0 == "---" { exit }
        index($0, key ":") == 1 { sub(/^[^:]*:[[:space:]]*/, ""); print; exit }
    ' "$2"
}

invariant_checks=0
invariant_failures=0
for invariant_file in .bboxy/invariants/*.md; do
    [ -f "$invariant_file" ] || continue
    [ "$(frontmatter_field type "$invariant_file")" = invariant ] || continue
    invariant_name=${invariant_file##*/}
    invariant_name=${invariant_name%.md}
    if [ "$invariant_name" = parity-geometry ] && [ "${1:-}" != --parity ]; then
        printf 'invariant %s: skipped (requires --parity)\n' "$invariant_name"
        continue
    fi
    invariant_checks=$((invariant_checks + 1))
    check_command=$(frontmatter_field check "$invariant_file")
    if [ -z "$check_command" ]; then
        printf 'invariant %s: fail (missing check)\n' "$invariant_name"
        invariant_failures=$((invariant_failures + 1))
    elif sh -c "$check_command"; then
        printf 'invariant %s: pass\n' "$invariant_name"
    else
        check_exit=$?
        printf 'invariant %s: fail (exit %s)\n' "$invariant_name" "$check_exit"
        invariant_failures=$((invariant_failures + 1))
    fi
done
printf 'invariants: %s checks, %s failures\n' "$invariant_checks" "$invariant_failures"
[ "$invariant_failures" -eq 0 ]
