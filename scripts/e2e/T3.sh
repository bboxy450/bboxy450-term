# coverage: partial; missing: resizing a pane (only the window is resized); no stale cells after the redraw
# T3: resizing the window resizes the pane's PTY and the program redraws at the new size. The
# layout owns a pane's size under a window (Oracle ruling 2026-10-24): ctl resizes the window
# with app resize, and the shell's stty size must equal the size the pane's screen reports.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
# The window takes a new pane into its layout on its next frame; until it does, a direct
# resize still reaches the PTY. Once laid out, seat resize is refused layout_owned.
tries=0
until out=$(ctl seat resize "$seat" 100x30); [ $? -ne 0 ]; do
    tries=$((tries + 1)); [ "$tries" -le 50 ] || fail "seat resize under a window was still accepted after 10 s: $out"; sleep 0.2
done
printf '%s\n' "$out" | grep -q '"kind":"layout_owned"' || fail "seat resize refused without layout_owned: $out"
# pane SEAT: the seat's screen size as COLSxROWS (rows is the list of visible lines).
pane() { ctl seat screen "$1" | python3 -I -c 'import json,sys; d=json.loads(sys.stdin.readline())["data"]; print(str(d["cols"]) + "x" + str(len(d["rows"])))'; }
# settled SEAT: the pane's size once two reads half a second apart agree.
settled() {
    a=$(pane "$1"); n=0
    while sleep 0.5; b=$(pane "$1"); [ "$a" != "$b" ]; do
        a=$b; n=$((n + 1)); [ "$n" -le 20 ] || fail "the pane never settled (last $b)"
    done
    echo "$b"
}
last=""
for grid in 120x40 80x24; do
    before=$(settled "$seat")
    out=$(ctl app resize "$grid") || fail "app resize $grid refused: $out"
    tries=0
    while :; do
        size=$(pane "$seat")
        [ "$size" != "$before" ] && break
        tries=$((tries + 1)); [ "$tries" -le 50 ] || fail "app resize $grid left the pane at $size"; sleep 0.2
    done
    size=$(settled "$seat")
    cols=${size%x*}; rows=${size#*x}
    run_line "$seat" 'echo size-$(stty size | tr " " x)'
    wait_text "$seat" "(?m)^size-${rows}x${cols}\$"
    [ "$size" != "$last" ] || fail "two window sizes gave the pane the same size $size"
    note "app resize $grid: pane $size, stty ${rows}x${cols}"
    last=$size
done
