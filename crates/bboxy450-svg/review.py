# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Reference-only causal paint controls, PNG sampling witness and review index."""
import html
from io import BytesIO
import json
import os
import math
from pathlib import Path

from corpus import BASE, digest, dump
from references import CAPTURE_NOISE_LIMITS, capture_noise, compare_png, policy, within_capture_noise
from reference_browser import require_version


REFERENCE_INSPECTION = "phase-a-inspected.json"


def comparison_images(out, comparisons):
    files = {}
    for first, second, row in comparisons:
        size = (row["width"], row["height"])
        for path in (first, second):
            files[str(Path(path).relative_to(out))] = (size, None)
        files[str(Path(row["diff"]).relative_to(out))] = (size, row["changed_fraction"])
    return files


def inspect_images(out, receipt, files):
    from PIL import Image

    if not files:
        raise ValueError(f"no comparison images for {receipt}")
    hashes = {}
    for path, (size, changed_fraction) in files.items():
        try:
            raw = (out / path).read_bytes()
            with Image.open(BytesIO(raw), formats=("PNG",)) as png:
                if png.size != size:
                    raise ValueError(f"inspection dimensions differ: {path}")
                png.load()
                if changed_fraction is not None:
                    # The shared Rust comparator uses red=255 for changed pixels
                    # and gray <=127 for the rest (bbt-render/src/chrome/diff.rs).
                    if (png.mode != "RGBA"
                            or png.histogram()[255] / (size[0] * size[1]) != changed_fraction):
                        raise ValueError(f"inspection diff does not match the changed fraction: {path}")
        except OSError as error:
            raise ValueError(f"cannot inspect image: {path}") from error
        hashes[path] = digest(raw)
    dump(out / receipt, {"files": hashes})


def require_inspection(out, receipt, files):
    try:
        recorded = json.loads((out / receipt).read_text())["files"]
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise ValueError(f"missing or invalid image inspection: {receipt}") from error
    if not files or not isinstance(recorded, dict) or recorded.keys() != files.keys():
        raise ValueError(f"incomplete image inspection: {receipt}")
    for path, expected in recorded.items():
        try:
            actual = digest((out / path).read_bytes())
        except OSError as error:
            raise ValueError(f"missing or stale image inspection: {path}") from error
        if actual != expected:
            raise ValueError(f"missing or stale image inspection: {path}")


def reference_images(out, comparisons):
    return comparison_images(
        out, ((row["chrome"]["file"], row["resvg"], row) for row in comparisons["comparisons"])
    )


def run(out):
    from PIL import Image
    chrome = json.loads((out / "references/chrome.json").read_text())
    comparisons = json.loads((out / "reference-comparisons.json").read_text())
    oracle = out / "oracle-build/debug/bboxy450-svg-reference"
    directory = out / "review"
    directory.mkdir(exist_ok=True)

    def compare(a, b, name):
        diff = directory / (name+"-diff.png")
        return {**compare_png(oracle, a, b, diff), "diff": str(diff)}

    controls, interpolation = [], []
    pixels = [(255, 0, 0, 255), (0, 255, 0, 255), (0, 0, 255, 128), (255, 255, 255, 0)]
    for run in chrome["runs"]:
        key, view, captures = run["id"], run["view"], run["captures"]
        if key not in ("isolation-gradient", "isolation-png-alpha", "isolation-png-transform-clip"):
            continue
        prefix = key+"-"+view["id"]
        kind = "without-gradient" if key == "isolation-gradient" else "without-image"
        result = compare(captures["complete"]["file"], captures[kind]["file"], prefix+"-omitted")
        if within_capture_noise(result):
            raise ValueError("capture noise bound accepted dropped paint: " + prefix)
        if result["changed_fraction"] <= policy()["vector_budget"]:
            raise ValueError("dropped-paint control did not fail: " + prefix)
        controls.append({"case": key, "view": view, "omission": kind, **result})
        if key != "isolation-png-alpha":
            continue
        # A bounded camera samples the four premultiplied source texels in sRGB.
        # This reference calculation is not an SVG interpreter or native painter.
        x, y, w, h = view["camera"]
        scale = view["scale"]
        pw, ph = math.ceil(w*scale), math.ceil(h*scale)
        rgba = bytearray([255]) * (pw*ph*4)
        for py in range(ph):
            for px in range(pw):
                wx, wy = x+(px+0.5)/scale, y+(py+0.5)/scale
                if not (4 <= wx < 60 and 4 <= wy < 60):
                    continue
                sx, sy = (wx-4)/28-0.5, (wy-4)/28-0.5
                ix, iy = math.floor(sx), math.floor(sy)
                fx, fy = sx-ix, sy-iy
                total, alpha = [0., 0., 0.], 0.
                for dx, dy, factor in ((0, 0, (1-fx)*(1-fy)), (1, 0, fx*(1-fy)), (0, 1, (1-fx)*fy), (1, 1, fx*fy)):
                    color = pixels[min(1, max(0, iy+dy))*2+min(1, max(0, ix+dx))]
                    a = color[3]/255
                    alpha += a*factor
                    for c in range(3):
                        total[c] += color[c]*a*factor
                offset = (py*pw+px)*4
                rgba[offset:offset+4] = bytes([*(round(v+255*(1-alpha)) for v in total), 255])
        expected = directory / (prefix+"-bilinear.png")
        Image.frombytes("RGBA", (pw, ph), bytes(rgba)).save(expected)
        smooth = compare(captures["image-auto"]["file"], expected, prefix+"-bilinear")
        nearest = compare(captures["image-auto"]["file"], captures["image-nearest"]["file"], prefix+"-nearest")
        if smooth["changed_fraction"] > policy()["text_budget"] or nearest["changed_fraction"] <= policy()["text_budget"]:
            raise ValueError("PNG interpolation isolation failed: " + str((view, smooth, nearest)))
        interpolation.append({"case": key, "view": view, "expected": str(expected), "bilinear": smooth,
                              "nearest_broken_control": nearest, "source": captures["image-auto"]})
    if len(controls) != 9 or len(interpolation) != 3:
        raise ValueError("missing paint/isolation scale")
    dump(out / "paint-controls.json", {"controls": controls, "interpolation": interpolation,
         "sampling": "premultiplied-alpha bilinear sRGB; default image placement; nearest must visibly disagree"})
    inspect_images(out, REFERENCE_INSPECTION, reference_images(out, comparisons))
    return {"controls": controls, "interpolation": interpolation, "comparisons": comparisons}


def report(out, comparisons, advisories):
    rows = sorted(comparisons["comparisons"], key=lambda r: r["changed_fraction"], reverse=True)
    text = ["# SVG corpus reference review", "", "Reference captures only. Native acceptance is recorded by native.py.", "",
            f"{len(rows)} compatible vector-layer pairs; {len(advisories['cases'])} frozen pixel advisories. Geometry remains measured and cannot become advisory.", ""]
    cells = []
    for row in rows:
        images = [row["chrome"]["file"], row["resvg"], row["diff"]]
        relative = [str(Path(p).relative_to(out)) for p in images]
        title = row["case"]+" / "+row["view"]["id"]
        text += [f"## {title}", "", f"Changed {row['changed_fraction']:.9%}; public geometry maximum {row['public_geometry']['maximum_error_px']:.9g} px.", "",
                 " | ".join(f"[{name}]({path})" for name, path in zip(("Chrome", "resvg", "difference"), relative)), ""]
        cells.append("<section><h2>"+html.escape(title)+"</h2><p>Changed "+str(row["changed_fraction"])+"; reference comparison</p>"+
                     "".join('<a href="'+html.escape(p)+'"><img alt="'+name+'" src="'+html.escape(p)+'"></a>' for name,p in zip(("Chrome", "resvg", "difference"), relative))+"</section>")
    (out / "report.md").write_text("\n".join(text))
    (out / "index.html").write_text('<!doctype html><meta charset="utf-8"><title>SVG corpus references</title><style>body{font:16px system-ui}img{max-width:32%;image-rendering:pixelated}section{border-top:1px solid #aaa}</style><h1>SVG corpus references</h1><p>Reference-only Chrome / resvg / difference, worst first.</p>'+"".join(cells))


def frozen_advisories(out, comparisons):
    require_inspection(out, REFERENCE_INSPECTION, reference_images(out, comparisons))
    frozen = json.loads((BASE / "fixtures/advisories.json").read_text())
    require_version(comparisons["chrome"]["Browser"], frozen["chrome"])
    if frozen["policy"] != policy():
        raise ValueError("frozen numeric comparison policy differs from canonical constants")
    rows = comparisons["comparisons"]
    expected = {(r["case"], r["view"]["id"]) for r in rows if r["over_vector_budget"]}
    actual = {(r["case"], r["view"]["id"]) for r in frozen["cases"]}
    if actual != expected or len(actual) != len(frozen["cases"]):
        raise ValueError("frozen reference advisory set changed")
    manifests=[("corpus",json.loads((BASE/"corpus/manifest.json").read_text())),("fixtures",json.loads((BASE/"fixtures/manifest.json").read_text()))]
    fractional={c["id"] for _,m in manifests for c in m["cases"] if any(not float(v).is_integer() for v in c["viewBox"])}
    refreshed = {}
    noise = []
    oracle = out / "oracle-build/debug/bboxy450-svg-reference"
    for entry in frozen["cases"]:
        row = next(r for r in rows if (r["case"], r["view"]) == (entry["case"], entry["view"]))
        files = (row["chrome"]["file"], row["resvg"], row["diff"])
        hashes=[digest(Path(p).read_bytes()) for p in files]
        if entry["source_sha256"]!=row["source_sha256"]:raise ValueError("frozen advisory authored source changed")
        case, view = entry["case"], entry["view"]["id"]
        baseline = BASE / "fixtures/advisory-chrome" / f"{case}-{view}-shapes.png"
        pinned = entry["pixel_sha256"]
        if not baseline.is_file() or digest(baseline.read_bytes()) != pinned[0]:
            raise ValueError(f"missing or stale frozen Chrome baseline: {case}/{view}")
        drift = abs(entry["changed_fraction"] - row["changed_fraction"])
        if not drift and pinned == hashes:
            continue
        if os.environ.get("UPDATE_REFERENCES") == "1" and case in fractional:
            entry["changed_fraction"] = row["changed_fraction"]
            entry["pixel_sha256"] = hashes
            refreshed[(case, view)] = (baseline, files[0])
            continue
        # A different resvg image, an unbound diff or a diff-only change is not Chrome noise.
        if (hashes[1] != pinned[1] or hashes[0] == pinned[0]
                or hashes[0] != row["chrome"]["sha256"] or hashes[2] != row["file_hashes"][files[2]]
                or drift > CAPTURE_NOISE_LIMITS["changed_fraction"]):
            raise ValueError(f"frozen advisory changed outside Chrome noise limits: {case}/{view}")
        metrics = capture_noise(oracle, baseline, files[0])
        if not metrics["passed"]:
            raise ValueError(f"frozen Chrome noise exceeds limits: {case}/{view} {metrics}")
        noise.append({"case": case, "view": view, "pinned_sha256": pinned, "observed_sha256": hashes,
                      "advisory_fraction_drift": drift, **metrics})
    if frozen["font_set"] != comparisons["font_set"] or frozen["resvg"] != comparisons["oracle"]["resvg"]:
        raise ValueError("frozen advisory tool/font identity changed")
    if refreshed:
        for baseline, current in refreshed.values():
            baseline.write_bytes(Path(current).read_bytes())
        dump(BASE/"fixtures/advisories.json",frozen)
        dump(out/"fractional-advisory-refresh.json",{"rows":list(refreshed),"authority":"specs/svg-renderer.den@60841c3b9bf030a2f9b2a6786b982d2d10382640ca196ce0a5304f889de07199"})
    dump(out / "frozen-advisory-noise.json", {"limits": CAPTURE_NOISE_LIMITS, "rows": noise})
    report(out, comparisons, frozen)
    return frozen


if __name__ == "__main__":
    import sys
    result = run(Path(sys.argv[1]).resolve())
    print(json.dumps({"paint_controls": len(result["controls"]), "interpolation": result["interpolation"]}, indent=2))
