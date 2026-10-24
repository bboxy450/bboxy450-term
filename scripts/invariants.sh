#!/bin/sh
# Run the one-line shell checks in invariant frontmatter. README/body text is not executable.
set -eu
# The machine-wide sccache server stalls agent builds (clients idle at 0% CPU for minutes,
# ~7% hits across private targets): a chokepoint under no-chokepoints (#287). Call rustc directly.
export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

frontmatter_field() {
    awk -v key="$1" '
        NR == 1 { if ($0 != "---") exit; next }
        $0 == "---" { exit }
        index($0, key ":") == 1 { sub(/^[^:]*:[[:space:]]*/, ""); print; exit }
    ' "$2"
}

run_slow=false
run_parity=false
for flag in "$@"; do
    case "$flag" in
        --slow) run_slow=true ;;
        --parity) run_parity=true ;;
        *) printf 'unknown invariant flag: %s\n' "$flag" >&2; exit 2 ;;
    esac
done

invariant_checks=0
invariant_failures=0
invariant_skipped=0
for invariant_file in .bboxy/invariants/*.md; do
    [ -f "$invariant_file" ] || continue
    [ "$(frontmatter_field type "$invariant_file")" = invariant ] || continue
    invariant_name=${invariant_file##*/}
    invariant_name=${invariant_name%.md}
    if [ "$(frontmatter_field slow "$invariant_file")" = true ] && [ "$run_slow" = false ]; then
        gate=$(frontmatter_field gate "$invariant_file")
        if [ "$run_parity" = false ] || [ "$gate" != parity ]; then
            printf 'invariant %s: skipped (slow)\n' "$invariant_name"
            invariant_skipped=$((invariant_skipped + 1))
            continue
        fi
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
printf 'invariants: %s checks, %s failures, %s slow checks skipped\n' "$invariant_checks" "$invariant_failures" "$invariant_skipped"
[ "$invariant_failures" -eq 0 ]
