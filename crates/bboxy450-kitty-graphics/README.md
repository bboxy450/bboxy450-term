# bboxy450-kitty-graphics

The Kitty terminal graphics protocol for bboxy450-term: the subset that puts a
picture in a pane. A program writes `ESC _ G <keys> ; <base64> ESC \` and the
image appears at the cursor, scrolling with its line.

Supported: transmit (`a=t`, `a=T`, `a=p`) by direct data, chunked (`m=1`),
zlib (`o=z`); formats RGB (`f=24`), RGBA (`f=32`), PNG (`f=100`); cell extents
(`c`, `r`), source rectangle (`x`, `y`, `w`, `h`), z-index, cursor movement
(`C`), quiet replies (`q`), delete (`a=d` with `d=a`/`A`/`i`/`I`) and the
query (`a=q`). Not supported: files, shared memory, animation, Unicode
placeholders.

The crate owns [`Graphics`]: images, placements and chunk assembly, bounded by
`MAX_IMAGES` and `MAX_IMAGE_PIXELS`. The terminal core feeds it commands with
the cursor position, writes the reply back to the program, and projects the
placements against its scroll origin. `encode_png` is the other direction: the
bytes a program writes to show a PNG.

Independent implementation from the public specification
(https://sw.kovidgoyal.net/kitty/graphics-protocol/); no code copied.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
