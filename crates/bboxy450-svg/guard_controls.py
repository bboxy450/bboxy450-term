# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Safe, reference-only font refusals. These do not claim native admission proof."""
import json
from pathlib import Path
import subprocess

from corpus import BASE, digest, dump, load_fonts


def run(out):
    root = out / "font-controls"
    root.mkdir(exist_ok=True)
    identity = load_fonts()[1]
    source = BASE / "fixtures/refusals/invalid-family.svg"
    raw = source.read_bytes()
    assert digest(raw) == "1043e461bee4f58c04a78760b44db0e9421c4f904cc5619a3826bdedbf2041f1"
    empty = root / "empty-face.svg"
    # Hidden text has no platform glyphs; it must not evade the actual-face gate.
    empty.write_text(f'<svg xmlns="http://www.w3.org/2000/svg" width="48" height="32" viewBox="0 0 48 32" data-bboxy-font-set="{identity}"><text display="none">Hidden</text></svg>')
    manifest = json.loads((BASE / "corpus/manifest.json").read_text())
    original = next(c for c in manifest["cases"] if c["id"] == "vega-line")
    cache_key = digest(json.dumps({"capture": digest((BASE / "capture.mjs").read_bytes()),
                                  "control": digest((BASE / "guard_controls.py").read_bytes()), "font_set": identity,
                                  "source": digest(raw), "empty": digest(empty.read_bytes()), "views": original["views"],
                                  "browser": json.loads((out / "references/chrome.json").read_text())["browser"]}, sort_keys=True).encode())
    cache = out / "font-controls.json"
    if cache.is_file():
        previous = json.loads(cache.read_text())
        expected_controls = {(name, v["id"]) for name in ("invalid-family", "empty-face") for v in original["views"]}
        if previous.get("cache_key") == cache_key and {(c["control"], c["view"]) for c in previous["controls"]} == expected_controls and all(c["exit"] == 1 and c["custody"]["reaped"] and c["custody"]["profile_removed"] for c in previous["controls"]):
            if all(Path(c["refusal"]).is_file() and digest(Path(c["refusal"]).read_bytes()) == c["refusal_sha256"] for c in previous["controls"]):
                return previous
    controls = []
    for name, svg, box, reason in (("invalid-family", source, original["viewBox"], "unquoted numeric CSS family falls back despite matching identity"),
                                    ("empty-face", empty, [0, 0, 48, 32], "empty actual platform font list")):
        for view in original["views"]:
            directory = root / (name + "-" + view["id"])
            camera = view["camera"] if name == "invalid-family" else [0, 0, 48, 32]
            request = root / (name + "-" + view["id"] + "-request.json")
            dump(request, {"out": str(directory), "font_directory": str(BASE / "fonts"),
                           "cases": [{"id": name, "exporter": "refusal control", "svg": str(svg), "sha256": digest(svg.read_bytes()),
                                      "viewBox": box, "views": [{**view, "camera": camera}]}]})
            with (root / (name + "-" + view["id"] + ".log")).open("w") as log:
                result = subprocess.run(["node", str(BASE / "capture.mjs"), str(request)], stdout=log, stderr=subprocess.STDOUT)
            if result.returncode != 1:
                raise ValueError("font refusal did not fail: " + name)
            refusal = json.loads((directory / "font-refusal.json").read_text())
            custody = json.loads((directory / "owned-process.json").read_text())
            if refusal["source_sha256"] != digest(svg.read_bytes()) or refusal["view"]["id"] != view["id"] or not custody["reaped"] or not custody["profile_removed"]:
                raise ValueError("missing refusal/custody evidence")
            if name == "invalid-family" and (refusal["diagnostics"]["unquoted"] or not any(not f["isCustomFont"] for f in refusal["platformFonts"])):
                raise ValueError("wrong invalid-family refusal cause")
            if name == "empty-face" and refusal["platformFonts"]:
                raise ValueError("wrong empty-face refusal cause")
            controls.append({"control": name, "view": view["id"], "expected_reason": reason, "exit": result.returncode,
                             "refusal": str(directory / "font-refusal.json"), "refusal_sha256": digest((directory / "font-refusal.json").read_bytes()),
                             "source_sha256": digest(svg.read_bytes()), "custody": custody})
    result = {"cache_key": cache_key, "native_refusal": "pending renderer phase", "font_set": identity, "capture_source_sha256": digest((BASE / "capture.mjs").read_bytes()), "controls": controls}
    dump(out / "font-controls.json", result)
    return result
