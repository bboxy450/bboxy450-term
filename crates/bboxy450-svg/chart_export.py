# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Shared chart exporter and font policy for stored images and the reference corpus."""
# Adapted from bboxy450-term's SVG corpus font and measurement fixtures.
# https://github.com/bboxy450/bboxy450-term/blob/main/crates/bboxy450-svg/corpus.py
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

import base64
from contextlib import contextmanager
import hashlib
import io
import json
from pathlib import Path
import platform
import re
import sys
import threading
import xml.etree.ElementTree as ET

FONT_DIR = Path(__file__).resolve().parent / "fonts"
_export_owner = threading.Lock()
_vega_thread = None


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require_vega_owner():
    if _vega_thread != threading.get_ident():
        raise ValueError("Vega conversion requires its synchronous owner")


def load_fonts(directory=FONT_DIR):
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
            if (names.getDebugName(1) != manifest["family"]
                    or names.getDebugName(6) != face["postscript_name"]
                    or font["OS/2"].usWeightClass != face["weight"] or "fvar" in font):
                raise ValueError("chart font face disagrees with the manifest")
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
def vega_execution(directory=FONT_DIR):
    """The synchronous corpus process owns configuration and conversion together."""
    import vl_convert as vlc
    global _vega_thread
    if sys.platform != "darwin" or platform.machine() != "arm64":
        raise ValueError("chart export unavailable: macOS arm64 required")
    if not _export_owner.acquire(blocking=False):
        raise ValueError("concurrent Vega conversion refused")
    try:
        _vega_thread = threading.get_ident()
        settings = dict(num_workers=1, base_url=False, allowed_base_urls=[], auto_google_fonts=False,
                        allow_google_fonts=False, missing_fonts="error", embed_local_fonts=False,
                        subset_fonts=False, google_fonts=[], max_v8_heap_size_mb=512,
                        max_v8_execution_time_secs=15, vega_plugins=[], plugin_import_domains=[],
                        allow_per_request_plugins=False, per_request_plugin_import_domains=[])
        vlc.configure(**settings)
        vlc.set_font_directories([str(directory)])
        if any(vlc.get_config()[key] != value for key, value in settings.items()):
            raise ValueError("effective Vega configuration differs")
        yield vlc.get_config()
    finally:
        vlc.configure(embed_local_fonts=False)
        _vega_thread = None
        _export_owner.release()


def selected_vega_fonts(definition, fonts, weights):
    """Verify full selected bytes; metadata mode never escapes this helper."""
    import vl_convert as vlc
    from fontTools.ttLib import TTFont
    require_vega_owner()
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


def svg_font_identity(svg, identity):
    """Record the selected font manifest without rewriting the exported CSS."""
    if "data-bboxy-font-set" in svg or "<svg " not in svg:
        raise ValueError("cannot record font identity on the exported SVG")
    return svg.replace("<svg ", '<svg data-bboxy-font-set="' + identity + '" ', 1)


def font_weight(value):
    if value == "normal":
        return 400
    if value == "bold":
        return 700
    return int(value)


def prepare_vega(plot, directory):
    import copy
    import vl_convert as vlc
    from importlib.metadata import version

    if version("altair") != "5.5.0" or version("vl-convert-python") != "2.0.0rc7":
        raise RuntimeError("chart export requires the Make-managed Python exporter environment")
    directory = Path(directory)
    fonts, identity = load_fonts(directory)
    definition = copy.deepcopy(plot if isinstance(plot, dict) else plot.to_dict())
    external_resources(definition)
    family = "'" + fonts["family"].replace("\\", "\\\\").replace("'", "\\'") + "'"

    def font_fields(value):
        if isinstance(value, list):
            for child in value:
                font_fields(child)
        elif isinstance(value, dict):
            for key, child in value.items():
                if key in ("datasets", "values"):
                    continue
                if key.lower().endswith("font"):
                    if child not in (family, fonts["family"]):
                        raise ValueError("use the bundled chart font in " + key)
                    value[key] = family
                else:
                    font_fields(child)

    font_fields(definition)
    config = definition.setdefault("config", {})
    config["font"] = family
    for key, fields in (
        ("axis", {"labelFont": family, "titleFont": family, "labelFontWeight": 400, "titleFontWeight": 700}),
        ("legend", {"labelFont": family, "titleFont": family, "labelFontWeight": 400, "titleFontWeight": 700}),
        ("title", {"font": family, "fontWeight": 700}),
    ):
        block = config.setdefault(key, {})
        for field, value in fields.items():
            block.setdefault(field, value)

    snapshots = []
    def data_values(value):
        if isinstance(value, list):
            for child in value:
                data_values(child)
        elif isinstance(value, dict):
            if isinstance(value.get("datasets"), dict):
                snapshots.extend(value["datasets"].values())
            if isinstance(value.get("data"), dict) and "values" in value["data"]:
                snapshots.append(value["data"]["values"])
            for key, child in value.items():
                if key not in ("datasets", "values"):
                    data_values(child)

    data_values(definition)
    with vega_execution(directory) as effective:
        vega = vlc.vegalite_to_scenegraph(definition, vl_version="5.20")
        def weights(node):
            if node.get("marktype") == "text":
                for row in node["items"]:
                    yield font_weight(row.get("fontWeight", 400))
            else:
                for child in node.get("items", ()):
                    yield from weights(child)

        selected = selected_vega_fonts(definition, fonts, set(weights(vega["scenegraph"])))
        svg = final_vega_svg(vlc.vegalite_to_svg(definition, vl_version="5.20", bundle=False))
        if vlc.get_config() != effective or effective["embed_local_fonts"]:
            raise ValueError("Vega configuration changed during metadata/final conversion")
        actual_weights = {font_weight(row["attributes"].get("font-weight", 400)) for row in emitted_text(svg)}
        if actual_weights != {face["weight"] for face in selected}:
            raise ValueError("final SVG text differs from selected font metadata")
        export = {"vl_version": "5.20", "vega_version": vlc.get_vega_version(),
                  "font_set": identity, "font_family": fonts["family"], "selected": selected,
                  "configuration": effective}
    return svg_font_identity(svg, identity).encode(), {"definition": definition, "export": export}, snapshots


def matplotlib_svg(fig, fonts, paths, directory, metadata):
    """Match each emitted text run with the exporter face, metrics and anchor."""
    stream = io.StringIO()
    from matplotlib import font_manager
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
        face = next(f for f in fonts["faces"] if (directory / f["file"]).resolve() == resolved)
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
        fig.savefig(stream, format="svg", metadata=metadata)
    exported = emitted_text(stream.getvalue())
    if len(exported) != len(svg_rows):
        raise ValueError("missing Matplotlib text correspondence")
    for row, actual in zip(svg_rows, exported):
        if row["text"] != actual["text"]:
            raise ValueError("Matplotlib emitted text differs")
        row["emitted"] = actual
    return stream.getvalue(), svg_rows


def exact_matplotlib_text(fig, paths, *, family=None):
    from matplotlib import font_manager
    from matplotlib.text import Text
    examples = []
    for text in fig.findobj(Text):
        props = text.get_fontproperties()
        weight = props.get_weight()
        bold = weight in (700, "bold")
        selected = paths[700 if bold else 400]
        if family is not None and text.get_text():
            if text.get_usetex():
                raise ValueError("Matplotlib chart text cannot run TeX")
            if weight not in (400, 700, "normal", "regular", "bold") or props.get_style() != "normal":
                raise ValueError("Matplotlib text requires a bundled chart font weight/style")
            if props.get_file() is not None and Path(props.get_file()).resolve() != selected.resolve():
                raise ValueError("Matplotlib text requested a different font file")
            if props.get_family() not in (["sans-serif"], [family]):
                raise ValueError("Matplotlib text requested a different font family")
        examples.append((text, props, font_manager.FontProperties(
            fname=str(selected), size=text.get_fontsize(), weight="bold" if bold else "normal")))
    for text, _, exact in examples:
        text.set_fontproperties(exact)
    return [(text, props) for text, props, _ in examples]


def prepare_matplotlib(fig, directory):
    import matplotlib as mpl
    from matplotlib import font_manager
    from matplotlib.backends.backend_agg import FigureCanvasAgg

    if mpl.__version__ != "3.10.7" or sys.platform != "darwin" or platform.machine() != "arm64":
        raise RuntimeError("Matplotlib chart export requires the pinned macOS arm64 environment")
    if not _export_owner.acquire(blocking=False):
        raise ValueError("concurrent chart conversion refused")
    previous, canvas = [], fig.canvas
    try:
        fonts, identity = load_fonts(directory)
        paths = {f["weight"]: directory / f["file"] for f in fonts["faces"]}
        options = {"font.family": fonts["family"], "svg.fonttype": "none",
                   "svg.hashsalt": "bboxy450-stored-artifact-v1", "text.usetex": False}
        for path in paths.values():
            font_manager.fontManager.addfont(str(path))
        with mpl.rc_context(options):
            previous = exact_matplotlib_text(fig, paths, family=fonts["family"])
            FigureCanvasAgg(fig).draw()
            svg, rows = matplotlib_svg(fig, fonts, paths, directory, {"Date": None})
        config = {"backend": "matplotlib", "version": mpl.__version__, "font_set": identity,
                  "configuration": options, "font_files": fonts["faces"], "text": rows,
                  "dpi": fig.dpi, "size_inches": fig.get_size_inches().tolist()}
        snapshots = []
        for axes in fig.axes:
            for line in axes.lines:
                snapshots.append({"x": line.get_xdata(orig=False).tolist(),
                                  "y": line.get_ydata(orig=False).tolist()})
            for collection in axes.collections:
                data = collection.get_array()
                if data is not None:
                    snapshots.append({"values": data.tolist(),
                                      "offsets": collection.get_offsets().tolist()})
            for image in axes.images:
                snapshots.append({"values": image.get_array().tolist(),
                                  "extent": list(image.get_extent())})
            for container in axes.containers:
                if container.__class__.__name__ == "BarContainer" and container.datavalues is not None:
                    snapshots.append({"values": container.datavalues.tolist(),
                                      "orientation": container.orientation})
        return svg_font_identity(svg, identity).encode(), {"export": config}, snapshots
    finally:
        for text, props in previous:
            text.set_fontproperties(props)
        fig.set_canvas(canvas)
        _export_owner.release()


def prepare_plot(plot, directory):
    """Return an SVG, immutable export data and literal source snapshots."""
    directory = Path(directory)
    if isinstance(plot, dict) or callable(getattr(plot, "to_dict", None)):
        return prepare_vega(plot, directory)
    from matplotlib.figure import Figure
    if isinstance(plot, Figure):
        return prepare_matplotlib(plot, directory)
    raise TypeError("display requires a Vega-Lite definition, Altair chart or Matplotlib Figure")
