# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Exporter correspondence and advance containment; no native paint or geometry."""
import json
import math

from census import declarations
from corpus import BASE, dump, load_fonts
from references import policy


def advance_containment(reserved, rendered, anchor, axis_scale, tolerance):
    if anchor not in ("start", "middle", "end"):
        raise ValueError("unknown baseline anchor mode")
    if not all(math.isfinite(v) and v >= 0 for v in (reserved, rendered, axis_scale, tolerance)) or axis_scale == 0:
        raise ValueError("missing/nonfinite advance")
    reserved *= axis_scale
    fraction = {"start": 0, "middle": 0.5, "end": 1}[anchor]
    return {"reserved_physical": reserved, "rendered_physical": rendered, "overflow_px": rendered-reserved,
            "allowed_total_px": tolerance, "allowed_ends_px": [fraction*tolerance, (1-fraction)*tolerance],
            "slot_from_anchor": [-fraction*reserved, (1-fraction)*reserved], "passed": rendered <= reserved+tolerance}


def compare_exporters(out):
    source = BASE / "corpus"
    manifest = json.loads((source / "manifest.json").read_text())
    chrome = json.loads((out / "references/chrome.json").read_text())
    fonts, identity = load_fonts()
    tolerance = policy()["text_geometry_px"]
    measurements, failures = [], []
    for case in manifest["cases"]:
        exported = json.loads((source / case["exporter_evidence"]).read_text())
        config = json.loads((source / case["definition"]).read_text())["export"]
        if exported.get("font_set") != identity or config.get("font_set") != identity or case["font_set"] != identity:
            raise ValueError("exporter font identity missing/mismatch")
        for run in [r for r in chrome["runs"] if r["id"] == case["id"]]:
            reference = run["geometry"]["texts"]
            if len(exported["text"]) != len(reference) or len(run["platformFontNodes"]) != len(reference):
                failures.append({"case": case["id"], "view": run["view"]["id"], "reason": "text/face correspondence count differs"})
                continue
            for index, (exp, ref, actual_fonts) in enumerate(zip(exported["text"], reference, run["platformFontNodes"])):
                emitted = exp["emitted"]
                attrs = emitted["attributes"]
                style = dict(declarations(attrs.get("style", "")))
                anchor = style.get("text-anchor", attrs.get("text-anchor", "start"))
                face = next(f for f in fonts["faces"] if f["weight"] == exp["weight"])
                problems = []
                if exp["font_sha256"] != face["sha256"] or exp["face"] != face["file"]:
                    problems.append("measured font bytes differ")
                if case["exporter"] == "vl-convert-python" and (exp["expected_font_sha256"] != face["sha256"] or exp["expected_face"] != face["file"]):
                    problems.append("requested exporter font bytes differ")
                if exp.get("backend_postscript_name", exp.get("resolved_face")) != face["postscript_name"]:
                    problems.append("actual exporter face differs")
                if not actual_fonts["fonts"] or any(not f["isCustomFont"] or f["postScriptName"] != face["postscript_name"] for f in actual_fonts["fonts"]):
                    problems.append("actual reference face differs from expected weight")
                if emitted["source"] != ref["source"] or emitted["text"] != exp["text"] or emitted["text"] != ref["text"]:
                    problems.append("emitted text/source differs")
                if attrs != ref["attributes"] or emitted["transforms"] != ref["transforms"]:
                    problems.append("emitted attributes/transform differ")
                if str(exp["weight"]) != ref["weight"] or abs(exp["font_size"]-float(ref["font_size"].removesuffix("px")))*ref["advance_axis_scale"] > tolerance:
                    problems.append("emitted size/weight differs")
                if anchor != ref["anchor_mode"]:
                    problems.append("emitted anchor mode differs")
                # XML facts are checked against DOM facts, then both use the same
                # measured reference transform. No guessed layout-box baseline.
                x, y = float(attrs.get("x", 0)), float(attrs.get("y", 0))
                a, b, c, d, e, f = ref["matrix"]
                scale = run["view"]["scale"]
                expected_anchor = [(a*x+c*y+e)*scale, (b*x+d*y+f)*scale]
                anchor_error = max(abs(a-b) for a, b in zip(expected_anchor, ref["baseline_anchor"]))
                if not math.isfinite(anchor_error) or anchor_error > tolerance:
                    problems.append("emitted baseline anchor differs")
                containment = advance_containment(exp["advance"], ref["advance"], anchor, ref["advance_axis_scale"], tolerance)
                if not containment["passed"]:
                    problems.append("shaped advance exceeds exporter reservation")
                if not all(math.isfinite(v) for v in exp["bounds"]):
                    problems.append("missing/nonfinite raw exporter bounds")
                row = {"case": case["id"], "view": run["view"], "source": ref["source"], "index": index, "text": exp["text"],
                       "face": exp["face"], "weight": exp["weight"], "raw_exporter_bounds": exp["bounds"], "raw_chrome_bounds": ref["bounds"],
                       "emitted_anchor": expected_anchor, "anchor_mode": anchor, "anchor_error_px": anchor_error,
                       "containment": containment, "problems": problems}
                measurements.append(row)
                if problems:
                    failures.append(row)
    result = {"phase": "A; native equality gate not yet available", "measurements": measurements, "failures": failures,
              "raw_layout_boxes": "retained separately; unlike heights and left draw/first-character origins are not equality failures",
              "advisory": False}
    dump(out / "exporter-comparisons.json", result)
    return result
