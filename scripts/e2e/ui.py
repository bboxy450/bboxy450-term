"""Assertions for the UI night scripts. Only the app's ctl socket receives input.

This reads produced-layout receipts, not a second terminal or a reconstructed UI.
Historical cell/heap/paint observations missing from ctl remain coverage gaps.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def require(condition, reason):
    if not condition:
        raise AssertionError(reason)


def pane(layout, seat):
    rows = [p for p in layout["panes"] if p["seat"] == seat]
    require(len(rows) == 1, f"{seat}: expected one produced pane, got {len(rows)}")
    return rows[0]


def nodes(layout):
    def walk(node):
        yield node
        for child in node.get("children", []):
            yield from walk(child)
    for root in layout["tree"]:
        yield from walk(root)


def frame(layout):
    seq = layout["frame_seq"]
    require(type(seq) is int and seq > 0, "no produced-frame receipt")
    return seq


class App:
    def __init__(self):
        self.binary = os.environ["BBT"]
        self.work = Path(os.environ["E2E_WORK"])
        require(os.environ["BBOXY_CTL_ISSUER"].startswith("test-account:"),
                "UI night scripts require the runner's verified throwaway account")
        self.work.mkdir(parents=True, exist_ok=True)

    def ctl(self, *args, label=None, refused=False):
        result = subprocess.run([self.binary, "ctl", *map(str, args)],
                                capture_output=True, text=True, timeout=40,
                                stdin=subprocess.DEVNULL)
        if label:
            (self.work / f"{label}.jsonl").write_text(result.stdout)
        rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        require(len(rows) == 1, f"{args}: expected one reply: {result.stdout[:1000]}")
        reply = rows[0]
        if refused:
            require(result.returncode != 0 and reply.get("ok") is False,
                    f"{args}: expected a structured refusal, got ok={reply.get('ok')} frame={reply.get('data', {}).get('frame_seq')}")
            return reply["error"]
        require(result.returncode == 0 and reply.get("ok") is not False,
                f"{args}: {result.returncode}: {result.stdout[:1500]} {result.stderr[:500]}")
        return reply["data"]

    def spawn(self, name):
        seat = self.ctl("seat", "spawn", "zsh", "--place", "tab", "--cwd", self.work,
                        "--name", name)["seat"]
        layout = self.ctl("ui", "focus", seat)
        require(layout["focused"] == seat, "focus receipt names another pane")
        pane(layout, seat)
        return seat

    def line(self, seat, text):
        self.ctl("seat", "type", seat, text)
        self.ctl("seat", "key", seat, "enter")

    def wait(self, seat, text):
        self.ctl("seat", "wait", seat, "--text", text, "--timeout", "30000")

    def capture(self, seat, label, after):
        path = self.work / f"{label}.png"
        data = self.ctl("seat", "capture", seat, path, label=label)
        require(data["frame_seq"] >= after, "capture predates the action's frame")
        require(path.read_bytes()[:8] == b"\x89PNG\r\n\x1a\n", "capture is not a PNG")
        return path

    def fixture(self, seat, count):
        # Reset before numbered output; displayed cells still need a public observer.
        # Keep the process alive and echo a nonce after returning to the live tail.
        script = self.work / "numbered.py"
        script.write_text("import sys\n"
                          "sys.stdout.write('\\x1bc')\n"
                          f"for n in range(1, {count + 1}): print(f'line {{n:05}}')\n"
                          "print('NUMBERED_DONE', flush=True)\n"
                          "for line in sys.stdin: print('LIVE-' + line.strip(), flush=True)\n")
        import shlex
        self.line(seat, "exec python3 -I " + shlex.quote(str(script)))
        self.wait(seat, "(?m)^NUMBERED_DONE$")


def s1(app):
    seat = app.spawn("S1 numbered history")
    app.fixture(seat, 20_000)
    top = app.ctl("ui", "scroll", seat, "top", label="s1-top")
    require(pane(top, seat)["scroll"] == {"top_row": 0, "live": False},
            "line 1 must have wire coordinate zero")
    app.capture(seat, "s1-top", frame(top))
    # Cover each absolute row coordinate once, in order. Viewports can overlap at
    # the tail: only their not-yet-covered suffix belongs to the next interval.
    # These are coordinates, NOT a claim that ctl read the GUI's displayed bytes.
    covered, previous_frame, steps = 0, 0, 0
    layout = top
    with (app.work / "s1-traversal.jsonl").open("w") as log:
        while True:
            view = pane(layout, seat)
            at, live = view["scroll"]["top_row"], view["scroll"]["live"]
            require(type(at) is int and at >= 0, "indexed page has no absolute row")
            require(frame(layout) > previous_frame, "scroll reused an older produced frame")
            require(at <= covered < at + view["rows"], "a page skipped or repeated its row interval")
            end = min(20_000, at + view["rows"])
            require(end > covered, "paging made no coordinate progress")
            log.write(json.dumps({"from": covered, "to_exclusive": end,
                                  "top_row": at, "live": live,
                                  "frame_seq": frame(layout)}) + "\n")
            covered, previous_frame = end, frame(layout)
            steps += 1
            if covered == 20_000:
                break
            require(not live and steps < 20_000, "live tail reached before row 20000")
            layout = app.ctl("ui", "scroll", seat, "row", covered)
    bottom = app.ctl("ui", "scroll", seat, "bottom", label="s1-bottom")
    require(pane(bottom, seat)["scroll"]["live"], "bottom did not follow the live tail")
    require(frame(bottom) > previous_frame, "bottom returned an old frame")
    app.capture(seat, "s1-bottom", frame(bottom))
    app.line(seat, "S1_TAIL_NONCE")
    app.wait(seat, "(?m)^LIVE-S1_TAIL_NONCE$")
    print(f"S1: 20000 row coordinates exactly once/in order in {steps} produced pages; "
          "top=0, live join and captures; displayed bytes/lazy-page metrics unobserved")


def history_sample(app, label):
    data = app.ctl("debug", "metrics", label=label)
    require(data["ui"]["available"], "GUI history metrics unavailable")
    history = data["ui"]["history"]
    require(history["available"] and history["complete"], "history sample omitted panes")
    require(history["omitted_panes"] == 0, "incomplete history aggregate")
    require(type(history["retained_history_bytes"]) is int, "heap is unobserved")
    for row in history["panes"]:
        require(type(row["retained_history_bytes"]) is int, "pane heap is unobserved")
        require(0 < row["retained_history_bytes"] <= row["budget_bytes"] <= history["per_pane_budget_bytes"],
                "history pane exceeds its documented budget")
        require(0 <= row["pages_resident"] <= 1, "more than one input page resident")
        require(row["pages_fetched"] == row["pages_evicted"] + row["pages_resident"],
                "page lifetime counters contradict residency")
    require(history["retained_history_bytes"] >= sum(r["retained_history_bytes"] for r in history["panes"]),
            "aggregate understates its retained pane heaps")
    return history


def s2(app):
    import shlex
    app.ctl("ui", "clock", "freeze")
    seat = app.spawn("S2 retained gap")
    script = app.work / "gap.py"
    script.write_text("import sys, termios\n"
                      "t=termios.tcgetattr(0); t[3] &= ~termios.ECHO; termios.tcsetattr(0,termios.TCSANOW,t)\n"
                      "sys.stdout.write('\\x1bc')\n"
                      "for n in range(1,5): print(f'S2_BEFORE_{n:02}')\n"
                      "print('S2_READY_A',flush=True)\n"
                      "assert input()=='hole'\n"
                      "for n in range(1,4): print(f'S2_LOST_{n:02}')\n"
                      "print('S2_READY_B',flush=True)\n"
                      "assert input()=='after'\n"
                      "for n in range(1,101): print(f'S2_AFTER_{n:03}')\n"
                      "print('S2_READY_C',flush=True)\n"
                      "for line in sys.stdin: print('S2_LIVE-'+line.strip(),flush=True)\n")
    app.line(seat, "exec python3 -I " + shlex.quote(str(script)))
    app.wait(seat, "(?m)^S2_READY_A$")
    before = app.ctl("seat", "screen", seat, label="s2-before")
    app.line(seat, "hole")
    app.wait(seat, "(?m)^S2_READY_B$")
    middle = app.ctl("seat", "screen", seat, label="s2-middle-live")
    first, last = before["sequence"] + 1, middle["sequence"]
    require(last >= first, "capture range did not advance")
    require(any("S2_LOST_01" in str(r) for r in middle["rows"]), "live capture lost original bytes")
    app.line(seat, "after")
    app.wait(seat, "(?m)^S2_READY_C$")
    end = app.ctl("seat", "screen", seat)["sequence"]
    installed = app.ctl("debug", "fault", "ring-gap", seat, f"{first}..{last}", label="s2-mask")
    require((installed["from"], installed["to"]) == (first,last), "mask range changed")
    top = app.ctl("ui", "scroll", seat, "top", label="s2-top")
    require(not pane(top,seat)["scroll"]["live"], "gap fixture has no history")
    sample = history_sample(app,"s2-metrics")
    row = next(r for r in sample["panes"] if r["seat"] == seat)
    require(row["gap_markers"] == [{"row":5,"from":first,"to":last}],
            f"gap was duplicated or moved from boundary row5: {row['gap_markers']}")
    require(row["omitted_gap_markers"] == 0, "gap observations truncated")
    replay = app.ctl("seat","screen",seat,"--scrollback",f"1..{end}",label="s2-projected-ring")
    rows = replay["rows"]
    require(sum("gap" in str(r).lower() for r in rows)==1, "retained read did not report one gap")
    text = "\n".join(str(r) for r in rows)
    require("S2_BEFORE_04" in text and "S2_AFTER_001" in text, "surviving side was lost")
    require("S2_LOST_" not in text, "mask silently reintroduced missing bytes")
    visible = pane(top, seat)
    historical = visible.get("historical")
    require(historical is not None and historical.get("lines") is not None,
            "produced frame has no observed historical cells")
    lines = historical["lines"]
    require(historical["top_row"] == 0 and lines[:7] == [
        "S2_BEFORE_01", "S2_BEFORE_02", "S2_BEFORE_03", "S2_BEFORE_04",
        "S2_READY_A", "", "S2_AFTER_001"], f"wrong rendered cells at boundary: {lines[:8]}")
    require(not any("S2_LOST_" in line for line in lines), "lost cells reached the GPU source")
    path = app.capture(seat,"s2-gap",frame(top))
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from rim_clock import png, region
    from collections import Counter
    image = png(path)
    cell_height = historical["pixel_size"][1] / len(lines)
    require(cell_height.is_integer(), "historical grid has fractional device cells")
    height = int(cell_height)
    # The prepared cell row is empty. Only the actual foreground gap caption
    # can put ink here; a missing or one-row-shifted marker leaves it blank.
    area = [2, 5 * height, min(image[0], historical["pixel_size"][0]) - 2, 6 * height]
    pixels = region(image, area)
    background = Counter(pixels[i:i+4] for i in range(0,len(pixels),4)).most_common(1)[0][0]
    def marker_ink(raw):
        return sum(raw[i:i+4] != background for i in range(0,len(raw),4))
    require(marker_ink(pixels) > 20, "gap marker has no rendered ink at boundary row5")
    stride = (area[2] - area[0]) * 4
    require(sum(marker_ink(pixels[y*stride:(y+1)*stride]) > 0 for y in range(height)) >= 4,
            "gap caption is missing; only a horizontal rule was rendered")
    require(marker_ink(background * (len(pixels)//4)) == 0,
            "blank-row negative control was accepted")
    evidence = Path(__file__).resolve().parents[2] / "target" / "history-gap"
    evidence.mkdir(parents=True, exist_ok=True)
    import shutil
    retained = evidence / f"{seat}.png"
    shutil.copyfile(path, retained)
    proof = evidence / f"{seat}.json"
    proof.write_text(json.dumps({
        "frame_seq":frame(top), "historical":historical, "marker_device_rect":area,
        "marker_ink_pixels":marker_ink(pixels), "capture":str(retained),
    }, indent=2) + "\n")
    app.ctl("ui","scroll",seat,"bottom")
    app.line(seat,"NONCE")
    app.wait(seat,"(?m)^S2_LIVE-NONCE$")
    require(app.ctl("seat","screen",seat)["generation"] == middle["generation"],
            "gap replaced the live process")
    print("S2: gap once at row5; actual produced historical cells and captured marker ink at that empty row; both surviving sides and live input preserved. Torn-ring production: native torn_ring_suffix_projects_one_gap_at_the_boundary_row witness.")
    print(f"S2 retained proof: {proof}; marker_ink_pixels={marker_ink(pixels)}")


def s3(app):
    # Three sequential real viewports. Changing tabs releases the old watch;
    # this deliberately does not claim a 50-watched simultaneous setup.
    for n in range(3):
        seat = app.spawn(f"S3 budget {n}")
        app.fixture(seat,20_000)
        app.ctl("ui","scroll",seat,"top")
        before = history_sample(app,f"s3-{n}-top-metrics")
        require(len(before["panes"]) == 1 and before["panes"][0]["seat"] == seat,
                "active Playback owner missing")
        fetched = before["panes"][0]["pages_fetched"]
        require(fetched>0, "top fetched no page")
        require(before["retained_history_bytes"] <= 4_000_000, "aggregate heap exceeds budget")
        app.ctl("ui","scroll",seat,"row",1_500)
        after = history_sample(app,f"s3-{n}-seek-metrics")
        require(after["retained_history_bytes"] <= 4_000_000, "seek exceeded aggregate heap budget")
        row = next(r for r in after["panes"] if r["seat"]==seat)
        require(row["pages_fetched"] > fetched, "seek fetched no bounded page")
        require(row["pages_evicted"] > 0, "consumed page payloads were not released")
        app.ctl("ui","scroll",seat,"bottom")
        released = history_sample(app,f"s3-{n}-released-metrics")
        require(released["panes"] == [], "return-live retained Playback pages")
        require(released["retained_history_bytes"] < before["retained_history_bytes"], "history heap was not released")
    print("S3: three sequential actual GUI Playback owners below4MB each/aggregate, bounded page residency/eviction and release on live; 50-watched setup and paging hitch meter unproved")


def s4(app):
    seat = app.spawn("S4 history admission")
    app.ctl("app", "resize", "512x64", label="s4-resize")
    layout = app.ctl("ui", "focus", seat)
    view = pane(layout, seat)
    # Use a real core-valid viewport. No giant synthetic Screen or ctl seat-resize
    # bypass: the window owns the grid and its live program sees these dimensions.
    require(2 <= view["cols"] <= 768 and 1 <= view["rows"] <= 300,
            "fixture is outside supported live dimensions")
    # Resize setup must reach the terminal Replica too. A solved pane rectangle
    # alone does not prove that the queued PTY/GUI resize has been applied.
    for _ in range(100):
        screen = app.ctl("seat", "screen", seat)
        if screen["cols"] == view["cols"] and len(screen["rows"]) == view["rows"]:
            break
        time.sleep(0.1)
    else:
        raise AssertionError(f"S4 fixture_dimensions_unavailable: pane={view['cols']}x{view['rows']} "
                             f"Replica={screen['cols']}x{len(screen['rows'])}; live grid caps cells at 32768")
    app.line(seat, "for i in {1..1000}; do printf 'S4_%04d\\n' $i; done; printf 'S4_BEFORE\\n'")
    app.wait(seat, "(?m)^S4_BEFORE$")
    generation = app.ctl("seat", "screen", seat)["generation"]
    # #83 admits supported large panes with a dimension-scaled budget.
    app.ctl("ui", "scroll", seat, "top", label="s4-supported")
    sample = history_sample(app, "s4-heap")
    require(any(p["seat"] == seat for p in sample["panes"]), "supported pane has no heap sample")
    probe = app.ctl("debug", "history-admission", "2048x128", label="s4-refusal")
    require(probe["cells"] == 2048 * 128 and probe["core_retained_heap_bytes"] > 0,
            "over-budget fixture has no real core cells")
    require(probe["refusal"] == "history_viewport_exceeds_budget", f"unrelated refusal: {probe}")
    require(probe["retained_heap_bytes"] == 0, "refused fixture retained Playback heap")
    app.ctl("ui", "scroll", seat, "bottom")
    app.line(seat, "printf 'S4_AFTER\\n'")
    app.wait(seat, "(?m)^S4_AFTER$")
    screen = app.ctl("seat", "screen", seat, label="s4-live")
    require(screen["generation"] == generation, "history refusal replaced the live process")
    layout = app.ctl("ui", "layout", label="s4-layout")
    require(pane(layout, seat)["scroll"]["live"], "history refusal stranded the live pane")
    print(f"S4: supported live {view['cols']}x{view['rows']} heap bounded; allocated core 2048x128 "
          "refused history_viewport_exceeds_budget with zero Playback heap; same-generation live input works")


def k2(app):
    first = app.spawn("K2 focused seat")
    peer = app.spawn("K2 other seat")
    app.ctl("ui", "focus", first)
    app.ctl("seat", "wait", first, "--state", "idle", "--timeout", "30000")
    before = app.ctl("seat", "ls")
    def registry(rows):
        return {row["record"]["id"]: row["record"] for row in rows}
    original = registry(before)
    require(not original[first]["muted"] and not original[peer]["muted"], "fixture starts muted")
    muted = app.ctl("ui", "key", "cmd-m", label="k2-muted")
    records = registry(app.ctl("seat", "ls", label="k2-roster"))
    require(set(records) == set(original), "mute removed or added a seat")
    require(records[first]["muted"] and not records[peer]["muted"],
            "Cmd-M did not mute only the focused seat")
    require(muted["focused"] == first, "mute changed focus")
    require(any(n["kind"].endswith("mutemark") for n in nodes(muted)),
            "produced sidebar lacks its muted mark")
    app.capture("window", "k2-muted", frame(muted))
    sent = app.ctl("message", "send", first, "K2_OPERATOR_DM", label="k2-operator-dm")
    require(sent.get("kind") == "stored", f"operator DM was not stored while muted: {sent.get('kind')}")
    unmuted = app.ctl("ui", "key", "cmd-m", label="k2-unmuted")
    require(frame(unmuted) > frame(muted), "unmute reused the mute frame")
    require(not registry(app.ctl("seat", "ls"))[first]["muted"], "unmute did not publish")
    require(not any(n["kind"].endswith("mutemark") for n in nodes(unmuted)),
            "unmute left a stale sidebar mark")
    print("K2: idle focused seat alone muted/unmuted, roster and sidebar mark retained; "
          "operator DM stored; amber pixels/native delivery/non-operator delay unobserved")


def l1(app):
    app.spawn("L1 existing pane")
    original = {r["record"]["id"] for r in app.ctl("seat", "ls")}
    before = app.ctl("ui", "layout")
    opened = app.ctl("ui", "key", "cmd-t", label="l1-open")
    require(opened["launcher_open"], "Cmd-T did not open the launcher")
    require(frame(opened) > frame(before), "launcher reply predates its frame")
    panels = [n for n in nodes(opened) if n["kind"] == "named:picker"]
    require(len(panels) == 1, "produced layout lacks the launcher panel")
    x, y, width, height = panels[0]["rect"]
    require(width > 0 and height > 0 and x >= 0 and y >= 0, "launcher has no visible bounds")
    require(abs(x + width / 2 - opened["size"][0] / 2) < 1,
            "launcher is not horizontally centred")
    require(y + height <= opened["size"][1] + 1, "launcher extends below the window")
    app.capture("window", "l1-open", frame(opened))
    closed = app.ctl("ui", "key", "esc", label="l1-close")
    require(not closed["launcher_open"], "Esc left the launcher open")
    require(not any(n["kind"] == "named:picker" for n in nodes(closed)), "Esc left launcher nodes")
    require(frame(closed) > frame(opened), "Esc returned an old frame")
    require({r["record"]["id"] for r in app.ctl("seat", "ls")} == original,
            "Cmd-T/Esc spawned a seat")
    require(closed["focused"] == before["focused"] and closed["tab"] == before["tab"],
            "Cmd-T/Esc changed the focus anchor")
    print("L1: Cmd-T opens centred retained launcher over an existing pane; "
          "Esc removes it, zero spawns, same focus/tab; blur/labels/search/presets unobserved")


if __name__ == "__main__":
    try:
        {"S1": s1, "S2": s2, "S3": s3, "S4": s4, "K2": k2, "L1": l1}[sys.argv[1]](App())
    except (AssertionError, KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"FAIL: {sys.argv[1]}: {error}", file=sys.stderr)
        sys.exit(1)
