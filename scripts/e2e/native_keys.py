# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Independent Kitty byte fixtures at the shared normalized platform boundary.

The finite platform domain is printable ASCII, Unicode layout samples, winit
functional/keypad/modifier identities and every Shift/Alt/Control combination.
Command belongs to the app. OS input methods and hardware delivery are outside
this headless boundary; preedit/commit/cancel enter the real IME handler.
"""
import json
import shlex
import string
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ui import App, require, frame, pane
from keyboard_checks import RECEIVER


def key(logical, physical, event="press", modifiers=None, **fields):
    return dict(kind="key", logical=logical, unmodified=logical, physical=physical,
                event=event, modifiers=modifiers or {}, **fields)


def input_events(app, events):
    result = None
    for at in range(0, len(events), 32):
        result = app.ctl("ui", "input", json.dumps(events[at:at+32], ensure_ascii=False))
    return result


# Functional identities and codepoints from the protocol's functional-key table:
# https://sw.kovidgoyal.net/kitty/keyboard-protocol/#functional-key-definitions
FUNCTIONAL = {
    "Escape": ("Escape", "27", "u"), "Enter": ("Enter", "13", "u"),
    "Tab": ("Tab", "9", "u"), "Backspace": ("Backspace", "127", "u"),
    "Insert": ("Insert", "2", "~"), "Delete": ("Delete", "3", "~"),
    "PageUp": ("PageUp", "5", "~"), "PageDown": ("PageDown", "6", "~"),
    "ArrowUp": ("ArrowUp", "1", "A"), "ArrowDown": ("ArrowDown", "1", "B"),
    "ArrowLeft": ("ArrowLeft", "1", "D"), "ArrowRight": ("ArrowRight", "1", "C"),
    "Home": ("Home", "1", "H"), "End": ("End", "1", "F"),
    "CapsLock": ("CapsLock", "57358", "u"), "ScrollLock": ("ScrollLock", "57359", "u"),
    "NumLock": ("NumLock", "57360", "u"), "PrintScreen": ("PrintScreen", "57361", "u"),
    "Pause": ("Pause", "57362", "u"), "ContextMenu": ("ContextMenu", "57363", "u"),
}
for n, base in enumerate(["1", "1", "13", "1", "15", "17", "18", "19", "20", "21", "23", "24"], 1):
    FUNCTIONAL[f"F{n}"] = (f"F{n}", base, {1:"P", 2:"Q", 3:"~", 4:"S"}.get(n, "~"))
for n in range(13, 36):
    FUNCTIONAL[f"F{n}"] = (f"F{n}", str(57376+n-13), "u")
for name, code in {"MediaPlay":57428, "MediaPause":57429, "MediaPlayPause":57430,
                   "MediaStop":57432, "MediaFastForward":57433, "MediaRewind":57434,
                   "MediaTrackNext":57435, "MediaTrackPrevious":57436, "MediaRecord":57437,
                   "AudioVolumeDown":57438, "AudioVolumeUp":57439, "AudioVolumeMute":57440}.items():
    FUNCTIONAL[name] = ("KeyJ", str(code), "u")


def sequence(base, ending, mask, event):
    parameter = str(1+mask)
    if event != "press":
        parameter += ":" + ("2" if event == "repeat" else "3")
    # Arrow/Home/F1/F2/F4 omit the default 1 on an unmodified press.
    if mask == 0 and event == "press":
        return f"\x1b[{'' if base == '1' and ending in 'ABCDHFPQS' else base}{ending}".encode()
    return f"\x1b[{base};{parameter}{ending}".encode()


def k1(app):
    seat = app.spawn("K1 normalized native receiver")
    peer = app.ctl("seat", "spawn", "zsh", "--place", "right", "--cwd", app.work,
                   "--name", "K1 navigation peer")["seat"]
    app.ctl("ui", "focus", seat)
    cases = []
    def add(label, events, expected):
        cases.append(dict(label=label, inputs=events, expected=expected.hex(),
                          marker=f"__K1_{len(cases)}__"))
    ascii_keys = {}
    for c in string.ascii_letters:
        ascii_keys[c] = ("Key"+c.upper(), c.lower())
    for c in string.digits:
        ascii_keys[c] = ("Digit"+c, c)
    for lo, hi, physical in [("`","~","Backquote"),("-","_","Minus"),("=","+","Equal"),
                             ("[","{","BracketLeft"),("]","}","BracketRight"),("\\","|","Backslash"),
                             (";",":","Semicolon"),("'",'"',"Quote"),(",","<","Comma"),
                             (".",">","Period"),("/","?","Slash"),(" "," ","Space")]:
        ascii_keys[lo] = (physical, lo)
        ascii_keys[hi] = (physical, lo)
    for lo, hi in zip("1234567890", "!@#$%^&*()"):
        ascii_keys[hi] = ("Digit"+lo, lo)
    require(set(ascii_keys) == set(chr(i) for i in range(32,127)), "incomplete printable key domain")
    keys = [(c, physical, str(ord(base)), "u", base) for c,(physical,base) in ascii_keys.items()]
    keys += [(name,physical,base,end,name) for name,(physical,base,end) in FUNCTIONAL.items()]
    for logical,physical,base,end,unmodified in keys:
        for mask in range(8):
            mods = dict(shift=bool(mask&1), alt=bool(mask&2), control=bool(mask&4))
            events = [key(logical,physical,event,mods,**({"text":logical} if len(logical)==1 else {}))
                      for event in ["press","repeat","release"]]
            for event in events:
                event["unmodified"] = unmodified
            expected = b"".join(sequence(base,end,mask,e) for e in ["press","repeat","release"])
            # Shift history navigation remains reserved by the app in Kitty mode.
            if mask == 1 and logical in ["PageUp","PageDown","Home","End"]:
                expected = b""
            add(f"{logical}/{mask}", events, expected)
    # Every Command modifier combination stays out of the PTY.
    for mask in range(8):
        mods = dict(command=True, shift=bool(mask&1), alt=bool(mask&2), control=bool(mask&4))
        add(f"Command/{mask}", [key("j","KeyJ","press",mods), key("j","KeyJ","repeat"), key("j","KeyJ","release")], b"")
    for side,offset in [("Left",0),("Right",6)]:
        for name,flag,code in [("Shift","shift",57441),("Control","control",57442),("Alt","alt",57443)]:
            physical = name+side
            add(physical,[key(name,physical,"press",location=side),key(name,physical,"release",location=side)],
                sequence(str(code+offset),"u",{"shift":1,"alt":2,"control":4}[flag],"press")+
                sequence(str(code+offset),"u",0,"release"))
    for caps,num in [(True,False),(False,True),(True,True)]:
        add(f"locks/{caps}/{num}", [key("a","KeyA",modifiers=dict(caps_lock=caps,num_lock=num))],
            sequence("97","u",64*int(caps)+128*int(num),"press"))
    for n in range(10):
        add(f"keypad/{n}",[key(str(n),f"Numpad{n}",e,location="Numpad") for e in ["press","repeat","release"]],
            b"".join(sequence(str(57399+n),"u",0,e) for e in ["press","repeat","release"]))
    for logical,physical,code in [(".","NumpadDecimal",57409),("/","NumpadDivide",57410),
            ("*","NumpadMultiply",57411),("-","NumpadSubtract",57412),("+","NumpadAdd",57413),
            ("Enter","NumpadEnter",57414),("=","NumpadEqual",57415),(",","NumpadComma",57416)]:
        add(f"keypad/{physical}",[key(logical,physical,e,location="Numpad") for e in ["press","repeat","release"]],
            b"".join(sequence(str(code),"u",0,e) for e in ["press","repeat","release"]))
    for index,logical in enumerate(["ArrowLeft","ArrowRight","ArrowUp","ArrowDown","PageUp","PageDown","Home","End","Insert","Delete","Clear"]):
        add(f"keypad/{logical}",[key(logical,"Numpad0",e,location="Numpad") for e in ["press","repeat","release"]],
            b"".join(sequence(str(57417+index),"u",0,e) for e in ["press","repeat","release"]))
    add("IME preedit/cancel",[dict(kind="preedit",text="に",cursor=[0,3]),dict(kind="cancel_composition")],b"")
    add("IME commit",[dict(kind="preedit",text="日本",cursor=[0,6]),dict(kind="commit",text="日本語🙂")],"日本語🙂".encode())
    add("Unicode layout",[key("ф","KeyA",text="ф")],sequence("1092","u",0,"press"))
    cases.append(dict(label="reserved navigation",nav=True,expected="",marker="__K1_NAV__"))
    receiver = app.work/"k1-native-receiver.py"
    spec,report = app.work/"k1-native-cases.json",app.work/"k1-native-received.json"
    receiver.write_text(RECEIVER.replace('>1uK1_READY','>11uK1_READY').replace(
        'del pending[:end+len(marker)]',
        'del pending[:end+len(marker)]\n        Path(sys.argv[2]+".progress").write_text(str(len(actual))+" "+case.get("label",""))'))
    (app.work/"k1-native-cases.json").write_text(json.dumps(cases))
    spec = app.work/"k1-native-stream.json"
    expected = b"".join(bytes.fromhex(case["expected"]) for case in cases)
    spec.write_text(json.dumps([dict(marker=cases[-1]["marker"], expected=expected.hex())]))
    app.line(seat,"exec "+" ".join(map(shlex.quote,[sys.executable,"-I","-B",str(receiver),str(spec),str(report)])))
    app.wait(seat,"(?m)^K1_READY$")
    import time
    deadline = time.monotonic()+20
    while True:
        visible = pane(app.ctl("ui","layout"),seat).get("rendered")
        if visible and any("K1_READY" in line for line in visible.get("lines",[])):
            break
        require(time.monotonic()<deadline,"GUI did not ingest the negotiated Kitty screen")
        time.sleep(0.02)
    events = []
    for case in cases[:-1]:
        events.extend(case["inputs"])
    input_events(app,events)
    for logical,physical,target in [("ArrowRight","ArrowRight",peer),("ArrowLeft","ArrowLeft",seat),
                                     ("Tab","Tab",peer),("Tab","Tab",seat)]:
        mods=dict(command=True,alt=True,shift=target==seat and logical=="Tab")
        state=input_events(app,[key(logical,physical,modifiers=mods)])
        require(state["focused"]==target,f"reserved {logical} did not focus {target}")
        input_events(app,[key(logical,physical,"release")])
    # Up/down are also reserved even without a neighbor; no terminal bytes leak.
    input_events(app,[key(name,name,event,dict(command=True,alt=True) if event=="press" else {})
                      for name in ["ArrowUp","ArrowDown"] for event in ["press","release"]])
    app.ctl("seat","type",seat,cases[-1]["marker"])
    app.wait(seat,"(?m)^K1_DONE$")
    actual=json.loads(report.read_text())
    require(len(actual["actual"])==1,"receiver omitted the complete native stream")
    received = bytes.fromhex(actual["actual"][0])
    mismatch = next((i for i,(a,b) in enumerate(zip(expected,received)) if a!=b), min(len(expected),len(received)))
    offset, label = 0, "trailing bytes"
    for case in cases:
        length = len(bytes.fromhex(case["expected"]))
        if offset <= mismatch < offset+length:
            label = case["label"]
            break
        offset += length
    require(received==expected,f"{label} at byte {mismatch}: expected {expected[mismatch:mismatch+40].hex()}, received {received[mismatch:mismatch+40].hex()} (length {len(received)}/{len(expected)})")
    require(actual["trailing"]=="","unexpected trailing native bytes")
    app.capture("window","k1-native-navigation",frame(app.ctl("ui","layout")))
    print(f"K1: {len(cases)} exact raw PTY cases: printable/function/keypad keys; all Shift/Alt/Control and Command combinations; press/repeat/release, locks, Unicode, IME and reserved navigation")


if __name__ == "__main__":
    k1(App())
