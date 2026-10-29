#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""D1/D2 assertions over admitted Knowledge reads and the native produced frame.

No parser/engine in this driver predicts production geometry. Expected edge
relationships are finite fixture data; coordinates come from the actual modal.
"""
import hashlib
from collections import Counter
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import time
import zlib


def require(value, reason):
    if not value:
        raise AssertionError(reason)


def nodes(layout):
    def walk(node, key=""):
        if node["kind"].startswith("named:"):
            key = node["kind"][6:]
        yield key, node
        for child in node.get("children", []):
            yield from walk(child, key)
    for root in layout["tree"]:
        yield from walk(root)


def png(path):
    """Read the app capture's bounded, lossless RGBA8 output; no image rewriting."""
    data = path.read_bytes()
    require(data[:8] == b"\x89PNG\r\n\x1a\n", "not a native PNG capture")
    at, compressed, size = 8, bytearray(), None
    while at < len(data):
        length = struct.unpack_from(">I", data, at)[0]
        kind, value = data[at+4:at+8], data[at+8:at+8+length]
        if kind == b"IHDR":
            width, height, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", value)
            require(depth == 8 and color == 6 and interlace == 0, "capture format changed")
            require(0 < width*height <= 16_000_000, "capture exceeds fixture bounds")
            size = width, height
        elif kind == b"IDAT":
            compressed.extend(value)
        at += length+12
    require(size is not None, "capture has no dimensions")
    width, height = size
    raw = zlib.decompress(compressed)
    stride = width*4
    require(len(raw) == (stride+1)*height, "invalid capture rows")
    rows, previous = [], bytearray(stride)
    for y in range(height):
        offset = y*(stride+1)
        mode, row = raw[offset], bytearray(raw[offset+1:offset+1+stride])
        for i in range(stride):
            left, up, upper_left = (row[i-4] if i >= 4 else 0), previous[i], (previous[i-4] if i >= 4 else 0)
            if mode == 1:
                predict = left
            elif mode == 2:
                predict = up
            elif mode == 3:
                predict = (left+up)//2
            elif mode == 4:
                p = left+up-upper_left
                distance = [abs(p-left), abs(p-up), abs(p-upper_left)]
                predict = [left, up, upper_left][distance.index(min(distance))]
            else:
                require(mode == 0, "invalid PNG filter")
                predict = 0
            row[i] = (row[i]+predict) & 255
        rows.append(row)
        previous = row
    return width, height, rows


def ink_bounds(image, rect, color=None):
    width, height, rows = image
    x, y, w, h = rect
    pixels = [(xx,yy,tuple(rows[yy][xx*4:xx*4+4]))
        for yy in range(max(0,math.floor(y)),min(height,math.ceil(y+h)))
        for xx in range(max(0,math.floor(x)),min(width,math.ceil(x+w)))]
    background = Counter(rgba[:3] for _,_,rgba in pixels).most_common(1)[0][0] if pixels else (0,0,0)
    direction = [color[i]*255-background[i] for i in range(3)] if color else []
    length = sum(value*value for value in direction)
    points = []
    for xx,yy,(r,g,b,alpha) in pixels:
        if color is None:
            visible = min(r,g,b) > 170
        else:
            # Native antialias mixes the submitted color with this captured
            # background. Thin hollow task glyphs need not reach solid white.
            delta = [value-background[i] for i,value in enumerate([r,g,b])]
            coverage = sum(a*b for a,b in zip(delta,direction))/length if length else 0
            residual = max(abs(delta[i]-coverage*direction[i]) for i in range(3))
            visible = 0.15 <= coverage <= 1.05 and residual <= 12
        if alpha > 240 and visible:
            points.append((xx, yy))
    require(points, "the measured glyph run painted no foreground ink")
    return [min(x for x,y in points), min(y for x,y in points),
            max(x for x,y in points)-min(x for x,y in points)+1,
            max(y for x,y in points)-min(y for x,y in points)+1]


def region_hash(image, rect):
    width,height,rows = image
    x,y,w,h = rect
    left,top = max(0,math.floor(x)),max(0,math.floor(y))
    right,bottom = min(width,math.ceil(x+w)),min(height,math.ceil(y+h))
    require(right > left and bottom > top,"diagram capture region is empty")
    digest = hashlib.sha256()
    for row in rows[top:bottom]:
        digest.update(row[left*4:right*4])
    return digest.hexdigest()


# Adapted from bboxy450-term scripts/e2e/ui.py App and src/e2e_night.rs:
# the existing ctl door, runner identity and produced-frame receipts are retained.
class Documents:
    def __init__(self, promise):
        self.promise = promise
        self.binary = Path(os.environ["BBT"]).resolve()
        self.work = Path(os.environ["E2E_WORK"]).resolve()
        self.work.mkdir(parents=True, exist_ok=True)
        require(os.environ["BBOXY_CTL_ISSUER"].startswith("test-account:"), "not the runner's admitted private identity")
        self.test_run = os.environ["BBOXY_CTL_ISSUER"].split(":",1)[1]
        self.counter, self.fixtures, self.checks = 0, [], []
        status = self.ctl("app", "status")
        require(status["mode"] == "headless" and status["windows"] == 0 and status["custody_backend"] == "file", "not the private headless/file-custody app")
        self.ctl("ui", "clock", "freeze")

    def ctl(self, *args, payload=None):
        self.counter += 1
        result = subprocess.run([str(self.binary), "ctl", *map(str,args)],
            input=None if payload is None else json.dumps(payload), text=True,
            capture_output=True, timeout=40)
        rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        with (self.work/"document-calls.jsonl").open("a") as log:
            log.write(json.dumps({"args":list(map(str,args)),"exit":result.returncode,"rows":rows,"stderr":result.stderr})+"\n")
        require(result.returncode == 0 and len(rows) == 1 and rows[0].get("ok"), f"ctl {args}: {result.returncode}: {rows}")
        return rows[0]["data"]

    def fixture(self, name, body):
        request = {"path":f"e2e/{self.promise}-{name}.md", "body":body,
            "frontmatter":{"type":"e2e", "title":f"{self.promise} synthetic {name}",
                "function":self.promise, "group":"documents", "status":"untested",
                "result":"Synthetic rendering fixture; this metadata is not a verdict"}}
        (self.work/f"fixture-{name}.json").write_text(json.dumps(request,indent=2)+"\n")
        written = self.ctl("record","write","-",payload=request)["record"]
        fetched = self.ctl("record","fetch",request["path"])
        require(fetched["body"] == body and fetched["digest"] == written["digest"], "admitted fixture fetch changed the exact source")
        self.fixtures.append({"name":name,"path":request["path"],"body_sha256":hashlib.sha256(body.encode()).hexdigest(),"record":fetched})
        return fetched

    def open(self, record, wanted):
        self.ctl("ui","open","docs",record["path"])
        deadline = time.monotonic()+25
        # Ready title first, then use the existing document scroll control.
        while True:
            layout = self.ctl("ui","layout")
            if any(node.get("text") == record["frontmatter"]["title"] for _,node in nodes(layout)):
                break
            require(time.monotonic() < deadline, "native modal never loaded its admitted record")
            time.sleep(0.05)
        self.ctl("ui","key","end")
        while True:
            layout = self.ctl("ui","layout")
            if all(any(text in str(node.get("text", "")) for _,node in nodes(layout)) for text in wanted):
                break
            require(time.monotonic() < deadline, "native modal omitted required fixture text")
            time.sleep(0.05)
        label = f"{len(self.checks):03}-{record['slug']}-{self.counter}"
        (self.work/f"{label}-layout.json").write_text(json.dumps(layout,indent=2)+"\n")
        capture = self.ctl("seat","capture","window",self.work/f"{label}.png")
        require(capture["offscreen"] is True and capture["stale"] is False and capture["frame_seq"] >= layout["frame_seq"], "capture does not name this produced native modal")
        return layout, png(self.work/f"{label}.png")

    def observation(self, layout, record):
        observed = layout.get("document_observation")
        require(observed is not None, "private current document paint observation is unavailable")
        require(observed["frame_seq"] == layout["frame_seq"] > 0, "document observation belongs to another frame")
        require(observed.get("private_test_run") == self.test_run, "paint is not bound to this verified typed private test-run identity")
        source = observed["source"]
        require(source["record_id"] == record["id"] and source["path"] == record["path"] and source["digest"] == record["digest"] and source["etag"] == record["etag"], "paint is not bound to the exact admitted fixture")
        require(source["root"] == record["knowledge_root_id"], "paint crossed a Knowledge provider root")
        require(source["workspace"]["swarm"] == layout["swarm"], "paint belongs to another visible workspace")
        require(not any(observed["omitted"].values()), "full promise cannot accept omitted document facts")
        return observed

    def finish(self, failure):
        verdict = {"promise":self.promise,"pass":failure is None,"failure":failure,
            "checks":self.checks,"fixtures":self.fixtures,"binary_sha256":hashlib.sha256(self.binary.read_bytes()).hexdigest(),
            "helper_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
        (self.work/"document-verdict.json").write_text(json.dumps(verdict,indent=2)+"\n")
        # The standard runner retires successful private accounts; retain only
        # immutable fixture/paint/capture evidence, never custody or HOME bytes.
        kept = self.binary.parent.parent/"document-evidence"/f"{self.promise}-{os.environ['E2E_APP']}"
        kept.mkdir(parents=True,exist_ok=False)
        for path in self.work.iterdir():
            if path.is_file() and path.name != "output.log":
                shutil.copyfile(path,kept/path.name)
        print(json.dumps({"promise":self.promise,"pass":failure is None,"checks":len(self.checks),"evidence":str(kept),"failure":failure}),flush=True)


def d1(app):
    style = json.loads((Path(__file__).resolve().parents[2]/"design/style.json").read_text())["type"]
    heading_tokens = ["doc_title","heading","subtitle","doc_heading","body","section"]
    heading_fonts = []
    cases = [(f"heading{level}", "#"*level+f" HeadingLevel{level}\n\nBodyGlyphProbeHgMQ\n",
              [f"HeadingLevel{level}","BodyGlyphProbeHgMQ"]) for level in range(1,7)] + [
        ("lists", "- BulletAlpha\n- BulletBeta\n\n3. OrderedThree\n4. OrderedFour\n", ["•","BulletAlpha","BulletBeta","3.","OrderedThree","4.","OrderedFour"]),
        ("table", "| HeaderLeft | HeaderRight |\n| :--- | ---: |\n| CellAlpha | CellBeta |\n", ["HeaderLeft","HeaderRight","CellAlpha","CellBeta"]),
        ("code", "```python\nprint('CodeGlyphProbe')\n```\n", ["print('CodeGlyphProbe')"]),
        ("tasks", "- [x] DoneTask\n- [ ] OpenTask\n", ["☑ DoneTask","☐ OpenTask"]),
        ("footnotes", "FootnoteBody[^proof]\n\n[^proof]: FootnoteDefinition\n", ["FootnoteBody[proof]","[proof]","FootnoteDefinition"]),
    ]
    for name,body,wanted in cases:
        record = app.fixture(name,body)
        layout,image = app.open(record,wanted)
        present = {node.get("text"):node["rect"] for _,node in nodes(layout) if node.get("text")}
        observed = app.observation(layout,record)
        body_node = next(node for _,node in nodes(layout) if node["kind"] == "named:body")
        body_keys = {key for key,_ in nodes({"tree":[body_node]})}
        body_runs = [run for run in observed["texts"] if run["key"] in body_keys]
        painted = {}
        for run in body_runs:
            painted[run["key"]] = painted.get(run["key"],"")+run["text"]
        require(all(any(text in value for value in painted.values()) for text in wanted), f"{name}: visible text has no native paint")
        feature_keys = {key for key,value in painted.items() if any(text in value for text in wanted)}
        feature_ink = []
        for run in body_runs:
            if run["key"] in feature_keys and run["text"].strip():
                ink = ink_bounds(image,run["rect"],run["color"])
                feature_ink.append({"key":run["key"],"text":run["text"],"ink":ink,"font_size":run["font_size"]})
                if not name.startswith("heading"):
                    token = "caption" if name == "code" else "body"
                    if name == "footnotes" and run["text"] == "[proof]":
                        token = "caption" if run["key"].endswith("/label") else "micro"
                    require(abs(run["font_size"]-style[token]) < 0.001,f"{name}: native font lost the {token} type token")
        if name.startswith("heading"):
            level = int(name[-1])
            heading = next(t for t in observed["texts"] if t["text"] == wanted[0])
            expected = style[heading_tokens[level-1]]
            require(abs(heading["font_size"]-expected) < 0.001 and heading["weight"] == 700,
                    f"H{level} did not paint its baselined named style: {heading}")
            require(heading["line_height"] >= expected and present[wanted[0]][3] >= expected,
                    f"H{level} line geometry clips its native type size")
            require(ink_bounds(image,heading["rect"],heading["color"])[3] >= expected*0.55,
                    f"H{level} painted glyphs below their baselined true size")
            heading_fonts.append(heading["font_size"])
            require(all(a > b for a,b in zip(heading_fonts,heading_fonts[1:])),
                    f"native heading hierarchy collapsed: {heading_fonts}")
        if name == "code":
            require(any(run["mono"] and "CodeGlyphProbe" in run["text"] for run in observed["texts"]), "code was not painted with the native monospace font")
        if name == "table":
            require(present["HeaderLeft"][0] < present["HeaderRight"][0] and present["CellAlpha"][1] > present["HeaderLeft"][1], "table lost its actual row/column geometry")
        app.checks.append({"feature":name,"frame":layout["frame_seq"],"text_geometry":{text:present.get(text) for text in wanted},"native_ink":feature_ink})
        if name == "heading1":
            original = next(t for t in observed["texts"] if t["text"] == "BodyGlyphProbeHgMQ")
            original_ink = ink_bounds(image,original["rect"])
            app.ctl("app","resize","75x46")
            small,small_image = app.open(record,wanted)
            smaller = next(t for t in app.observation(small,record)["texts"] if t["text"] == original["text"])
            small_ink = ink_bounds(small_image,smaller["rect"])
            wide_body = next(node["rect"] for key,node in nodes(layout) if key == "body" and node["kind"] == "named:body")
            narrow_body = next(node["rect"] for key,node in nodes(small) if key == "body" and node["kind"] == "named:body")
            require(small["size"][0] < layout["size"][0] and narrow_body[2] < wide_body[2], "true-size proof never changed the modal's actual width")
            require(abs(smaller["rect"][2]-original["rect"][2]) < 0.001, "resizing scaled the glyph run's natural logical width")
            require(abs(smaller["font_size"]-original["font_size"]) < 0.001 and abs(smaller["line_height"]-original["line_height"]) < 0.001, "resizing scaled native text geometry")
            require(original_ink[3] >= original["line_height"]*0.45 and small_ink[3] >= smaller["line_height"]*0.45, "painted glyphs were scaled inside their logical line")
            require(all(abs(small_ink[i]-original_ink[i]) <= 1 for i in [2,3]), "window resize changed actual glyph ink size")
            app.checks.append({"feature":"true-size", "wide_body":wide_body,"narrow_body":narrow_body,"wide":original,"narrow":smaller,"wide_ink":original_ink,"narrow_ink":small_ink})
            app.ctl("app","resize","200x64")


    fitted_table_and_dismissal(app, style)


def fitted_table_and_dismissal(app, style):
    # Actual native cell/paint receipts decide fit; no browser or duplicate solver.
    # Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    app.ctl("ui", "key", "esc")
    before = app.ctl("ui", "layout")
    values = ["/workspace/long_component/document.md", "CenteredIdentifierAcrossSeveralSegments", "RightIdentifierAcrossSeveralSegments"]
    body = ("| Path | Center | Right |\n| :--- | :---: | ---: |\n"
            f"| `{values[0]}` | **{values[1][:8]}**{values[1][8:]} | `{values[2]}` |\n"
            "| x | 123 | 45 |\n")
    record = app.fixture("fitted-table", body)
    layout, image = app.open(record, ["Path", "Center", "Right", "123", "45"])
    observed = app.observation(layout, record)
    keyed = {key:node for key,node in nodes(layout) if node["kind"].startswith("named:")}
    panel, table = keyed["document"]["rect"], keyed["block0"]["rect"]
    require(table[2] > 576 and table[0]+table[2] <= panel[0]+panel[2]+0.01,
            "table did not use the actual modal column beyond readable prose width")
    ink = []
    for row, expected in [(1, values), (2, ["x", "123", "45"])]:
        heights = []
        for col, value in enumerate(expected):
            cell = keyed[f"block0/row{row}/cell{col}"]["rect"]
            text_key = f"block0/row{row}/cell{col}/text"
            painted = [r for r in observed["texts"] if r["key"] == text_key or r["key"].startswith(text_key+"/")]
            require("".join(r["text"] for r in painted) == value,
                    f"fitted cell {row}/{col} changed or lost rich source text")
            require(cell[0] >= table[0]-0.01 and cell[0]+cell[2] <= table[0]+table[2]+0.01,
                    "fitted table cell exceeds its offered native viewport")
            heights.append(cell[3])
            for run in painted:
                require(abs(run["font_size"]-style["body"]) < 0.001,
                        "table fit changed the native Body font size")
                require(run["rect"][0] >= cell[0]-0.01 and run["rect"][0]+run["rect"][2] <= cell[0]+cell[2]+0.01,
                        "table text paints outside its retained cell")
                ink.append({"key":run["key"], "text":run["text"], "ink":ink_bounds(image,run["rect"],run["color"])})
        require(max(heights)-min(heights) < 0.01, "fitted cells lost shared row heights")
    app.checks.append({"feature":"fitted-rich-table", "frame":layout["frame_seq"], "panel":panel, "table":table, "native_ink":ink})

    def click(rect):
        x,y,w,h = rect
        return app.ctl("ui", "click", x+w*0.5, y+h*0.5)

    # Inert panel padding and a body/selection-start click must never dismiss.
    inside = app.ctl("ui", "click", panel[0]+1, panel[1]+panel[3]*0.5)
    require(inside["document"] == layout["document"], "empty interior panel space dismissed the document")
    selected = click(next(run["rect"] for run in observed["texts"] if run["text"] == "123"))
    require(selected["document"] == layout["document"], "body selection-start click dismissed the document")
    find = click(keyed["action4"]["rect"])
    require(find["document"] == layout["document"] and any("find is not available yet" in str(n.get("text", "")) for _,n in nodes(find)),
            "footer command was replaced by outside dismissal")
    find_keyed = {key:n for key,n in nodes(find) if n["kind"].startswith("named:")}
    closed = click(find_keyed["action6"]["rect"])
    anchor = ["swarm", "tab", "focused", "page"]
    require(closed["document"] is None and all(closed[k] == before[k] for k in anchor),
            "footer close did not restore the original focus anchor")

    reopened, _ = app.open(record, ["123", "45"])
    panel = next(n["rect"] for key,n in nodes(reopened) if key == "document" and n["kind"] == "named:document")
    settings = reopened["regions"]["settings"]
    point = [settings[0]+settings[2]*0.5, settings[1]+settings[3]*0.5]
    require(not (panel[0] <= point[0] < panel[0]+panel[2] and panel[1] <= point[1] < panel[1]+panel[3]),
            "fixture must click a real underlying Settings control outside the modal")
    dismissed = app.ctl("ui", "click", *point)
    require(dismissed["document"] is None and all(dismissed[k] == before[k] for k in anchor),
            "backdrop click did not consume dismissal and restore focus; underlying Settings received the click")
    require(not any(n["kind"] == "named:document" for _,n in nodes(dismissed)), "dismissed modal remains in the produced tree")
    app.checks.append({"feature":"modal-dismissal", "inside_frame":inside["frame_seq"], "body_frame":selected["frame_seq"],
                       "footer_frame":closed["frame_seq"], "backdrop_frame":dismissed["frame_seq"], "outside_underlying_settings":point,
                       "focus_anchor":{k:dismissed[k] for k in anchor}})


def point(path, value):
    return [path["origin"][i]+path["scale"]*value[i] for i in range(2)]


def border_distance(point,rect):
    x,y,w,h = rect
    px,py = point
    if x-0.25 <= px <= x+w+0.25 and y-0.25 <= py <= y+h+0.25:
        return min(abs(px-x),abs(px-x-w),abs(py-y),abs(py-y-h))
    return float("inf")


def d2(app):
    cases = [
        ("lr", "flowchart LR\nA[Alpha] --> B[Beta]\nB --> C[Gamma]\n", [("Alpha","Beta"),("Beta","Gamma")]),
        ("branch", "flowchart TD\nA[North] --> B[West]\nA --> C[East]\nB --> D[South]\nC --> D\n", [("North","West"),("North","East"),("West","South"),("East","South")]),
        ("cycle", "flowchart LR\nA[CycleAlpha] --- B[CycleBeta]\nB --- C[CycleGamma]\nC --- A\n", [("CycleAlpha","CycleBeta"),("CycleBeta","CycleGamma"),("CycleGamma","CycleAlpha")]),
    ]
    records = [(name,app.fixture(name,"```mermaid\n"+source+"```\n"),edges) for name,source,edges in cases]
    for index,(name,record,edges) in enumerate(records):
        wanted = sorted({value for edge in edges for value in edge})
        layout,image = app.open(record,wanted)
        observed = app.observation(layout,record)
        keyed = {key:node for key,node in nodes(layout) if node["kind"].startswith("named:")}
        bounds = {}
        for text in observed["texts"]:
            m = re.search(r"^(.*)/label(\d+)(?:/(?:line|l)\d+)?$",text["key"])
            if m and text["text"] in wanted:
                base = f"{m[1]}/node{m[2]}"
                node = keyed.get(base+"/bounds",keyed.get(base))
                require(node is not None,"native Mermaid node bounds are unavailable")
                bounds[text["text"]] = node["rect"]
        require(set(bounds) == set(wanted),"fence did not use the retained native Mermaid canvas")
        for a,left in bounds.items():
            for b,right in bounds.items():
                if a < b:
                    overlap = min(left[0]+left[2],right[0]+right[2])-max(left[0],right[0]) > 0.01 and min(left[1]+left[3],right[1]+right[3])-max(left[1],right[1]) > 0.01
                    require(not overlap,f"native nodes overlap: {a}/{b}")
        paths = {path["key"]:path for path in observed["paths"]}
        for i,(a,b) in enumerate(edges):
            found = [p for key,p in paths.items() if key.endswith(f"/edge/{i}")]
            require(len(found) == 1,f"edge {i} has no submitted native path")
            path = found[0]
            start = point(path,path["commands"][0]["points"])
            tail = path["commands"][-1]
            end = point(path,tail["points"][-1] if tail["op"] == "cubic" else tail["points"])
            head = paths.get(path["key"]+"/end")
            if head:
                end = point(head,head["commands"][1]["points"])
            require(border_distance(start,bounds[a]) <= 0.25,f"edge {i} starts away from {a}'s border: {start}/{bounds[a]}")
            require(border_distance(end,bounds[b]) <= 0.25,f"edge {i} ends away from {b}'s border: {end}/{bounds[b]}")
        # A different document forces disposal/rebuild before reopening this
        # unchanged source. Comparing a cached receipt to itself is insufficient.
        _,other,other_edges = records[(index+1)%len(records)]
        app.open(other,[other_edges[0][0]])
        repeated,repeated_image = app.open(record,wanted)
        again = app.observation(repeated,record)
        require(repeated["frame_seq"] > layout["frame_seq"], "determinism compared the same frame")
        require(again["texts"] == observed["texts"] and again["paths"] == observed["paths"] and again["clips"] == observed["clips"],"unchanged native diagram rebuilt with different geometry")
        points = [[r[0],r[1]] for r in bounds.values()]+[[r[0]+r[2],r[1]+r[3]] for r in bounds.values()]
        for path in observed["paths"]:
            for command in path["commands"]:
                if command["op"] == "cubic":
                    points.extend(point(path,p) for p in command["points"])
                elif command["op"] in ["move","line"]:
                    points.append(point(path,command["points"]))
        left,top = min(p[0] for p in points)-3,min(p[1] for p in points)-3
        rect = [left,top,max(p[0] for p in points)+3-left,max(p[1] for p in points)+3-top]
        pixels = region_hash(image,rect)
        require(region_hash(repeated_image,rect) == pixels,"unchanged native diagram rebuilt with different captured pixels")
        app.checks.append({"fixture":name,"node_bounds":bounds,"edges":edges,"first_frame":layout["frame_seq"],"repeat_frame":repeated["frame_seq"],"capture_region":rect,"pixels_sha256":pixels,"geometry_sha256":hashlib.sha256(json.dumps([observed["texts"],observed["paths"],observed["clips"]],sort_keys=True).encode()).hexdigest()})


def main():
    app, failure = None,None
    try:
        require(sys.argv[1:] in [["D1"],["D2"]],"expected one document promise")
        app = Documents(sys.argv[1])
        (d1 if app.promise == "D1" else d2)(app)
    except Exception as error:
        failure = f"{type(error).__name__}: {error}"
    finally:
        if app is not None:
            app.finish(failure)
    if failure:
        print("FAIL: "+failure,file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
