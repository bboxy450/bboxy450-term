# bbt-render — W3.1

Headless wgpu **30.0.1 / Metal**, with no window or `Term` dependency. The GUI
parity target remains `ux-prototype-v2`; chrome/showcase parity is later work.

`render_to_rgba(&scene, width, height)` shares one device and its pipelines per
process. `HeadlessRenderer` also allows explicit ownership. `Scene::smoke` builds
the 80×24 fixture; `write_png` writes unpadded RGBA to a PNG. The initial output
target is RGBA8 sRGB; shader colour conversion and GPU blending operate in linear
space. Text uses a grayscale R8 atlas, doubled as needed without moving old slots.

```sh
cargo test -p bbt-render -- --nocapture
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

- JetBrains Mono Regular is the unmodified fixture in `tests/fonts`, with its
  **OFL-1.1** notice beside it. Upstream revision
  `19371302b95d218af43299bce79ddbddd0bc364d`; SHA-256
  `e6fd0d7e91550b3ed2b735d4312474362c4716edc4fc0577a0f61ed782d5aed1`.
  Source: <https://github.com/JetBrains/JetBrainsMono>. The font retains its own
  license; the Rust crate remains MIT OR Apache-2.0.
- SF Mono uses the existing macOS Terminal.app `SFMono-Terminal.ttf` when present
  (effective name `SFMonoTerminal-Regular`); no Apple font is redistributed.
- Menlo comes from the operating system.

The instance layout is **32 bytes**: u16 cell coordinates, i16 bearing/size,
four u16 atlas pixel coordinates, foreground/background RGBA8 and u32 flags.
The referenced upstream Alacritty struct is actually 36 bytes with f32 UVs;
integer atlas coordinates retain its fields while meeting this brief's size.

Smoke assertions cover exact background bytes, 256-byte-padded GPU readback,
ASCII ink in its expected cells and an asymmetric glyph's upright orientation.
The allocator test preserves old masks/coordinates across atlas growth.

Not implemented: CJK/emoji, shaping/bidi/ligatures, colour atlas, decorations,
sprites/cursor, damage retention, term-core corpus/goldens/diffs, chrome/fx,
surface presentation, Linux/mobile and hosted-runner Metal verification. ASCII
outside the grid or unsupported text returns an error instead of fake glyphs.
Tests require a real Metal adapter on macOS and do not silently pass without it;
only an observed CI failure can justify the brief's environment-variable gate.
