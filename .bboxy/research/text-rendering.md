---
id: "52PGE8HG77JR1CX0MQAKXVJMXT"
type: "research"
number: 82
title: "Text rendering — how ghostty does it, and what we build on wgpu"
created: "2026-10-21T19:27:17Z"
updated: "2026-10-24T00:10:38Z"
---
# Text rendering — how ghostty does it, and what we build on wgpu

Status: RESEARCH, 2026-10-21. Source: ghostty `521cf2c5d30bd48028ca61546c64e7913764073d` (main, 2026-10-21 "Update VOUCHED list (#14798)"), shallow clone at `/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/3544d975-706b-4ce4-ba8c-d9734c0db856/scratchpad/clones/ghostty`. Every `file:line` below is relative to that clone's `src/`. Crate versions and licenses from the crates.io API on 2026-10-21. Zed and wezterm notes are from their `main` branches via raw GitHub, read through a summarizer, not line-verified — marked INFERRED where the summarizer could not quote.

Links: [[vision]] · [[terminal-core]] · [[gpu-terminals-rust]]

## The one-paragraph answer

Ghostty's text looks the way it does because of five decisions, not one: (1) on macOS it lets Apple's rasterizer draw every glyph into an alpha-only bitmap with subpixel positioning on and quantization off, at the glyph's exact fractional bearing inside the cell, so each glyph is rasterized once and then placed on integer pixels; (2) the cell grid is integer pixels computed by rounding the font's advance and line height, with the baseline re-centered so rounding error splits evenly; (3) blending runs on a Display P3 surface, by default in gamma space the way Terminal.app does it, optionally in linear space with a luminance-remapped alpha so dark text does not go thin; (4) underlines, cursors, box drawing, braille, block elements and Powerline are never font glyphs, they are drawn procedurally per cell size by a small 2D rasterizer into the same atlas; (5) the GPU does almost nothing clever: one full-screen fragment pass reads a per-cell background colour buffer, one instanced quad pass samples two atlases with nearest filtering, and the CPU only rebuilds rows that changed.

## Backend selection

- `font/backend.zig:60` — default backend is `.coretext` on Darwin (CoreText for discovery, rasterization and shaping; no HarfBuzz linked) and `.fontconfig_freetype` elsewhere. `backend.zig:21-26` — `coretext_harfbuzz` (CoreText discovery+raster, HarfBuzz shaping) and `coretext_freetype` are supported alternates; the comment says `coretext_freetype` exists for "people who want to self-compile Ghostty and prefer the freetype aesthetic".
- `font/shape.zig:20-29` — shaper is HarfBuzz for every FreeType backend and for `coretext_harfbuzz`; the CoreText shaper only for `.coretext`.

## 1. Font discovery and fallback

### Collection build order (what is loaded, in priority order)

`font/SharedGridSet.zig:204-237` — user `font-family` / `-bold` / `-italic` / `-bold-italic` faces are added as deferred faces, `fallback = false`.
`SharedGridSet.zig:258` — `completeStyles` synthesizes missing bold/italic/bold-italic from regular (see §3).
`SharedGridSet.zig:261-318` — embedded JetBrains Mono variable (regular, `wght=700` for bold, italic file, italic+`wght=700`) added as `fallback = true` with `size_adjustment = default_fallback_adjustment`.
`SharedGridSet.zig:320-334` — embedded Symbols Nerd Font, `fallback = true`, `size_adjustment = .none` (comment: "No size adjustment for the symbols font").
`SharedGridSet.zig:336-372` — on macOS, "Apple Color Emoji" looked up by exact family through `discoverExactFamily` and added as fallback; comment: always prefer the official one even if the user installed other emoji fonts.
`SharedGridSet.zig:375-401` — elsewhere, embedded Noto Color Emoji plus Noto Emoji (text presentation).
`font/embedded.zig:9-22` — the embedded files.

### Resolution per codepoint

`font/CodepointResolver.zig:88-118` — the algorithm is written as a numbered doc comment: (1) disabled style → regular; (2) `font-codepoint-map` override wins regardless of style/presentation; (3) sprite codepoints always go to the sprite face; (4) exact style+presentation in loaded fonts, in load order; (5) non-regular style falls back to regular *before* discovery ("styles in other fonts often change metrics like glyph widths"); (6) regular + discovery enabled → system fallback search; (7) last resort: regular with any presentation.
`CodepointResolver.zig:152-157` — default presentation decided by `uucode.get(.is_emoji_presentation, cp)` (Unicode `Emoji_Presentation` property) when no VS15/VS16 is present.
`CodepointResolver.zig:170-219` — discovery fallback: `discoverFallback` with `codepoint`, `size`, `bold`, `italic`, `monospace = false`; each candidate is re-checked with `hasCodepoint(cp, p_mode)` because "we can't search presentation in discovery"; the found face is appended to the collection permanently with `size_adjustment = default_fallback_adjustment`.
`font/main.zig:73` — `default_fallback_adjustment = .ic_width`.
`font/Collection.zig:572-662` — `scaleFactor`: fallback faces are scaled so a metric matches the primary face, in em units; tries `ic_width` (advance of U+6C34 水), then `ex_height`, `cap_height`, `line_height` when the face does not define the previous one. Comment: "functions very much like the `font-size-adjust` CSS property".
`font/face/coretext.zig:808-846` — ic_width is discarded when the glyph bounds exceed its advance (Nerd Font patcher damage).

### CoreText discovery

`font/discovery.zig:162-244` — `Descriptor.toCoreTextDescriptor`: family, style name, codepoint as a `CTFontDescriptor` character set attribute, symbolic traits bold/italic/monospace, variations.
`discovery.zig:371-400` — `discover`: `CTFontCollectionCreateWithFontDescriptors` → `createMatchingFontDescriptors` → copy to a slice → sort by `Score`.
`discovery.zig:656-690` — `Score` is a packed struct compared as an integer; fields in increasing precedence: `glyph_count: u16`, `fuzzy_style: u8`, `bold`, `italic`, `exact_style`, `monospace`, `codepoint`. So "has the codepoint" outranks everything, then monospace, then exact style string, then italic over bold ("a font that's italic when it shouldn't be … is a bigger problem than being the wrong weight").
`discovery.zig:691-760` — scoring loads the font (`CTFontCreateWithFontDescriptor` at 12pt), tests `getGlyphsForCharacters`, and refines bold/italic from the `head.macStyle` and `OS/2` tables rather than trusting symbolic traits alone.
`discovery.zig:409-448` — `discoverExactFamily`: `CTFontCreateWithName` then verify `copyFamilyName` equals the request, because CTFontCreateWithName "never returns null: if the requested font isn't installed it returns a substitute font".
`discovery.zig:450-512` — `discoverFallback`: CJK Unified Ideographs U+4E00–U+9FFF bypass descriptor matching and go straight to `CTFontCreateForString` "because there isn't a better way … that properly takes into account system locale"; otherwise descriptor search, and if that returns nothing for a codepoint, `CTFontCreateForString` (issue #2499).
`discovery.zig:515-612` — `discoverCodepoint`: pick the collection's own face in the requested style as the "original" font, build a one-codepoint CFString, call `CTFontCreateForString(original, str, range)` (range length in UTF-16 units), reject a result whose PostScript name is `LastResort`.
`discovery.zig:354-367` — `warmup` touches "AppleColorEmoji" at startup because "the first CoreText query in a process initializes the system font database, which takes multiple milliseconds".

### Fontconfig discovery (Linux)

`discovery.zig:120-150` — pattern gets `charset` with the codepoint and always `spacing=monospace` ("helps prefer it", not exclude).
`discovery.zig:292-340` — `discoverFallback` sorts by fontconfig's own `FcFontSort` and keeps the charset for later checks.

### Emoji / colour glyphs

`face/coretext.zig:103-106` — a face gets a `ColorState` only when `CTFontGetSymbolicTraits` has `color_glyphs`.
`coretext.zig:905-946` — `ColorState.init`: presence of an `sbix` table marks *every* glyph as colour ("we can improve this later"); otherwise the `SVG ` table is parsed and `hasGlyph(id)` decides per glyph.
`coretext.zig:953-968` — `isColorGlyph`.
`CodepointResolver.zig:303-314` — presentation of a rendered glyph = `.emoji` iff `isColorGlyph`, which selects the colour atlas.
`font/SharedGrid.zig:416-432` — every emoji glyph is forced to constraint `size = .cover`, centred both axes, `pad_left/right = 0.025` ("doesn't quite touch the edges").
`font/shaper/run.zig:162-176` — VS15 (U+FE0E) / VS16 (U+FE0F) as the *first* grapheme extension codepoint sets explicit text/emoji presentation.
`run.zig:318-382` — `indexForCell`: a grapheme with combining marks needs one font that covers every codepoint; ZWJ/VS are skipped; extension codepoints are not required to match the base presentation because emoji fonts "support the base emoji with emoji presentation but not certain ZWJ-combined characters like the male and female signs".

## 2. Shaping

### Runs from grid cells

`font/shaper/run.zig:47-304` — `RunIterator.next`, one row at a time, never across rows. Rules in order:
- `:54-61` trim trailing empty cells; `:65-67` skip `invisible` cells.
- `:96-101` break at selection start and end+1 (selection colours must not share a ligature).
- `:104-107` spacer head/tail cells of wide chars are skipped.
- `:112-137` break "bad ligatures": `f`+`l`, `f`+`i`, `s`+`t` are always split so coding fonts do not form typographic ligatures in code.
- `:139-147` break when the style changes, except background colour (`comparableStyle` zeroes `bg_color`, `:387-396`): "prevents a single glyph for `>=` to be rendered with one color when the two components have different styling".
- `:151-159` font style = bold / italic / bold_italic / regular from SGR flags.
- `:178-209` break around the cursor cell (configurable `font-shaping-break = cursor`, `Config.zig:357-382`) so the character under the cursor is never inside a ligature; a grapheme under the cursor is not split.
- `:215-249` each cell's font is resolved through `indexForCell`; if nothing covers it, U+FFFD, then space. `:255` the run ends when the font index changes.
- `:264-267` Kitty graphics placeholder becomes a space; `:273` codepoint 0 becomes a space.
- `:89-91, 288-292` the run hash uses *relative* cluster positions plus length and font index, so identical text at different columns shares one cache entry.
`font/shape.zig:41-58` — shaper output `Cell { x: u16, x_offset: i16, y_offset: i16, glyph_index: u32 }`: one entry per glyph, positioned on a grid cell with an integer pixel nudge.
`font/shaper/Cache.zig:3-6, 33-45` — shaping results cached by run hash in a fixed 256-bucket × 8-entry table; header comment: shaping "was the most expensive part of rendering text (accounting for 96% of frame time on my machine)".

### Features

`font/shaper/feature.zig:287-289` — `default_features = { liga = 1 }` only. `Config.zig:231-253` — `font-feature` accepts HarfBuzz/CSS syntax; "To disable programming ligatures, use `-calt`"; "`-calt, -liga, -dlig`" disables most. `quirks.disableDefaultFontFeatures` (`face/coretext.zig:115`) drops the defaults for known-broken fonts (Monaspace).

### CoreText shaper (`.coretext` backend)

`font/shaper/coretext.zig:115-171` — features become a `kCTFontFeatureSettingsAttribute` dictionary with OpenType tag/value pairs; two dictionaries, with and without defaults.
`coretext.zig:175-200` — a `CTTypesetter` is created with `kCTTypesetterOptionForcedEmbeddingLevel = 0` to kill BiDi: "we force LTR no matter what" (issues #1737, #1442); the attributed-string writing-direction attribute was tried and "still allow[ed] some weird results".
`coretext.zig:333-349` — sprite ("special") fonts are not shaped; codepoint == glyph index.
`coretext.zig:357-385` — UTF-16 string → `CFAttributedString` with the per-face font attr dict (cached per face, `:585-640`, font copied with the feature descriptor) → `CTTypesetterCreateLine` → `CTLineGetGlyphRuns`.
`coretext.zig:436-511` — cluster → cell mapping: `x = cluster`, `x_offset = round(position.x − cell_offset.x)`, `y_offset = round(position.y)`; the long comment at `:441-490` explains the Chakma/Indic reordering case where a first-in-cluster codepoint appears after glyphs of a later cluster.
`coretext.zig:525-541` — if any run is non-monotonic or RTL, cells are sorted by `x` (cold path).
`coretext.zig:200-215, 271` — CoreText objects are released on a dedicated `cf_release` thread; `endFrame` flushes the pool once per frame (`renderer/generic.zig:1379`).

### HarfBuzz shaper

`font/shaper/harfbuzz.zig:262-281` — buffer: content type unicode, cluster level `characters` ("matching the CoreText shaper, and allowing us to use our same ligature detection heuristics"), direction forced LTR ("our renderers assume a strictly increasing X value"); `finalize` calls `guessSegmentProperties`.
`harfbuzz.zig:284-293` — the HarfBuzz cluster value is an *index* into a side array that maps back to the real cell cluster.
`harfbuzz.zig:131-150` — `hb_shape(face.hb_font, buf, features)`; the grid lock is exclusive because FreeType faces are not thread-safe.
`harfbuzz.zig:232-250` — offsets and advances are 26.6 fixed point, rounded with `(v + 0b100_000) >> 6`.

### Grapheme width

`Config.zig:495-515` — `grapheme-width-method = unicode` default; mode 2027 forces it. Cells carry the grapheme's extra codepoints (`cell.hasGrapheme()`), so the shaper sees whole clusters.

## 3. Rasterization

### CoreText path (`font/face/coretext.zig:289-567`)

- `:301` `CTFontGetBoundingRectsForGlyphs` gives the exact outline box (origin bottom-left, +Y up).
- `:315-320` synthetic bold grows the rect by `line_width` and shifts origin by half; skipped for sbix bitmaps.
- `:326-334` boxes under 0.25px in either axis return a zero-sized glyph.
- `:347-356` `constraint.constrain(...)` (Nerd Font / symbol / emoji rules, see §6) remaps size and position in cell-relative space.
- `:370-380` glyph is centred in the integer cell when the cell is wider than the face advance: `x += (cell_width − face_width)/2`; for a negative difference only the fractional part is kept "for consistent subpixel positioning".
- `:385-390` sbix bitmaps are snapped to whole pixels.
- `:396` `canvas_padding = 1` when `thicken` is on ("font smoothing adds no more than 1 extra pixel to any edge").
- `:400-413` **subpixel positioning mechanism**: `px_x = floor(x)`, `frac_x = x − floor(x)`; canvas size is `ceil(width + frac_x)`; the fraction is baked into the raster via `translateCTM` (`:522-526`). The bearings sent to the GPU are integers. The fractional offset is a deterministic function of glyph + constraint, so each glyph is rasterized once and reused at every column; this is "subpixel positioning" in the sense of placing the outline correctly within the cell, not Zed-style per-position variants.
- `:415-432` mask glyphs: 8-bit, colour space `kCGColorSpaceLinearGray`, `kCGImageAlphaOnly`. Colour glyphs: 32-bit, colour space Display P3, `byte_order_32_little | premultiplied_first` → premultiplied BGRA.
- `:453-461` `CGBitmapContextCreate` over a zeroed buffer.
- `:478-483` `setAllowsFontSmoothing(true)`, `setShouldSmoothFonts(opts.thicken)` — comment: "'Font smoothing' is what we call 'thickening', it's an attempt to compensate for optical thinning of fonts, but at this point it's just something that makes the text look closer to system applications if users want that."
- `:485-488` `setAllowsFontSubpixelPositioning(true)`, `setShouldSubpixelPositionFonts(true)`.
- `:490-494` `setAllowsFontSubpixelQuantization(false)`, `setShouldSubpixelQuantizeFonts(false)` — "we very carefully manage the position of our glyphs ourselves".
- `:496-498` antialiasing on. **Hinting: CoreText has none to turn off; outlines are unhinted, AA only.**
- `:500-508` fill colour for masks = `thicken_strength/255` gray (default 255), so `font-thicken-strength` scales coverage (`Config.zig:343-355`: "`0` does not correspond to *no* thickening, rather … the lightest available").
- `:510-515` synthetic bold: `CGTextDrawingMode.fill_stroke` with `setLineWidth(line_width)`.
- `:528-534` `scaleCTM(width/rect.width, height/rect.height)` applies constraints as a scale.
- `:536-545` `CTFontDrawGlyphs` at `(−rect.origin.x, −rect.origin.y)`.
- `:548-549` `atlas.reserve` then `atlas.set`; `:553` `offset_y = px_y + px_height` (distance from cell bottom to glyph top, the convention the vertex shader expects).

Synthetic styles: `coretext.zig:183-199` bold line width `max(points/14, 1)` ("a line width of 1 looks good to me at a certain point size"); `:41-49, 174-178` italic via `CTFontCreateCopyWithAttributes` with an affine skew `c = 0.267949` (≈ tan 15°). `Collection.zig:321-467` `completeStyles` builds synthetic italic from regular, bold from regular, bold-italic from bold (italicize) or italic (embolden); `Config.zig:200-229` `font-synthetic-style` can disable each.

### FreeType path (`font/face/freetype.zig`)

- `:355-392` load flags: hinting only if configured *and* the glyph has no constraint ("it'll mess up the hinting anyway when it moves or resizes the glyph"); `target = mono | light | normal`; `no_svg = true` always ("requires bundling another dependency").
- `Config.zig:9912-9924` defaults: `hinting=true, force-autohint=false, monochrome=false, autohint=true, light=true` — i.e. **FreeType light autohinting**, matching "most Fontconfig-aware software".
- `:441-448` synthetic bold `FT_Outline_Embolden` by `ceil(size.metrics.height × 64/2048)` ("1 pixel per 64 units of height").
- `:503-512` re-centering offset is rounded to whole pixels "because under FreeType, the outlines will be hinted … If we move the outlines by a non-whole-pixel amount, it completely ruins the hinting."
- `:528-569` the outline points are transformed manually in f64 (subtract bearings, scale to constraint, add new bearings, skew by `tan 12°` for synthetic italic) instead of `FT_Set_Transform` ("16.16 coefficients … weird issues").
- `:572-576` `FT_Render_Glyph` normal or mono; `:579-590` `FT_Bitmap_Convert` to 8bpp; `:741-748` `num_grays < 256` scaled up to 255.
- No LCD/subpixel AA anywhere. No gamma at raster time on either backend; gamma is a shader concern (§7).

### Metrics (`font/Metrics.zig:227-325`, `face/coretext.zig:570-868`)

- Cell width = max advance over printable ASCII (`coretext.zig:767-805`); ASCII height from the same glyphs' bounding box.
- Vertical metrics: `hhea` first unless `OS/2 fsSelection.use_typo_metrics`, then `OS/2 sTypo*`, then `usWin*` ("pretty much what FreeType does", `coretext.zig:642-702`).
- `Metrics.zig:239-266` `cell_width = round(face_width)`, `cell_height = round(ascent − descent + line_gap)`; long comment on why round beats ceil (≤0.5px error, consistent across DPI; glyph overflow is the font author's problem, fix with `adjust-cell-height`).
- `:271-290` half the line gap goes above, half below; `cell_baseline = round(face_baseline − (cell_height − face_height)/2)` centres the face in the rounded cell; `face_y` records the residual.
- `:298-301` `underline_thickness = max(1, ceil(...))`, positions rounded; `:318-320` overline and box thickness = underline thickness, cursor height = cell height. All adjustable via `adjust-*` (`Config.zig:438-488`).

## 4. Glyph atlas

`font/Atlas.zig:1-7` — skyline bin packing from Jylänki's "A Thousand Ways to Pack the Bin", "based heavily on Nicolas P. Rougier's freetype-gl". `:9-14` always square, packed strides only.
`:53-67` formats: `grayscale` (1 bpp), `bgr` (3), `bgra` (4). `SharedGrid.zig:92-95` two atlases per grid: 512² grayscale, 512² bgra.
`:136-210` `reserve`: best-fit = lowest resulting `y + height`, tie-break narrowest node; new skyline node inserted, overlapping nodes shrunk, adjacent same-height nodes merged (`:238-251`).
`:375` a 1px border is kept around the texture "to avoid artifacting when sampling"; `:218, 229` fit rejects anything touching `size − 1`.
`:46-51` `modified` and `resized` atomic counters bumped on every `set`/`grow`; the renderer compares them per swap-chain frame.
`:314-364` `grow(size_new)`: allocate, zero, copy old rows 1..size−1, append one node for the new right-hand strip, bump both counters. `SharedGrid.zig:443-451` on `AtlasFull` the grid doubles the atlas and retries once.
**Eviction: none.** `:367-376` `clear()` exists but is only used on font change (new `SharedGrid`); glyphs are keyed forever by `GlyphKey` (`SharedGrid.zig:501-530`: face index, glyph id, render options packed into a u64).
GPU copies: `renderer/Metal.zig:347-375` `r8unorm` for grayscale, `bgra8unorm_srgb` for colour (so sampling returns linear premultiplied). `renderer/generic.zig:1889-1904` each of the 3 frame states owns its own texture pair and `*_modified` watermark; `:3677-3693` `syncAtlasTexture` re-creates the texture if the atlas outgrew it and uploads the **entire** atlas with one `replaceRegion` (`metal/Texture.zig:92-117`). No dirty-rect upload.

## 5. GPU pipeline

### Pipelines and vertex formats (`renderer/metal/shaders.zig`)

`:13-46` five pipelines: `bg_color` (full-screen triangle, blending off), `cell_bg` (full-screen triangle, blending on), `cell_text` (per-instance vertex step, blending on), `image` (per-instance), `bg_image`. `metal/Pipeline.zig:126-133` all blending is premultiplied: `src = one`, `dst = one_minus_source_alpha` for RGB and alpha.
`:194-264` `Uniforms`: `projection_matrix`, `screen_size`, `cell_size`, `grid_size`, `grid_padding`, `padding_extend` bitmask, `min_contrast`, `cursor_pos`, `cursor_color`, `bg_color`, bools `cursor_wide`, `use_display_p3`, `use_linear_blending`, `use_linear_correction`.
`:267-290` `CellText` instance = **32 bytes** (test asserts it): `glyph_pos: [2]u32`, `glyph_size: [2]u32`, `bearings: [2]i16`, `grid_pos: [2]u16`, `color: [4]u8`, `atlas: u8`, `bools: u8 {no_min_contrast, is_cursor_glyph}`. `:293` `CellBg = [4]u8`.

### Shaders (`renderer/shaders/shaders.metal`)

- `full_screen_vertex` `:191-217` one oversized triangle clipped to the viewport.
- `bg_color_fragment` `:224-233` paints the terminal background via `load_color` (not clear colour, because clear would need CPU-side colour conversion, `generic.zig:1927-1930`).
- `cell_bg_fragment` `:451-510` — **no per-cell quads for backgrounds**: the fragment computes `grid_pos = floor((pixel − grid_padding.wx) / cell_size)` and indexes a `uchar4` buffer of `rows × cols` colours; out-of-grid pixels either return transparent or clamp to the edge cell when the matching `EXTEND_*` bit is set (`window-padding-color = extend`). Comment admits per-fragment colour conversion is wasted work.
- `cell_text_vertex` `:556-676` — corner from `vertex_id` (triangle strip of 4), `offset.y = cell_size.y − bearings.y` (bearing is "distance from the bottom of the cell to the top of the glyph", ASCII diagram at `:580-610`, `offset.y` at `:615`), position = `cell_pos + glyph_size × corner + offset`, texture coordinates in **pixels** (sampler uses `coord::pixel`); colour is loaded linear; the cell's bg colour is fetched from the bg buffer and composited over the global bg for the min-contrast test; `:658-673` any glyph at `cursor_pos` that is not the cursor glyph itself has its colour replaced with `cursor_color` (block cursor text inversion done on the GPU).
- `cell_text_fragment` `:678-765` — sampler `nearest` (`:687`), `clamp_to_edge`; grayscale: `color *= a` (premultiplied mask); colour atlas: pass through premultiplied linear, un-linearize if not linear blending. §7 has the blending maths.

### Frame construction (`renderer/generic.zig`)

- `updateFrame` `:1371-1713` — the terminal mutex is held only inside the `critical` block (`:1420-1557`): skip entirely while mode 2026 synchronized output is set (`:1424-1427`), `RenderState.beginUpdate` snapshots rows (style denormalization deferred to `endUpdate` outside the lock, `:1491-1498`), scrollbar, preedit, Kitty placements, OSC 8 hover links. Then `rebuildCells` under `draw_mutex`.
- `rebuildCells` `:2630-2920` — full rebuild only when `state.dirty == .full` or the grid resized; otherwise **row dirty flags**: `if (!dirty.*) continue; self.cells.clear(y);` (`:2733-2736`). Cursor handled last (`:2762-2896`): block cursor sets `uniforms.cursor_pos/cursor_color` (`:2835-2894`) so the shader recolours glyphs under it.
- `rebuildRow` `:2925-3371` — per row: run iterator → shaper cache → `addUnderline` first ("so that they layer underneath text … when a colored underline is used which intersects parts of the text (descenders)", `:3261-3264`), overline, glyphs, strikethrough last. Links get a single underline, or double if already underlined (`:3245-3258`). Faint = alpha `faint-opacity` (default 0.5, `:3193`). Background alpha rules for transparent windows `:3203-3226`.
- `renderer/cell.zig:34-74` `Contents`: `bg_cells` flat `[rows × cols]u8x4`; `fg_rows` one ArrayList per row plus a cursor list at index 0 (block cursor, drawn first so text draws on top) and one at `rows+1` (hollow/bar/underline cursors drawn last) (`:136-154`). Rows pre-reserve `cols × 3` glyph slots (`:100-105`).
- `drawFrameLocked` `:1740-2043` — `needs_redraw = size_changed || swap_chain_rebuilt || cells_rebuilt || images_rebuilt || animation || sync` (`:1792-1806`); otherwise return without touching the GPU. Then: wait on the swap-chain semaphore (`SwapChain`, `:305-363`, `swap_chain_count = 3` in `Metal.zig:38`), resize target if needed, `uniforms.sync`, `cells_bg.sync` (whole array), `cells.syncFromArrayLists` (`:1878-1880`; concatenates all row lists into one instance buffer — `metal/buffer.zig:127-183`, memcpy into `MTLBuffer.contents`, doubles capacity on growth), atlas texture sync by watermark (`:1889-1904`), then one render pass: bg colour/image → Kitty below bg → `cell_bg` → Kitty below text → `cell_text` instanced `fg_count` → Kitty above → debug overlay (`:1912-2010`); custom shaders run as post passes ping-ponging two textures (`:2013-2041`).
- So "avoid re-uploading unchanged cells" is CPU-side: dirty rows skip shaping and list rebuild; the GPU instance buffer is still fully rewritten on every *drawn* frame (32 B × glyph count, trivially small), and frames with no change are not drawn at all.
- `metal/Device.zig:44-47` storage mode `shared` on unified memory, `managed` otherwise; `Metal.zig:246-255` buffers are `cpu_cache_mode = write_combined` ("the CPU writes to this resource but never reads it").

### Presentation and vsync

- `metal/Target.zig:45-105` render target is an **IOSurface-backed MTLTexture**, pixel format `bgra8unorm` or `bgra8unorm_srgb` by blending mode; the IOSurface colour space is Display P3 — comment: "This allows us to have 'Apple-style' alpha blending, since it seems to be the case that Apple apps like Terminal and TextEdit render text in the display's color space using converted colors, which reduces, but does not fully eliminate blending artifacts."
- `metal/Frame.zig:67-91` command-buffer completion handler presents by handing the IOSurface to the `CALayer` (`metal/IOSurfaceLayer.zig:52-80`, always on the main thread) and reports `Health`.
- `renderer/DisplayLink.zig:1-30` a `CVDisplayLink`; start/stop only on the main thread because `CVDisplayLinkStop` can deadlock against CoreVideo's own reconfiguration (discussion #14150). `generic.zig:1273-1310` `syncDisplayLink`: the link runs only while visible *and* something changed or animates; `:1186-1194` `hasVsync`; `renderer/Thread.zig:462-472` when the link runs, the thread draws only on the link's `draw_now` tick; otherwise it draws immediately after `updateFrame` (`:537-589`). Wakeups come from the IO thread's mailbox (`:474-517`); the old 10 ms coalescing timer is commented out.
- `Config.zig:2084` `window-vsync = true`.

## 6. Subpixel positioning, HiDPI, min contrast, cursors, underlines, sprites

- **Subpixel positioning** — see §3: fractional bearings baked into the raster, integer placement, `shouldSubpixelQuantizeFonts = false`. Shaper nudges (`x_offset`, `y_offset`) are integer pixels (`shape.zig:50-51`).
- **HiDPI** — `font/face.zig:29` `default_dpi = 72` on macOS, 96 elsewhere; `:46-58` `pixels = points × dpi / 72`; `Metal.zig:126` layer `contentsScale = scaleFactor`; `:200-217` surface size = bounds × scale clamped to `max_texture_size`. Everything downstream is in device pixels (`renderer/size.zig:20-27`). Odd pixel sizes via half points (`Config.zig:271-276`).
- **Minimum contrast** — `shaders.metal:91-130` Rec. 709 luminance, WCAG 2.0 contrast ratio `(max+0.05)/(min+0.05)`, `contrasted_color` returns pure white or black whichever contrasts more; applied in the vertex shader when `min_contrast > 1` and `NO_MIN_CONTRAST` is clear (`:653-656`). `renderer/cell.zig:294-296` box drawing, block elements, legacy computing and Powerline opt out so bars stay seamless; emoji are excluded by taking the colour-atlas path. `Config.zig:773-784` default `1` (off); "1.1 is a good value" to avoid invisible text.
- **Cursor styles** — `renderer/cursor.zig:7-68`: `block`, `block_hollow`, `bar`, `underline`, `lock`; priority: preedit → block; password input → lock; unfocused → hollow; blink-off → none. `generic.zig:3539-3626` `addCursor`: sprites rendered via the sprite face at 1 or 2 cells wide; `lock` uses Nerd Font U+F023 (`:3594-3596`). `sprite/draw/special.zig:282-372`: `cursor_rect` fills the cell; `cursor_hollow_rect` fills then clears the inside by `cursor_thickness`; `cursor_bar` is shifted left by `(cursor_thickness+1)/2` "so that it sits centered between characters"; `cursor_underline` reuses the underline position with cursor thickness. `cursor-opacity` applies only when focused (`generic.zig:3561-3564`). `sprite/Face.zig:208-217` cursors use `metrics.cursor_height` (`adjust-cursor-height`) and are re-centred vertically (`:242-250`).
- **Underline styles** — `special.zig`: `underline` `:13-37` rect at `underline_position`, clamped to the canvas; `underline_double` `:39-72` one thickness above and below the single position; `underline_dotted` `:74-133` circles of radius `√½ × thickness` ("otherwise dotted underlines look somewhat anemic"), dot count = clamp(ceil(w/4r), floor(w/3r), floor(w/(2r+1))) ≥ 1; `underline_dashed` `:135-165` `dash_width = width/3 + 1`, every other dash; `underline_curly` `:167-235` one cosine-like cycle per cell from two cubic Béziers with curvature `r = 0.4`, amplitude `width/π`, round caps, stroke width = underline thickness, clamped so it never leaves the canvas. `strikethrough` `:237-253`, `overline` `:255-280`.
- **Sprite face (procedural glyphs)** — `font/sprite.zig:17-35` sprites live at codepoints above U+10FFFF. `sprite/Face.zig:57-145` a comptime table of `draw<CP>` / `draw<MIN>_<MAX>` functions collected from `draw/block.zig`, `box.zig`, `braille.zig`, `branch.zig` (git-branch drawing), `geometric_shapes.zig`, `powerline.zig`, `symbols_for_legacy_computing.zig` (+supplement); overlaps are compile errors. `Face.zig:219-223` canvas = cell plus a quarter cell of padding on every side so strokes can overshoot and join seamlessly (`draw/README.md`); `sprite/canvas.zig:6, 104` rasterizer is `z2d` (pure Zig 2D path filler) into an alpha8 surface; `:166-210` fully transparent rows/columns are trimmed before atlas insertion. `CodepointResolver.zig:139-145` the sprite face wins before any font, so box drawing/Powerline/braille/blocks are always drawn by ghostty, never by the font. `draw/box.zig:35-48` box characters are a packed `Lines {up,right,down,left: none|light|heavy|double}`; `draw/common.zig:24-35` `Thickness.light/heavy` scale `metrics.box_thickness` (= underline thickness). `draw/powerline.zig:24-58` arrows are `canvas.triangle` over the full cell. Reference PNGs per size in `sprite/testdata/` guard regressions.
- **Nerd Font / symbol constraints** — `font/nerd_font_attributes.zig` is a generated 3-level lookup of per-codepoint `Constraint`s derived from the Nerd Fonts `font-patcher` rules (`nerd_font_codegen.py`); `generic.zig:3498-3509` applies it, or `{ size = .fit }` for any `isSymbol` codepoint (PUA, arrows, dingbats, misc symbols, `cell.zig:230-246`). `cell.zig:250-290` a symbol may use 2 cells when the next cell is a space and the previous cell is not another symbol. `font/Glyph.zig:83-167` constraint vocabulary: `size none|fit|cover|fit_cover1|stretch`, align `start|end|center|center1`, paddings, `max_xy_ratio`, height basis `cell|icon`; `:216-270` scaling preserves the glyph's position inside its "scale group" so multi-glyph icons stay aligned.
- **Window padding colour extension** heuristics `renderer/row.zig` + `generic.zig:2945-2970`.

## 7. Gamma and blending constants, with the reasoning in the comments

- `shaders.metal:34-50` `sRGB_XYZ` (D50-adapted, Bruce Lindbloom) and `XYZ_DP3`; composed into `sRGB_DP3`. TODO at `:54-56`: should be a uniform.
- `:62-89` exact sRGB transfer: linearize `v ≤ 0.04045 ? v/12.92 : ((v+0.055)/1.055)^2.4`; unlinearize `v ≤ 0.0031308 ? 12.92v : 1.055 v^(1/2.4) − 0.055`.
- `:91-93` luminance = `dot(rgb, (0.2126, 0.7152, 0.0722))`.
- `:136-179` `load_color(u8x4, display_p3, linear)`: input colours are 8-bit sRGB (or P3 when `window-colorspace = display-p3`, `Config.zig:2202-2212`); always linearized, converted sRGB→P3 if needed ("our output is always Display P3"), re-encoded unless linear blending, then premultiplied.
- `Config.zig:384-411` `alpha-blending`: `native` (default on macOS: blend in the OS native space, P3 on macOS), `linear` ("eliminate the darkening artifacts around the edges of text that are very visible when certain color combinations are used (e.g. red / green), but makes dark text look much thinner than normal and light text much thicker. This is also sometimes known as 'gamma correction'"), `linear-corrected` (default elsewhere: "Same as `linear`, but with a correction step applied for text that makes it look nearly or completely identical to `native`, but without any of the darkening artifacts").
- Mechanism for linear: `Metal.zig:188-195, 220-233` the render target and custom-shader textures switch to `bgra8unorm_srgb`: "Using an `*_srgb` pixel format makes Metal gamma encode the pixels written to it *after* blending, which means we get linear alpha blending rather than gamma-incorrect blending." Shaders output linear; hardware blends linear; storage re-encodes. No shader-side blend emulation.
- Correction (`shaders.metal:713-731`, identical in `shaders/glsl/cell_text.f.glsl:45-67`): with coverage `a`, fg luminance `L_f`, bg luminance `L_b` (both linear): `blend_l = linearize(unlinearize(L_f)·a + unlinearize(L_b)·(1−a))`; `a' = clamp((blend_l − L_b)/(L_f − L_b), 0, 1)`; skipped when `|L_f − L_b| ≤ 0.001` "to avoid numbers going haywire". Comment: "This yields virtually identical results for grayscale blending, and very similar but non-identical results for color blending." In words: compute the luminance a gamma-space blend *would* have produced, then pick the linear alpha that lands on that luminance, so weight matches `native` while chroma fringes vanish.
- `:742-760` colour glyphs are "already premultiplied linear colors" because the colour atlas is `bgra8unorm_srgb` and CoreText drew them premultiplied in P3.
- Grayscale mask glyph quads are integer-aligned and sampled with `filter::nearest` (`:683-688`) — no bilinear softening; crispness comes from the raster, not the sampler.
- `metal/Target.zig:45-50` IOSurface in Display P3 (quoted in §5).
- Rasterizer side: mask colour space `linearGray` (`coretext.zig:424`), i.e. coverage is linear alpha with no gamma pre-compensation; the only weight knob is `font-thicken` (CoreText font smoothing) and `font-thicken-strength` as a coverage multiplier.

## Rust mapping

Versions and licenses from crates.io, 2026-10-21.

| Stage | ghostty | Rust crate | License | Reaches ghostty quality? |
|---|---|---|---|---|
| Discovery, macOS | CoreText descriptors + `CTFontCreateForString` | `objc2-core-text` 0.3.2 + `objc2-core-graphics` 0.3.2 (`objc2` family) | Zlib OR Apache-2.0 OR MIT | Yes, 1:1: same API surface. Alternative `core-text` 22.0.0 (servo, MIT/Apache-2.0) is older-style but complete. Zed uses `font-kit` + `core-text`. |
| Discovery, Linux | fontconfig | `fontdb` 0.24.0 (MIT) | MIT | Partially. fontdb loads system dirs and fontconfig config but has no `FcFontSort`/charset query; per-codepoint fallback is our own loop over faces (cosmic-text does exactly this with static Chromium/Firefox fallback lists). Or FFI to fontconfig (`fontconfig` crate, MIT, needs libfontconfig). `font-kit` 0.14.3 wraps both but is a thick abstraction. |
| Shaping | HarfBuzz (Linux, `coretext_harfbuzz`) / CoreText | `harfrust` 0.14.0 (MIT) — HarfBuzz port; cosmic-text's README now names HarfRust, `rustybuzz` 0.20.1 last released 2024-11 (INFERRED: superseded). `harfbuzz_rs` FFI if parity bugs appear. | MIT | Yes. Ghostty's Linux shaper *is* HarfBuzz; `coretext_harfbuzz` is a supported ghostty macOS config. Cluster level `characters`, forced LTR, feature strings all map directly. CoreText shaping via `objc2-core-text` `CTTypesetter` is possible if we want byte-identical macOS shaping; not needed. |
| Raster, macOS | CoreText into `CGBitmapContext` | `objc2-core-graphics` (`CGBitmapContextCreate`, `CGContextSetShouldSmoothFonts`, `…SubpixelPositionFonts`, `…SubpixelQuantizeFonts`) + `CTFontDrawGlyphs` | Zlib/Apache/MIT | Yes, exact. This is the only way to get Apple's rasterizer and Apple Color Emoji (sbix); Zed's `gpui_apple/text_system.rs` does the same calls (alpha-only gray context, P3/RGB for emoji, quantization off, smoothing only when "dilation" > 0). |
| Raster, Linux | FreeType light autohint | `swash` 0.2.10 (Apache-2.0 OR MIT): `ScaleContext`, `Render` with `Source::Outline/ColorOutline/ColorBitmap`, `.hint(true)`, fractional `offset`, embolden + affine transform built in | MIT/Apache | Close but not equal: swash hints with its own TrueType interpreter port; it has no FreeType autohinter, so CFF/unhinted fonts at small sizes look lighter than ghostty-Linux. `skrifa` 0.48.0 (MIT/Apache, Google fontations) has `HintingInstance` with a FreeType-derived autohinter port (INFERRED, verify) and COLRv1; pair with `zeno` 0.3.3 for filling. Exact parity: `freetype-sys` FFI (FreeType is FTL/GPL-2 dual). **Gap: measure swash vs skrifa vs FreeType side by side at 11–14 px before committing.** |
| Raster, generic pure-Rust | — | `ab_glyph` 0.2.32 (Apache-2.0), `fontdue` 0.9.5 (MIT/Apache/Zlib) | | No hinting, no colour, no variations in fontdue. Below bar for the grid; fine for nothing we need. |
| Colour emoji | sbix via CoreText; CBDT/COLRv0 via FreeType | CoreText on macOS; `swash` `ColorBitmap` (CBDT Noto) + `ColorOutline` (COLRv0) on Linux; `skrifa::color` for COLRv1 | | macOS yes. Linux: ghostty itself only does CBDT/COLRv0 and disables SVG, so swash matches. |
| Unicode data | `uucode` | `unicode-width` 0.2.2 (UAX [[d11-supervisor-death-survivable\|#11]]) + `unicode-segmentation` 1.13.3 (grapheme clusters) + `icu_properties` or `unicode-properties` for `Emoji_Presentation` | MIT/Apache | Yes. The terminal core (forked `alacritty_terminal`) already decides widths; renderer only needs `Emoji_Presentation`. |
| Atlas | own skyline packer, square, grows ×2 | `etagere` 0.3.0 (MIT/Apache, shelf packing, used by Zed and glyphon) or ~300 lines of our own port of `Atlas.zig` | | Either. Own port keeps grow-in-place + whole-texture upload semantics; etagere adds deallocation we do not need (ghostty never evicts). |
| Layout + GPU text, batteries included | — | `cosmic-text` 0.19.0 (MIT/Apache) + `glyphon` 0.12.0 (MIT/Apache/Zlib, cosmic-text + etagere + wgpu, R8 mask + colour atlas) | | Wrong shape for the grid: cosmic-text lays out proportional paragraphs (`Buffer`, wrapping, BiDi); we build runs from cells. glyphon has no instancing per cell, no min-contrast, no linear-corrected blend, no sprites. Candidate for chrome paragraphs only; not for v0.1. |
| Sprites (box, Powerline, braille, underlines, cursors) | `z2d` | `zeno` 0.3.3 (Apache/MIT, path fill with AA into alpha8) or `tiny-skia` 0.11 (BSD-3). Port `sprite/draw/*.zig` (MIT) — ~4,400 lines of Zig, of which `special.zig` 372 + `box.zig` 929 + `block.zig` 177 + `powerline.zig` 400 + `braille.zig` 148 cover 95 % of what a coding terminal shows. | | Yes, it is arithmetic; the reference PNGs in `sprite/testdata/` can be reused as golden tests. |
| Nerd Font constraints | generated table | port `nerd_font_tables.zig` (MIT) to a Rust 3-level LUT, or rerun `nerd_font_codegen.py` to emit Rust | MIT | Yes. |
| GPU | Metal / OpenGL | `wgpu` 30.0.1 (MIT/Apache) — WGSL, `Bgra8UnormSrgb` surface formats on Metal for the linear-blend trick, instanced draws, storage buffers for the bg grid | | Yes. Everything in `shaders.metal` is plain WGSL; no Metal-only feature is used. wgpu also exposes dual-source blending (Zed's subpixel shader uses `@blend_src`) if we ever want LCD AA; ghostty does not. |

### What we write ourselves

1. **Run iterator** over our grid rows: the twelve rules in §2, including the `f`+`l`/`f`+`i`/`s`+`t` ligature breaks, cursor break, selection break, style-except-bg break, grapheme-covering font search. ~400 lines.
2. **Codepoint resolver + collection** with styles, fallback flag, `ic_width` size adjustment, synthetic bold/italic policy. ~800 lines.
3. **Atlas** (two: `R8Unorm` mask, `Bgra8UnormSrgb` colour), grow-by-doubling, modified counters, per-frame-slot textures. ~300 lines.
4. **Cell contents**: flat bg colour buffer + per-row instance lists + two cursor lists, row-dirty rebuild. ~250 lines.
5. **WGSL**: `bg_color`, `cell_bg` (storage-buffer lookup by pixel), `cell_text` (32-byte instance, vertex-id corners, min-contrast, cursor recolour, linear-corrected alpha), `image`, post-process chain. ~600 lines, a straight transliteration of `shaders.metal`.
6. **Sprite face**: canvas with quarter-cell padding, trim, draw functions, golden tests. 1–4 k lines depending on coverage.
7. **Frame scheduler**: wake on mux diff, `needs_redraw` gate, `PresentMode::Fifo` for pacing (a `CVDisplayLink` port is not needed when wgpu's swapchain already blocks on vsync and we only draw on change — matches [[vision]] "render on change with the glow shader as the only continuous animator"). Triple buffering via wgpu's own frame acquisition instead of a hand-rolled semaphore swap chain.

### Zed's gpui, for contrast

Zed splits text into a platform text system and an atlas. On macOS (`crates/gpui_apple/src/text_system.rs`) it draws through CoreText exactly like ghostty (alpha-only gray `CGBitmapContext`, antialias on, subpixel positioning on, quantization off, RGB premultiplied context for `AppleColorEmoji`), but it rasterizes **N subpixel variants per glyph** (`SUBPIXEL_VARIANTS_X`, offset `variant / SUBPIXEL_VARIANTS` added to the draw origin) because proportional text lands at arbitrary fractional x; ghostty avoids that cost because every glyph sits at a deterministic offset inside a fixed cell. Zed's "font smoothing" is a dilation level 0–4 derived from the foreground luminance. Its Metal atlas (`gpui_apple/src/metal_atlas.rs`) uses `etagere::BucketedAtlasAllocator`, `A8Unorm` for monochrome and `BGRA8Unorm` for polychrome, 1024² textures growing by adding textures up to 16384², and `replaceRegion` per tile. The Metal `monochrome_sprite_fragment` does `color.a *= sample.a` with bilinear sampling, straight alpha, no gamma handling. The wgpu backend (`gpui_wgpu/src/cosmic_text_system.rs`) uses cosmic-text + swash with `.hint(true)`, `Source::ColorOutline → ColorBitmap → Outline` for emoji, and a `shaders_subpixel.wgsl` doing LCD-style per-channel coverage with dual-source blending and a "contrast and gamma correction" step — Zed chose subpixel AA where ghostty chose grayscale + linear-corrected. For a cell grid, ghostty's model is simpler and the result is the one the operator asked for.

### wezterm, for contrast

wezterm uses FreeType on every platform including macOS (`wezterm-font/src/rasterizer/freetype.rs`): default `freetype_load_target = "Normal"` (ghostty-Linux defaults to light), optional `HorizontalLcd`/`VerticalLcd` subpixel modes that collapse to `alpha = max(r,g,b)` after swizzling for BGR panels, synthetic italic via `FT_Set_Transform` with `FAKE_ITALIC_SKEW`, synthetic bold via a `synthesize_bold` flag into `load_and_render_glyph`, and it converts gray coverage to sRGB per channel while keeping alpha linear (`linear_u8_to_srgb8`, "RGB values are gamma-adjusted while the A value is linear") — a hand-rolled approximation of what ghostty gets from an `_srgb` render target. Everything, including masks, lands in one RGBA atlas (`Image::with_rgba32`), and when it fills the whole glyph cache is recreated larger (`OutOfTextureSpace`). Box drawing and underlines are procedural (`glyphcache.rs` `line_sprite` with `draw_curly` as a sine wave; `block_sprite`), so it shares ghostty's sprite philosophy, but shaping goes through HarfBuzz FFI and rendering through OpenGL or wgpu with proportional-text generality that a terminal does not need. Read it for the mux; do not copy its text path.

## Recommendation

Build ghostty's `coretext_harfbuzz` configuration in Rust, with the Linux half swapped to pure Rust, and keep the shader side a transliteration.

- **Discovery**: `objc2-core-text` on macOS with the same `Score` ordering, `CTFontCreateForString` for CJK and last-resort codepoints, `LastResort` rejection, Apple Color Emoji by exact family. `fontdb` on Linux with a per-codepoint coverage loop. Bundle JetBrains Mono variable (OFL) and Symbols Nerd Font (MIT) as fallbacks exactly as ghostty does.
- **Shaping**: `harfrust` everywhere, cluster level characters, forced LTR, `liga` on by default, `-calt` user override, run cache keyed by position-independent hash. One shaper on both platforms is the biggest simplification over ghostty and costs nothing visible.
- **Raster**: CoreText via `objc2-core-graphics` on macOS with smoothing off by default (`font-thicken` as an option), subpixel positioning on, quantization off, linear-gray alpha mask, P3 premultiplied BGRA for emoji. Linux: `swash` first; benchmark against `skrifa`+`zeno`; fall back to FreeType FFI only if neither reaches light-autohint quality at 12 px. Metrics: port `Metrics.calc` verbatim (round, half-gap, centred baseline).
- **Atlas**: own skyline port, two textures, grow ×2, whole-texture upload on change, one texture pair per in-flight frame.
- **GPU**: WGSL ports of the five pipelines; `Bgra8UnormSrgb` render target; **default `alpha-blending = linear-corrected`** with sRGB→P3 conversion (ghostty's own non-macOS default and the comment says it is visually identical to `native` minus the fringes — the "sexiest" option; the Oracle may flip to `native` after a side-by-side). Min-contrast, cursor recolour and padding-extend as in the Metal source. Nearest sampling on integer-aligned quads.
- **Sprites**: `zeno`, port `special.zig` → `box.zig` → `block.zig` → `powerline.zig` → `braille.zig` in that order, golden-test against `sprite/testdata/*.png`.
- **Frame loop**: row-dirty rebuild, `needs_redraw` gate, draw only on change, present with Fifo. Skip the CVDisplayLink.
- **Not used**: cosmic-text, glyphon, ab_glyph, fontdue, font-kit, FreeType on macOS, LCD subpixel AA.

Gaps: (1) swash/skrifa vs FreeType light autohint quality on Linux — no data until measured; (2) whether `harfrust` reproduces CoreText's output for Apple system fonts with AAT `morx` tables (HarfBuzz does support AAT; `harfrust` AAT support unverified) — matters only if the primary font is an Apple font like Menlo/SF Mono; (3) `skrifa` autohinter claim marked INFERRED above.

## Provenance

From source (line-cited): everything in §1–§7 and the "What we write ourselves" counts of ghostty files. From crates.io API: versions/licenses in the mapping table. From GitHub raw via summarizer, not line-verified: Zed and wezterm paragraphs, swash/cosmic-text/glyphon capability notes. Inferred: rustybuzz→harfrust succession, skrifa autohinter, Zed subpixel variant count, swash hinting engine quality.
