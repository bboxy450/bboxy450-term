# T3: resizing resizes the PTY and the program redraws at the new size.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
for size in 100x30 72x20; do
    ctl seat resize "$seat" "$size" >/dev/null || fail "resize to $size refused"
    cols=${size%x*}; rows=${size#*x}
    run_line "$seat" 'echo size-$(stty size | tr " " x)'
    wait_text "$seat" "(?m)^size-${rows}x${cols}\$"
    [ "$(ctl seat screen "$seat" | jget data.cols)" = "$cols" ] || fail "the screen is not $cols columns after resize"
done
