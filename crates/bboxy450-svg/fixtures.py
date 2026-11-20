# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Handwritten isolation inputs and declarative native refusal/boundary recipes.

Phase A checks fixture custody and independent references. Admission of these
negative inputs belongs to the later native parser, never Chrome or resvg.
"""
import base64
import io
import json
import re
import struct
import xml.etree.ElementTree as ET
import zlib

from corpus import BASE, digest, dump, load_fonts


def generate(out):
    from PIL import Image
    fonts, identity = load_fonts()
    image = Image.new("RGBA", (2, 2))
    image.putdata([(255, 0, 0, 255), (0, 255, 0, 255), (0, 0, 255, 128), (255, 255, 255, 0)])
    stream = io.BytesIO()
    image.save(stream, format="PNG")
    payload = base64.b64encode(stream.getvalue()).decode()
    uri = "data:image/png;base64,\n " + payload[:40] + "\t\r\n" + payload[40:]
    source = {
        "path-commands": ('<path d="M4 4H28v24h-24z M36 4L60 4L60 28L36 28Z M4 36Q16 24 28 36C40 48 4 60 28 60 M40 40A8 8 0 1 1 56 56" fill="#467ac1" stroke="#152a42" stroke-width="2"/>', ["M", "L", "H", "v", "h", "z", "Z", "Q", "C", "A", "subpaths", "close"]),
        "winding": ('<path d="M4 4H60V60H4Z M20 20H44V44H20Z" fill="#463c9a" fill-rule="evenodd"/><path d="M24 24H40V40H24Z" fill="#f0a030"/>', ["fill", "fill-rule", "evenodd", "nonzero"]),
        "stroke": ('<style type="text/css">*{stroke-linejoin:round;stroke-linecap:butt}</style><path d="M4 8L20 28L36 8L52 28" fill="none" stroke="#164a75" stroke-width="4" stroke-linejoin="miter" stroke-miterlimit="4"/><path d="M4 44L60 44" style="fill:none;stroke:#a42a45;stroke-width:6;stroke-dasharray:8,4;stroke-dashoffset:2;stroke-linecap:round;opacity:1"/><line x1="4" y1="58" x2="60" y2="58" stroke="#224433" stroke-width="2" opacity="0.5"/>', ["stroke", "joins", "caps", "miter", "dash", "dash-offset", "style", "universal-selector", "line"]),
        "affine-opacity": ('<g transform="translate(8,8)" opacity="0.6"><g transform="rotate(90,20,20)"><path d="M4 4H36V20H4Z" fill="#ce3640"/><path d="M12 12H44V28H12Z" fill="#267bb0"/></g></g><path d="M0 0H12V8H0Z" transform="matrix(1,0,0.5,1,8,44)" fill="#18854a" fill-opacity="0.7" stroke="#202020" stroke-opacity="0.8"/>', ["translate", "rotate", "matrix", "scale", "nested-group", "group-opacity", "fill-opacity", "stroke-opacity"]),
        "clip-use": ('<defs><path id="tile" d="M0 0H32V24H0Z"/><clipPath id="clip"><rect x="12" y="12" width="40" height="40"/></clipPath></defs><g clip-path="url(#clip)"><use xlink:href="#tile" x="4" y="4" fill="#e85531"/><use href="#tile" x="28" y="32" fill="#305aa3"/><path d="M0 60L60 0" stroke="#202020" stroke-width="6"/></g><path display="none" d="M0 0H64V64H0Z"/>', ["defs", "use", "internal-reference", "clip", "rect", "display-none"]),
        "viewport": ('<svg x="8" y="8" width="48" height="40" viewBox="0 0 24 16" preserveAspectRatio="xMidYMid meet"><path d="M0 0H24V16H0Z" fill="#3284ad"/><path d="M0 0L24 16M24 0L0 16" stroke="#122f44" stroke-width="2"/></svg>', ["viewport", "viewBox", "preserveAspectRatio"]),
        "gradient": ('<defs><linearGradient id="ramp" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#ff0000"/><stop offset="0.5" stop-color="#00ff00"/><stop offset="1" stop-color="#0000ff"/></linearGradient><linearGradient id="vertical" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#000000"/></linearGradient></defs><rect x="4" y="4" width="56" height="24" fill="url(#ramp)"/><g transform="translate(4,32) scale(2,1)"><rect width="28" height="28" fill="url(#vertical)"/></g>', ["linearGradient", "stop", "direction", "interpolation", "objectBoundingBox", "pad", "sRGB", "gradient-transform-by-object"]),
        "png-alpha": (f'<image x="4" y="4" width="56" height="56" href="{uri}"/>', ["inline-PNG", "whitespace-base64", "RGBA", "alpha", "default-interpolation", "preserveAspectRatio"]),
        "png-transform-clip": (f'<defs><clipPath id="clip"><rect x="8" y="8" width="44" height="44"/></clipPath></defs><g clip-path="url(#clip)" transform="translate(64,0) scale(-1,1)"><image x="4" y="4" width="56" height="56" href="{uri}"/><image x="12" y="12" width="16" height="16" href="{uri}"/></g>', ["inline-PNG", "placement", "flip", "clip", "dedup"]),
        "text": (f'<g font-family="\'{fonts["family"]}\'" font-size="10" fill="#152c40"><text x="4" y="14" font-weight="400">Regular AV</text><text x="32" y="30" font-weight="700" text-anchor="middle">Bold AV</text><text x="60" y="44" style="font-family: \'{fonts["family"]}\'; font-size: 10px; font-weight: 400; text-anchor: end">End</text><text transform="translate(16,60) rotate(-90)" font-weight="400">Turn</text><text x="32" y="60" font-weight="400"><tspan dx="0" dy="0">A</tspan><tspan dx="1" dy="-2">V</tspan></text></g>', ["text", "tspan", "start", "middle", "end", "rotation", "weight-400", "weight-700", "kerning", "text-placement"]),
        "outlined-text": ('<path d="M8 52L24 12H32L48 52H40L36 40H20L16 52Z M23 32H33L28 20Z" fill="#224f81" fill-rule="evenodd"/>', ["outlined-text", "path-text"]),
        "metadata": ('<metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Inert credit</dc:title></metadata><title>Accessible title</title><desc>Inert description</desc><g role="img" aria-label="blue square" pointer-events="none" class="case"><rect width="64" height="64" fill="#347aa0"/></g>', ["metadata", "namespaces", "aria", "role", "pointer-events", "class", "title", "desc", "PUBLIC-doctype"]),
    }
    label=f'<text x="4" y="32" font-family="\'{fonts["family"]}\'" font-size="14" font-weight="400">MMMM</text>'
    square='<rect x="24" y="8" width="32" height="40" fill="#e05030"/>'
    image_markup=f'<image x="0" y="0" width="64" height="64" href="{uri}"/>'
    source.update({
        "composition-text-under-shape":(label+square,["text","rect","paint-order"]),
        "composition-shape-over-text":('<rect x="0" y="0" width="64" height="64" fill="#cceeff"/>'+label+square,["text","rect","paint-order","occlusion"]),
        "composition-image-under-text":('<rect width="64" height="64" fill="white"/>'+image_markup+label,["text","inline-PNG","paint-order"]),
        "composition-clip-spanning-layers":('<defs><clipPath id="composition-clip"><rect x="8" y="8" width="48" height="40"/></clipPath></defs><g clip-path="url(#composition-clip)">'+image_markup+label+'<rect x="0" y="36" width="64" height="24" fill="#2050c0"/></g>',["text","inline-PNG","rect","clip","paint-order"]),
        "fractional-viewport":('<rect width="64" height="64" fill="#ddeeff"/>'+label,["text","rect","fractional-viewport"]),
    })
    cases = []
    directory = out / "isolation"
    directory.mkdir(parents=True, exist_ok=True)
    for name, (body, features) in source.items():
        height=63.25 if name=="fractional-viewport" else 64
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="{height}" viewBox="0 0 64 {height}" data-bboxy-font-set="{identity}">{body}</svg>\n'
        if name == "metadata":
            svg = '<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">\n' + svg
        file = directory / (name + ".svg")
        file.write_text(svg)
        cases.append({"id": "isolation-"+name, "svg": "isolation/"+file.name, "sha256": digest(file.read_bytes()),
                      "features": features, "text": name == "text" or name.startswith("composition-") or name=="fractional-viewport", "image": name.startswith("png-") or name in ("composition-image-under-text","composition-clip-spanning-layers"),
                      "composition": name.startswith("composition-"), "viewBox": [0, 0, 64, height], "views": [{"id": str(s)+"x", "scale": s,
                      "camera": ([16,40,48,24] if name=="fractional-viewport" else [16,16,48,32]) if s==8 else [0,0,64,64]} for s in (1, 2, 8)]})
    negatives = {
        "script": '<script>alert(1)</script>', "event": '<rect onclick="alert(1)"/>',
        "foreign-object": '<foreignObject/>', "external-href": '<use href="https://invalid.example/x.svg#p"/>',
        "file-href": '<image href="file:///etc/passwd"/>', "css-import": '<style>@import "https://invalid.example/x.css";</style>',
        "external-paint": '<path d="M0 0L1 1" fill="url(https://invalid.example/paint)"/>',
        "entity": '<!ENTITY secret SYSTEM "file:///etc/passwd">', "cycle": '<defs><g id="a"><use href="#a"/></g></defs><use href="#a"/>',
        "unknown-paint": '<path d="M0 0L1 1" unknown-paint="red"/>',
        "radial-gradient": '<defs><radialGradient id="p"/></defs><rect width="4" height="4" fill="url(#p)"/>',
        "png-malformed-padding": '<image href="data:image/png;base64,AA=A"/>',
        "png-invalid-byte": '<image href="data:image/png;base64,AAAA!"/>',
        "png-signature": '<image href="data:image/png;base64,YWJjZA=="/>',
        "png-foreign-MIME": '<image href="data:image/jpeg;base64,AAAA"/>',
        "png-svg": '<image href="data:image/svg+xml;base64,AAAA"/>',
        "png-option": f'<image image-rendering="pixelated" href="{uri}"/>',
        "nonfinite": '<g transform="scale(NaN)"><rect width="1" height="1"/></g>',
        "missing-font-identity": f'<text font-family="\'{fonts["family"]}\'">A</text>',
        "changed-font-identity": f'<text font-family="\'{fonts["family"]}\'">A</text>',
        "unsupported-weight": f'<text font-family="\'{fonts["family"]}\'" font-weight="500">A</text>',
        "invalid-family": '<text font-family="Source Sans 3" font-weight="400">A</text>',
        "font-css": '<style>@font-face{font-family:x;src:url(data:font/ttf;base64,AAAA)}</style>',
        "unsupported-glyph": f'<text font-family="\'{fonts["family"]}\'" font-weight="400">\U0010ffff</text>',
        "filter": '<filter id="f"/>', "mask": '<mask id="m"/>', "animation": '<animate attributeName="x"/>',
    }
    # Keep every negative well formed apart from the named fault. In particular,
    # a missing image size or unused/malformed gradient must not mask its guard.
    for name, body in list(negatives.items()):
        if "<image " in body:
            negatives[name] = body.replace("<image ", '<image x="4" y="4" width="56" height="56" ', 1)
    stops = '<stop offset="0" stop-color="#ff0000"/><stop offset="1" stop-color="#0000ff"/>'
    bad_gradients = {"gradient-href": 'href="#q"', "gradient-units": 'gradientUnits="userSpaceOnUse"',
                     "gradient-transform": 'gradientTransform="rotate(10)"', "gradient-spread": 'spreadMethod="reflect"',
                     "gradient-interpolation": 'color-interpolation="linearRGB"'}
    for name, attributes in bad_gradients.items():
        negatives[name] = '<defs><linearGradient id="q">'+stops+'</linearGradient><linearGradient id="p" '+attributes+'>'+stops+'</linearGradient></defs><rect width="64" height="64" fill="url(#p)"/>'
    for name, offsets in {"gradient-order": (1, 0), "gradient-offset": (-0.1, 1.1), "gradient-stops": (0,)}.items():
        bad_stops = ''.join(f'<stop offset="{offset}" stop-color="#224466"/>' for offset in offsets)
        negatives[name] = '<defs><linearGradient id="p">'+bad_stops+'</linearGradient></defs><rect width="64" height="64" fill="url(#p)"/>'
    negative_rows = []
    refusal_dir = out / "refusals"
    refusal_dir.mkdir(exist_ok=True)
    for name, body in negatives.items():
        marker = "" if name == "missing-font-identity" else f' data-bboxy-font-set="{"sha256:bad" if name == "changed-font-identity" else identity}"'
        svg = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"{marker}>{body}</svg>\n'
        if name == "entity":
            svg = '<!DOCTYPE svg [<!ENTITY secret SYSTEM "file:///etc/passwd">]><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><text>&secret;</text></svg>'
        file = refusal_dir / (name+"-isolation.svg")
        file.write_text(svg)
        negative_rows.append({"id": name, "svg": "refusals/"+file.name, "sha256": digest(file.read_bytes()),
                              "expected": "unsupported" if name in {"unknown-paint", "radial-gradient", "filter", "mask", "animation", "unsupported-glyph"} else "refused",
                              "native_status": "pending native parser; never executed by references"})
    for name, raw in {"invalid-utf8": b'<svg xmlns="http://www.w3.org/2000/svg">\xff</svg>',
                      "malformed-xml": b'<svg xmlns="http://www.w3.org/2000/svg"><g></svg>'}.items():
        file = refusal_dir / (name+"-isolation.svg")
        file.write_bytes(raw)
        negative_rows.append({"id": name, "svg": "refusals/"+file.name, "sha256": digest(raw), "expected": "refused",
                              "native_status": "pending native parser; never executed by references"})
    # Compact recipes materialize only in a private native test directory. Both
    # sides of every numeric boundary are explicit, without megabytes of goldens.
    limits = {"svg_bytes": 16777216, "xml_nodes": 50000, "depth": 64, "path_commands": 8192,
              "scene_commands": 262144, "glyphs": 65536, "png_bytes": 16777216,
              "png_side": 2047, "png_distinct": 16, "png_pixels": 4000000,
              "gradient_stops": 256, "capture_side": 4096, "capture_pixels": 16777216}
    boundaries = [{"limit": k, "maximum": v, "accepted": v, "rejected": v+1,
                   "native_status": "pending native admission witness"} for k, v in limits.items()]
    boundaries += [{"limit": k, "accepted": 1, "rejected": 0, "native_status": "pending native admission witness"}
                   for k in ("viewport_width", "viewport_height", "png_width", "png_height", "capture_scale")]
    result = {"phase": "A; fixture data, not native admission proof", "cases": cases, "negatives": negative_rows,
              "boundaries": boundaries, "boundary_materializer": "../fixtures.py:boundary_input",
              "png": {"sha256": digest(stream.getvalue()), "size": [2, 2], "rgba": list(image.getdata())}}
    dump(out / "manifest.json", result)
    return result


def boundary_input(name, value):
    """Materialize one boundary recipe for the native test, never a reference.

    PNG byte/side/pixel tests call the shared decoder directly, so the enclosing
    SVG byte ceiling cannot accidentally satisfy a different guard first.
    """
    from PIL import Image
    fonts, identity = load_fonts()
    def svg(body, attributes='viewBox="0 0 64 64"'):
        return f'<svg xmlns="http://www.w3.org/2000/svg" {attributes} data-bboxy-font-set="{identity}">{body}</svg>'.encode()
    def png(width, height, color=(10, 20, 30, 255)):
        stream = io.BytesIO()
        Image.new("RGBA", (width, height), color).save(stream, format="PNG")
        return stream.getvalue()
    if name == "svg_bytes":
        raw = svg("<!---->")
        return {"api": "prepare", "input": raw.replace(b"<!---->", b"<!--"+b"x"*(value-len(raw))+b"-->")}
    if name == "xml_nodes":
        return {"api": "prepare", "input": svg('<rect width="1" height="1"/>'*(value-1))}
    if name == "depth":
        return {"api": "prepare", "input": svg("<g>"*(value-2)+'<rect width="1" height="1"/>'+"</g>"*(value-2))}
    if name == "path_commands":
        return {"api": "prepare", "input": svg('<path d="M0 0'+'L1 1'*(value-1)+'"/>')}
    if name == "scene_commands":
        paths = []
        remaining = value
        while remaining:
            count = min(remaining, 8192)
            paths.append('<path d="M0 0'+'L1 1'*(count-1)+'"/>')
            remaining -= count
        return {"api": "prepare", "input": svg("".join(paths))}
    if name == "glyphs":
        return {"api": "prepare", "input": svg(f'<text font-family="\'{fonts["family"]}\'" font-weight="400">'+"A"*value+"</text>")}
    if name == "gradient_stops":
        stops = ''.join(f'<stop offset="{i/(value-1)}" stop-color="#224466"/>' for i in range(value))
        return {"api": "prepare", "input": svg('<defs><linearGradient id="p">'+stops+'</linearGradient></defs><rect width="64" height="64" fill="url(#p)"/>')}
    if name.startswith("viewport_"):
        w, h = (value, 1) if name.endswith("width") else (1, value)
        return {"api": "prepare", "input": svg('<rect width="1" height="1"/>', f'viewBox="0 0 {w} {h}"')}
    if name == "png_bytes":
        raw = png(1, 1)
        data = b"bboxy\0"+b"x"*(value-len(raw)-18)
        chunk = b"tEXt"+data
        raw = raw[:-12]+struct.pack(">I", len(data))+chunk+struct.pack(">I", zlib.crc32(chunk))+raw[-12:]
        assert len(raw) == value
        return {"api": "decode_png", "inputs": [raw]}
    if name in ("png_side", "png_width", "png_height"):
        w, h = (1, value) if name == "png_height" else (value, 1)
        if value == 0:
            # A valid PNG container whose IHDR declares the rejected zero side.
            raw = png(1, 1)
            data = struct.pack(">II", w, h)+raw[24:29]
            chunk = b"IHDR"+data
            raw = raw[:12]+chunk+struct.pack(">I", zlib.crc32(chunk))+raw[33:]
        else:
            raw = png(w, h)
        return {"api": "decode_png", "inputs": [raw]}
    if name == "png_distinct":
        return {"api": "decode_png", "inputs": [png(1, 1, (i, 0, 0, 255)) for i in range(value)]}
    if name == "png_pixels":
        inputs = [png(2000, 2000)]
        if value > 4000000:
            inputs.append(png(value-4000000, 1))
        return {"api": "decode_png", "inputs": inputs}
    if name in ("capture_side", "capture_pixels", "capture_scale"):
        if name == "capture_pixels":
            camera, scale = [0, 0, 4096, 4096 if value == 16777216 else 4097], 1
        elif name == "capture_scale":
            camera, scale = [0, 0, 1, 1], value
        else:
            camera, scale = [0, 0, value, 1], 1
        return {"api": "render", "camera": camera, "scale": scale, "input": svg('<rect width="1" height="1"/>')}
    raise ValueError("unknown boundary recipe: "+name)


def check_boundaries(out, manifest):
    key = digest((BASE / "fixtures.py").read_bytes())
    path = out / "boundary-inputs.json"
    if path.is_file():
        previous = json.loads(path.read_text())
        if previous.get("source_sha256") == key:
            return previous
    rows = []
    for boundary in manifest["boundaries"]:
        name = boundary["limit"]
        for disposition in ("accepted", "rejected"):
            value = boundary[disposition]
            request = boundary_input(name, value)
            if request["api"] == "decode_png":
                inputs = request["inputs"]
                dimensions = [struct.unpack(">II", raw[16:24]) for raw in inputs]
                measured = {"png_bytes": max(map(len, inputs)), "png_side": max(max(d) for d in dimensions),
                            "png_width": dimensions[0][0], "png_height": dimensions[0][1],
                            "png_distinct": len({digest(raw) for raw in inputs}),
                            "png_pixels": sum(w*h for w, h in dimensions)}[name]
            else:
                inputs = [request["input"]]
                root = ET.fromstring(inputs[0])
                def depth(node):
                    return 1+max((depth(n) for n in node), default=0)
                commands = [len(re.findall("[MmZzLlHhVvCcSsQqTtAa]", n.attrib["d"])) for n in root.iter() if "d" in n.attrib]
                metrics = {"svg_bytes": len(inputs[0]), "xml_nodes": len(list(root.iter())), "depth": depth(root),
                           "path_commands": max(commands, default=0), "scene_commands": sum(commands),
                           "glyphs": sum(len(n.text or "") for n in root.iter() if n.tag.endswith("}text")),
                           "gradient_stops": sum(n.tag.endswith("}stop") for n in root.iter())}
                if name.startswith("viewport_"):
                    metrics[name] = float(root.attrib["viewBox"].split()[2 if name.endswith("width") else 3])
                if request["api"] == "render":
                    _, _, w, h = request["camera"]
                    scale = request["scale"]
                    metrics.update(capture_side=max(w, h)*scale, capture_pixels=w*h*scale*scale, capture_scale=scale)
                measured = metrics[name]
            if measured != value and not (name == "capture_pixels" and disposition == "rejected" and measured >= value):
                raise ValueError("boundary recipe does not reach its named bound: "+str((name, value, measured)))
            rows.append({"limit": name, "value": value, "measured": measured, "intended_native_result": disposition,
                         "api": request["api"], "input_sha256": [digest(raw) for raw in inputs],
                         "native_admission": "not run in Phase A"})
    result = {"source_sha256": key, "recipes": rows, "native_admission": "pending renderer phase"}
    dump(path, result)
    return result


def coverage(exported, isolation):
    def key(row):
        category, feature = row["category"], row["feature"]
        if category == "attribute":
            feature = feature.split("/@")[-1]
        if category == "reference":
            feature = "inline-PNG" if feature.startswith("data:image/png;base64,") else "internal-reference"
        return category, feature
    witnesses = {}
    for row in isolation["features"]:
        witnesses.setdefault(key(row), set()).update(row["cases"])
    rows, gaps = [], []
    for row in exported["features"]:
        inert = row["disposition"].startswith("deliberately inert")
        cases = sorted(witnesses.get(key(row), []))
        if not cases and not inert:
            gaps.append(key(row))
        rows.append({"category": row["category"], "feature": row["feature"], "isolation_cases": cases,
                     "disposition": "explicitly inert" if inert else "isolation/reference present; native witness pending"})
    if gaps:
        raise ValueError("missing rendering-feature isolations: "+str(gaps))
    return {"features": rows, "missing": 0, "native_implementation": "not claimed by Phase A"}
