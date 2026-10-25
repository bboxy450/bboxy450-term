#!/bin/sh
set -eu
parity_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
selected_cases=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        --root|--cases)
            [ "$#" -ge 2 ] || { echo 'parity: missing selector value' >&2; exit 2; }
            if [ "$1" = --root ]; then parity_root=$2; else selected_cases=$2; fi
            shift 2 ;;
        *) printf 'parity: unknown argument: %s\n' "$1" >&2; exit 2 ;;
    esac
done
cd -- "$parity_root"
# lab checks first (true-size type, no UA chrome; renderer.md §4.4b): fail fast before any capture
# A diagnosed case is the predicate; unrelated pages must not decide its rerun.
if [ -z "$selected_cases" ]; then scripts/lab-check.sh; fi
parity_out="${PARITY_OUT:-target/parity}"
mkdir -p "$parity_out"
cargo build -p bboxybook --locked
bboxybook_bin="${CARGO_TARGET_DIR:-target}/debug/bboxybook"
# The old registry has no complete geometry fixtures. Retain its pixel report
# alongside the current gate; never invent geometry or relax the current gate.
if [ -z "${BBOXYBOOK_GALLERY:-}" ] && [ -z "$selected_cases" ]; then
    legacy_out="$parity_out/legacy"
    mkdir -p "$legacy_out"
    "$bboxybook_bin" manifest --legacy > "$legacy_out/requested.json"
    node crates/bboxybook/capture.mjs "$legacy_out" "$legacy_out/requested.json"
    "$bboxybook_bin" replay-images --out "$legacy_out/native" --manifest "$legacy_out/cases.json"
    "$bboxybook_bin" compare --legacy --out "$legacy_out" --manifest "$legacy_out/cases.json"
fi
"$bboxybook_bin" manifest > "$parity_out/requested.json"
if [ -n "$selected_cases" ]; then
    python3 -I - "$selected_cases" "$parity_out/requested.json" <<'PY'
import json, pathlib, sys
wanted = json.loads(sys.argv[1])
if not isinstance(wanted, list) or not wanted or any(not isinstance(v, str) or not v for v in wanted):
    print('parity: invalid case ids', file=sys.stderr)
    raise SystemExit(2)
path = pathlib.Path(sys.argv[2]); cases = json.loads(path.read_text())
def key(row):
    return f"{row['component']}/{row['slug']}-{row['density']}-{row['theme']}.png"
missing = set(wanted) - {key(row) for row in cases}
if missing:
    print('parity: case not in candidate manifest: ' + ', '.join(sorted(missing)), file=sys.stderr)
    raise SystemExit(2)
path.write_text(json.dumps([row for row in cases if key(row) in wanted]) + '\n')
PY
fi
capture_status=0
node crates/bboxybook/capture.mjs "$parity_out" "$parity_out/requested.json" || capture_status=$?
"$bboxybook_bin" render --all --out "$parity_out/native" --manifest "$parity_out/cases.json"
"$bboxybook_bin" compare --strict-geometry --out "$parity_out" --manifest "$parity_out/cases.json"
exit "$capture_status"
