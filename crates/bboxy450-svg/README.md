# SVG corpus review

Phase A prepares the corpus and independent references. Native interpretation,
rendering and app integration wait for the signed internal review clearance.

Run `make svg-loop CORPUS_ONLY=1`. Make owns a private Python 3.12 environment,
the sole approved vl-convert-python 2.0.0rc7 macOS arm64 wheel and the private
resvg oracle. No system font or global Python installation is used.
`UPDATE_CORPUS=1` refreshes derived data; exact approved SVG, scenegraph and
measurement pins still refuse an unexpected change. Other platforms report
chart export unavailable.

The five RC7 charts regenerate from unchanged Altair definitions and data.
Selected-font metadata returns complete Regular/Bold bytes and actual PostScript
identities matching the sole font manifest. One synchronous execution owns the
converter; a temporary metadata helper restores nonembedding mode in finally.
Every SVG before/after metadata is identical. Empty/unexpected/altered metadata,
concurrent conversion, skipped restore and controlled external URLs are refused.
The HTTP server is independently reached before its request counter is reset.

The five Matplotlib SVGs and inline heatmap PNG remain unchanged. SVG-backend
Text bounds, advances, emitted anchors and actual selected file/face are recorded;
Agg evidence stays separate. Vega's reactive geometry copies its real bounds
before cloneScenegraph omits them. The pinned measurement schema retains those
raw layout boxes without equating them to browser text-cell boxes.

All 507 chart text correspondences and containment checks pass at 1x/2x/8x.
Actual browser faces are audited for each node and expected weight. Shaped
advance fits the exporter reservation plus one total 0.5 physical-pixel allowance;
middle anchoring splits it. Wider/narrower controls reuse the existing
JetBrainsMono test fixture without making it a production font.

`fixtures/` contains twelve handwritten positive scenes, negative SVG inputs,
materializable boundary recipes and the pre-native reference advisory list.
`fixtures/coverage.json` connects every admitted exporter census feature with
isolation evidence. Negative inputs are never executed in Chrome/resvg. Phase A
verifies their custody and boundary construction; native admission remains a
required later check. The original invalid-family Vega export is retained as a
negative, and private target output preserves the retired 1.9 evidence.

The loop records 66 full Chrome captures and 57 compatible vector-layer pairs.
Text-only and image-only scenes use Chrome; disabled resvg text/raster features
supply no reference for those layers. Public path bounds/endpoints include
expanded uses. Raw floating-point differences remain visible; this is not a
native exact-vector geometry receipt. Host camera transforms correct CSS layout
rounding without rewriting the SVG source.

High-contrast RGBA PNG sampling matches bounded premultiplied bilinear sRGB
sampling at every scale; nearest sampling disagrees visibly. Dropping either
gradient or image paint fails its isolation comparison. Frozen advisories name
only independent-reference vector-layer pixel differences. They never waive
geometry, unsupported features, font identity, text or containment.

References require exact source/font/tool/program/camera keys and intact geometry
and PNG files. Only private owned Chrome PIDs are reaped and private profiles
removed. The loop writes `target/svg-loop/index.html`, `report.md`, JSON,
captures and recorded actual image inspection. Run `make svg-review` to inspect
the existing paint/interpolation evidence without another browser run.
Normal `make svg-loop` still fails until native renderer acceptance exists.

The oracle includes the existing bbt-render comparator directly. Both the loop
and bboxybook consume the policy constants beside that comparator.

Authored chart/isolation fixtures:
https://github.com/bboxy450/bboxy450-term.
Unmodified exporters: https://github.com/vega/vl-convert (MIT) and
https://github.com/matplotlib/matplotlib (Matplotlib license).
Source Sans 3 provenance, SIL OFL 1.1, copyright and Reserved Font Name notice
remain in `fonts/manifest.json` and adjacent upstream `LICENSE.md`.
Project attribution: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.
