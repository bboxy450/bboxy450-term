# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Inspect exporter syntax without implementing an SVG interpreter."""
import hashlib
import io
import re
import xml.etree.ElementTree as ET

SVG = "http://www.w3.org/2000/svg"
XLINK = "http://www.w3.org/1999/xlink"
METADATA = {"http://www.w3.org/1999/02/22-rdf-syntax-ns#", "http://purl.org/dc/elements/1.1/", "http://creativecommons.org/ns#"}
ELEMENTS = {"svg", "g", "rect", "circle", "ellipse", "line", "polyline", "polygon", "path", "defs", "use", "clipPath", "text", "tspan", "style", "metadata", "title", "desc", "linearGradient", "stop", "image"}
ATTRS = set("version viewBox width height x y x1 x2 y1 y2 dx dy cx cy r rx ry points d id class transform fill fill-opacity fill-rule stroke stroke-width stroke-opacity stroke-linejoin stroke-linecap stroke-miterlimit stroke-dasharray stroke-dashoffset opacity display visibility clip-path clip-rule clipPathUnits preserveAspectRatio font-family font-size font-weight text-anchor dominant-baseline alignment-baseline rotate style role pointer-events data-bboxy-font-set type offset stop-color stop-opacity".split())
STYLES = set("fill fill-opacity fill-rule stroke stroke-width stroke-opacity stroke-linejoin stroke-linecap stroke-miterlimit stroke-dasharray stroke-dashoffset opacity display visibility clip-path clip-rule font font-family font-size font-weight text-anchor".split())
INERT = {"class", "role", "pointer-events", "data-bboxy-font-set", "version", "id"}


def name(raw):
    if raw.startswith("{"):
        namespace, local = raw[1:].split("}", 1)
        return namespace, local
    return "", raw


def short(value):
    if len(value) <= 256:
        return value
    return {"prefix": value[:100], "bytes": len(value.encode()), "sha256": hashlib.sha256(value.encode()).hexdigest()}


def declarations(value):
    return [tuple(s.strip() for s in declaration.split(":", 1)) for declaration in value.split(";") if ":" in declaration]


def census(directory, cases):
    features = {}
    gaps = {}
    fonts = {}
    counts = []

    def row(category, feature, case, witness, disposition, value=None):
        key = (category, feature)
        item = features.setdefault(key, {"category": category, "feature": feature, "disposition": disposition, "cases": set(), "witnesses": []})
        item["cases"].add(case)
        item["witnesses"].append({"case": case, "source": witness, **({"value": short(value)} if value is not None else {})})
        if disposition == "unsupported: Architect decision required":
            gaps.setdefault(category + ":" + feature, set()).add(case)

    admitted = "admitted grammar: implementation and native witness pending"
    unsupported = "unsupported: Architect decision required"
    inert = "deliberately inert: descriptive/accessibility metadata has no pixels"
    for case in cases:
        key = case["id"]
        raw = (directory / case["svg"]).read_bytes()
        source = raw.decode()
        for _, (prefix, uri) in ET.iterparse(io.BytesIO(raw), events=("start-ns",)):
            row("namespace", prefix + "=" + uri, key, case["svg"], admitted if uri in (SVG, XLINK) else "deliberately inert: namespace declarations do not paint; each element/attribute is checked separately")
        declaration = re.search(r"<!DOCTYPE[\s\S]*?>", source)
        if declaration:
            row("metadata", "PUBLIC doctype", key, case["svg"], "deliberately inert: parser never resolves the external DTD", declaration.group())
        if "<!ENTITY" in source:
            raise ValueError("entity declaration in generated export")
        root = ET.fromstring(raw)
        glyphs, path_commands = 0, 0
        ids = {n.attrib["id"]: name(n.tag)[1] for n in root.iter() if "id" in n.attrib}

        def walk(node, location, inherited, in_metadata=False):
            nonlocal glyphs, path_commands
            ns, tag = name(node.tag)
            metadata = in_metadata or tag == "metadata"
            disposition = inert if metadata or tag in ("title", "desc") else admitted if ns == SVG and tag in ELEMENTS else unsupported
            row("element", node.tag, key, location, disposition)
            style = dict(inherited)
            style.update({k: v for k, v in node.attrib.items() if k.startswith("font-")})
            for attr, value in node.attrib.items():
                ans, aname = name(attr)
                d = inert if metadata or aname in INERT or aname.startswith("aria-") else admitted if (not ans and aname in ATTRS) or (aname == "href" and ans in ("", XLINK)) else unsupported
                inline_png = tag == "image" and value.startswith("data:image/png;base64,")
                if aname == "href" and not value.startswith("#") and not inline_png:
                    d = unsupported
                row("attribute", node.tag + "/@" + attr, key, location + "/@" + aname, d, value)
                if aname == "d":
                    commands = re.findall("[MmZzLlHhVvCcSsQqTtAa]", value)
                    path_commands += len(commands)
                    for command in sorted(set(commands)):
                        row("path-command", command, key, location + "/@d", admitted, value)
                if aname == "transform":
                    for form, args in re.findall(r"([A-Za-z]+)\s*\(([^)]*)\)", value):
                        row("transform", form, key, location + "/@transform", admitted if form in ("matrix", "translate", "scale", "rotate", "skewX", "skewY") else unsupported, args)
                if aname == "style":
                    for prop, val in declarations(value):
                        row("style", prop, key, location + "/@style", admitted if prop in STYLES else unsupported, val)
                        style[prop] = val
                for reference in re.findall(r"url\(\s*['\"]?([^)'\" ]+)", value) + ([value] if aname == "href" else []):
                    target = ids.get(reference[1:]) if reference.startswith("#") else "bounded inline PNG; inspect encoded bytes before reference capture" if inline_png else None
                    row("reference", reference, key, location + "/@" + aname, admitted if target else unsupported, target or "external/unresolved")
            if tag == "style" and node.text:
                for selector, content in re.findall(r"([^{}]+)\{([^{}]*)\}", node.text):
                    row("style-selector", selector.strip(), key, location, admitted if selector.strip() == "*" else unsupported, content)
                    for prop, val in declarations(content):
                        row("style", prop, key, location, admitted if prop in STYLES else unsupported, val)
            if tag in ("text", "tspan"):
                text = node.text or ""
                if text:
                    # Record the emitted shorthand verbatim, as well as inherited longhands.
                    shorthand = style.get("font", "")
                    family = style.get("font-family") or (re.search(r"[\d.]+px\s+(.+)$", shorthand).group(1) if re.search(r"[\d.]+px\s+(.+)$", shorthand) else "UNSPECIFIED")
                    weight = style.get("font-weight") or (shorthand.split()[0] if re.match(r"(?:bold|[4-7]00)\s", shorthand) else "400")
                    font_key = (family, weight)
                    item = fonts.setdefault(font_key, {"family": family, "weight": weight, "cases": set(), "glyphs": set(), "witnesses": []})
                    item["cases"].add(key)
                    item["glyphs"].update(text)
                    item["witnesses"].append({"case": key, "source": location, "text": text, "style": style})
                    glyphs += len(text)
            if metadata and node.text and node.text.strip():
                row("metadata", node.tag + "/text()", key, location, inert, node.text.strip())
            siblings = {}
            for child in node:
                siblings[child.tag] = siblings.get(child.tag, 0) + 1
                walk(child, location + "/" + name(child.tag)[1] + f"[{siblings[child.tag]}]", style, metadata)

        walk(root, case["svg"] + ":/svg[1]", {})
        counts.append({"case": key, "xml_nodes": sum(1 for _ in root.iter()), "path_commands": path_commands, "glyphs": glyphs})
    result = []
    for feature in sorted(features):
        item = features[feature]
        item["cases"] = sorted(item["cases"])
        result.append(item)
    font_rows = []
    for font in sorted(fonts):
        item = fonts[font]
        item["cases"] = sorted(item["cases"])
        item["glyphs"] = [{"char": c, "codepoint": f"U+{ord(c):04X}"} for c in sorted(item["glyphs"])]
        font_rows.append(item)
    return {"phase": "A; syntax census only; no native feature is claimed implemented", "features": result,
            "font_usage": font_rows, "counts": counts,
            "decision_gaps": [{"feature": k, "cases": sorted(v)} for k, v in sorted(gaps.items())]}
