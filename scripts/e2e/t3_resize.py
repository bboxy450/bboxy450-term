"""Real SIGWINCH and complete-grid pixel assertions, with no OS input."""
import json
import math
import os
from pathlib import Path
import select
import shlex
import signal
import subprocess
import sys
import termios
import time
import tty

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ui import App, pane, require

OLD = [180, 30, 80]
NEW = [12, 86, 156]


def program(directory):
    require(os.isatty(0) and os.isatty(1), "resize fixture requires its real seat PTY")
    directory = Path(directory)
    saved = termios.tcgetattr(0)
    reader, writer = os.pipe()
    os.set_blocking(writer, False)
    prior_fd = signal.set_wakeup_fd(writer)
    prior_handler = signal.signal(signal.SIGWINCH, lambda *_: None)
    paints = signals = 0

    def paint(kind):
        nonlocal paints
        size = os.get_terminal_size(0)
        rows, cols = map(int, subprocess.check_output(["stty", "size"], stdin=0).split())
        require((cols, rows) == (size.columns, size.lines), "stty and tty ioctl disagree")
        char, bg = ("A", OLD) if kind == "sentinel" else (" ", NEW)
        text = "\x1b[0m\x1b[?25l\x1b[?7l\x1b[38;2;255;255;255m" + f"\x1b[48;2;{bg[0]};{bg[1]};{bg[2]}m"
        text += "".join(f"\x1b[{row + 1};1H" + char * cols for row in range(rows))
        data = memoryview(text.encode())
        while data:
            data = data[os.write(1, data):]
        paints += 1
        state = {"paint": paints, "kind": kind, "signals": signals, "cols": cols,
                 "rows": rows, "stty": [cols, rows], "bg": bg,
                 "pid": os.getpid(), "tty": os.ttyname(0)}
        temp = directory / "t3-state.tmp"
        temp.write_text(json.dumps(state) + "\n")
        temp.replace(directory / "t3-state.json")

    try:
        tty.setraw(0)
        paint("sentinel")
        while True:
            ready, _, _ = select.select([0, reader], [], [])
            if reader in ready:
                signals += os.read(reader, 4096).count(signal.SIGWINCH)
                paint("clear")
            if 0 in ready:
                command = os.read(0, 1024)
                if not command or b"q" in command:
                    break
                if b"a" in command:
                    paint("sentinel")
    finally:
        signal.set_wakeup_fd(prior_fd)
        signal.signal(signal.SIGWINCH, prior_handler)
        os.close(reader)
        os.close(writer)
        termios.tcsetattr(0, termios.TCSANOW, saved)


def observed(app, seat, after, kind):
    deadline = time.monotonic() + 45
    while True:
        path = app.work / "t3-state.json"
        state = json.loads(path.read_text()) if path.exists() else None
        if state is None:
            screen = app.ctl("seat", "screen", seat)
            text = "\n".join(screen["rows"])
            require("Traceback (most recent call last)" not in text,
                    "resize fixture failed before readiness: " + text)
        if state and state["paint"] > after and state["kind"] == kind:
            layout = app.ctl("ui", "layout")
            visible = pane(layout, seat)
            source = visible.get("rendered")
            screen = app.ctl("seat", "screen", seat, "--cells")
            cols, rows = state["cols"], state["rows"]
            char = "A" if kind == "sentinel" else ""
            expected = [char * cols] * rows
            rgb = dict(zip(("r", "g", "b"), state["bg"]))
            if (source and source.get("pixel_size") and
                    [visible["cols"], visible["rows"]] == state["stty"] == [screen["cols"], len(screen["rows"])] and
                    source["view"]["generation"] == screen["generation"] and
                    source["sequence"] >= screen["sequence"] and
                    source["lines"] == screen["rows"] == expected and
                    all(cell["bg"] == {"Spec": rgb} for row in screen["cells"] for cell in row)):
                return layout, state
        require(time.monotonic() < deadline, f"resize/redraw did not converge: {state}")
        time.sleep(0.01)  # condition watchdog, never a settling-time assertion


def pixels(app, seat, layout, state, label):
    from rim_clock import png, region

    path = app.capture("window", label, layout["frame_seq"])
    require(json.loads((app.work / "t3-state.json").read_text())["paint"] == state["paint"],
            "program repainted while its capture was being observed")
    (app.work / f"{label}-source.json").write_text(json.dumps({
        "layout": layout, "program": state,
        "capture": json.loads((app.work / f"{label}.jsonl").read_text()),
    }, indent=2) + "\n")
    image = png(path)
    require(list(image[:2]) == list(layout["size"]), "capture is not the headless 1:1 frame")
    visible = pane(layout, seat)
    x, y, w, h = visible["rect"]
    x, y = math.floor(x + 0.5), math.floor(y + 0.5)
    width, height = visible["rendered"]["pixel_size"]
    require(0 < width <= math.floor(w) and 0 < height <= math.floor(h), "prepared grid overhangs its pane")
    cols, rows = state["cols"], state["rows"]
    require(width % cols == 0 and height % rows == 0, "grid pixels do not divide into its actual cells")
    cell_w, cell_h = width // cols, height // rows
    data = region(image, [x, y, x + width, y + height])
    if state["kind"] == "sentinel":
        ink = 0
        for row in range(rows):
            for col in range(cols):
                cell = region(image, [x + col * cell_w, y + row * cell_h,
                                      x + (col + 1) * cell_w, y + (row + 1) * cell_h])
                ink += any(min(cell[i:i + 3]) > 200 for i in range(0, len(cell), 4))
        require(ink == cols * rows, f"old sentinel was not drawn in every cell: {ink}/{cols * rows}")
        return {"sentinel_ink_cells": ink, "grid_pixels": width * height}
    bad = sum(any(abs(data[i + c] - NEW[c]) > 2 for c in range(3)) or data[i + 3] != 255
              for i in range(0, len(data), 4))
    require(bad == 0, f"{bad} stale or incorrectly repainted pixels in the resized grid")
    body = region(image, [x, y, x + math.floor(w), y + math.floor(h)])
    stale = sum((body[i] > 100 and body[i + 1] < 70 and body[i + 2] < 110) or
                min(body[i:i + 3]) > 200 for i in range(0, len(body), 4))
    require(stale == 0, f"{stale} old ink/background pixels remain in the pane body")
    return {"grid_pixels": width * height, "wrong_grid_pixels": bad, "stale_body_pixels": stale}


def main():
    app = App()
    app.ctl("ui", "clock", "freeze")
    seat = app.spawn("T3 SIGWINCH redraw")
    app.line(seat, shlex.join(["exec", "python3", "-I", "-B", str(Path(__file__).resolve()),
                              "--program", str(app.work)]))
    layout, state = observed(app, seat, 0, "sentinel")
    refusal = app.ctl("seat", "resize", seat, "100x30", refused=True)
    require(refusal["kind"] == "layout_owned", "the window lost ownership of PTY sizing")
    records = []
    for name, grid in (("window-shrink", "80x24"), ("window-grow", "120x40"), ("pane-split", None)):
        previous_paint = state["paint"]
        app.ctl("seat", "type", seat, "a")
        before, seeded = observed(app, seat, previous_paint, "sentinel")
        old = pixels(app, seat, before, seeded, "t3-" + name + "-before")
        if grid:
            app.ctl("app", "resize", grid)
        else:
            app.ctl("seat", "spawn", "zsh", "--place", "right", "--cwd", app.work,
                    "--name", "T3 sibling")
            app.ctl("ui", "focus", seat)
        after, state = observed(app, seat, seeded["paint"], "clear")
        require(state["signals"] > seeded["signals"], "actual SIGWINCH was not received")
        require([pane(before, seat)["cols"], pane(before, seat)["rows"]] != state["stty"],
                "the action did not resize the actual pane PTY")
        require((before["size"] != after["size"]) == bool(grid),
                "pane-only and window resize origins were not independent")
        repaint = pixels(app, seat, after, state, "t3-" + name + "-after")
        row = {"action": name, "before": [seeded["cols"], seeded["rows"]],
               "after": state["stty"], "signals": state["signals"] - seeded["signals"],
               "frame_seq": after["frame_seq"], **old, **repaint}
        records.append(row)
        print(json.dumps(row), flush=True)
    (app.work / "t3-observations.json").write_text(json.dumps(records, indent=2) + "\n")
    print("T3: window and pane resizes deliver SIGWINCH/stty size; every new-grid pixel is repainted")


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--program":
        program(sys.argv[2])
    else:
        main()
