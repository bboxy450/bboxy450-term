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
The reference saves twelve final screenshots and two intermediate screenshots
(50% progress and the active alternate screen). Reports record browser version,
input/image digests and the exact viewport comparison against `bbt-core`.
The comparison includes every cell's content, width, colours and six style
flags, plus cursor position and shape. The two cursor scenes also verify steady (nonblinking) state. It deliberately does not equate the implementations'
private mode bits or claim a complete OSC 8/underline-style semantic comparison.

`--strict` fails if any viewport differs. Without it the script still reports
every difference and captures all scenes; exit zero then means capture completed.
Pixel/font fidelity remains an image judgement. iTerm2 and Ghostty comparisons
remain **unproven** until those real app captures can be obtained. This xterm.js
reference is the amended T1 gate authorized by Vesper over ICQ on 2026-10-22.

## Headless cells gallery (T2a)

```sh
cargo run -p bboxy450-render --example terminal_gallery -- crates/bboxy-firehose/reference/gallery/before --baseline
cargo run -p bboxy450-render --example terminal_gallery -- crates/bboxy-firehose/reference/gallery/native
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

## Live pane audit

From the repository root, build a debug app and the release fixture, then:

    cargo build -p bboxy450-term --locked
    cargo build --release -p bboxy-firehose --locked
    python3 crates/bboxy-firehose/reference/live_capture.py --out target/terminal-live
    python3 crates/bboxy-firehose/reference/live_gallery.py \
      --live target/terminal-live --headless crates/bboxy-firehose/reference/gallery/native \
      --out target/terminal-live-gallery

The driver uses only our app's flags and GPU capture. It opens one zsh at a time
in a private runtime, fits its actual window to an 80×24 pane, disables PTY echo,
and sends the real seed42 scene through the app's input queue. Grid readiness
holds input until both pane geometry and the received Screen report 80×24.
The delayed capture interval starts after readiness. A missing grid fails after
10 seconds with verification-grid-size-unreachable and orderly child shutdown.

The theme, grid-size and grid-hash flags are for verification. Hash metadata
accompanies the actual GPU PNG as .grid.json, with the model's dimensions,
cursor, modes, exact firehose-v1 hash and pane crop. The driver rejects a hash
mismatch. Cleanup signals only recorded private PIDs, records each signal and
real app exit, and removes only its own runtime. External SIGTERM cleanup is
reported as exit -15, never as a graceful exit0.

The app must contain the debug-only memory-custody selector; the driver checks
for it before launching and sets BBOXY_TEST_CUSTODY=memory. A plain release
app is rejected. Optimized measurements require building with
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true; never assume the environment
variable alone selects memory in a production release.

The gallery keeps original pixels, checks captured-image digests and reports
both raw live/headless differences and a separately named metric excluding the
focused cursor cell. Every generated image starts with its read status unproven;
record actual image reads separately. The current machine supplies actual live
1x only. Use the headless scale2 gallery for the 2x gate, and keep live2x and
operator-supplied iTerm2/Ghostty comparisons explicitly unproven.

## Additional T2 layers

`cursor-beam` and `cursor-underline` use DECSCUSR 6 and 4. They show a steady,
focused cursor over the blue lower-left cell. Other scenes retain the unfocused
cursor used by the original gallery. Every scene prefix restores the default
cursor style so seeded playback cannot inherit a previous fixture's shape. The
frozen screen hash schema remains unchanged; cursor shape and steady state are
checked separately against the pinned xterm parser.

The xterm DOM clips double and wavy underlines at its 15px row boundary. Native
keeps both double strokes and the complete wave inside the cell. The overflow
probe and its image read are recorded in `.bboxy/runs/t2-decorations.log`; this
reference limitation is retained in raw pixel scores.

The additional layer gallery is `layers/index.html` (48 web | native | diff
cases). Its baseline uses the accepted pre-layer terminal renderer, with new
cursor fixtures captured before their thickness correction. Rebuild its report
with `python3 crates/bboxy-firehose/reference/gallery.py --root crates/bboxy-firehose/reference/layers`.
The original `gallery/` remains the historical T2a comparison. The new `proofs/`
contains the per-layer before/after crops and the DOM clipping diagnostic.
