"""L1: actual launcher input, produced text/bounds, and independent blur pixels."""
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import shlex
import subprocess
import sys

spec = importlib.util.spec_from_file_location("night_ui", Path(__file__).with_name("ui.py"))
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)
require = ui.require


def picker(layout):
    panels = [node for node in ui.nodes(layout) if node["kind"] == "named:picker"]
    require(layout["launcher_open"] and len(panels) == 1, "launcher has no produced panel")
    return panels[0]


def walk(node):
    yield node
    for child in node.get("children", []):
        yield from walk(child)


def rows(layout):
    panel = picker(layout)
    x, y, width, height = panel["rect"]
    result = []
    for node in walk(panel):
        if not re.fullmatch(r"named:picker/div/opt\d*", node["kind"]):
            continue
        rx, ry, rw, rh = node["rect"]
        require(rw > 0 and rh > 0 and rx >= x and ry >= y
                and rx + rw <= x + width + 1 and ry + rh <= y + height + 1,
                "a launcher row is outside the visible panel")
        texts = [n["text"] for n in walk(node) if n["kind"] == "text" and "text" in n]
        require(texts, "produced launcher row omitted its text")
        result.append(texts)
    return result


def pattern(app, seat):
    source = app.work / "launcher-pattern.py"
    source.write_text(
        "import os,sys,termios\n"
        "t=termios.tcgetattr(0);t[3]&=~termios.ECHO;termios.tcsetattr(0,termios.TCSANOW,t)\n"
        "w,h=os.get_terminal_size();out=['\\x1b[2J\\x1b[?25l']\n"
        "for y in range(1,h+1):\n"
        " out.append(f'\\x1b[{y};1H')\n"
        " for x in range(w):\n"
        "  v=235 if (x//6+y//4)%2 else 20\n"
        "  out.append(f'\\x1b[48;2;{v};{v};{v}m ')\n"
        "out.append('\\x1b[0m\\x1b[2;2HL1_PATTERN_READY')\n"
        "sys.stdout.write(''.join(out));sys.stdout.flush()\n"
        "for line in sys.stdin: pass\n")
    app.line(seat, "exec python3 -I " + shlex.quote(str(source)))
    app.wait(seat, "L1_PATTERN_READY")


def l1(app):
    seat = app.spawn("L1 existing pane")
    app.ctl("app", "resize", "200x60")
    pattern(app, seat)
    original = {row["record"]["id"] for row in app.ctl("seat", "ls")}
    before = app.ctl("ui", "clock", "freeze", label="l1-before-layout")
    before_png = app.capture("window", "l1-before", ui.frame(before))
    opened = app.ctl("ui", "key", "cmd-t", label="l1-open-layout")
    panel = picker(opened)
    x, y, width, height = panel["rect"]
    require(ui.frame(opened) > ui.frame(before), "launcher reply predates its frame")
    require(width > 0 and height > 0 and x >= 0 and y >= 0, "launcher has no visible bounds")
    require(abs(x + width / 2 - opened["size"][0] / 2) < 1,
            "launcher is not horizontally centred")
    require(y + height <= opened["size"][1] + 1, "launcher extends below the window")
    root = rows(opened)
    for label in ("Claude", "Codex", "pi", "omp", "zsh"):
        require(any(label in row for row in root), f"no visible harness row {label}: {root}")
    opened_png = app.capture("window", "l1-open", ui.frame(opened))
    proof = {"size": opened["size"], "picker": panel["rect"], "pane": ui.pane(before, seat)["rect"],
             "before": str(before_png), "after": str(opened_png), "theme": "dark", "density": "cozy"}
    proof_path = app.work / "l1-blur-input.json"
    proof_path.write_text(json.dumps(proof, indent=2) + "\n")

    roles = app.ctl("ui", "key", "right", label="l1-roles")
    role_rows = rows(roles)
    presets = ("Oracle", "Architect", "Creativity", "Executor")
    for role in presets:
        require(any(role in row for row in role_rows), f"no visible Claude preset {role}: {role_rows}")
    for index, role in enumerate(presets):
        if index:
            app.ctl("ui", "key", "cmd-t", "right")
        chosen = app.ctl("ui", "key", *(["down"] * index), "right", label=f"l1-preset-{role.lower()}")
        labels = [n["text"] for n in walk(picker(chosen)) if n["kind"] == "text" and "text" in n]
        require("Claude ›" in labels and role + " ›" in labels and rows(chosen),
                f"selecting {role} did not open its placement level: {labels}")
        escaped = app.ctl("ui", "key", "esc", "esc", "esc")
        require(not escaped["launcher_open"], f"Esc did not close the {role} preset path")
        require({row["record"]["id"] for row in app.ctl("seat", "ls")} == original,
                f"inspecting the {role} preset spawned a seat")
    app.ctl("ui", "key", "cmd-t")
    searched = app.ctl("ui", "key", *"codex", label="l1-search-codex")
    found = rows(searched)
    require(any(n.get("text") == "codex" for n in walk(picker(searched))),
            "the produced query field did not paint the typed search")
    require(len(found) < len(root) and any("Codex" in row for row in found),
            f"query did not filter to Codex: {found}")
    require(not any("Claude" in row or "zsh" in row for row in found),
            f"query retained unrelated harnesses: {found}")
    app.ctl("ui", "key", *(["backspace"] * len("codex")))
    empty = app.ctl("ui", "key", *"zzzzzz", label="l1-search-empty")
    require(rows(empty) == [], "a query with no matches still painted result rows")
    restored = app.ctl("ui", "key", *(["backspace"] * len("zzzzzz")))
    require(rows(restored) == root, "clearing search did not restore the root harnesses")
    closed = app.ctl("ui", "key", "esc", label="l1-close")
    require(not closed["launcher_open"] and not any(n["kind"] == "named:picker" for n in ui.nodes(closed)),
            "Esc left the launcher open")
    require(ui.frame(closed) > ui.frame(restored), "Esc returned an old frame")
    require({row["record"]["id"] for row in app.ctl("seat", "ls")} == original,
            "launcher inspection or Esc spawned a seat")
    require(closed["focused"] == before["focused"] and closed["tab"] == before["tab"],
            "launcher inspection changed the focus anchor")
    # Reuse the approved CSS filter over exactly the actual pre-launcher bitmap.
    browser = subprocess.Popen(["node", str(Path(__file__).with_name("launcher-blur.mjs")), str(proof_path)],
                               start_new_session=True)
    try:
        code = browser.wait(timeout=65)
    except subprocess.TimeoutExpired:
        # This group was created above for this helper and its private browser only.
        require(os.getpgid(browser.pid) == browser.pid, "blur helper lost its owned process group")
        os.killpg(browser.pid, signal.SIGTERM)
        try:
            browser.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(browser.pid, signal.SIGKILL)
            browser.wait()
        raise
    require(code == 0, f"approved blur/negative-control comparison exited {code}")
    print("L1: centred launcher, visible harness labels and role presets, positive/empty/cleared "
          "search, Esc/no spawn/focus retained; approved blur pixels and no-blur negative control")


if __name__ == "__main__":
    try:
        l1(ui.App())
    except (AssertionError, KeyError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"FAIL: L1: {error}", file=sys.stderr)
        sys.exit(1)
