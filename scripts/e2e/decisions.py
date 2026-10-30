#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""N2: real admitted decision metadata appears in the produced native Graph."""
import json
import os
from pathlib import Path
import sys
import time

sys.path.insert(0,str(Path(__file__).resolve().parent))
from documents import Documents, nodes, require


def n2(app):
    seat=app.ctl("seat","spawn","zsh","--place","tab","--cwd",app.work,"--name","N2 decision graph")["seat"]
    app.ctl("ui","focus",seat)
    relations=["decides","constrains","evidence","answers","amends"]
    links=[]
    root=app.work.parent/"repository"/".bboxy"
    def seed(path,number,kind,title,links=None):
        target=root/path
        target.parent.mkdir(parents=True,exist_ok=True)
        require(not target.exists(),"fixture path was already occupied")
        metadata=dict(id="0"*25+str(number),number=number,type=kind,title=title,status="decided" if kind=="decision" else "active",links=links or [])
        pending=target.with_suffix(".fixture")
        with pending.open("x") as output:output.write("---\n"+json.dumps(metadata)+"\n---\n"+title+"\n")
        pending.replace(target)
    for number,relation in enumerate(relations,1):
        seed(f"notes/N2-target-{relation}.md",number,"note",f"N2 {relation} endpoint")
        links.append(dict(to=f"[[N2-target-{relation}]]",rel=relation))
    path="decisions/d6-N2-choice.md"
    seed(path,6,"decision","N2 live decision",links)
    deadline=time.monotonic()+20
    while True:
        try:
            record=app.ctl("record","fetch",path)
            if record.get("parse_error") is None and record.get("number")==6:break
        except AssertionError:
            pass
        require(time.monotonic()<deadline,"private decision fixture was never admitted by the real index")
        time.sleep(0.05)
    require(record["frontmatter"]["links"]==links,"provider changed explicit edge types")
    app.fixtures.append({"name":"decision","path":path,"record":record})
    app.ctl("ui","key","cmd-g")
    deadline=time.monotonic()+25
    while True:
        layout=app.ctl("ui","layout")
        node=next((node for _,node in nodes(layout) if node["kind"]=="named:decisions/node0"),None)
        texts="\n".join(n.get("text","") for _,n in nodes({"tree":[node]})) if node else ""
        if "N2 live decision" in texts and all(f"N2-target-{rel}" in texts for rel in relations):
            break
        require(time.monotonic()<deadline,"native Graph did not publish the decision and all five typed endpoints")
        time.sleep(0.03)
    require(layout["page"]=="graph","decision node was not on the Graph tab")
    for i,rel in enumerate(relations):
        edge=next((node for _,node in nodes(layout) if node["kind"]==f"named:decisions/node0/edge{i}/l0/rel"),None)
        require(edge is not None and "".join(n.get("text","") for _,n in nodes({"tree":[edge]}))==rel,f"Graph lost the exact {rel} edge type")
    capture=app.ctl("seat","capture","window",app.work/"N2-graph.png")
    require(capture["offscreen"] is True and capture["stale"] is False and capture["frame_seq"]>=layout["frame_seq"],"Graph capture predates the produced decision tree")
    (app.work/"N2-layout.json").write_text(json.dumps(layout,indent=2)+"\n")
    app.checks.append({"decision":record["id"],"number":record["number"],"edges":links,"frame":layout["frame_seq"],"capture_frame":capture["frame_seq"]})


def check_night(path):
    rows=[json.loads(line) for line in Path(path).read_text().splitlines() if line.startswith("{")]
    row=next(r for r in rows if r.get("kind")=="e2e" and r.get("function")=="N2")
    require(row["exit"]==0 and row["verdict"]=="pass" and row["coverage"]=="full" and not row["missing"],str(row))
    app=next(r for r in rows if r.get("kind")=="app" and r.get("function")=="N2")
    require(app["mode"]=="headless" and app["windows"]==0 and app["custody_backend"]=="file" and app["issued_by"].startswith("test-account:"),"not a private native app")
    deadline=time.monotonic()+3
    while True:
        try:os.kill(app["pid"],0)
        except ProcessLookupError:break
        require(time.monotonic()<deadline,"N2 app did not finish orderly teardown")
        time.sleep(0.02)
    print(json.dumps({"N2":"pass","pid_gone":app["pid"],"evidence":row["evidence"]}))


def main():
    if len(sys.argv)==3 and sys.argv[1]=="--check-night":
        check_night(sys.argv[2])
        return 0
    app,failure=None,None
    try:
        app=Documents("N2")
        n2(app)
    except Exception as error:
        failure=f"{type(error).__name__}: {error}"
    finally:
        if app is not None:app.finish(failure)
    if failure:
        print("FAIL: "+failure,file=sys.stderr)
        return 1
    return 0

if __name__=="__main__":raise SystemExit(main())
