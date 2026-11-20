# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Native captures against exact-key references, using the shared Rust comparator."""
import copy
import html
import json
import math
import os
from pathlib import Path
import subprocess

from corpus import BASE, digest, dump, load_fonts
from measurements import advance_containment
from references import policy, run as reference_run
from review import frozen_advisories, run as review_run


def shapes(reference, native):
    expected = {r["source"]: r for r in reference}
    actual = {r["source"]: r for r in native}
    if len(actual) != len(native) or actual.keys() != expected.keys():
        raise ValueError("missing/duplicate native public geometry")
    if not expected:
        raise ValueError("missing public reference geometry")
    errors = []
    for source, ref in expected.items():
        row = actual[source]
        if ref.get("endpoints") is None or row.get("endpoints") is None:
            raise ValueError("missing native path endpoints: " + source)
        a = [*ref["bounds"], *sum(ref["endpoints"], [])]
        b = [*row["bounds"], *sum(row["endpoints"], [])]
        if len(a) != len(b) or not all(math.isfinite(n) for n in a+b):
            raise ValueError("missing/nonfinite native public geometry")
        errors.append({"source": source, "maximum_error_px": max(abs(x-y) for x, y in zip(a,b)), "exact": a == b, "reference": ref, "native": row})
    return {"missing": 0, "paths": errors, "passed": all(r["exact"] for r in errors)}


def text_geometry(reference, native, exported=None):
    if len(reference) != len(native):
        raise ValueError("missing native text correspondence")
    if exported is not None and len(exported) != len(native):
        raise ValueError("missing native/exporter text correspondence")
    rows = []
    tolerance = policy()["text_geometry_px"]
    for index, (ref, row) in enumerate(zip(reference, native)):
        problems, errors = [], []
        if row["source"] != ref["source"] or row["text"] != ref["text"] or str(row["weight"]) != ref["weight"] or row["anchor_mode"] != ref["anchor_mode"]:
            problems.append("native text/source/weight/anchor differs")
        if len(row["chars"]) != len(ref["chars"]):
            problems.append("native character correspondence differs")
        pairs = [(row["bounds"],ref["bounds"]),(row["baseline_anchor"],ref["baseline_anchor"]),([row["advance"]],[ref["advance"]])]
        for a,b in zip(row["chars"],ref["chars"]):
            pairs.extend((a[k],b[k]) for k in ("start","end","box"))
        for a,b in pairs:
            if len(a)!=len(b) or not all(math.isfinite(n) for n in a+b):
                problems.append("missing/nonfinite native text geometry")
            else:
                errors.extend(abs(x-y) for x,y in zip(a,b))
        maximum=max(errors,default=0)
        if maximum>tolerance:
            problems.append("native/Chrome text exceeds geometry tolerance")
        containment=None
        if exported is not None:
            exp=exported[index]
            containment=advance_containment(exp["advance"],row["advance"],row["anchor_mode"],row["advance_axis_scale"],tolerance)
            if not containment["passed"]: problems.append("native advance exceeds exporter reservation")
            if row["postscript"]!=exp.get("backend_postscript_name",exp.get("resolved_face")):
                problems.append("native/exporter actual face differs")
        rows.append({"source":row["source"],"text":row["text"],"maximum_error_px":maximum,"containment":containment,"problems":problems,"native":row,"chrome":ref})
    return {"measurements":rows,"passed":all(not r["problems"] for r in rows)}


def check_reference(run):
    for image in run["captures"].values():
        path=Path(image["file"])
        if not path.is_file() or digest(path.read_bytes())!=image["sha256"]:
            raise ValueError("stale/missing reference capture: "+str(path))


def admission(out,program):
    from fixtures import boundary_input
    import base64
    fonts=BASE/"fonts"
    manifest=json.loads((BASE/"fixtures/manifest.json").read_text())
    directory=out/"native-boundaries";directory.mkdir(exist_ok=True)
    rows=[]
    def invoke(args):
        result=subprocess.run([str(program),*args],capture_output=True,text=True,timeout=120)
        return {"command":[str(program),*args],"exit_code":result.returncode,"stdout":result.stdout,"stderr":result.stderr}
    for case in manifest["negatives"]:
        file=BASE/"fixtures"/case["svg"]
        if digest(file.read_bytes())!=case["sha256"]:raise ValueError("negative source pin differs")
        result=invoke(["prepare",str(file),str(fonts)])
        diagnostics=json.loads(result["stdout"]).get("diagnostics",[]) if result["stdout"] else []
        expected="unsupported" if case["expected"]=="unsupported" else "refused"
        rows.append({"case":case["id"],"kind":"negative","passed":result["exit_code"]!=0 and any(d["disposition"]==expected for d in diagnostics),**result})
    for boundary in manifest["boundaries"]:
        name=boundary["limit"]
        for side in ("accepted","rejected"):
            request=boundary_input(name,boundary[side]);results=[]
            if request["api"]=="decode_png" and name not in ("png_distinct","png_pixels"):
                file=directory/"input.png";file.write_bytes(request["inputs"][0])
                results.append(invoke(["decode-png",str(file),str(next(b["accepted"] for b in manifest["boundaries"] if b["limit"]=="png_pixels"))]))
            elif request["api"]=="render":
                file=directory/"view.json";dump(file,{"camera":request["camera"],"scale":request["scale"]})
                results.append(invoke(["validate-view",str(file)]))
            else:
                if request["api"]=="decode_png":
                    # These two aggregate limits belong to actual SVG preparation,
                    # with compressed fixture PNGs well below the SVG byte ceiling.
                    images=''.join('<image width="1" height="1" href="data:image/png;base64,'+base64.b64encode(raw).decode()+'"/>' for raw in request["inputs"])
                    raw=('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" data-bboxy-font-set="'+load_fonts()[1]+'">'+images+'</svg>').encode()
                else:raw=request["input"]
                file=directory/"input.svg";file.write_bytes(raw)
                results.append(invoke(["prepare",str(file),str(fonts)]))
            passed=all(r["exit_code"]==0 for r in results) if side=="accepted" else any(r["exit_code"]!=0 for r in results)
            rows.append({"case":name,"kind":side,"value":boundary[side],"passed":passed,"commands":results})
    result={"rows":rows,"passed":all(r["passed"] for r in rows)}
    dump(out/"native-admission.json",result)
    return result


def broken_controls(out, program, rows, references):
    controls = []
    def refused(name, action):
        try:
            action()
        except ValueError as error:
            controls.append({"name": name, "passed": True, "diagnostic": str(error)})
        else:
            controls.append({"name": name, "passed": False})
    vector = next(r for r in rows if r.get("geometry", {}).get("passed"))
    native = json.loads(Path(vector["native_json"]).read_text())
    expected = [p["reference"] for p in vector["geometry"]["paths"]]
    refused("missing-public-geometry", lambda: shapes(expected, native["geometry"][:-1]))
    text = next(r for r in rows if r.get("text_geometry", {}).get("measurements"))
    measured = text["text_geometry"]["measurements"]
    refused("dropped-text", lambda: text_geometry([r["chrome"] for r in measured], [r["native"] for r in measured][:-1]))
    stale = copy.deepcopy(references["runs"][0])
    next(iter(stale["captures"].values()))["sha256"] = "0" * 64
    refused("stale-reference-hash", lambda: check_reference(stale))
    file = out / "native-boundaries/unknown-paint.svg"
    file.write_text('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="4" height="4" foreign-paint="red"/></svg>')
    result = subprocess.run([str(program), "prepare", str(file), str(BASE / "fonts")], capture_output=True, text=True, timeout=30)
    diagnostics = json.loads(result.stdout).get("diagnostics", [])
    controls.append({"name": "unknown-paint", "passed": result.returncode != 0 and any(d["disposition"] == "unsupported" for d in diagnostics), "exit_code": result.returncode, "stdout": result.stdout, "stderr": result.stderr})
    # Causal mutations preserve the pinned reference and use the real native painter.
    # Removing an occluder, a foreground layer or the shared clip must break the
    # non-advisory complete-render gate, not just a bookkeeping assertion.
    import xml.etree.ElementTree as ET
    manifest=json.loads((BASE/"fixtures/manifest.json").read_text())
    for case in (c for c in manifest["cases"] if c.get("composition")):
        ref=next(r for r in references["runs"] if r["id"]==case["id"] and r["view"]["id"]=="1x")
        root=ET.fromstring((BASE/"fixtures"/case["svg"]).read_bytes())
        if case["id"].endswith(("text-under-shape","shape-over-text")):
            root.remove(list(root)[-1])
        elif case["id"].endswith("image-under-text"):
            root.remove(next(n for n in root if n.tag.endswith("}text")))
        else:
            next(n for n in root if n.tag.endswith("}g")).attrib.pop("clip-path")
        prefix=out/"native-boundaries"/case["id"]
        source=Path(str(prefix)+".svg");source.write_bytes(ET.tostring(root))
        request=Path(str(prefix)+"-view.json");dump(request,ref["view"])
        command=[str(program),"render",str(source),str(BASE/"fonts"),str(request),str(prefix)]
        painted=subprocess.run(command,capture_output=True,text=True,timeout=120)
        row={"name":case["id"],"command":command,"source_sha256":digest(source.read_bytes()),"exit_code":painted.returncode,"stdout":painted.stdout,"stderr":painted.stderr,"passed":False}
        if painted.returncode==0:
            reference=ref["captures"]["complete"]["file"]
            diff=Path(str(prefix)+"-diff.png")
            compared=subprocess.run([str(out/"oracle-build/debug/bboxy450-svg-reference"),"compare",reference,str(prefix)+".png",str(diff)],capture_output=True,text=True,timeout=30)
            row.update(comparator_exit=compared.returncode,comparator_stdout=compared.stdout,comparator_stderr=compared.stderr,reference=reference,native=str(prefix)+".png",diff=str(diff))
            if compared.returncode==0:
                row["pixels"]=json.loads(compared.stdout)
                row["passed"]=row["pixels"]["changed_fraction"]>policy()["text_budget"]
        controls.append(row)
    return {"rows": controls, "passed": all(c["passed"] for c in controls)}


def run(out, selected=None, refresh_references=True):
    if refresh_references:
        reference_run(out)
    references=json.loads((out/"references/chrome.json").read_text())
    capture_order=json.loads((out/"reference-comparisons.json").read_text())["capture_order"]
    frozen=json.loads((BASE/"fixtures/advisories.json").read_text())
    advisory_error=None
    try:
        frozen_advisories(out,review_run(out))
    except ValueError as error:
        advisory_error=str(error)
    if frozen["policy"]!=policy(): raise ValueError("numeric policy changed")
    advisory={(r["case"],r["view"]["id"]) for r in frozen["cases"]}
    program=out.parent/"debug/examples/svg-loop-native"
    oracle=out/"oracle-build/debug/bboxy450-svg-reference"
    binary_sha=digest(program.read_bytes())
    directory=out/"native";directory.mkdir(exist_ok=True)
    manifests=[("corpus",json.loads((BASE/"corpus/manifest.json").read_text())),("fixtures",json.loads((BASE/"fixtures/manifest.json").read_text()))]
    cases={c["id"]:(BASE/sub/c["svg"],c) for sub,m in manifests for c in m["cases"]}
    from census import census
    case_layers={name:{"render"} for name in cases}
    tags={"text":"text","tspan":"text","image":"image",**dict.fromkeys(("path","rect","circle","ellipse","line","polyline","polygon","use"),"vectors")}
    for sub,manifest in manifests:
        for feature in census(BASE/sub,manifest["cases"])["features"]:
            if feature["category"]!="element" or not feature["feature"].startswith("{http://www.w3.org/2000/svg}"):continue
            layer=tags.get(feature["feature"].split("}")[-1])
            if layer:
                for witness in feature["witnesses"]:
                    if "/defs[" not in witness["source"] and "/clipPath[" not in witness["source"]:
                        case_layers[witness["case"]].add(layer)
    if selected and set(selected)-cases.keys(): raise ValueError("unknown selected case")
    expected_views={(name,v["id"]) for name,(_,case) in cases.items() for v in case["views"]}
    actual_views={(ref["id"],ref["view"]["id"]) for ref in references["runs"]}
    if actual_views!=expected_views or len(actual_views)!=len(references["runs"]):
        raise ValueError("missing/duplicate reference case/scale")
    rows=[]
    for ref in references["runs"]:
        name=ref["id"];view=ref["view"]
        if selected and name not in selected: continue
        check_reference(ref)
        source,case=cases[name]
        if digest(source.read_bytes())!=case["sha256"] or ref["source_sha256"]!=case["sha256"]:
            raise ValueError("native source/reference pin differs: "+name)
        request=directory/"request.json";dump(request,view)
        for layer in ("vectors","text","image","render"):
            ref_geometry=out/"references"/(name+"-"+view["id"]+"-resvg-geometry.json")
            if layer not in case_layers[name]:continue
            if layer=="vectors" and not ref_geometry.is_file():raise ValueError("missing required vector reference")
            if layer=="text" and not ref["geometry"]["texts"]:raise ValueError("missing required text reference geometry")
            if layer=="image" and not any(x["tag"]=="image" for x in ref["geometry"]["shapes"]):raise ValueError("missing required image reference geometry")
            prefix=directory/(name+"-"+view["id"]+"-"+layer)
            capture=Path(str(prefix)+".json");png=Path(str(prefix)+".png")
            cache=Path(str(prefix)+"-key.json")
            key={"binary_sha256":binary_sha,"source_sha256":case["sha256"],"font_set":ref["font_set"],"view":view,"layer":layer}
            reusable=cache.is_file() and json.loads(cache.read_text()).get("request")==key and capture.is_file() and png.is_file()
            if reusable:
                old=json.loads(cache.read_text())
                reusable=old["png_sha256"]==digest(png.read_bytes()) and old["geometry_sha256"]==digest(capture.read_bytes())
            result=None
            if not reusable:
                args=[str(program),layer,str(source),str(BASE/"fonts"),str(request),str(prefix)]
                result=subprocess.run(args,capture_output=True,text=True,timeout=120)
                Path(str(prefix)+".stdout").write_text(result.stdout);Path(str(prefix)+".stderr").write_text(result.stderr)
            row={"case":name,"view":view,"layer":layer,"native":str(png),"native_json":str(capture),"source_sha256":case["sha256"],"native_exit":0 if reusable else result.returncode,"failures":[]}
            rows.append(row)
            if row["native_exit"]!=0:
                row["failures"].append("native preparation/painting failed");continue
            native=json.loads(capture.read_text())
            if native.get("diagnostics"):row["failures"].append("native diagnostics cannot pass")
            if native.get("viewport")!=ref["capture_viewport"] or native.get("view_transform")!=ref["view_transform"] or native.get("request")!={k:view[k] for k in ("camera","scale")} or native["font_set"]!=ref["font_set"] or native["width"]!=math.ceil(view["camera"][2]*view["scale"]) or native["height"]!=math.ceil(view["camera"][3]*view["scale"]):
                row["failures"].append("native font/camera identity differs")
            if layer=="vectors":
                expected=json.loads(ref_geometry.read_text())["paths"]
                try:
                    row["geometry"]=shapes(expected,native["geometry"])
                    if not row["geometry"]["passed"]: row["failures"].append("public native/resvg geometry differs")
                except ValueError as e:row["failures"].append(str(e))
                targets=[("resvg",out/"references"/(name+"-"+view["id"]+"-resvg.png")),("chrome",Path(ref["captures"]["shapes"]["file"]))]
            elif layer in ("text","render"):
                exported=json.loads((BASE/"corpus"/case["exporter_evidence"]).read_text())["text"] if "exporter_evidence" in case else None
                try:
                    row["text_geometry"]=text_geometry(ref["geometry"]["texts"],native["texts"],exported)
                    if not row["text_geometry"]["passed"]:row["failures"].append("text geometry or exporter containment differs")
                except ValueError as e:row["failures"].append(str(e))
                targets=[("chrome",Path(ref["captures"]["text" if layer=="text" else "complete"]["file"]))]
            else:
                targets=[("chrome",Path(ref["captures"]["image"]["file"]))]
            for label,reference in targets:
                diff=Path(str(prefix)+"-"+label+"-diff.png")
                compared=subprocess.run([str(oracle),"compare",str(reference),str(png),str(diff)],capture_output=True,text=True,timeout=30)
                if compared.returncode!=0:
                    row["failures"].append("shared pixel comparator failed");continue
                stat=json.loads(compared.stdout)
                is_advisory=(name,view["id"]) in advisory and label=="chrome" and layer in ("vectors","render")
                budget=policy()["text_budget"] if layer in ("text","render") else policy()["vector_budget"]
                stat.update(reference=str(reference),diff=str(diff),budget=budget,advisory=is_advisory)
                row[label]=stat
                if stat["changed_fraction"]>budget and not is_advisory:row["failures"].append(label+" pixel budget exceeded")
            dump(cache,{"request":key,"png_sha256":digest(png.read_bytes()),"geometry_sha256":digest(capture.read_bytes())})
    result={"runner_sha256":digest(Path(__file__).read_bytes()),"reference_program_sha256":references["source_sha256"],"policy":policy(),"advisories_current":advisory_error is None,"advisory_error":advisory_error,"complete_manifest":not selected,"binary_sha256":binary_sha,"font_set":references["font_set"] if "font_set" in references else frozen["font_set"],"runs":rows,"failures":[r for r in rows if r["failures"]],"proof_limit":"Native and resvg share tiny-skia path coverage; Chrome remains independent. Frozen common-vector advisories mechanically retain full renders as inspected evidence; isolated text/image gates remain hard."}
    required=set()
    for ref in references["runs"]:
        name=ref["id"];view=ref["view"]["id"]
        required.update((name,view,layer) for layer in case_layers[name])
    observed={(r["case"],r["view"]["id"],r["layer"]) for r in rows}
    result["missing_captures"]=len(required-observed)
    if len(observed)!=len(rows):raise ValueError("duplicate native case/scale")
    if not selected:
        result["admission"]=admission(out,program)
        result["broken_controls"]=broken_controls(out,program,rows,references)
    inspection=out/"native-inspected.json"
    recorded=json.loads(inspection.read_text()) if inspection.is_file() else {"files":{}}
    result["inspection_current"]=bool(recorded["files"]) and all(Path(p).is_file() and digest(Path(p).read_bytes())==sha for p,sha in recorded["files"].items())
    result["capture_order"]=capture_order
    result["passed"]= capture_order["passed"] and result["complete_manifest"] and result["advisories_current"] and not result["failures"] and not result["missing_captures"] and result.get("admission",{}).get("passed",False) and result.get("broken_controls",{}).get("passed",False) and result["inspection_current"]
    dump(out/"native-results.json",result)
    report(out,result)
    return result


def report(out,result):
    rows=sorted(result["runs"],key=lambda r:(bool(r["failures"]),max(r.get("resvg",{}).get("changed_fraction",0),r.get("chrome",{}).get("changed_fraction",0))),reverse=True)
    lines=["# Native SVG comparison", "",result["proof_limit"],"",f"{len(rows)} captures; {len(result['failures'])} failures; {result['missing_captures']} complete captures missing.",""]
    sections=[]
    for row in rows:
        title=row["case"]+" / "+row["view"]["id"]+" / "+row["layer"]
        lines.extend(["## "+title,"",str(row["failures"]),""])
        for label in ("resvg","chrome"):
            if label not in row:continue
            comparison=row[label]
            paths=[str(Path(p).relative_to(out)) for p in [comparison["reference"],row["native"],comparison["diff"]]]
            lines.extend([f"{label}: {comparison['changed_fraction']:.9%}; advisory={comparison['advisory']}",""," | ".join(f"[{n}]({p})" for n,p in zip(("reference","native","difference"),paths)),""])
            sections.append("<section><h2>"+html.escape(title+" / "+label)+"</h2><p>"+html.escape(str(row["failures"]))+"</p>"+"".join('<a href="'+html.escape(p)+'"><img src="'+html.escape(p)+'"></a>' for p in paths)+"</section>")
    (out/"report.md").write_text("\n".join(lines))
    (out/"index.html").write_text('<!doctype html><meta charset="utf-8"><title>Native SVG comparison</title><style>body{font:16px system-ui}img{max-width:32%;image-rendering:pixelated}section{border-top:1px solid #aaa}</style><h1>Native SVG comparison</h1>'+"".join(sections))


if __name__=="__main__":
    import sys
    selected=[x for x in os.environ.get("CASES","").replace(","," ").split() if x]
    result=run(Path(sys.argv[1]).resolve(),selected or None)
    print(json.dumps({"passed":result["passed"],"captures":len(result["runs"]),"failures":len(result["failures"]),"admission":result.get("admission",{}).get("passed"),"broken_controls":result.get("broken_controls",{}).get("passed"),"inspection_current":result["inspection_current"],"complete_manifest":result["complete_manifest"],"missing_captures":result["missing_captures"]}))
    sys.exit(0 if result["passed"] else 1)
