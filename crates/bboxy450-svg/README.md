# Native SVG corpus loop

`make svg-loop` regenerates the pinned exporters, checks source custody and
measurements, and compares retained native painting against the references.
`UPDATE_CORPUS=1` refreshes derived metadata; authored SVG, scenegraph and
measurement pins still refuse an unexpected change. `CORPUS_ONLY=1` is a
reference diagnostic and cannot claim renderer acceptance.

Make owns the private Python 3.12 environment, approved vl-convert-python
2.0.0rc7 macOS arm64 wheel, resvg oracle and SHA-256-pinned Chrome for Testing.
Chrome runs headlessly with a private profile under the selected target directory.
An explicit `CHROME` path is recorded in the results. Frozen comparisons require
the exact version in `fixtures/advisories.json`. No system font or global Python
installation is used. Other platforms report chart export unavailable.
`fonts/manifest.json` is the sole font identity; the app copies its same bytes
and upstream license to `Contents/Resources/chart-fonts/`.

Every declared case runs at 1x, 2x and an 8x camera crop. The shared viewport
function preserves user units and camera identity. The native parser refuses
unsupported paint, foreign namespaces, unverified fonts and bounded-input
violations before painting. `make test-svg` and `make test-svg-admission` check
parser/refusal fixtures and both sides of resource ceilings.

Vector bounds/endpoints must match resvg exactly. Native and resvg reuse the
same tiny-skia coverage engine; independent coverage comes from Chrome on every
non-advisory vector pair. Isolated text and images stay hard against Chrome.
The frozen common-vector advisory keys retain complete renders as inspected
evidence without waiving isolated text, images, geometry or diagnostics. New
composition and fractional-viewport cases cannot become advisory.

The pinned Chrome can rasterize the same SVG with small pixel differences.
For a differing case-order capture or frozen advisory, the shared comparator
allows at most 0.005% of pixels to exceed its eight-level channel tolerance
and normalized mean channel error of at most 0.00001. The reference loop
records the observed metrics and hashes. For an advisory, the comparison
uses the original Chrome PNG in `fixtures/advisory-chrome/`, verified against
its existing digest in `fixtures/advisories.json`. The resvg hash and advisory
keys stay exact, and the changed fraction against resvg may move by at most
0.00005. These Chrome PNGs were rendered from the pinned corpus and isolation
fixtures credited below. The dropped-paint controls must exceed the noise
bound. Native comparison budgets and inspection record digests stay exact.

`make svg-native-check` reuses intact exact-key captures and runs native causal
controls. The loop writes private images, geometry, worst-first triplets and
reports under `target/svg-loop/`. The gate decodes each comparison PNG, checks
its dimensions and the diff's changed-pixel count, then records the inspected
bytes by relative path and SHA-256. Missing or stale inspection prevents
acceptance. Only owned Chrome PIDs and private profiles are removed. The
private loop CLI is a Cargo example, not an installable application binary.

Source/exporter/font attribution and preserved upstream terms are recorded in
[ATTRIBUTION.md](ATTRIBUTION.md), [fonts/manifest.json](fonts/manifest.json) and
[fonts/LICENSE.md](fonts/LICENSE.md).
