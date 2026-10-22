# Local terminal reference

```sh
cargo build --release -p bboxy-firehose
node crates/bboxy-firehose/reference/capture.mjs --strict
```

The script uses the existing bboxybook capture approach: start a separate
headless Chrome with an ephemeral profile, drive CDP, load only local files and
save PNGs. It never attaches to the operator's browser. HTTP/HTTPS asset requests
are blocked. No CDN, npm install or network is needed to run the reference.
`CHROME` can select another installed Chromium binary. `SCALE=1|2` and
`THEME=dark|light` select variants. The default is dark at 1x.

Vendored packages are `@xterm/xterm` **6.0.0** and `@xterm/addon-unicode11`
**0.9.0**, pinned by `package-lock.json`. The Unicode 11 provider avoids the
base terminal's Unicode 6 emoji-width table. Both use the retained MIT license.
`vendor/provenance.json` records npm integrity and individual SHA-256 digests;
the capture script verifies the latter before executing the reference.

To reproduce the vendor files, use `npm ci --ignore-scripts` in this directory
and copy the packages' `lib/xterm.js`, `css/xterm.css`, `lib/addon-unicode11.js`
and licenses from `node_modules` to the corresponding `vendor/` paths. The
lockfile records the package tarball integrity. Never vendor a floating version.

Every input comes from the real release CLI with seed 42, 80×24, Menlo at 13px.
Foreground/background match `bbt-render::Scene::smoke`'s dark/light values.
The reference saves ten final screenshots and two intermediate screenshots
(50% progress and the active alternate screen). Reports record browser version,
input/image digests and the exact viewport comparison against `bbt-core`.
The comparison includes every cell's content, width, colours and six style
flags, plus the cursor. It deliberately does not equate the implementations'
private mode bits or claim a complete OSC 8/underline-style semantic comparison.

`--strict` fails if any viewport differs. Without it the script still reports
every difference and captures all scenes; exit zero then means capture completed.
Pixel/font fidelity remains an image judgement. iTerm2 and Ghostty comparisons
remain **unproven** until those real app captures can be obtained. This xterm.js
reference is the amended T1 gate authorized by Vesper over ICQ on 2026-10-22.

## Headless cells gallery (T2a)

```sh
cargo run -p bbt-render --example terminal_gallery -- crates/bboxy-firehose/reference/gallery/before --baseline
cargo run -p bbt-render --example terminal_gallery -- crates/bboxy-firehose/reference/gallery/native
for scale in 1 2; do
  for theme in dark light; do
    TERMINAL_GRID=1 TERMINAL_OUT=crates/bboxy-firehose/reference/gallery/web SCALE=$scale THEME=$theme node crates/bboxy-firehose/reference/capture.mjs --strict
  done
done
python3 crates/bboxy-firehose/reference/gallery.py
```

The gallery builder needs local Python packages Pillow and NumPy. Open
`gallery/index.html` for all 40 web | native | diff cases. PNGs retain their
original pixels, with horizontal scrolling. `gallery/read/` contains the same
comparisons in labelled strips for image reads; `report.json` records metrics,
image SHA-256s and geometry. The native pass uses the shared Metal device,
CoreText fallback and two retained atlases, with no ASCII substitutions.

`TERMINAL_GRID=1` normalizes xterm's DOM cell metrics to the native rounded
8×15 logical grid. It uses the pinned version's character-size service and
line-height option; both fonts stay Menlo 13, with no image or font rescaling.
The default T1 reference keeps browser-measured metrics. ANSI parsing and the
independent cell comparison are unchanged in either mode.

The legacy renderer only accepts ASCII: seven scenes fail before rendering.
Their before images explicitly use replacement `?` diagnostics and retain the
original error in the manifest and gallery. Only codex, palette and alternate
have actual legacy renders. Before/after percentages use the same padded union
canvas so the legacy's taller rows do not change the denominator; a separate
native-viewport percentage avoids hiding that padding. A pixel differs when
any RGB channel differs by more than 12/255. These are measurements, not a
claim of exact reference parity. The reference DOM renderer has missing
Powerline glyphs, and CoreText coverage/decoration shapes differ from Chrome.
Real iTerm2/Ghostty, live pane input/selection, font resize latency and R5 load
measurements remain outside this headless checkpoint.
