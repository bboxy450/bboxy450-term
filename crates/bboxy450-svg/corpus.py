#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Pinned exporter corpus. This does not interpret, lower or render SVG natively."""
import argparse
import base64
from contextlib import contextmanager
import hashlib
import importlib.metadata
import io
import json
import os
from pathlib import Path
import platform
import re
import sys
import threading
import xml.etree.ElementTree as ET

BASE = Path(__file__).resolve().parent
_vega_owner = threading.Lock()
_vega_thread = None
KINDS = ("bar", "line", "scatter", "heatmap", "layered")
DATA = [
    {"label": "Alpha", "x": 1, "y": 3, "other": 2, "group": "First long legend label"},
    {"label": "Beta", "x": 2, "y": 5, "other": 4, "group": "Second long legend label"},
    {"label": "Gamma", "x": 3, "y": 2, "other": 3, "group": "First long legend label"},
    {"label": "Delta", "x": 4, "y": 6, "other": 5, "group": "Second long legend label"},
    {"label": "Epsilon", "x": 5, "y": 4, "other": 6, "group": "First long legend label"},
]
HEAT = [
    {"column": "A", "row": "Top", "value": 1},
    {"column": "B", "row": "Top", "value": 3},
    {"column": "C", "row": "Top", "value": 5},
    {"column": "A", "row": "Bottom", "value": 6},
    {"column": "B", "row": "Bottom", "value": 4},
    {"column": "C", "row": "Bottom", "value": 2},
]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n")


def load_fonts():
    directory = BASE / "fonts"
    raw = (directory / "manifest.json").read_bytes()
    manifest = json.loads(raw)
    from fontTools.ttLib import TTFont
    for entry in [*manifest["faces"], manifest["license"]]:
        path = directory / entry["file"]
        if path.is_symlink() or digest(path.read_bytes()) != entry["sha256"]:
            raise ValueError(f"font/resource digest mismatch: {path}")
    for face in manifest["faces"]:
        with TTFont(directory / face["file"]) as font:
            names = font["name"]
            assert names.getDebugName(1) == manifest["family"]
            assert names.getDebugName(6) == face["postscript_name"]
            assert font["OS/2"].usWeightClass == face["weight"]
            assert "fvar" not in font
    return manifest, "sha256:" + digest(raw)


def external_resources(value):
    """Fail before converter entry; allowed_base_urls=[] remains a separate guard."""
    if isinstance(value, dict):
        for key, child in value.items():
            if key in ("url", "href"):
                raise ValueError("external resource: " + key)
            if key == "mark" and (child == "image" or isinstance(child, dict) and child.get("type") == "image"):
                raise ValueError("external resource: image mark")
            external_resources(child)
    elif isinstance(value, list):
        for child in value:
            external_resources(child)


@contextmanager
def vega_execution():
    """The synchronous corpus process owns configuration and conversion together."""
    import vl_convert as vlc
    global _vega_thread
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise ValueError("chart export unavailable: macOS arm64 required")
    if not _vega_owner.acquire(blocking=False):
        raise ValueError("concurrent Vega conversion refused")
    try:
        _vega_thread = threading.get_ident()
        settings = dict(num_workers=1, base_url=False, allowed_base_urls=[], auto_google_fonts=False,
                        allow_google_fonts=False, missing_fonts="error", embed_local_fonts=False,
                        subset_fonts=False, google_fonts=[], max_v8_heap_size_mb=512,
                        max_v8_execution_time_secs=15, vega_plugins=[], plugin_import_domains=[],
                        allow_per_request_plugins=False, per_request_plugin_import_domains=[])
        vlc.configure(**settings)
        vlc.set_font_directories([str(BASE / "fonts")])
        if any(vlc.get_config()[key] != value for key, value in settings.items()):
            raise ValueError("effective Vega configuration differs")
        yield vlc.get_config()
    finally:
        vlc.configure(embed_local_fonts=False)
        _vega_thread = None
        _vega_owner.release()


def selected_vega_fonts(definition, fonts, weights):
    """Verify full selected bytes; metadata mode never escapes this helper."""
    import vl_convert as vlc
    from fontTools.ttLib import TTFont
    if _vega_thread != threading.get_ident():
        raise ValueError("Vega conversion requires its synchronous owner")
    vlc.configure(embed_local_fonts=True)
    try:
        payload = vlc.vegalite_fonts(definition, vl_version="5.20", include_font_face=True,
                                     subset_fonts=False, auto_google_fonts=False)
    finally:
        vlc.configure(embed_local_fonts=False)
    if not payload:
        raise ValueError("empty selected-font metadata")
    actual = []
    for info in payload:
        for variant in info["variants"]:
            weight = int(variant["weight"])
            expected = next((f for f in fonts["faces"] if f["weight"] == weight), None)
            match = re.search(r"base64,([A-Za-z0-9+/=]+)", variant.get("font_face") or "")
            if expected is None or weight not in weights or match is None:
                raise ValueError("unexpected/missing selected-font metadata")
            raw = base64.b64decode(match.group(1), validate=True)
            with TTFont(io.BytesIO(raw)) as face:
                name = face["name"].getDebugName(6)
            if digest(raw) != expected["sha256"] or name != expected["postscript_name"]:
                raise ValueError("selected-font bytes/face mismatch")
            actual.append({"weight": weight, "sha256": digest(raw), "bytes": len(raw), "postscript_name": name})
    if sorted(f["weight"] for f in actual) != sorted(weights):
        raise ValueError("unexpected/missing selected-font metadata")
    return actual


def final_vega_svg(svg):
    if re.search(r"@font-face|@import|data:(?:font|application/(?:font|x-font))", svg, re.I):
        raise ValueError("final SVG contains embedded font CSS/import/data-font")
    return svg


def vega_exports(out, fonts, identity):
    import altair as alt
    import vl_convert as vlc
    if _vega_thread != threading.get_ident():
        raise ValueError("Vega conversion requires its synchronous owner")
    # CSS strings, unlike logical font identities, require quoting numeric tokens.
    family = "'" + fonts["family"].replace("\\", "\\\\").replace("'", "\\'") + "'"
    data = alt.InlineData(values=DATA)
    xy = alt.Chart(data).encode(x=alt.X("x:Q", title="Sample"), y=alt.Y("y:Q", title="Value"))
    charts = {
        "bar": alt.Chart(data).mark_bar().encode(x="label:N", y="y:Q"),
        "line": xy.mark_line(point=True),
        "scatter": xy.mark_circle(size=80).encode(color="group:N"),
        "heatmap": alt.Chart(alt.InlineData(values=HEAT)).mark_rect().encode(x="column:N", y="row:N", color="value:Q"),
        "layered": xy.mark_line().encode(color="group:N") + xy.mark_point(size=70).encode(color="group:N"),
    }
    cases = []
    for kind in KINDS:
        chart = charts[kind].properties(width=300, height=180, title=f"{kind.title()} comparison")
        chart = chart.configure(
            font=family,
            axis={"labelFont": family, "titleFont": family, "labelFontWeight": 400, "titleFontWeight": 700},
            legend={"labelFont": family, "titleFont": family, "labelFontWeight": 400, "titleFontWeight": 700, "labelLimit": 74},
            title={"font": family, "fontWeight": 700},
        )
        spec = chart.to_dict()
        external_resources(spec)
        config = {"vl_version": "5.20", "vega_version": vlc.get_vega_version(), "configuration": vlc.get_config(), "font_set": identity, "font_family": fonts["family"],
                  "registered_directory": "fonts", "font_weights": [f["weight"] for f in fonts["faces"]]}
        before = final_vega_svg(vlc.vegalite_to_svg(spec, vl_version=config["vl_version"], bundle=False))
        graph = vlc.vegalite_to_scenegraph(spec, vl_version=config["vl_version"])
        weights = {int(t["attributes"]["font-weight"]) for t in emitted_text(before)}
        selected = selected_vega_fonts(spec, fonts, weights)
        svg = final_vega_svg(vlc.vegalite_to_svg(spec, vl_version=config["vl_version"], bundle=False))
        if svg != before or vlc.get_config()["embed_local_fonts"]:
            raise ValueError("metadata changed final SVG/configuration")
        key = "vega-" + kind
        dump(out / (key + ".json"), {"literal_data": HEAT if kind == "heatmap" else DATA, "definition": spec, "export": config})
        dump(out / (key + ".scenegraph.json"), graph)
        dump(out / (key + ".measurements.json"), vega_text_measurements(graph, svg, fonts, identity, selected))
        dump(out / (key + ".font-proof.json"), {"selected": selected, "before_sha256": digest(before.encode()),
             "after_sha256": digest(svg.encode()), "effective_configuration": vlc.get_config()})
        cases.append((key, svg, "vl-convert-python", config))
    return cases


def emitted_text(svg):
    """Record exported XML facts; no SVG interpretation or native geometry."""
    rows = []

    def walk(node, location, transforms):
        transforms = [*transforms, node.attrib.get("transform", "")]
        if node.tag.split("}")[-1] == "text":
            rows.append({"source": location, "text": "".join(node.itertext()), "attributes": dict(node.attrib), "transforms": transforms})
        siblings = {}
        for child in node:
            tag = child.tag.split("}")[-1]
            siblings[tag] = siblings.get(tag, 0) + 1
            walk(child, location + f"/{tag}[{siblings[tag]}]", transforms)

    walk(ET.fromstring(svg), "/svg[1]", [])
    return rows


def vega_text_measurements(graph, svg, fonts, identity, selected):
    """Read real Vega bounds through its existing reactive-geometry surface.

    Uses https://vega.github.io/vega/docs/marks/#reactive-geometry.
    The converter's clone omits bounds. Dependent rect encodings copy actual
    text bounds into whitelisted x/y/width/height fields before that clone.
    Original items retain their layout; angle-zero probes measure advance.
    No Chrome metric or independently implemented font measurement is used.
    """
    import vl_convert as vlc
    if _vega_thread != threading.get_ident():
        raise ValueError("Vega conversion requires its synchronous owner")

    def texts(node):
        if node.get("marktype") == "text":
            yield from node["items"]
        else:
            # Vega paints stable z-index order, not the clone's insertion order.
            for child in sorted(node.get("items", []), key=lambda c: c.get("zindex", 0)):
                yield from texts(child)

    items = list(texts(graph["scenegraph"]))
    marks = []
    for i, item in enumerate(items):
        for suffix, properties in (("layout", item), ("advance", {**item, "x": 0, "y": 0, "angle": 0, "align": "left", "baseline": "alphabetic", "dx": 0, "dy": 0})):
            label = f"text-{i}-{suffix}"
            marks.extend([
                {"name": label, "type": "text", "encode": {"enter": {k: {"value": v} for k, v in properties.items()}}},
                {"name": label + "-bounds", "type": "rect", "from": {"data": label}, "encode": {"update": {
                    "x": {"field": "bounds.x1"}, "y": {"field": "bounds.y1"},
                    "width": {"signal": "datum.bounds.x2-datum.bounds.x1"}, "height": {"signal": "datum.bounds.y2-datum.bounds.y1"}}}},
            ])
    probe = {"width": 1, "height": 1, "padding": 0, "marks": marks}
    measured = vlc.vega_to_scenegraph(probe)
    values = {m["name"]: m["items"][0] for m in measured["scenegraph"]["items"][0]["items"]}
    probe_text = emitted_text(vlc.vega_to_svg(probe, bundle=False))[::2]
    exported = emitted_text(svg)
    if len(items) != len(exported) or len(probe_text) != len(items):
        raise ValueError("missing Vega text correspondence")
    rows = []
    for i, (item, actual, expected) in enumerate(zip(items, exported, probe_text)):
        for key in ("text", "attributes"):
            if actual[key] != expected[key]:
                raise ValueError("Vega text correspondence differs: " + str((i, actual, expected)))
        layout = values[f"text-{i}-layout-bounds"]
        advance = values[f"text-{i}-advance-bounds"]["width"]
        weight = int(item["fontWeight"])
        face = next(f for f in fonts["faces"] if f["weight"] == weight)
        actual_face = next(f for f in selected if f["weight"] == weight)
        rows.append({"text": actual["text"], "raw_text": item["text"], "raw_scenegraph_item": item,
                     "bounds": [layout[k] for k in ("x", "y", "width", "height")], "advance": advance,
                     "face": face["file"], "font_sha256": actual_face["sha256"], "expected_face": face["file"], "expected_font_sha256": face["sha256"],
                     "expected_postscript_name": face["postscript_name"], "resolved_face": actual_face["postscript_name"],
                     "weight": weight, "font_size": item["fontSize"], "emitted": actual})
    return {"coordinates": "raw Vega text parent coordinates; reserved advance along unrotated text axis", "font_set": identity,
            "measurement_api": "vl_convert.vega_to_scenegraph; dependent marks read actual text bounds", "probe": probe,
            "probe_scenegraph": measured, "text": rows}


def matplotlib_exports(out, fonts, identity):
    import matplotlib as mpl
    mpl.use("Agg")
    from matplotlib import font_manager
    from matplotlib.backends.backend_agg import FigureCanvasAgg
    from matplotlib.figure import Figure
    family = fonts["family"]
    paths = {f["weight"]: BASE / "fonts" / f["file"] for f in fonts["faces"]}
    for path in paths.values():
        font_manager.fontManager.addfont(str(path))
    cases = []
    with mpl.rc_context({"font.family": family, "font.weight": 400, "axes.titleweight": 700,
                         "axes.labelweight": 700, "svg.fonttype": "none", "svg.hashsalt": "bboxy450-svg-phase-a"}):
        for kind in KINDS:
            fig = Figure(figsize=(4.5, 3.2), dpi=72)
            canvas = FigureCanvasAgg(fig)
            ax = fig.add_axes([0.16, 0.22, 0.76, 0.62])
            x, y = [r["x"] for r in DATA], [r["y"] for r in DATA]
            if kind == "bar":
                ax.bar(x, y)
                ax.set_xticks(x, [r["label"] for r in DATA], rotation=25)
            elif kind == "line":
                ax.plot(x, y, marker="o")
            elif kind == "scatter":
                ax.scatter(x, y, s=55, c=[0, 1, 0, 1, 0], cmap="viridis")
            elif kind == "heatmap":
                cells = ax.pcolormesh([[1, 3, 5], [6, 4, 2]], cmap="viridis", vmin=1, vmax=6)
                fig.colorbar(cells, ax=ax)
            else:
                ax.plot(x, y, marker="o", label="First long legend label")
                ax.plot(x, [r["other"] for r in DATA], "--s", label="Second long legend label")
                ax.legend(loc="upper left", fontsize=8, borderpad=0.2, labelspacing=0.2)
            ax.set_title(f"{kind.title()} comparison")
            ax.set_xlabel("Sample")
            ax.set_ylabel("Value")
            # Exact file FontProperties are assigned before Agg layout and SVG export.
            from matplotlib.text import Text
            for text in fig.findobj(Text):
                weight = text.get_fontproperties().get_weight()
                bold = weight in (700, "bold")
                selected = paths[700 if bold else 400]
                text.set_fontproperties(font_manager.FontProperties(fname=str(selected), size=text.get_fontsize(), weight="bold" if bold else "normal"))
            canvas.draw()
            rows = []
            for text in fig.findobj(Text):
                if not text.get_visible() or not text.get_text():
                    continue
                props = text.get_fontproperties()
                resolved = Path(font_manager.findfont(props)).resolve()
                assert resolved in [p.resolve() for p in paths.values()]
                box = text.get_window_extent(canvas.get_renderer())
                position = text.get_transform().transform(text.get_position())
                rows.append({"text": text.get_text(), "weight": 700 if resolved == paths[700].resolve() else 400,
                             "face": resolved.name, "bounds": [box.x0, fig.bbox.height - box.y1, box.width, box.height],
                             "anchor": [float(position[0]), float(fig.bbox.height - position[1])],
                             "rotation": text.get_rotation(), "ha": text.get_ha(), "va": text.get_va(), "font_size": text.get_fontsize()})
            key = "matplotlib-" + kind
            config = {"font_family": family, "font_set": identity, "svg.fonttype": "none", "dpi": 72, "font_files": [p.name for p in paths.values()], "svg.hashsalt": mpl.rcParams["svg.hashsalt"]}
            dump(out / (key + ".json"), {"literal_data": HEAT if kind == "heatmap" else DATA, "export": config})
            stream = io.StringIO()
            from matplotlib.backends.backend_svg import RendererSVG
            from unittest.mock import patch
            original_draw_text = RendererSVG.draw_text
            svg_rows = []

            def measured_draw_text(renderer, gc, x, y, value, prop, angle, ismath=False, mtext=None):
                if ismath or mtext is None:
                    raise ValueError("unmeasured exporter text")
                box = mtext.get_window_extent(renderer)
                width, height, descent = renderer.get_text_width_height_descent(value, prop, False)
                resolved = Path(font_manager.findfont(prop)).resolve()
                assert resolved in [p.resolve() for p in paths.values()]
                backend_font = renderer._text2path._get_font(prop)
                if Path(backend_font.fname).resolve() != resolved:
                    raise ValueError("exporter resolved foreign face")
                face = next(f for f in fonts["faces"] if (BASE / "fonts" / f["file"]).resolve() == resolved)
                if backend_font.postscript_name != face["postscript_name"]:
                    raise ValueError("exporter resolved foreign face")
                svg_rows.append({"text": value, "baseline_start": [x, y], "angle": angle,
                                 "bounds": [box.x0, renderer.height - box.y1, box.width, box.height],
                                 "advance": width, "ink_height": height, "descent": descent,
                                 "face": resolved.name, "font_sha256": digest(resolved.read_bytes()), "backend_postscript_name": backend_font.postscript_name,
                                 "weight": 700 if resolved == paths[700].resolve() else 400,
                                 "font_size": prop.get_size_in_points()})
                return original_draw_text(renderer, gc, x, y, value, prop, angle, ismath, mtext)

            with patch.object(RendererSVG, "draw_text", measured_draw_text):
                fig.savefig(stream, format="svg", metadata={"Date": "2026-11-19T00:00:00Z", "Creator": "bboxy450 SVG corpus; matplotlib " + mpl.__version__})
            exported = emitted_text(stream.getvalue())
            if len(exported) != len(svg_rows):
                raise ValueError("missing Matplotlib text correspondence")
            for row, actual in zip(svg_rows, exported):
                if row["text"] != actual["text"]:
                    raise ValueError("Matplotlib emitted text differs")
                row["emitted"] = actual
            dump(out / (key + ".measurements.json"), {"coordinates": "SVG viewBox units; upper-left origin; SVG backend dpi=72",
                                                      "font_set": identity, "text": svg_rows, "agg_text": rows})
            cases.append((key, stream.getvalue(), "matplotlib", config))
            fig.clear()
    return cases


def generate(out):
    fonts, identity = load_fonts()
    out.mkdir(parents=True, exist_ok=True)
    cases = []
    versions = {p: importlib.metadata.version(p) for p in ("altair", "matplotlib", "vl-convert-python")}
    with vega_execution():
        vega = vega_exports(out, fonts, identity)
    for key, svg, exporter, config in vega + matplotlib_exports(out, fonts, identity):
        assert "data-bboxy-font-set" not in svg
        svg = svg.replace("<svg ", '<svg data-bboxy-font-set="' + identity + '" ', 1)
        raw = svg.encode()
        (out / (key + ".svg")).write_bytes(raw)
        element = ET.fromstring(raw)
        box = [float(n) for n in re.split(r"[, ]+", element.attrib["viewBox"].strip())]
        viewport = [0, 0, int(box[2] + 0.999), int(box[3] + 0.999)]
        crop = [round(box[2] / 3, 3), round(box[3] / 3, 3), 48, 32]
        cases.append({"id": key, "exporter": exporter, "svg": key + ".svg", "sha256": digest(raw),
                      "definition": key + ".json", "viewBox": box, "viewport": viewport,
                      "views": [{"id": "1x", "scale": 1, "camera": viewport}, {"id": "2x", "scale": 2, "camera": viewport}, {"id": "8x", "scale": 8, "camera": crop}],
                      "exporter_evidence": key + ".measurements.json",
                      "measurement_status": "real exporter bounds/advance, emitted SVG text and exact selected bytes/face" if exporter == "vl-convert-python" else "SVG-backend Text bounds, advances, emitted anchors and actual face; Agg retained separately",
                      "font_set": identity, "export_configuration_sha256": digest(json.dumps(config, sort_keys=True).encode()),
                      "generator": "../corpus.py"})
        if exporter == "vl-convert-python":
            cases[-1].update({"scenegraph_sha256": digest((out / (key + ".scenegraph.json")).read_bytes()),
                              "measurements_sha256": digest((out / (key + ".measurements.json")).read_bytes())})
    dump(out / "manifest.json", {"phase": "A; corpus only; native renderer absent", "font_set": identity,
                                 "exporters": versions, "generator_sha256": {p: digest((BASE / p).read_bytes()) for p in ("corpus.py", "census.py", "references.py", "measurements.py", "guard_controls.py", "capture.mjs", "reference.rs", "reference_setup.py", "test_corpus.py", "fixtures.py", "review.py")},
                                 "requirements_sha256": digest((BASE / "requirements.lock").read_bytes()), "cases": cases})
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out = args.out.resolve()
    for key in ("MPLCONFIGDIR", "DENO_DIR", "XDG_CACHE_HOME"):
        os.environ[key] = str(args.out / key.lower())
    generated = args.out / "generated"
    cases = generate(generated)
    from census import census
    report = census(generated, cases)
    dump(generated / "census.json", report)
    from fixtures import generate as fixture_generate
    fixture_out = args.out / "generated-fixtures"
    fixture_manifest = fixture_generate(fixture_out)
    fixture_census = census(fixture_out, fixture_manifest["cases"])
    dump(fixture_out / "census.json", fixture_census)
    from fixtures import coverage, check_boundaries
    feature_coverage = coverage(report, fixture_census)
    dump(fixture_out / "coverage.json", feature_coverage)
    dump(args.out / "census.json", {"exported": report, "isolations": fixture_census, "coverage": feature_coverage})
    boundary_evidence = check_boundaries(args.out, fixture_manifest)
    source = BASE / "corpus"
    # UPDATE refreshes derived metadata, never authorizes a new positive SVG.
    pins = {c["id"]: c for c in json.loads((source / "manifest.json").read_text())["cases"]}
    for case in cases:
        for field in ("sha256", "scenegraph_sha256", "measurements_sha256"):
            if field in case and pins[case["id"]].get(field) != case[field]:
                raise ValueError("approved corpus digest changed; a signed amendment is required: " + case["id"] + ":" + field)
    update = os.environ.get("UPDATE_CORPUS") == "1"
    mismatches = []
    for path in sorted(generated.iterdir()):
        target = source / path.name
        if update:
            target.parent.mkdir(exist_ok=True)
            target.write_bytes(path.read_bytes())
        elif not target.is_file() or path.read_bytes() != target.read_bytes():
            mismatches.append(path.name)
    for path in sorted(fixture_out.rglob("*")):
        if not path.is_file():
            continue
        target = BASE / "fixtures" / path.relative_to(fixture_out)
        if update:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(path.read_bytes())
        elif not target.is_file() or target.read_bytes() != path.read_bytes():
            mismatches.append("fixtures/" + str(path.relative_to(fixture_out)))
    result = {"phase": "A", "native_renderer": "absent", "python": platform.python_version(),
              "phase_a_complete": False, "reference_status": "pending reference/measurement checks",
              "cases": len(cases), "font_set": load_fonts()[1], "corpus_mismatches": mismatches,
              "decision_gaps": report["decision_gaps"] + fixture_census["decision_gaps"], "corpus_only": os.environ.get("CORPUS_ONLY") == "1"}
    dump(args.out / "result.json", result)
    if mismatches or result["decision_gaps"]:
        print(json.dumps(result, indent=2))
        return 1
    from references import run as reference_run
    from guard_controls import run as guard_run
    from measurements import compare_exporters
    references = reference_run(args.out)
    controls = guard_run(args.out)
    measurements = compare_exporters(args.out)
    from review import run as review_run, frozen_advisories
    review = review_run(args.out)
    advisories = frozen_advisories(args.out, review)
    result["reference_status"] = "all 66 full Chrome captures and 57 compatible shape-layer comparisons recorded"
    result["font_refusal_controls"] = len(controls["controls"])
    result["measurement_failures"] = len(measurements["failures"])
    result["isolation_cases"] = len(fixture_manifest["cases"])
    result["negative_inputs"] = len(fixture_manifest["negatives"])
    result["boundary_inputs"] = len(boundary_evidence["recipes"])
    result["paint_controls"] = len(review["controls"])
    result["frozen_pixel_advisories"] = len(advisories["cases"])
    result["frozen_advisory_sha256"] = digest((BASE / "fixtures/advisories.json").read_bytes())
    result["reference_geometry_missing"] = sum(r["public_geometry"]["missing"] for r in references["comparisons"])
    result["phase_a_complete"] = not measurements["failures"]
    dump(args.out / "result.json", result)
    print(json.dumps(result, indent=2))
    if not result["phase_a_complete"]:
        print("Phase A failed: exporter correspondence/containment differs.", file=sys.stderr)
        return 1
    if os.environ.get("CORPUS_ONLY") != "1":
        print("Native renderer gate unavailable: Phase A does not prove renderer acceptance.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
