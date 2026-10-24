# T2: typing reaches the program and echoes; a 100 kB paste arrives intact.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" 'echo typed-$((6*7))'
wait_text "$seat" '(?m)^typed-42$'
src="$E2E_WORK/paste.src"
python3 -I -c '
import sys
line = "0123456789abcdefghijklmnopqrstuvwxyz-ABCDEFGHIJKLMNOPQRSTUVWXYZ+0123456789:.\n"
sys.stdout.write((line * 1300)[:100000])' > "$src"
run_line "$seat" "cat > '$E2E_WORK/paste.out'"
sleep 0.5
out=$(ctl seat paste "$seat" "$src") || fail "paste refused: $out"
ctl seat key "$seat" enter ctrl-d >/dev/null || fail "ctrl-d refused"
run_line "$seat" "wc -c < '$E2E_WORK/paste.out' | tr -d ' ' | sed 's/^/bytes-/'"
wait_text "$seat" '(?m)^bytes-[0-9]+$' 60000
python3 -I -c '
import sys
a = open(sys.argv[1], "rb").read()[:100000]; b = open(sys.argv[2], "rb").read()
sys.exit(0 if a == b else 1)' "$E2E_WORK/paste.out" "$src" || fail "the 100 kB paste did not arrive intact ($(wc -c < "$E2E_WORK/paste.out") bytes)"
