"""Partial K1/K3 checks through existing ctl controls, never desktop input."""
import json
from pathlib import Path
import shlex
import string
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ui import App, frame, require


# Independent expectations from kitty's keyboard protocol, enhancement flag 1:
# https://sw.kovidgoyal.net/kitty/keyboard-protocol/#disambiguate-escape-codes
# This covers the CLI's named press subset, not native platform input events.
NAMED = {
    "enter": b"\r", "tab": b"\t", "backspace": b"\x7f", "space": b" ",
    "esc": b"\x1b[27u", "shift-tab": b"\x1b[9;2u",
    "up": b"\x1b[A", "down": b"\x1b[B", "left": b"\x1b[D", "right": b"\x1b[C",
    "home": b"\x1b[H", "end": b"\x1b[F", "pageup": b"\x1b[5~",
    "pagedown": b"\x1b[6~", "delete": b"\x1b[3~",
}

RECEIVER = r'''
import json,os,sys,termios,tty
from pathlib import Path
cases=json.loads(Path(sys.argv[1]).read_text())
saved=termios.tcgetattr(0);actual=[];pending=bytearray()
try:
    tty.setraw(0)
    os.write(1,b"\x1b[>1uK1_READY\r\n")
    for case in cases:
        marker=case["marker"].encode()
        while marker not in pending:
            block=os.read(0,4096)
            if not block:raise RuntimeError("input ended before the case marker")
            pending.extend(block)
        end=pending.index(marker)
        actual.append(bytes(pending[:end]).hex())
        del pending[:end+len(marker)]
    Path(sys.argv[2]).write_text(json.dumps({"actual":actual,"trailing":pending.hex()}))
finally:
    os.write(1,b"\x1b[<u")
    termios.tcsetattr(0,termios.TCSANOW,saved)
os.write(1,b"\r\nK1_DONE\r\n")
'''


def k1(app):
    seat = app.spawn("K1 raw Kitty receiver")
    cases = [{"literal": "K1_plain", "expected": b"K1_plain".hex()}]
    cases += [{"key": key, "expected": value.hex()} for key, value in NAMED.items()]
    cases += [{"key": "ctrl-" + letter,
               "expected": f"\x1b[{ord(letter)};5u".encode().hex()}
              for letter in string.ascii_lowercase]
    cases += [{"chrome": True, "expected": ""},
              {"literal": "K1_final", "expected": b"K1_final".hex()}]
    for i, case in enumerate(cases):
        case["marker"] = f"__K1_{seat}_{i}__"
    spec = app.work / "k1-cases.json"
    report = app.work / "k1-received.json"
    receiver = app.work / "k1-receiver.py"
    spec.write_text(json.dumps(cases))
    receiver.write_text(RECEIVER)
    app.line(seat, "exec " + " ".join(map(shlex.quote, [
        sys.executable, "-I", "-B", str(receiver), str(spec), str(report)])))
    app.wait(seat, "(?m)^K1_READY$")
    for case in cases:
        if "key" in case:
            app.ctl("seat", "key", seat, case["key"])
        elif "literal" in case:
            app.ctl("seat", "type", seat, case["literal"])
        else:
            opened = app.ctl("ui", "key", "cmd-t")
            require(opened["launcher_open"], "Cmd-T did not open the app launcher")
            closed = app.ctl("ui", "key", "esc")
            require(not closed["launcher_open"], "Esc did not dismiss the launcher")
            require(closed["focused"] == seat, "chrome changed the receiver's focus")
        app.ctl("seat", "type", seat, case["marker"])
    app.wait(seat, "(?m)^K1_DONE$")
    result = json.loads(report.read_text())
    require(len(result["actual"]) == len(cases), "raw receiver lost a case")
    for i, (case, actual) in enumerate(zip(cases, result["actual"])):
        require(actual == case["expected"],
                f"case {i} {case.get('key', case.get('literal', 'chrome'))}: "
                f"expected {case['expected']}, received {actual}")
    require(result["trailing"] == "", "unexpected bytes followed the final marker")
    print("K1: 41 named Kitty presses and two literal inputs received exactly; "
          "Cmd-T/Esc produced no PTY bytes; native platform combinations remain unproven")


def k3(app):
    seat = app.spawn("K3 Graph shortcut")
    before = app.ctl("ui", "layout")
    require(before["page"] is None, "fixture did not start on the terminal stage")
    opened = app.ctl("ui", "key", "cmd-g", label="k3-open")
    require(opened["page"] == "graph", "Cmd-G did not select Graph")
    require(frame(opened) > frame(before), "Graph reply did not follow a produced frame")
    app.capture("window", "k3-graph", frame(opened))
    closed = app.ctl("ui", "key", "esc", label="k3-close")
    require(closed["page"] is None and closed["focused"] == seat,
            "dismissing Graph did not restore the terminal focus")
    app.line(seat, "printf 'K3_TERMINAL_OK\\n'")
    app.wait(seat, "(?m)^K3_TERMINAL_OK$")
    import time
    from documents import Documents
    from ui import pane
    docs = Documents("K3")
    record = docs.fixture("cmd-click", "K3_DOCUMENT_MODAL_BODY\n")
    tokens = [record["path"], f"[[{record['slug']}|document alias]]"]
    receiver = app.work/"k3-links.py"
    receiver.write_text("import os,time\nos.write(1," + repr(("\x1b[2J\x1b[H"+"\r\n".join(tokens)+"\r\nK3_LINKS_READY\r\n").encode()) + ")\ntime.sleep(120)\n")
    app.line(seat,"exec "+shlex.quote(sys.executable)+" -I -B "+shlex.quote(str(receiver)))
    app.wait(seat,"(?m)^K3_LINKS_READY$")
    for i,token in enumerate(tokens):
        state = app.ctl("ui","layout")
        displayed = pane(state,seat)
        lines = displayed["rendered"]["lines"]
        row = next(n for n,line in enumerate(lines) if token in line)
        column = lines[row].index(token)+3
        x,y,w,h = displayed["rect"]
        click = dict(kind="click",x=round(x+(column+0.5)*w/displayed["cols"]),
                     y=round(y+(row+0.5)*h/displayed["rows"]))
        plain = app.ctl("ui","input",json.dumps(click))
        require(plain["document"] is None,"plain click opened a document")
        click["modifiers"] = dict(command=True)
        app.ctl("ui","input",json.dumps(click),label=f"k3-cmd-click-{i}")
        deadline = time.monotonic()+25
        while True:
            opened = app.ctl("ui","layout")
            observed = opened.get("document_observation")
            if observed and observed["source"]["record_id"] == record["id"]:
                break
            require(time.monotonic()<deadline,f"Cmd-click {token} did not load the document modal")
            time.sleep(0.05)
        require(opened["document"] is not None,"modal has no document locator")
        docs.observation(opened,record)
        app.capture("window",f"k3-document-{i}",frame(opened))
        closed=app.ctl("ui","key","esc")
        require(closed["document"] is None and closed["focused"]==seat,"closing document lost terminal focus")
    print("K3: Cmd-G selects Graph; Cmd-click on a Markdown path and aliased wikilink loads the exact admitted document; captures and Esc/focus verified")


if __name__ == "__main__":
    {"K1": k1, "K3": k3}[sys.argv[1]](App())
