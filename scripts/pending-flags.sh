#!/bin/sh
# A pending invariant must carry `slow: true` and a `gate:` so scripts/invariants.sh
# skips it in the default run: that script runs every file regardless of status.
# Refuse the record otherwise (Oracle, 2026-10-25, after #318 broke default CI).
set -eu
cd "$(dirname "$0")/.."
rc=0
for f in .bboxy/invariants/*.md .bboxy/invariants/*.den; do
    [ -f "$f" ] || continue
    case "$f" in
        *.den)
            status=$(python3 -I scripts/kb_record.py status "$f")
            [ "$status" = "pending" ] || continue
            slow=$(python3 -I scripts/kb_record.py slow "$f")
            gate=$(python3 -I scripts/kb_record.py gate "$f")
            ;;
        *)
            fm=$(awk 'NR==1 && $0!="---" {exit} NR>1 && $0=="---" {exit} NR>1 {print}' "$f")
            status=$(printf '%s\n' "$fm" | awk -F': *' '$1=="status" {print $2; exit}')
            [ "$status" = "pending" ] || continue
            slow=$(printf '%s\n' "$fm" | awk -F': *' '$1=="slow" {print $2; exit}')
            gate=$(printf '%s\n' "$fm" | awk -F': *' '$1=="gate" {print $2; exit}')
            ;;
    esac
    if [ "$slow" != "true" ] || [ -z "$gate" ]; then
        echo "pending-flags: $f is pending without 'slow: true' and a 'gate:' (default invariants would run its check)" >&2
        rc=1
    fi
done
[ $rc -eq 0 ] && echo "pending-flags: held"
exit $rc
