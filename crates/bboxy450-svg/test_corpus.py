# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Corpus admission checks only: these are not native renderer tests."""
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import shutil
import tempfile
import threading
import urllib.request
import unittest
from unittest.mock import patch

import corpus
from census import census
from measurements import advance_containment
from reference_browser import capture_browser, frozen_version
from references import policy
from review import comparison_images, inspect_images, require_inspection


class CorpusChecks(unittest.TestCase):
    def setUp(self):
        self.temp_root = corpus.BASE.parents[1] / "target/svg-loop/test-temp"
        self.temp_root.mkdir(parents=True, exist_ok=True)

    def test_image_inspection_refuses_missing_and_changed_images(self):
        from PIL import Image

        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            root = Path(tmp)
            reference, native, diff = (root / name for name in ("reference.png", "native.png", "diff.png"))
            for path in (reference, native, diff):
                Image.new("RGBA", (2, 1), "black").save(path)
            row = {"diff": diff, "width": 2, "height": 1, "changed_fraction": 0.0}
            files = comparison_images(root, [(reference, native, row)])
            receipt = "inspected.json"

            with self.assertRaisesRegex(ValueError, "missing or invalid image inspection"):
                require_inspection(root, receipt, files)
            inspect_images(root, receipt, files)
            require_inspection(root, receipt, files)

            Image.new("RGBA", (2, 1), "red").save(native)
            with self.assertRaisesRegex(ValueError, "stale image inspection: native[.]png"):
                require_inspection(root, receipt, files)
            native.unlink()
            with self.assertRaisesRegex(ValueError, "missing or stale image inspection: native[.]png"):
                require_inspection(root, receipt, files)

    def test_image_inspection_rejects_a_diff_that_disagrees_with_comparison(self):
        from PIL import Image

        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            root = Path(tmp)
            diff = root / "diff.png"
            Image.new("RGBA", (2, 1), "red").save(diff)
            with self.assertRaisesRegex(ValueError, "inspection diff does not match"):
                inspect_images(root, "inspected.json", {diff.name: ((2, 1), 0.0)})
            self.assertFalse((root / "inspected.json").exists())

    def test_wrong_browser_is_refused_before_capture(self):
        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            root = Path(tmp)
            browser = root / "explicit browser"
            browser.write_text('#!/bin/sh\nprintf "Google Chrome 1.2.3.4\\n"\n')
            browser.chmod(0o700)
            with patch.dict("os.environ", CHROME=str(browser)), self.assertRaisesRegex(
                ValueError, f"frozen {frozen_version()}, observed Chrome/1[.]2[.]3[.]4"
            ):
                capture_browser(root)

    def test_font_byte_substitution_is_refused(self):
        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            root = Path(tmp)
            shutil.copytree(corpus.BASE / "fonts", root / "fonts")
            manifest = json.loads((root / "fonts/manifest.json").read_text())
            path = root / "fonts" / manifest["faces"][0]["file"]
            data = bytearray(path.read_bytes())
            data[-1] ^= 1
            path.write_bytes(data)
            with patch.object(corpus, "BASE", root), self.assertRaisesRegex(ValueError, "digest mismatch"):
                corpus.load_fonts()

    def test_urls_and_images_are_refused_before_any_request(self):
        requests = []

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append(self.path)
                self.send_response(200)
                self.end_headers()
                self.wfile.write(b"[]")

            def log_message(self, *_):
                pass

        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            url = f"http://127.0.0.1:{server.server_port}/forbidden"
            with urllib.request.urlopen(url, timeout=3) as response:
                self.assertEqual(response.status, 200)
            self.assertEqual(requests, ["/forbidden"])
            requests.clear()
            for spec in ({"data": {"url": url}}, {"mark": "image"}, {"mark": {"type": "image"}, "encoding": {"url": {"value": url}}}):
                with self.assertRaisesRegex(ValueError, "external resource"):
                    corpus.external_resources(spec)
            # Check the pinned converter's own HTTP guard independently of our helper.
            import vl_convert as vlc
            with corpus.vega_execution(), self.assertRaisesRegex(PermissionError, "VLC_ACCESS_DENIED"):
                vlc.vegalite_to_svg({"data": {"url": url}, "mark": "bar", "encoding": {"x": {"field": "x", "type": "quantitative"}}}, vl_version="5.20", bundle=False)
            self.assertEqual(requests, [])
            corpus.dump(self.temp_root.parent / "network-control.json", {"reachable_requests": 1, "converter_requests": 0,
                         "effective_configuration": vlc.get_config(), "denied": "VLC_ACCESS_DENIED"})
        finally:
            server.shutdown()
            thread.join(timeout=5)
            server.server_close()
            self.assertFalse(thread.is_alive())

    def test_matplotlib_live_export_keeps_font_identity_and_data(self):
        from matplotlib.figure import Figure
        from chart_export import FONT_DIR, prepare_plot
        figure = Figure(figsize=(4, 2.5), dpi=72)
        plot = figure.subplots()
        plot.plot([1, 2, 3], [4, 2, 6])
        plot.set_title("Measured")
        plot.set_xlabel("Time")
        old_canvas = figure.canvas
        picture, definition, data = prepare_plot(figure, FONT_DIR)
        marker = definition["export"]["font_set"].encode()
        self.assertIn(b'data-bboxy-font-set="' + marker + b'"', picture)
        self.assertTrue(definition["export"]["text"])
        self.assertEqual(data, [{"x": [1., 2., 3.], "y": [4., 2., 6.]}])
        self.assertIs(figure.canvas, old_canvas)
        plot.set_title("Foreign", fontfamily="DejaVu Sans")
        with self.assertRaisesRegex(ValueError, "font family"):
            prepare_plot(figure, FONT_DIR)

    def test_metadata_restore_identity_and_single_owner(self):
        import copy
        import vl_convert as vlc
        fonts, _ = corpus.load_fonts()
        spec = json.loads((corpus.BASE / "corpus/vega-line.json").read_text())["definition"]
        with corpus.vega_execution():
            # A competing execution refuses instead of changing the owner's globals.
            errors = []
            def compete():
                try:
                    with corpus.vega_execution():
                        pass
                except ValueError as error:
                    errors.append(str(error))
            thread = threading.Thread(target=compete)
            thread.start()
            thread.join(3)
            self.assertFalse(thread.is_alive())
            self.assertEqual(errors, ["concurrent Vega conversion refused"])
            with patch.object(vlc, "vegalite_fonts", side_effect=ValueError("metadata failed")), self.assertRaisesRegex(ValueError, "metadata failed"):
                corpus.selected_vega_fonts(spec, fonts, {400, 700})
            self.assertFalse(vlc.get_config()["embed_local_fonts"])
            original = vlc.vegalite_fonts
            payloads = []
            def record(*args, **kwargs):
                payload = original(*args, **kwargs)
                payloads.append(payload)
                return payload
            with patch.object(vlc, "vegalite_fonts", side_effect=record):
                self.assertEqual(len(corpus.selected_vega_fonts(spec, fonts, {400, 700})), 2)
            bad_weight = copy.deepcopy(payloads[0])
            bad_weight[0]["variants"][0]["weight"] = "500"
            bad_bytes = copy.deepcopy(payloads[0])
            css = bad_bytes[0]["variants"][0]["font_face"]
            bad_bytes[0]["variants"][0]["font_face"] = css.replace("base64,", "base64,AAAA", 1)
            for payload in ([], payloads[0][:-1], bad_weight, bad_bytes):
                with patch.object(vlc, "vegalite_fonts", return_value=payload), self.assertRaises(Exception):
                    corpus.selected_vega_fonts(spec, fonts, {400, 700})
                self.assertFalse(vlc.get_config()["embed_local_fonts"])
            # Deliberately skip the helper's restore: final mode must refuse it.
            vlc.configure(embed_local_fonts=True)
            try:
                bad_svg = vlc.vegalite_to_svg(spec, vl_version="5.20", bundle=False)
                with self.assertRaisesRegex(ValueError, "embedded font"):
                    corpus.final_vega_svg(bad_svg)
            finally:
                vlc.configure(embed_local_fonts=False)
            corpus.dump(self.temp_root.parent / "metadata-controls.json", {"concurrent": errors,
                         "exception_restore": True, "empty_missing_unexpected_altered_refused": True,
                         "skipped_restore_refused": True, "final_configuration": vlc.get_config()})

    def test_substituted_exporter_backend_face_is_refused(self):
        import matplotlib as mpl
        from matplotlib import font_manager
        from matplotlib.textpath import TextToPath
        foreign = Path(mpl.get_data_path()) / "fonts/ttf/DejaVuSans.ttf"
        actual = font_manager.get_font(str(foreign))
        manifest, identity = corpus.load_fonts()
        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            with patch.object(TextToPath, "_get_font", return_value=actual), self.assertRaisesRegex(ValueError, "exporter resolved foreign face"):
                corpus.matplotlib_exports(Path(tmp), manifest, identity)

    def test_narrower_fixture_fits_but_actual_face_is_refused(self):
        from matplotlib import font_manager
        from matplotlib.textpath import TextToPath
        manifest, identity = corpus.load_fonts()
        source = corpus.BASE / "fonts" / manifest["faces"][0]["file"]
        foreign = corpus.BASE.parent / "bbt-render/tests/fonts/JetBrainsMono-Regular.ttf"
        measure = TextToPath()
        props = lambda file: font_manager.FontProperties(fname=str(file), size=10)
        reserved = measure.get_text_width_height_descent("WWWWWW", props(source), False)[0]
        narrow = measure.get_text_width_height_descent("WWWWWW", props(foreign), False)[0]
        self.assertLess(narrow, reserved)
        for scale in (1, 2, 8):
            self.assertTrue(advance_containment(reserved, narrow*scale, "middle", scale, policy()["text_geometry_px"])["passed"])
        actual = font_manager.get_font(str(foreign))
        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            with patch.object(TextToPath, "_get_font", return_value=actual), self.assertRaisesRegex(ValueError, "exporter resolved foreign face"):
                corpus.matplotlib_exports(Path(tmp), manifest, identity)

    def test_wider_fixture_exceeds_total_advance_allowance(self):
        from matplotlib import font_manager
        from matplotlib.textpath import TextToPath
        manifest, _ = corpus.load_fonts()
        source = corpus.BASE / "fonts" / manifest["faces"][0]["file"]
        foreign = corpus.BASE.parent / "bbt-render/tests/fonts/JetBrainsMono-Regular.ttf"
        measure = TextToPath()
        props = lambda file: font_manager.FontProperties(fname=str(file), size=10)
        reserved = measure.get_text_width_height_descent("iiiiii", props(source), False)[0]
        wide = measure.get_text_width_height_descent("iiiiii", props(foreign), False)[0]
        self.assertGreater(wide, reserved)
        tolerance = policy()["text_geometry_px"]
        for scale in (1, 2, 8):
            for anchor in ("start", "middle", "end"):
                self.assertFalse(advance_containment(reserved, wide*scale, anchor, scale, tolerance)["passed"])
        # A middle anchor splits one total allowance; it cannot get two copies.
        self.assertFalse(advance_containment(10, 10 + tolerance*1.5, "middle", 1, tolerance)["passed"])
        self.assertEqual(advance_containment(10, 10 + tolerance, "middle", 1, tolerance)["allowed_ends_px"], [tolerance/2, tolerance/2])

    def test_unknown_paint_is_a_decision_gap_and_metadata_is_inert(self):
        with tempfile.TemporaryDirectory(dir=self.temp_root) as tmp:
            root = Path(tmp)
            svg = '<svg xmlns="http://www.w3.org/2000/svg"><metadata><foreign xmlns="urn:descriptive">origin</foreign></metadata><path d="M0,0L1,1" unknown-paint="red"/></svg>'
            (root / "case.svg").write_text(svg)
            report = census(root, [{"id": "case", "svg": "case.svg"}])
            self.assertEqual(len(report["decision_gaps"]), 1)
            foreign = next(row for row in report["features"] if row["category"] == "element" and row["feature"] == "{urn:descriptive}foreign")
            self.assertTrue(foreign["disposition"].startswith("deliberately inert"))
            self.assertTrue(any("unknown-paint" in gap["feature"] for gap in report["decision_gaps"]))

    def test_exact_font_set_covers_every_exported_glyph(self):
        from fontTools.ttLib import TTFont
        manifest, _ = corpus.load_fonts()
        source = corpus.BASE / "corpus/census.json"
        if not source.is_file():
            self.skipTest("initial corpus generation has not run")
        report = json.loads(source.read_text())
        for usage in report["font_usage"]:
            self.assertEqual(usage["family"].strip("'\""), manifest["family"])
            face = next(f for f in manifest["faces"] if str(f["weight"]) == usage["weight"])
            with TTFont(corpus.BASE / "fonts" / face["file"]) as font:
                cmap = font.getBestCmap()
                self.assertTrue(all(ord(g["char"]) in cmap for g in usage["glyphs"]))


if __name__ == "__main__":
    unittest.main()
