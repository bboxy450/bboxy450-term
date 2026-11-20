# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Phase A references only: project mutually supported layers, never native paint."""
import json
import math
import os
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

from corpus import BASE, digest, dump, load_fonts


def policy():
    source = BASE.parent / "bbt-render/src/chrome/diff.rs"
    raw = source.read_bytes()
    text = raw.decode()
    def constant(name):
        value = re.search(r"(?m)^pub const " + name + r": f64 = ([\d.]+);$", text)
        if not value:
            raise ValueError("canonical comparison constant missing: " + name)
        return float(value[1])
    return {"vector_budget": constant("VECTOR_CHANGED_BUDGET"), "text_budget": constant("TEXT_CHANGED_BUDGET"),
            "text_geometry_px": constant("TEXT_GEOMETRY_TOLERANCE_PX"), "source": str(source.relative_to(BASE.parents[1]))}


def shapes_only(raw, viewport, root_clip=False):
    """Reference projection: remove only the two layers this resvg cannot paint.

    This is XML custody for an existing oracle, not SVG interpretation/lowering.
    IDs on the trusted projection connect public oracle bounds to Chrome paths.
    """
    root = ET.fromstring(raw)
    root.set("width",str(viewport[2]))
    root.set("height",str(viewport[3]))
    root.set("viewBox"," ".join(map(str,viewport)))
    locations = {}
    serial = 0

    def walk(node, location, definitions=False):
        nonlocal serial
        tag = node.tag.split("}")[-1]
        definitions = definitions or tag == "defs"
        if tag in {"path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "use"} and not definitions:
            serial += 1
            if "id" not in node.attrib:
                node.set("id", f"bboxy-reference-{serial}")
            locations[node.attrib["id"]] = location
        siblings = {}
        for child in list(node):
            name = child.tag.split("}")[-1]
            siblings[name] = siblings.get(name, 0) + 1
            if name in {"text", "image"}:
                node.remove(child)
            else:
                walk(child, location + f"/{name}[{siblings[name]}]", definitions)

    walk(root, "/svg[1]")
    if root_clip:
        ns="{http://www.w3.org/2000/svg}"
        defs=ET.SubElement(root,ns+"defs")
        clip=ET.SubElement(defs,ns+"clipPath",{"id":"bboxy-loop-viewport"})
        ET.SubElement(clip,ns+"rect",dict(zip(("x","y","width","height"),map(str,viewport))))
        group=ET.Element(ns+"g",{"clip-path":"url(#bboxy-loop-viewport)"})
        for child in list(root):
            if child is not defs:
                root.remove(child);group.append(child)
        root.append(group)
    return ET.tostring(root), locations


def compare_geometry(shapes, paths):
    """Compare public independent-reference measurements without omitting uses.

    Keep exact equality and floating-point deltas visible. The later vector
    native gate still requires exact geometry; reference arithmetic is not a
    new native tolerance or a geometry advisory.
    """
    shapes = {s["source"]: s for s in shapes if s["tag"] != "image"}
    observed = {}
    for p in paths:
        if not p.get("source") or p["source"] in observed:
            raise ValueError("missing/duplicate public oracle path correspondence")
        observed[p["source"]] = p
    if shapes.keys() != observed.keys() or not shapes:
        raise ValueError("missing public reference geometry: " + str(shapes.keys() ^ observed.keys()))
    rows = []
    for source, shape in shapes.items():
        path = observed[source]
        if shape["endpoints"] is None or path["endpoints"] is None:
            raise ValueError("missing public path endpoints: " + source)
        a = [*shape["bounds"], *sum(shape["endpoints"], [])]
        b = [*path["bounds"], *sum(path["endpoints"], [])]
        if not all(math.isfinite(v) for v in [*a, *b]):
            raise ValueError("nonfinite public reference geometry")
        errors = [abs(x-y) for x, y in zip(a, b)]
        rows.append({"source": source, "chrome": shape, "resvg": path, "exact": a == b,
                     "maximum_error_px": max(errors), "errors_px": errors})
    maximum = max(r["maximum_error_px"] for r in rows)
    if maximum > policy()["text_geometry_px"]:
        raise ValueError("independent public reference geometry differs: " + str(maximum))
    return {"paths": rows, "missing": 0, "maximum_error_px": maximum,
            "exact_paths": sum(r["exact"] for r in rows),
            "native_geometry": "not yet implemented; reference float deltas do not waive its exact vector gate"}


def order_independence(out, request, chrome):
    """The same bounded cases must produce identical PNG bytes in every order."""
    names = ("vega-heatmap", "isolation-text", "isolation-png-alpha")
    cases = [next(c for c in request["cases"] if c["id"] == name) for name in names]

    def pixels(runs):
        return {(r["id"], r["view"]["id"], mode): capture["sha256"]
                for r in runs for mode, capture in r["captures"].items()}

    expected = pixels([r for r in chrome["runs"] if r["id"] in names])
    key = digest(json.dumps({"capture": chrome["source_sha256"], "checker": digest(Path(__file__).read_bytes()),
                             "font_set": chrome["font_set"], "browser": chrome["browser"], "cases": cases,
                             "pixels": sorted(expected.items())}, sort_keys=True).encode())
    root = out / "order-controls"
    root.mkdir(exist_ok=True)
    cache = root / "result.json"
    if cache.is_file():
        previous = json.loads(cache.read_text())
        if (previous.get("cache_key") == key and previous.get("passed")
                and len(previous["runs"]) == 5
                and all(Path(p).is_file() and digest(Path(p).read_bytes()) == sha
                        for row in previous["runs"] for p, sha in row["file_hashes"].items())):
            return previous
    rows = []
    orders = [(c["id"], [c]) for c in cases] + [("normal", cases), ("reverse", list(reversed(cases)))]
    for name, ordered in orders:
        directory = root / name
        recipe = root / (name + "-request.json")
        dump(recipe, {**request, "out": str(directory), "cases": ordered})
        with (root / (name + ".log")).open("w") as log:
            result = subprocess.run(["node", str(BASE / "capture.mjs"), str(recipe)], stdout=log, stderr=subprocess.STDOUT)
        if result.returncode:
            raise ValueError("order control capture failed: " + name)
        captured = json.loads((directory / "chrome.json").read_text())
        custody = json.loads((directory / "owned-process.json").read_text())
        wanted = {k: sha for k, sha in expected.items() if k[0] in {c["id"] for c in ordered}}
        observed = pixels(captured["runs"])
        differing = sorted(k for k in wanted.keys() | observed.keys() if wanted.get(k) != observed.get(k))
        row = {"order": name, "exit_code": result.returncode, "pngs": len(observed), "differing": differing,
               "custody": custody, "file_hashes": {**captured["file_hashes"],
                   str(directory / "chrome.json"): digest((directory / "chrome.json").read_bytes()),
                   str(directory / "owned-process.json"): digest((directory / "owned-process.json").read_bytes())},
               "passed": not differing and custody.get("reaped") and custody.get("profile_removed")}
        rows.append(row)
    result = {"cache_key": key, "cases": list(names), "capture_source_sha256": chrome["source_sha256"],
              "runs": rows, "passed": all(r["passed"] for r in rows)}
    dump(cache, result)
    if not result["passed"]:
        raise ValueError("capture PNG bytes depend on case order; see " + str(cache))
    return result


def run(out):
    source = BASE / "corpus"
    manifest = json.loads((source / "manifest.json").read_text())
    fixtures = json.loads((BASE / "fixtures/manifest.json").read_text())
    cases = [{**c, "source_directory": source} for c in manifest["cases"]]
    cases += [{**c, "source_directory": BASE / "fixtures"} for c in fixtures["cases"]]
    oracle=out / "oracle-build/debug/bboxy450-svg-reference"
    for case in cases:
        case["capture_views"]={v["id"]:json.loads(subprocess.check_output([str(oracle),"viewport",json.dumps(case["viewBox"]),json.dumps(v["camera"]),str(v["scale"])])) for v in case["views"]}
    refs = out / "references"
    refs.mkdir(parents=True, exist_ok=True)
    request = {"out": str(refs), "font_directory": str(BASE / "fonts"),
               "cases": [{**{k: v for k, v in c.items() if k != "source_directory"}, "svg": str(c["source_directory"] / c["svg"])} for c in cases]}
    dump(out / "capture-request.json", request)
    capture_sha = digest(b"".join(p.read_bytes() for p in (BASE / "capture.mjs", BASE.parent / "bboxybook/capture-cdp.mjs", BASE.parent / "bboxybook/capture-startup.mjs", BASE / "src/viewport.rs")))
    cache = refs / "chrome.json"
    chrome = json.loads(cache.read_text()) if cache.is_file() else {}
    browser_version = subprocess.check_output([os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"), "--version"], text=True, timeout=10).strip().replace("Google Chrome ", "Chrome/")
    # Camera numbers retain their values across Python/JS; JSON 108.0 versus
    # 108 is serialization noise, not a new camera or a stale reference.
    expected = {(c["id"], v["id"], c["sha256"], tuple(v["camera"]), v["scale"]) for c in cases for v in c["views"]}
    actual = {(r["id"], r["view"]["id"], r["source_sha256"], tuple(r["view"]["camera"]), r["view"]["scale"]) for r in chrome.get("runs", [])}
    files = chrome.get("file_hashes", {})
    intact = bool(files) and all(Path(p).is_file() and digest(Path(p).read_bytes()) == h for p, h in files.items())
    if chrome.get("source_sha256") != capture_sha or actual != expected or chrome.get("font_set") != load_fonts()[1] or chrome.get("browser", {}).get("Browser") != browser_version or not intact:
        subprocess.run(["node", str(BASE / "capture.mjs"), str(out / "capture-request.json")], check=True)
        chrome = json.loads(cache.read_text())
    approved = {f["postscript_name"] for f in load_fonts()[0]["faces"]}
    for r in chrome["runs"]:
        c = next(c for c in cases if c["id"] == r["id"])
        if (not r["geometry"]["shapes"] and not r["geometry"]["texts"]) or (c.get("text", True) and (not r["geometry"]["texts"] or not r["platformFontNodes"])):
            raise ValueError("missing chart geometry/font evidence: " + r["id"])
        for n in r["platformFontNodes"]:
            if not n["fonts"] or not any(f["glyphCount"] > 0 for f in n["fonts"]):
                raise ValueError("empty actual face evidence: " + r["id"])
            if any(not f["isCustomFont"] or f["postScriptName"] not in approved for f in n["fonts"]):
                raise ValueError("unapproved actual face: " + r["id"])
        for capture in r["captures"].values():
            if digest(Path(capture["file"]).read_bytes()) != capture["sha256"]:
                raise ValueError("stale reference pixels: " + capture["file"])
    oracle = out / "oracle-build/debug/bboxy450-svg-reference"
    identity = json.loads(subprocess.check_output([str(oracle), "identity"]))
    if identity["resvg"] != "0.48.1" or identity["features"] or identity["text"] or identity["raster_images"]:
        raise ValueError("wrong oracle capabilities")
    rules = policy()
    oracle_hash = digest(oracle.read_bytes())
    runner_hash = digest((BASE / "references.py").read_bytes())
    result_path = out / "reference-comparisons.json"
    previous = json.loads(result_path.read_text()) if result_path.is_file() else {}
    comparisons = []
    cache_hits = 0
    for c in cases:
        raw = (c["source_directory"] / c["svg"]).read_bytes()
        if digest(raw) != c["sha256"]:
            raise ValueError("source digest changed")
        for view in c["views"]:
            mapping=c["capture_views"][view["id"]]
            layer,locations=shapes_only(raw,mapping["viewport"],mapping["root_clip"])
            if not locations:
                continue
            svg=refs/(c["id"]+"-"+view["id"]+"-shapes.svg")
            svg.write_bytes(layer)
            prefix = refs / (c["id"] + "-" + view["id"])
            png, geometry, diff = (Path(str(prefix) + suffix) for suffix in ("-resvg.png", "-resvg-geometry.json", "-reference-diff.png"))
            browser = next(r for r in chrome["runs"] if r["id"] == c["id"] and r["view"] == view)
            key = digest(json.dumps({"svg": c["sha256"], "layer": digest(layer), "view": view, "font_set": load_fonts()[1],
                                     "oracle_binary": oracle_hash, "runner_program": runner_hash, "capture_program": capture_sha, "browser": chrome["browser"]["Browser"],
                                     "chrome_pixels": browser["captures"]["shapes"]["sha256"], "policy": rules}, sort_keys=True).encode())
            cached = next((r for r in previous.get("comparisons", []) if r.get("cache_key") == key), None)
            if cached and set(cached["file_hashes"]) == {str(png), str(geometry), str(diff)} and all(Path(p).is_file() and digest(Path(p).read_bytes()) == h for p, h in cached["file_hashes"].items()):
                comparisons.append(cached)
                cache_hits += 1
                continue
            subprocess.run([str(oracle), "render", str(svg), str(png), str(view["scale"]), *map(str, view["camera"]), str(geometry),json.dumps(c["viewBox"])], check=True)
            metrics = json.loads(subprocess.check_output([str(oracle), "compare", browser["captures"]["shapes"]["file"], str(png), str(diff)]))
            bounds = json.loads(geometry.read_text())
            for row in bounds["paths"]:
                row["source"] = locations.get(row["id"])
            dump(geometry, bounds)
            public_geometry = compare_geometry(browser["geometry"]["shapes"], bounds["paths"])
            comparisons.append({"case": c["id"], "view": view, "source_sha256": c["sha256"], "layer_sha256": digest(layer), "cache_key": key,
                                "file_hashes": {str(p): digest(p.read_bytes()) for p in (png, geometry, diff)},
                                "chrome": browser["captures"]["shapes"], "resvg": str(png), "diff": str(diff),
                                "oracle_geometry": str(geometry), **metrics,
                                "public_geometry": public_geometry,
                                "over_vector_budget": metrics["changed_fraction"] > rules["vector_budget"],
                                "over_mixed_budget": metrics["changed_fraction"] > rules["text_budget"]})
    capture_order = order_independence(out, request, chrome)
    result = {"phase": "reference evidence", "policy": rules, "oracle": identity,
              "oracle_binary_sha256": oracle_hash, "reference_cache_hits": cache_hits, "capture_order": capture_order,
              "font_set": load_fonts()[1], "chrome": chrome["browser"], "captures": len(chrome["runs"]), "comparisons": comparisons}
    dump(out / "reference-comparisons.json", result)
    return result


if __name__ == "__main__":
    import sys
    result = run(Path(sys.argv[1]).resolve())
    print(json.dumps({"compared": len(result["comparisons"]), "over_vector_budget": sum(r["over_vector_budget"] for r in result["comparisons"]),
                      "over_mixed_budget": sum(r["over_mixed_budget"] for r in result["comparisons"])}))
