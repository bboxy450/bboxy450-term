# bboxy450-render — W3.1

Headless wgpu **30.0.1**, with no window or `Term` dependency. macOS and iOS use
Metal and CoreText. Linux and Android use Vulkan or GLES and render terminal
and chrome text with Swash. `make check-ui-targets` checks all five target
triples from the repository toolchain file.

`render_to_rgba(&scene, width, height)` shares one device and its pipelines per
process. `HeadlessRenderer` also allows explicit ownership. `Scene::smoke` builds
the 80×24 fixture; `write_png` writes unpadded RGBA to a PNG. The initial output
target is RGBA8 sRGB; shader colour conversion and GPU blending operate in linear
space. Text uses a grayscale R8 atlas, doubled as needed without moving old slots.

```sh
cargo test -p bboxy450-render -- --nocapture
ls target/render/smoke/*.png | wc -l
```

Outputs: `target/render/{quad,backgrounds,text}.png` and 48 named variants under
`target/render/smoke/`, including `manifest.tsv` with requested/effective fonts.
The existing CI artifact upload already includes `target/render`.

The matrix covers 11/13/16 px × 1×/2× × dark/light with JetBrains Mono, SF Mono,
Menlo and a **fallback-pending ASCII placeholder**. That final slot explicitly
labels CJK/emoji as pending; these are not Unicode/fallback conformance results.
If a requested font is unavailable, Menlo is used and the manifest names it.
No font is installed into the user's library: registration is process-local.

- JetBrains Mono Regular is the bundled Swash face for terminal and chrome
  text, and also serves the macOS fixture matrix. The unmodified font and
  its **OFL-1.1** notice live together in `tests/fonts`. Upstream revision
  `19371302b95d218af43299bce79ddbddd0bc364d`; SHA-256
  `e6fd0d7e91550b3ed2b735d4312474362c4716edc4fc0577a0f61ed782d5aed1`.
  Source: <https://github.com/JetBrains/JetBrainsMono>. The font retains its own
  license; project-owned Rust contributions use the [Respectful Slop License 1.0-Draft](LICENSE), with prior grants preserved.
- SF Mono uses the existing macOS Terminal.app `SFMono-Terminal.ttf` when present
  (effective name `SFMonoTerminal-Regular`); no Apple font is redistributed.
- Menlo comes from the operating system on macOS and iOS. On Linux and Android
  the bundled face defines glyph coverage, and unsupported characters use its
  missing-glyph outline.

The instance layout is **32 bytes**: u16 cell coordinates, i16 bearing/size,
four u16 atlas pixel coordinates, foreground/background RGBA8 and u32 flags.
The referenced upstream Alacritty struct is actually 36 bytes with f32 UVs;
integer atlas coordinates retain its fields while meeting this brief's size.

Smoke assertions cover exact background bytes, 256-byte-padded GPU readback,
ASCII ink in its expected cells and an asymmetric glyph's upright orientation.
The allocator test preserves old masks/coordinates across atlas growth.

The legacy W3.1 fixture exercises printable ASCII, pixel placement and readback.
It rejects text outside the grid and characters outside its ASCII contract.
Tests require a real Metal adapter on macOS and do not silently pass without it;
only an observed CI failure can justify the brief's environment-variable gate.

Hosted `macos-15` verification passed at renderer commit
`cd5fa5ffd029483561951bf7e88650653843deb7`:
<https://github.com/bboxy450/bboxy450-term/actions/runs/39685613972>.
Its artifact contains 51 PNGs (48 smoke variants), and the actual downloaded text
image is upright. The runner resolved SF Mono to Menlo, so its manifest records
36 distinct image hashes versus 48 locally with SF Mono Terminal available.
No environment-variable skip was added. CI excerpts/artifact metadata are in
`.kb/runs/w3-1-ci.log`.

## Dependency name

The Rust library name remains `bbt_render`. Use the existing import name with a versioned package alias:

```toml
bbt-render = { package = "bboxy450-render", version = "0.1.0" }
```

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
