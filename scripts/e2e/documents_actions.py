#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""D4 modal controls against a real, private raw-PTY harness stub."""
import json
from pathlib import Path
import shlex
import sys
import time

sys.path.insert(0,str(Path(__file__).resolve().parent))
from documents import Documents, nodes, require

STUB = r'''
# Adapted from bboxy450-term scripts/e2e/keyboard_checks.py raw PTY receiver.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import json,os,sys,termios,tty
from pathlib import Path
saved=termios.tcgetattr(0);pending=bytearray();actual=[]
try:
    tty.setraw(0)
    os.write(1,b"\x1b[?2004hD4_READY\r\n")
    for n in range(2):
        marker=f"__D4_{n}__".encode()
        while marker not in pending:
            pending.extend(os.read(0,4096))
        end=pending.index(marker)
        actual.append(bytes(pending[:end]).hex())
        del pending[:end+len(marker)]
        Path(sys.argv[1]).write_text(json.dumps(actual))
        os.write(1,f"\r\nD4_FEED_{n}_OK\r\n".encode())
    while os.read(0,4096):pass
finally:
    termios.tcsetattr(0,termios.TCSANOW,saved)
'''


def d4(app):
    seat=app.ctl("seat","spawn","zsh","--place","tab","--cwd",app.work,"--name","D4 raw harness stub")["seat"]
    app.ctl("ui","focus",seat)
    stub,report=app.work/"d4-stub.py",app.work/"d4-received.json"
    stub.write_text(STUB)
    app.ctl("seat","type",seat,"exec "+" ".join(map(shlex.quote,[sys.executable,"-I","-B",str(stub),str(report)])))
    app.ctl("seat","key",seat,"enter")
    app.ctl("seat","wait",seat,"--text","(?m)^D4_READY$","--timeout","30000")
    deadline=time.monotonic()+20
    while True:
        layout=app.ctl("ui","layout")
        if any("D4_READY" in line for p in layout["panes"] for line in (p.get("rendered") or {}).get("lines",[])):
            break
        require(time.monotonic()<deadline,"stub readiness did not reach the GUI")
        time.sleep(0.02)
    body="# Exact source for the harness\n\n"+"\n\n".join(f"Paragraph {i:03}: **Markdown**, `code`, π and 日本語." for i in range(50))+"\n"
    record=app.fixture("actions",body)
    def text(layout):return "\n".join(n.get("text","") for _,n in nodes(layout))
    def click(layout,key):
        node=next(n for _,n in nodes(layout) if n["kind"]=="named:"+key)
        x,y,w,h=node["rect"]
        return app.ctl("ui","input",json.dumps(dict(kind="click",x=round(x+w/2),y=round(y+h/2))))
    bottom,_=app.open(record,["Paragraph 049"])
    top=app.ctl("ui","key","home")
    require("Paragraph 000" in text(top) and "Paragraph 049" not in text(top),"Home did not reveal the start of the document")
    page=app.ctl("ui","key","pagedown")
    require(page["frame_seq"]>top["frame_seq"] and text(page)!=text(top),"PageDown did not scroll the modal")
    end=app.ctl("ui","key","end")
    require("Paragraph 049" in text(end) and "Paragraph 000" not in text(end),"End did not reveal the final paragraph")
    copied=click(end,"action2")
    require(copied["document"] is not None,"Copy closed the modal")
    require(app.ctl("ui","clipboard-read")["test_clipboard"]==body,"Copy changed Markdown source bytes")
    app.ctl("ui","key","cmd-c")
    require(app.ctl("ui","clipboard-read")["test_clipboard"]==body,"Cmd-C copied rendered text rather than source")
    closed=click(app.ctl("ui","layout"),"action6")
    require(closed["document"] is None and closed["focused"]==seat,"Close did not restore the original harness")
    expected=(b"\x1b[200~"+body.encode()+b"\x1b[201~\r").hex()
    for index in range(2):
        opened,_=app.open(record,["Paragraph 049"])
        if index==0:
            fed=click(opened,"action0")
        else:
            # Command can disappear before the repeat/release. Neither may leak
            # into the now-uncovered harness after the feed closes the modal.
            events=[dict(kind="key",logical="Enter",unmodified="Enter",physical="Enter",event=event,
                         modifiers=dict(command=True) if event=="press" else {})
                    for event in ["press","repeat","release"]]
            fed=app.ctl("ui","input",json.dumps(events))
        require(fed["document"] is None and fed["focused"]==seat,"Feed did not close the modal onto its captured harness")
        app.ctl("seat","type",seat,f"__D4_{index}__")
        app.ctl("seat","wait",seat,"--text",f"(?m)^D4_FEED_{index}_OK$","--timeout","30000")
        received=json.loads(report.read_text())
        require(len(received)==index+1 and received[index]==expected,
                f"feed {index} delivered different or duplicate PTY bytes: {received}")
        app.checks.append({"feed":"footer" if index==0 else "Cmd-Enter/repeat/release","frame":fed["frame_seq"],"source_bytes":len(body.encode()),"received_bytes":len(bytes.fromhex(received[index]))})
    app.checks.append({"scroll_frames":[top["frame_seq"],page["frame_seq"],end["frame_seq"]],"clipboard":"exact source","closed_focus":seat})


def main():
    app,failure=None,None
    try:
        app=Documents("D4")
        d4(app)
    except Exception as error:
        failure=f"{type(error).__name__}: {error}"
    finally:
        if app is not None:app.finish(failure)
    if failure:
        print("FAIL: "+failure,file=sys.stderr)
        return 1
    return 0

if __name__=="__main__":raise SystemExit(main())
