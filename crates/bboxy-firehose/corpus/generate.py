#!/usr/bin/env python3
"""Author-time corpus generation. The binary never runs this or reads files."""
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
E = "\x1b"


def scene(name, text):
    HERE.joinpath(name + ".ansi").write_bytes(text.replace("\n", "\r\n").encode())


scene("codex", f"""{E}[1;38;2;64;180;255mCodex / synthetic session 0042{E}[0m
{E}[2mModel: fixture-agent | sandbox: isolated | no private content{E}[0m

{E}[36m> Repair the viewport resize regression.{E}[0m
{E}[1mexec{E}[0m  cargo test -p fixture-core -- resize
{E}[2m  cwd: /synthetic/workspace{E}[0m
{E}[32m  test resize_preserves_cursor ... ok{E}[0m
{E}[32m  test resize_keeps_wide_cells ... ok{E}[0m

{E}[1mapply_patch{E}[0m  src/viewport.rs
{E}[36m@@ -41,3 +41,4 @@{E}[0m
{E}[31m- let rows = incoming_rows;{E}[0m
{E}[32m+ let rows = incoming_rows.max(1);{E}[0m
{E}[32m+ grid.resize(cols, rows);{E}[0m
  publish_damage();

{E}[38;5;214mreview{E}[0m  Cursor clamped before the next frame is published.
{E}[1;32mPASS{E}[0m  12 tests | 0 failures | 2 files changed
""")

scene("claude", f"""{E}[1;38;2;217;119;87mClaude Code / synthetic session 0017{E}[0m
{E}[2mAll paths, messages and tool results in this scene are invented.{E}[0m

{E}[35m❯{E}[0m Explain the damaged-row publication path.

{E}[1m⏺ Read{E}[0m(src/parser_pool.rs)
  {E}[2m└─ 64 lines read{E}[0m
{E}[1m⏺ Search{E}[0m(pattern: "publish_damage", path: "src")
  {E}[2m└─ Found 3 matches{E}[0m

{E}[1m⏺ Agent{E}[0m(Verify wide-character bounds)
  ├─ {E}[36mRead{E}[0m tests/wide_cells.rs
  ├─ {E}[36mBash{E}[0m cargo test wide_cells
  └─ {E}[32mDone{E}[0m (4 tests passed)

{E}[3mThe parser owns its shard; the UI copies published rows.{E}[0m
{E}[38;5;245mNo lock is held while encoding a frame.{E}[0m
{E}[32m✓{E}[0m Review complete · 0 pending tool calls
""")

palette = f"{E}[1mPalette and text decorations{E}[0m\n\nANSI 16 foregrounds:\n"
palette += " ".join(f"{E}[{30+i if i < 8 else 90+i-8}m{i:02}{E}[0m" for i in range(16)) + "\n\n"
palette += "256-colour cube and grey ramp:\n"
for start in (16, 52, 88, 124, 160, 196, 232):
    palette += "".join(f"{E}[48;5;{i}m  " for i in range(start, min(start+18, 256))) + f"{E}[0m\n"
palette += "\nRGB ramp: " + "".join(f"{E}[48;2;{i*9};{190-i*4};{255-i*5}m " for i in range(28)) + f"{E}[0m\n"
palette += f"{E}[1mbold{E}[0m  {E}[2mdim{E}[0m  {E}[3mitalic{E}[0m  {E}[1;3mbold italic{E}[0m  {E}[7mreverse{E}[0m  {E}[9mstrike{E}[0m\n"
palette += "  ".join(f"{E}[4:{style}m{label}{E}[0m" for style, label in enumerate(("single", "double", "curly", "dotted", "dashed"), 1)) + "\n"
palette += f"{E}[58:2::255:96:128m{E}[4:3mcoloured underline{E}[0m   {E}[30;47m black on light {E}[0m\n"
scene("palette", palette)

scene("unicode", f"""{E}[1mUnicode, cell widths and procedural glyphs{E}[0m

ASCII       | ABCDEFGHIJKLMNOPQRSTUVWXYZ | 0123456789
Latin       | café naïve Ångström Straße | Ελληνικά Кириллица
Combining   | é ä ô ñ Å | ẋ̣ z̄ | café
CJK         | 中文測試 日本語 한국어 | 全角：１２３
Emoji       | 🦊 🚀 🔧 🌒 | 👩‍💻 👨‍👩‍👧‍👦 🏳️‍🌈 | 👍🏽
Width ruler | 1234567890123456789012345678901234567890

┌──────────────────┬──────────────────┐
│ parser shard 01  │ render seat 0042 │
├──────────────────┼──────────────────┤
│ {E}[32mREADY{E}[0m            │ {E}[36m120 Hz{E}[0m           │
└──────────────────┴──────────────────┘
Double box  ╔══════╦══════╗  ║ ╠ ╣ ╬ ╚ ╝
Rounded     ╭──────╮ ╰──────╯   ━┃┏┓┗┛┣┫╋
Powerline   {E}[30;46m main {E}[36;44m{E}[37;44m src {E}[34;40m{E}[0m     
Blocks      ▁▂▃▄▅▆▇█ ▏▎▍▌▋▊▉ ░▒▓█ ▀▄▐▌
Braille     ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏ ⣿
""")

progress = f"{E}[1mLive tool progress / cursor rewrites{E}[0m\n\n"
for i in range(21):
    progress += f"\r{E}[2K{E}[36m{'⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'[i % 10]}{E}[0m compiling [{'█'*i}{'░'*(20-i)}] {i*5:3}%"
progress += f"\r{E}[2K{E}[32m✓{E}[0m compiling [{'█'*20}] 100%\n"
progress += "parser: queued\nrender: queued\nframes: queued\n"
progress += f"{E}[3A{E}[2Kparser: {E}[32mcomplete{E}[0m\n{E}[2Krender: {E}[32mcomplete{E}[0m\n{E}[2Kframes: {E}[32mcomplete{E}[0m\n"
progress += f"\nCursor horizontal: WRONG{E}[5D{E}[K{E}[1mRIGHT{E}[0m\n"
progress += "Final counters: 120 frames | 0 dropped | 42 synthetic tools\n"
scene("progress", progress)

alternate = f"{E}[1mPrimary screen before the tool{E}[0m\nPersistent transcript line A\nPersistent transcript line B\n"
alternate += f"{E}[?1049h{E}[2J{E}[H{E}[1;36mAlternate-screen synthetic inspector{E}[0m\n\n"
alternate += "┌──────────────────────────────────┐\n│ seat 0042    parser shard 01      │\n│ rows 24      columns 80           │\n└──────────────────────────────────┘\n"
alternate += f"{E}[8;1H{E}[7m selected: viewport.rs {E}[0m{E}[10;1HInspecting…\r{E}[2KInspection complete.\n"
alternate += f"{E}[?1049l{E}[5;1H{E}[32mReturned to the primary screen.{E}[0m\nThe two transcript lines above survived unchanged.\n"
scene("alternate", alternate)

scroll = f"{E}[1mScroll region, links and synchronized output{E}[0m\nPinned header: outside rows 4–10\n"
scroll += f"{E}[12;1HPinned footer: outside rows 4–10{E}[4;10r{E}[4;1H"
for i in range(1, 13):
    scroll += f"{E}[38;5;{40+i}mregion event {i:02}: synthetic tool finished{E}[0m\n"
scroll += f"{E}[r{E}[14;1H{E}]8;id=t1-docs;https://example.invalid/fixture{E}\\{E}[4mSynthetic source link{E}[0m{E}]8;;{E}\\\n"
scroll += f"{E}[?2026h{E}[16;1HAtomic frame: [assembling]{E}[16;1HAtomic frame: {E}[1;32m[committed]{E}[0m{E}[?2026l\n"
scroll += f"{E}]7501;state=working:app=fixture:id=synthetic{E}\\{E}]7501;state=done:app=fixture:id=synthetic{E}\\{E}]7501;state=clear:id=synthetic{E}\\"
scroll += "\nBEL and ST terminators are both covered by the capture scenes.\n"
scene("scroll-links-sync", scroll)

for name, source, title in (
    ("osc7501-claude", "osc7501-claude.bin", "Claude status capture"),
    ("osc7501-preflight", "osc7501-claude-preflight.bin", "Claude preflight capture"),
):
    capture = ROOT.joinpath(".bboxy/runs", source).read_bytes()
    # The originals contain only OSC 7501 control sequences, no session text.
    display = f"{E}[1m{title}{E}[0m\r\n\r\n"
    display += f"Original control-only bytes: {len(capture)}\r\n"
    states = "idle → working → child working → clear"
    if name == "osc7501-claude":
        states += " → done"
    display += f"States: {states}\r\n"
    display += "Transport: OSC 7501, BEL-terminated\r\n\r\n"
    display += "Visible text is synthetic; the captured bytes are unmodified.\r\n"
    HERE.joinpath(name + ".ansi").write_bytes(capture + display.encode())

codex_captures = b"".join(p.read_bytes() for p in sorted(ROOT.joinpath(".bboxy/runs").glob("osc7501-codex*.bin")))
assert not codex_captures, "Review new capture bytes for private text before embedding"
scene("osc7501-codex", f"""{E}[1mCodex OSC 7501 / explicit capture gap{E}[0m

Original Codex capture entries: 10 files, all empty (0 bytes).
No captured Codex status is claimed.

The protocol exercise below is entirely synthetic:
  working → blocked (question) → working → done → clear
  OSC terminator: ST
{E}]7501;state=working:app=codex:id=synthetic{E}\\{E}]7501;state=blocked:app=codex:id=synthetic:kind=question{E}\\{E}]7501;state=working:app=codex:id=synthetic{E}\\{E}]7501;state=done:app=codex:id=synthetic{E}\\{E}]7501;state=clear:id=synthetic{E}\\
{E}[2mGap: real Codex status bytes — no data.{E}[0m
""")
