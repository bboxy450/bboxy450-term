# coverage: partial; missing: wide glyphs and emoji align to the grid (cell widths and continuations are not read); colours other than red
# T4: real cells: colours, bold, italics, underline, wide glyphs and emoji on the grid; the
# cursor shape follows the program.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" "printf '\\033[1mB\\033[0m \\033[3mI\\033[0m \\033[4mU\\033[0m \\033[31mR\\033[0m 漢x 😀y\\n'"
wait_text "$seat" '(?m)^B I U R 漢x 😀y$'
ctl seat screen "$seat" --cells | python3 -I -c '
import json, sys
data = json.loads(sys.stdin.readline())["data"]
row = next(r for r in data["cells"] if "".join(x["text"] for x in r).startswith("B I U R"))
def flags(ch):
    return next(x for x in row if ch in x["text"])
bold, italic, underline, red = flags("B"), flags("I"), flags("U"), flags("R")
errors = []
if not bold["flags"] & 2: errors.append("B is not bold")
if not italic["flags"] & 4: errors.append("I is not italic")
if not underline["flags"] & 8: errors.append("U is not underlined")
if red["fg"] == {"Named": "Foreground"}: errors.append("R has the default colour")
text = "".join(x["text"] for x in row)
if "漢x" not in text or "😀y" not in text: errors.append("wide glyphs did not keep their cells: " + text)
if errors:
    sys.exit("FAIL: " + "; ".join(errors))' || exit 1
run_line "$seat" "printf '\\033[5 q'; echo cursor-set"
wait_text "$seat" '(?m)^cursor-set$'
shape=$(ctl seat screen "$seat" | jget data.cursor_style)
echo "$shape" | grep -qi 'beam\|bar' || fail "the cursor shape did not follow the program: $shape"
