#!/bin/sh
# Run the one-line shell checks in invariant frontmatter. README/body text is not executable.
set -eu
invariant_tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
invariant_root=$(CDPATH='' cd -- "$invariant_tools/.." && pwd)

frontmatter_field() {
    awk -v key="$1" '
        NR == 1 { if ($0 != "---") exit; next }
        $0 == "---" { exit }
        index($0, key ":") == 1 { sub(/^[^:]*:[[:space:]]*/, ""); print; exit }
    ' "$2"
}

run_slow=false
run_parity=false
run_cargo=true
selected_gate=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        --slow) run_slow=true; shift ;;
        --parity) run_parity=true; shift ;;
        --no-cargo) run_cargo=false; shift ;;
        --root)
            [ "$#" -ge 2 ] || { echo 'invariants: --root requires a candidate directory' >&2; exit 2; }
            invariant_root=$2; shift 2 ;;
        --*) printf 'unknown invariant flag: %s\n' "$1" >&2; exit 2 ;;
        *)
            case "$1" in ''|*[!A-Za-z0-9_-]*) echo 'invariants: invalid gate name' >&2; exit 2 ;; esac
            [ -z "$selected_gate" ] || { echo 'invariants: select one gate' >&2; exit 2; }
            selected_gate=$1; shift ;;
    esac
done
cd -- "$invariant_root"
if [ -f "$invariant_tools/cache-env.sh" ]; then
    # shellcheck source=scripts/cache-env.sh
    . "$invariant_tools/cache-env.sh"
else
    export RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER=''
fi
if [ -n "$selected_gate" ]; then
    selected_file=".bboxy/invariants/$selected_gate.md"
    [ -f "$selected_file" ] && [ "$(frontmatter_field type "$selected_file")" = invariant ] || {
        printf 'invariants: unknown gate: %s\n' "$selected_gate" >&2; exit 2;
    }
fi

invariant_checks=0
invariant_failures=0
invariant_skipped=0
invariant_code_skipped=0
for invariant_file in .bboxy/invariants/*.md; do
    [ -f "$invariant_file" ] || continue
    [ "$(frontmatter_field type "$invariant_file")" = invariant ] || continue
    invariant_name=${invariant_file##*/}
    invariant_name=${invariant_name%.md}
    [ -z "$selected_gate" ] || [ "$invariant_name" = "$selected_gate" ] || continue
    if [ "$(frontmatter_field slow "$invariant_file")" = true ] && [ "$run_slow" = false ] && [ -z "$selected_gate" ]; then
        gate=$(frontmatter_field gate "$invariant_file")
        if [ "$run_parity" = false ] || [ "$gate" != parity ]; then
            printf 'invariant %s: skipped (slow)\n' "$invariant_name"
            invariant_skipped=$((invariant_skipped + 1))
            continue
        fi
    fi
    check_command=$(frontmatter_field check "$invariant_file")
    if [ "$run_cargo" = false ]; then
        if python3 -I scripts/landing.py check-is-cargo "$check_command"; then
            printf 'invariant %s: skipped (code tier)\n' "$invariant_name"
            invariant_code_skipped=$((invariant_code_skipped + 1))
            [ -z "$selected_gate" ] || { echo 'invariants: selected gate requires cargo' >&2; exit 2; }
            continue
        else
            selector_exit=$?
            [ "$selector_exit" -eq 1 ] || exit "$selector_exit"
        fi
    fi
    invariant_checks=$((invariant_checks + 1))
    if [ -z "$check_command" ]; then
        printf 'invariant %s: fail (missing check)\n' "$invariant_name"
        invariant_failures=$((invariant_failures + 1))
    elif sh -c '
        cargo() {
            subcommand=$1
            shift
            if [ -n "${BBOXY_CARGO_TARGET_DIR:-}" ]; then
                command cargo "$subcommand" --target-dir "$BBOXY_CARGO_TARGET_DIR" "$@"
            else
                command cargo "$subcommand" "$@"
            fi
        }
        eval "$1"
    ' invariant-check "$check_command"; then
        printf 'invariant %s: pass\n' "$invariant_name"
    else
        check_exit=$?
        printf 'invariant %s: fail (exit %s)\n' "$invariant_name" "$check_exit"
        invariant_failures=$((invariant_failures + 1))
    fi
done
printf 'invariants: %s checks, %s failures, %s slow checks skipped\n' "$invariant_checks" "$invariant_failures" "$invariant_skipped"
[ "$run_cargo" = true ] || printf 'invariants: %s code-tier checks skipped\n' "$invariant_code_skipped"
[ "$invariant_failures" -eq 0 ]
