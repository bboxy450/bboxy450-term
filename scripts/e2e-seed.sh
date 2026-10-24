#!/bin/sh
# The operator's morning e2e session in one command: build the release app, start it with
# --ctl on the real Keychain custody, seat a Codex executor in this repository, and delegate
# the A-0 seeding task to it as the Oracle (decisions #260, #262, #269).
# Usage: sh scripts/e2e-seed.sh            build, start (or reuse) the app, seat, delegate
#        sh scripts/e2e-seed.sh --dry-run  print every step, run nothing
#        sh scripts/e2e-seed.sh verify     the task's gate: 54 untested records, one per promise
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
root=$(pwd)
task=.bboxy/tasks/2026-10-24-a0-seed-e2e.md
spec=.bboxy/specs/e2e-acceptance.md
# One fixed path: Codex approves the seat's hooks once per hook command path (#265).
bin=$root/target/release/bboxy450-term
issuer=${BBOXY_CTL_ISSUER:-Vesper, Oracle}

if [ "${1:-}" = verify ]; then
    # Every `- **<ID>**` line of §1 is a promise; every .bboxy/e2e record must be one of them,
    # once, type e2e, status untested.
    promises=$(sed -n 's/^- \*\*\([A-Z][0-9]*\)\*\* .*/\1/p' "$spec" | sort)
    records=$(for f in .bboxy/e2e/*.md; do
        [ -f "$f" ] || continue
        awk 'NR == 1 && $0 != "---" { exit } NR > 1 && $0 == "---" { exit }
            { gsub(/"/, "") }
            /^type:/ { t = $2 } /^status:/ { s = $2 } /^function:/ { fn = $2 }
            END { print fn, t, s }' "$f"
    done | sort)
    want=$(printf '%s\n' "$promises" | grep -c .)
    have=$(printf '%s\n' "$records" | grep -c . || true)
    bad=$(printf '%s\n' "$records" | awk 'NF && ($2 != "e2e" || $3 != "untested")')
    ids=$(printf '%s\n' "$records" | awk 'NF { print $1 }')
    echo "e2e-seed verify: $have records, $want promises"
    [ -z "$bad" ] || { echo "not type e2e / status untested:"; echo "$bad"; exit 1; }
    [ "$ids" = "$promises" ] || {
        echo "records and promises differ (missing, doubled or invented):"
        printf '%s\n' "$promises" > "${TMPDIR:-/tmp}/e2e-want.$$"
        printf '%s\n' "$ids" | diff "${TMPDIR:-/tmp}/e2e-want.$$" - || true
        rm -f "${TMPDIR:-/tmp}/e2e-want.$$"
        exit 1
    }
    echo "e2e-seed verify: pass"
    exit 0
fi

dry=false
case "${1:-}" in
    --dry-run) dry=true ;;
    "") ;;
    *) echo "usage: sh scripts/e2e-seed.sh [--dry-run | verify]" >&2; exit 2 ;;
esac
step() {
    printf '+ %s\n' "$*"
    if ! $dry; then "$@"; fi
}

number=$(sed -n 's/^number: *\([0-9][0-9]*\)$/\1/p' "$task" | head -1)
[ -n "$number" ] || { echo "e2e-seed: $task has no number" >&2; exit 2; }

# Release build: it always uses the Keychain, whatever BBOXY_TEST_CUSTODY says; unset it so
# nothing suggests otherwise.
unset BBOXY_TEST_CUSTODY
step env CARGO_TARGET_DIR="$root/target" cargo build --release --locked -p bboxy450-term

if ! $dry && "$bin" ctl seats >/dev/null 2>&1; then
    echo "e2e-seed: an app with --ctl is already running; using it"
else
    log=$root/target/e2e-seed-app.log
    printf '+ %s --ctl > %s &\n' "$bin" "$log"
    if ! $dry; then
        "$bin" --ctl > "$log" 2>&1 &
        tries=0
        until "$bin" ctl seats >/dev/null 2>&1; do
            tries=$((tries + 1))
            [ "$tries" -le 120 ] || { echo "e2e-seed: the app did not open ctl in 60 s; see $log" >&2; exit 1; }
            sleep 0.5
        done
    fi
fi

printf '+ %s ctl --as %s spawn codex --role executor --placement new-tab --cwd %s --name e2e-seed\n' "$bin" "'$issuer'" "$root"
if $dry; then
    seat='<seat id>'
else
    spawned=$("$bin" ctl --as "$issuer" spawn codex --role executor --placement new-tab --cwd "$root" --name e2e-seed)
    echo "$spawned"
    seat=$(printf '%s\n' "$spawned" | sed -n 's/.*"seat":"\([0-9A-Z]*\)".*/\1/p')
    [ -n "$seat" ] || { echo "e2e-seed: spawn failed" >&2; exit 1; }
fi

printf '+ %s ctl --as %s delegate %s %s\n' "$bin" "'$issuer'" "'#$number'" "$seat"
if ! $dry; then "$bin" ctl --as "$issuer" delegate "#$number" "$seat"; fi

cat <<EOF

Seated $seat (Codex, executor) and delegated #$number.
Next, at the window:
  1. Codex asks once to approve its hooks for this build path: review and approve all of them.
  2. Watch the seat: it orients, then writes .bboxy/e2e records through the app.
  3. Completion: $bin ctl e2e | tail -1   shows untested 54, missing 0, doubled 0;
     $bin ctl reports                     shows its verify_result (exit 0).
EOF
