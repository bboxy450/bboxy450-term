# coverage: full
# harness: installed clients; model: scripted localhost (spec 0177a9cb)
. "$(dirname "$0")/lib.sh"
python3 -I -B "$(dirname "$0")/first_turn.py"
[ "$?" -eq 0 ] || fail 'L4 native first-turn routing or report assertion failed'
