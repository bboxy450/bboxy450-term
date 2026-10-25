"""Real Settings/layout and row/tab pixel checks. The runner owns the app."""
import json
from pathlib import Path
import sys

# -I excludes ambient modules; import only the committed neighbouring drivers.
sys.path.insert(0, str(Path(__file__).resolve().parent))


def r2():
    from ui import App, nodes, require
    app = App()
    first = app.spawn("R2 left")
    second = app.ctl("seat", "spawn", "zsh", "--place", "right",
                     "--cwd", app.work, "--name", "R2 right")["seat"]
    for seat in (first, second):
        app.line(seat, "printf '\\033[?25l\\nR2_READY\\n'; exec cat")
        app.wait(seat, "(?m)^R2_READY$")
    initial = app.ctl("ui", "layout")
    chrome = [node["rect"] for node in initial["tree"][:2]]
    measured = []
    for rim in (2, 4, 8):
        app.ctl("ui", "open", "settings", label=f"r2-settings-{rim}")
        # Search -> navigation; wrap backwards from Account through Shortcuts
        # and Notifications to Appearance. Additional sections precede these.
        page = app.ctl("ui", "key", "tab", "tab", "up", "up", "up")
        require(any(n["kind"] == "named:card0/r2/ctl" for n in nodes(page)),
                "Settings keyboard did not reach Appearance / Rim size")
        app.ctl("ui", "key", "tab", "tab", "tab")
        app.ctl("ui", "key", *("left",) * 6)
        if rim > 2:
            app.ctl("ui", "key", *("right",) * (rim - 2))
        app.ctl("ui", "key", "esc")
        layout = app.ctl("ui", "focus", first, label=f"r2-layout-{rim}")
        require(layout["page"] is None and len(layout["panes"]) == 2,
                "Settings did not return to the original two panes")
        require([node["rect"] for node in layout["tree"][:2]] == chrome,
                "Rim size moved titlebar or sidebar")
        outer = sorted([n["rect"] for n in nodes(layout) if n["kind"] == "named:pane"])
        require(len(outer) == 2, "expected two actual pane borders")
        left, right = outer
        sidebar_right = chrome[1][0] + chrome[1][2]
        gap = right[0] - (left[0] + left[2])
        insets = [left[0] - sidebar_right, layout["size"][0] - right[0] - right[2]]
        require(abs(gap - rim) < .01 and all(abs(v - rim / 2) < .01 for v in insets),
                f"Rim {rim}: gutter={gap}, insets={insets}")
        # Content padding and the title-strip offset do not become extra glow space.
        offsets = []
        for pane in layout["panes"]:
            center = pane["rect"][0] + pane["rect"][2] / 2
            border = next(rect for rect in outer if rect[0] < center < rect[0] + rect[2])
            offsets.append([pane["rect"][0] - border[0], pane["rect"][1] - border[1]])
        if measured:
            require(offsets == measured[0]["content_offsets"], "Rim size changed content padding")
        path = app.capture("window", f"r2-rim-{rim}", layout["frame_seq"])
        measured.append({"rim": rim, "gutter": gap, "insets": insets,
                         "content_offsets": offsets, "layout": layout, "capture": str(path)})
    (app.work / "r2-result.json").write_text(json.dumps(measured, indent=2) + "\n")
    print("R2: real Settings keyboard at 2/4/8 gives gutters 2/4/8 and insets 1/2/4; title/sidebar and content padding unchanged")


def r4(run):
    from rim_clock import nodes, changed, region, geometry
    seats = []
    for name, state in (("active", "working"), ("idle", "idle"), ("peer", "idle")):
        seat = run.shell("R4-" + name, stable=False)
        run.line(seat, "printf '\\033[?25l\\033]7501;state=" + state +
                 "\\007\\nR4-" + name + "-READY\\n'; exec cat")
        run.ctl("seat", "wait", seat, "--text", "(?m)^R4-" + name + "-READY$", "--timeout", "20000")
        seats.append(seat)
    run.ctl("seat", "wait", seats[1], "--state", "idle", "--timeout", "20000")
    states = {row["record"]["id"]: row["state"]["activity"]["kind"]
              for row in run.ctl("seat", "ls")}
    assert [states[seat] for seat in seats] == ["active", "idle", "idle"], "fixture activity differs from the rendered case"
    run.ctl("ui", "focus", seats[2])
    base = run.clock("advance", 900)
    a, ca = run.capture("row-tab-rest")
    later = run.clock("advance", 200)
    b, cb = run.capture("row-tab-alive")
    def rect(frame, name):
        found = nodes(frame, "named:" + name)
        assert len(found) == 1, (name, len(found))
        return found[0]["rect"]
    active_row = rect(base, "swarm0/body/tab0/seat0/row")
    active_tab = rect(base, "swarm0/body/tab0/header")
    idle_row = rect(base, "swarm0/body/tab1/seat0/row")
    idle_tab = rect(base, "swarm0/body/tab1/header")
    def border_band(r):
        x, y, w, h = r
        return (x + 24, y + h - 2, x + w - 24, y + h - 1)
    active_counts = [changed(a, b, border_band(r)) for r in (active_row, active_tab)]
    assert all(count > 0 for count in active_counts), "row/tab has no alive rim at rest"
    assert changed(a, b, border_band(idle_row)) == 0, "unfocused idle row has an alive rim"
    # The idle bottom edge must also be continuous with its own interior, so an
    # accidental static coloured rim cannot pass merely because it did not move.
    x, y, w, h = idle_row
    edge = region(a, (x + 24, y + h - 1, x + w - 24, y + h))
    inside = region(a, (x + 24, y + h - 2, x + w - 24, y + h - 1))
    assert max(abs(u - v) for i, (u, v) in enumerate(zip(edge, inside)) if i % 4 != 3) <= 2, "idle row retains a static rim edge"
    run.ctl("ui", "focus", seats[1])
    origin = run.clock("freeze")
    zero, cz = run.capture("idle-focus-origin")
    peak = run.clock("advance", 450)
    glow, cg = run.capture("idle-focus-peak")
    pane_x = min(n["rect"][0] for n in nodes(origin, "named:pane"))
    idle_row = rect(origin, "swarm0/body/tab1/seat0/row")
    idle_tab = rect(origin, "swarm0/body/tab1/header")
    x, y, w, h = idle_row
    tx, ty, tw, th = idle_tab
    row_start = max(y + h + 4, ty + th + 41)
    tab_end = min(ty - 4, y - 41)
    assert row_start + 4 < y + h + 40 and ty - (tab_end - 4) < 40, "no isolated swell sample fits"
    row_area = (x + 24, row_start, min(x + w - 24, pane_x - 44), row_start + 4)
    tab_area = (tx + 24, tab_end - 4, min(tx + tw - 24, pane_x - 44), tab_end)
    assert tab_area[3] < idle_row[1] - 40, "tab sample could contain the row swell"
    assert row_area[1] > idle_tab[1] + idle_tab[3] + 40, "row sample could contain the tab swell"
    counts = [changed(zero, glow, area) for area in (row_area, tab_area)]
    assert all(count > 0 for count in counts), "row/tab did not independently swell on focus"
    settled = run.clock("advance", 450)
    done, cd = run.capture("idle-focus-settled")
    assert all(changed(zero, done, area) == 0 for area in (row_area, tab_area)), "row/tab swell did not settle"
    assert geometry(origin) == geometry(peak) == geometry(settled), "rim/swell moved pane geometry"
    return {"promise": "R4", "active_rim_pixels": active_counts, "idle_row_static_edge": False,
            "independent_swell_pixels": counts, "row_sample": row_area, "tab_sample": tab_area,
            "captures": [ca, cb, cz, cg, cd], "cards": "not exercised: composer/card selection is not exposed by ctl"}


if __name__ == "__main__":
    if sys.argv[1] == "R2":
        r2()
    else:
        from rim_clock import Run
        run = Run("R4")
        result = None
        try:
            run.start()
            result = r4(run)
        finally:
            run.finish(result)
        print("R4: real rows/tabs carry alive rims and independent focus swells; idle unfocused row has no resting rim; card coverage unproven")
