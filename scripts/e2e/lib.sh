# Shared helpers for the e2e promise scripts (e2e-acceptance §4a, #282). Each script drives
# a running app through ctl only; its exit code is the verdict. The runner sets BBT (the
# app binary), BBOXY_RUN_DIR, E2E_WORK (a private directory) and E2E_APP (the app's pid).
set -u
ctl() { "$BBT" ctl "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
note() { echo "note: $*" >&2; }
# jget PATH: one value from the first JSON line on stdin (dots for keys, numbers for indexes).
jget() {
    python3 -I -c '
import json, sys
d = json.loads(sys.stdin.readline())
for k in sys.argv[1].split("."):
    d = d[int(k)] if k.isdigit() else d[k]
print(d if not isinstance(d, (dict, list)) else json.dumps(d))' "$1"
}
# spawn_zsh [NAME]: a new zsh seat in a new tab, working in E2E_WORK; prints its id.
spawn_zsh() {
    if [ -n "${1:-}" ]; then
        out=$(ctl seat spawn zsh --place tab --cwd "$E2E_WORK" --name "$1")
    else
        out=$(ctl seat spawn zsh --place tab --cwd "$E2E_WORK")
    fi || { echo "spawn refused: $out" >&2; return 1; }
    printf '%s\n' "$out" | jget data.seat
}
# wait_text SEAT REGEX [MS]
wait_text() {
    ctl seat wait "$1" --text "$2" --timeout "${3:-20000}" >/dev/null ||
        fail "the screen never showed /$2/: $(ctl seat screen "$1" --human | grep . | tail -5)"
}
# run_line SEAT TEXT: types the text and presses enter.
run_line() {
    out=$(ctl seat type "$1" "$2") || fail "typing into $1 was refused: $out"
    out=$(ctl seat key "$1" enter) || fail "enter into $1 was refused: $out"
}
# seat_field SEAT PATH: one field of the seat's record or state from seat ls.
seat_field() {
    ctl seat ls | python3 -I -c '
import json, sys
seats = json.loads(sys.stdin.readline())["data"]
s = [x for x in seats if x.get("record", {}).get("id") == sys.argv[1]]
d = s[0] if s else {}
for k in sys.argv[2].split("."):
    d = d.get(k, "") if isinstance(d, dict) else ""
print(d if not isinstance(d, (dict, list)) else json.dumps(d))' "$1" "$2"
}
# pid NAME: the app's current logic or supervisor process id: the app's child running
# `<binary> NAME`, read live, so a restarted one is found under its new pid.
pid() { ps -axo pid=,ppid=,command= | awk -v app="$E2E_APP" -v name="$1" '$2 == app && $NF == name { print $1 }' | tail -1; }
