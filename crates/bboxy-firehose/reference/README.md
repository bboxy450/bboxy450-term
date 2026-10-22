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
