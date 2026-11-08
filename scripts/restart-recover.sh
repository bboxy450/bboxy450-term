#!/bin/sh
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Hard restart that keeps every running seat. It snapshots the seats, closes the
# window (which detaches), kills logic before the supervisor so no seat exit is
# recorded, then starts a new backend with --recover-seats: each seat relaunches
# under its own id, role, swarm, tasks and native session. Only the exact owner
# PIDs of this run directory are signalled.
set -eu
APP="${INSTALL_DIR:-$HOME/Applications}/bboxy450-term.app"
BB="$APP/Contents/MacOS/bboxy450-term"
RUN="${BBOXY_RUN_DIR:-$HOME/Library/Application Support/bboxy450-term/run}"
snap="$RUN/restart-$(date +%Y%m%dT%H%M%S).json"
"$BB" ctl seat ls > "$snap"
running() {
    python3 -c 'import json,sys
d=json.load(open(sys.argv[1]))["data"]
print(" ".join(sorted(s["record"]["id"] for s in d if s["record"]["lifecycle"]=="open" and s["state"].get("process",{}).get("kind")=="running")))' "$1"
}
before=$(running "$snap")
echo "restart-recover: snapshot $snap ($(echo "$before" | wc -w | tr -d ' ') running seats)"
owner() { python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["process"]["pid"])' "$RUN/$1.owner.json"; }
ours() { ps -o command= -p "$1" 2>/dev/null | grep -qF "$BB"; }
absent() {
    for id in $1; do
        case " $2 " in
        *" $id "*) ;;
        *) printf '%s ' "$id" ;;
        esac
    done
}
gone() { i=0; while kill -0 "$1" 2>/dev/null; do i=$((i + 1)); [ $i -gt 300 ] && return 1; sleep 0.1; done; }
gui=$(owner gui); logic=$(owner logic); supervisor=$(owner supervisor)
for p in $gui $logic $supervisor; do ours "$p" || { echo "restart-recover: pid $p is not $BB; refusing" >&2; exit 2; }; done
kill -TERM "$gui"; gone "$gui"
# Logic first: once it is gone no seat exit can be recorded as an orderly stop.
kill -KILL "$logic"; gone "$logic"
kill -KILL "$supervisor"; gone "$supervisor"
open "$APP" --args --recover-seats
i=0; until "$BB" ctl app status >/dev/null 2>&1; do i=$((i + 1)); [ $i -gt 180 ] && { echo "restart-recover: app did not come back" >&2; exit 1; }; sleep 1; done
i=0; missing=$before
while [ -n "$missing" ] && [ $i -lt 180 ]; do
    "$BB" ctl seat ls > "$snap.after" 2>/dev/null || true
    now=$(running "$snap.after" 2>/dev/null || true)
    missing=$(absent "$before" "$now")
    [ -z "$missing" ] && break
    i=$((i + 2)); sleep 2
done
if [ -n "$missing" ]; then
    echo "restart-recover: not running after restart: $missing (snapshot $snap)" >&2
    exit 1
fi
echo "restart-recover: all $(echo "$before" | wc -w | tr -d ' ') seats running again"
