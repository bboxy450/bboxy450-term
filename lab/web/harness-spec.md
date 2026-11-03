<!--
Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
-->
# Harness face: native design spec

This spec is the 1:1 target for the native bboxy450-harness GUI (`src/frontends/gui/`). The reference is
`harness.html` + `harness.js` + `lab.css` (`.hx-*` rules) in this folder. Open any state with
`harness.html?state=<slug>&theme=dark`. Where this document and the page disagree, the page is right; fix this document.

| State slug | Shows |
|---|---|
| `finished` | the tight transcript: a full real run with quiet heads, folded successes and a one-line receipt |
| `standalone-window` | the app window: the term's title bar, no OS chrome, Cmd+= / Cmd+- zoom |
| `empty-session` | the face before the first message: mark, question, sub-line, three chips |
| `composer-states` | every composer state on one board, composer only (the p2 target) |
| `composer-idle` · `composer-focused` · `composer-typing` · `composer-running` | the full face with that composer state |
| `slash-menu` | the "/" completion menu over the composer, typed `/pro` (the p3 target) |
| `profiles` · `new-profile` | the profile switcher from the header chip, ACP agents vs models |
| `approval-pending` · `denied-unattended` | the approval inside its tool card; composer in `ask` state |
| `error` · `interrupted` | red error receipt with actions; neutral interrupted receipt |
| `acp-claude` · `acp-codex` · `acp-pi` · `acp-opencode-loop` · `acp-omp` · `mermaid` | real harness logs (`harness/gen-acp.sh`) |

`scripts/parity-web.sh` in bboxy450-harness drives the same fold with `Hx.view({})`. All fold changes below show up there unchanged.

## Tokens

Dark theme. **Alpha** is the web form. **Flat** is that colour blended over `surface`, for native fills that have no alpha.
`seat` is the pane's seat colour; it falls back to `accent`. The flat seat column uses `#2EE6A8`, the lab's seat.
Recompute the flat seat column for any other seat colour: `flat = seat·p + surface·(1−p)`.

| Token | Alpha (web) | Flat on surface |
|---|---|---|
| `bg` | `#0B0B10` | — |
| `surface` | `#12142A` | — |
| `fg` | `#F2EEE6` | — |
| `fg-dim` | `#8A90B0` | — |
| `accent` | `#2E6BFF` | — |
| `ok` | `#2EE6A8` | — |
| `bad` (errored) | `#FF4D5E` | — |
| `ask` (waiting) | `#FFB454` | — |
| `card` | fg 4% | `#1B1D32` |
| `card-hi` | fg 7% | `#222337` |
| `hair` (1px lines) | fg 10% | `#282A3D` |
| `focus-fill` | fg 5% | `#1D1F33` |
| `seat-10` / `-14` | seat 10% / 14% | `#152937` / `#16313C` |
| `seat-40` / `-55` | seat 40% / 55% | `#1D685C` / `#21886F` |
| `ask-5` / `-30` / `-45` | ask 5% / 30% / 45% | `#1E1C2C` / `#594437` / `#7D5C3D` |
| `bad-12` / `-25` | bad 12% / 25% | `#2E1B30` / `#4D2237` |
| `ok-12` / `-25` | ok 12% / 25% | `#152D39` / `#19484A` |
| `term-bg` | `#07080C` | — |
| `term-fg` | `#C9CFDC` | — |

Type: chrome `-apple-system / Inter`; mono `SF Mono / Menlo`. Sizes are in px at the comfortable density.
Body text is 13. Messages are 13.5. Meta text is 11–11.5. Group labels are 10 bold caps with +0.08em tracking.
The radius scale is 4 (tags), 5–7 (tiles), 8 (buttons, inner blocks), 11 (cards), 13 (composer, menus) and full (chips, send).

## Components

### Header profile chip
- The chip is 22 high with pill radius and padding `1 8`. Its fill is `card` and its text is 11.5 `fg-dim`.
- Hover or open: border `seat-55` 1px, fill `seat-10`, text `fg`.
- **Model profile:** a 6px seat dot, then `profile · model`, then a 10px chevron rotated down.
- **ACP agent profile** (`profile_changed.model` = `acp:<command> · <model>`): an 18px **round** agent tile, then
  `profile`, then ` · model` in `fg-dim`, then the `ACP` tag, then the chevron. The dot is not shown.
- `ACP` tag: 9.5 bold, tracking +0.06em, padding `0 5`, radius 4, text `#2EE6A8`→fg 85%, fill `kind` (`#16313C`).
- Phase text sits on the right with a 7px dot. In `ask` the text and dot use `ask` and read `Waiting for you · <verb>`.
- **Flat fallback:** the web header has a vertical seat-9% → transparent gradient. Use a flat `seat-6` (`#142132`) band,
  or plain `surface` with the 1px `hair` bottom line. The dot's glow is dropped.

### Composer (`.hx-comp`, `.hx-box`)
- Outer padding is `8 12 12`. The box has radius 13, padding `8 10 6`, a 1px border, max width 860, and is centred.
- The input is 13, line height 20, at least one line high. The placeholder is `fg-dim` at 70%.
- The tool row has a 6px top margin. It holds the `@ context` and `queue` chips (10.5, padding `0 6`, pill, `card-hi`
  fill, `hair` border), a spacer, the hint (10.5 `fg-dim`), then the send button.
- The send button is a **26px circle**. Its icon is 14 (the stop square is 12).

| State | When | Border | Fill | Ring | Placeholder / hint | Send |
|---|---|---|---|---|---|---|
| idle | phase idle, not focused, empty | `hair` | `card` | none | "Ask bboxy, @ a file, / a command" / none | `card-hi` fill, `fg-dim` arrow |
| focused | input has focus | `seat-55` | `focus-fill` | 3px `seat-14` outside | same / none | as idle |
| typing | text present, phase idle | `seat-55` (focused) | `focus-fill` | 3px `seat-14` | — / "⏎ send · ⇧⏎ newline" | **seat** fill, `on-accent` arrow, glow |
| running | turn running | `hair`, or focus colours while focused | `card` | as focus | "Queue a follow-up…" / "esc interrupt" | empty: `card-hi` + 1px `hair` inset, `fg` stop square, hover `bad`; with text: seat arrow, hint "⏎ queue · esc interrupt" |
| ask | an approval waits | `ask-45` | `card` | none | "Answer the approval above" in `ask` / "1 allow · 2 always · 3 deny" in `ask` | idle look |

- Typed tokens: a leading `/command` is mono 0.95em, weight 650, in `seat`. An `@path` is a mono 0.9em pill
  (padding `0 5`, radius 5, fill `seat-14`, text seat 80% over fg).
- `queue` chip with N queued follow-ups reads `queue · N` in `seat`, with border `seat-40` and fill `seat-10`.
- **Alive rim (replaces the top-edge sweep and the focus ring):** the composer box wears the term's pane rim, the same
  shader. Web uses `glow.js` `Glow.attach(wrap, …, { box, live: true, block: true })`. Native uses `bbt_ui::Style::new(Preset::PaneGlow)`
  with `overrides.rim = Some(ui::Rim { activity, swell, focused, seed, .. })`, as `bbt_ui::modules::pane` does (spec `rim-glow`).
  - Activity: `run` → Active, `ask` → Waiting (amber), error → Errored, everything else → Idle.
  - Focus fires the swell (`Glow.swell` / `Rim.swell`). The border stays constant at `seat-35`; all motion lives in the rim.
  - **Flat fallback:** the static `seat-55` border.
- **Flat fallback, focus ring:** draw a second rounded rect 3px outside the box (radius 16) filled `seat-14` under the
  box. Do not use a blurred shadow. The send glow is dropped.

### Queued follow-ups (above the composer)
- One row per queued text, 6px above the box, 4px apart.
- Each row: padding `4 5 4 10`, radius 10, a **1px dashed** `seat-40` border, fill `seat-6` (`#142132`), 12.5 text.
- Contents in order: a 14px seat clock icon, the text (one line, ellipsis), `after this turn` (later rows read
  `after #N`) at 10.5 `fg-dim`, and a 20px ✕ button (radius 6, hover `card-hi`).
- **Flat fallback:** if the toolkit has no dashed stroke, use a solid `seat-40` 1px border. The clock icon carries the meaning.

### "/" completion menu
- The menu is anchored to the composer's top-left: left 12, bottom 4px into the composer's top padding. Its width is
  min(460, pane − 24). It shows at most 8 rows plus group labels (list max-height 292) and scrolls the selected row into view.
- Card: radius 13, padding `4 6 0`, a 1px `hair` border, and an outer 1px `seat` 16% ring.
- Group labels are `Commands` and `Switch profile`: 10 bold caps, `fg-dim`, padding `6 8 2`.
- **Row** (32 high, radius 8, padding `0 8`, gap 9):
  - a leading tile: commands get an 18px radius-5 `card-hi` tile with an 11px icon in `fg-dim`; a profile row gets
    its provider or agent tile at 18px;
  - the command in mono 12.5: the typed prefix in **seat bold**, the rest in `fg`;
  - a detail in 12 `fg-dim`, one line with ellipsis: the model, or for an agent its command and model;
  - on the right, a `kbd` (`esc` on `/interrupt`) or the `ACP` tag for agent profiles.
- Selected row: fill `seat-14`, a 2px seat bar inset 8px top and bottom at the left edge, and the tile turns seat on `seat-14`.
  Hover: `card-hi`.
- Footer: a 1px `hair` top border, padding `6 12`, 10.5 `fg-dim`. It reads
  `[↑][↓] select [⏎] run [esc] close · N matches`.
- `kbd`: 16 high, min width 16, radius 4, mono 10, `card-hi` fill, `hair` border.
- Choices and filtering follow `editor.rs`: the commands `/interrupt`, `/stop`, `/profiles`, then `/profile NAME` for
  each profile, kept where `choice[1..]` starts with the query. **There are no debug badges and no bordered buttons.**
- **Flat fallback:** the web card is `surface` 92% with an 18px backdrop blur. Use solid `surface_2` (`#1B1F3F`) and keep
  the `hair` border. Replace the drop shadow with a 1px `bg` line under the bottom edge.

### Profile switcher (header chip → menu)
- Card at top 40, left 10, width min(440, pane − 20), radius 13. Material is as for the "/" menu, with the same flat fallback.
- At the top is a search field: margin 8, padding `7 10`, radius 9, `card` fill, `hair` border; focus draws a seat border
  and a 3px `seat-14` ring.
- **Two kind groups, agents first:**
  - `⏚ ACP AGENTS  N` and `✳ MODELS  N`: label 10 bold caps with a 12px icon, count right-aligned at 70%.
  - Models are ordered by provider (OpenRouter, Anthropic API, subscription, OpenAI, Local, other).
- Row: padding `6 8`, radius 9, gap 9. It holds a 24px tile, the name (650) with an optional `DEFAULT` tag, a sub-line
  in mono 11 `fg-dim`, a spacer, the context size, the auth chip, and a 16px check column.
  - **Model sub-line:** `model · Provider label` (the label in chrome font, 75%).
  - **Agent sub-line:** `$ <agent command> · <model>`; the `$` is in seat.
- **Tiles:** a model gets a **radius-7 square** and an agent gets a **circle**, both 24px.
  - Mark text is 10.5 weight 800, white. The fill is the provider or agent colour.
  - Agent colours: claude-agent-acp `C #D97757`, codex-acp `Cx #10A37F`, opencode `oc #5B6178`, pi-acp `π #FF4FA3`,
    omp `ω #B07BFF`, goose `G #C98A1B`; an unknown command gets its first letter on `#2EE6A8`.
  - Provider colours: OpenRouter `OR #7C6CFF`, Anthropic `A #D97757`, OpenAI `O #10A37F`, Local `L #9AA3C7`.
  - **Flat fallback:** the web tiles have a 135° colour → colour 55%-black gradient and a coloured drop shadow. Use the
    solid colour and no shadow.
- **Auth chip** (10.5 semibold, pill, padding `1 7`):
  - models: `env` / `Keychain` / `no key` in green (`ok-12` fill); `missing env` in red (`bad-12`); `not supported` neutral;
  - agents: `own login`, or `env N` when the agent needs env names. Both are neutral (`card-hi`, `fg-dim`) because the
    harness never sees an agent's credentials.
- Current row: fill `seat` 9% and a seat ✓. Disabled rows (missing key, unsupported) show their tile and name at 50% and
  do not respond to clicks.
- The footer has `+ New profile` in seat and `~/.bboxy450-harness/profiles` in mono 10.5.
- New profile has an `ACP agent` tab with the fields Name, Agent (command) and Model (optional). It previews the
  `provider: acp-agent / agent: command: [...]` frontmatter.

### Tool card, approval, statuses
- Card: radius 11, `card` fill, `hair` border. The header row is 7×10 padding with a 24px kind tile (radius 7, kind colour 16%).
- Status disc (18px):

| Status | Disc | Card border / fill |
|---|---|---|
| done | `ok` disc with a check | default |
| fail | `bad` disc with ✕ | `bad` 40% border, `bad` 6% fill |
| deny | `bad` disc with ✕ | `bad` 40% border, `bad` 6% fill |
| stop (interrupted / cancelled) | `fg-dim` disc with a 9px stop square | default border, **not red** |
| run | three bouncing seat dots | see below |
| ask | `ask` shield | 1px `ask` 60% border |

- **Run:** on web, a seat arc orbits the card border every 1.6s, with a soft seat glow.
  **Flat fallback:** a 1px `seat-55` border, and the three dots stepping opacity 0.5↔1 at 0.15s offsets.
- **Ask:** on web the card glows with a 24px ask shadow. **Flat fallback:** keep the 1px `ask` 60% border, no glow.
- **Approval pending** is inside the tool card:
  1. **a subject line first**: mono 12 on `term-bg`, padding `7 10`, radius 8. It shows `$ command` (or `› call summary`),
     then any paths in `fg-dim`, with the `$` in `ask` bold. Never show only the effect class.
  2. then the ask bar: `ask` 10% fill (flat `#2E2731`), radius 8, "Allow **exec**?", and the buttons
     `Allow 1` (ask fill, `#1A1300` text), `Always 2`, `Deny 3` (`card-hi`; Deny hovers `bad`). Each button shows its key at 10px 60%.
- The composer enters `ask` at the same time (see Composer).

### Turn receipt (`.hx-end`)
- Row with radius 11, padding `8 12`, 11.5 `fg-dim` tabular numbers. It holds an 18px disc, a bold 12.5 title, the
  duration, then `N rounds · N tools · N edits` (zeros omitted), tokens `in · out`, and cost `$`.
- **ok** (`end_turn`): title `Done`, `ok` disc with a check, 1px `ok-25` border. The web fill is an `ok-12` → transparent
  horizontal gradient. **Flat:** `ok` 6% (`#13202F`).
- **soft** (`interrupted`, `cancelled`): titles `Interrupted` / `Cancelled`, `fg-dim` disc with a stop square, `card` fill, `hair` border.
- **bad** (any other stop): `bad` disc with ✕, 1px `bad-25` border. Fill is a gradient on web; **flat:** `bad` 6% (`#211730`).
  - The title says what happened in words. For `error`, map `class`: transient → Connection dropped,
    rate_limited → Rate limited, auth → Not signed in, bad_request → Request refused, malformed → Unreadable reply,
    mid_stream → Reply cut off, config → Profile misconfigured, store → Could not save the session, tool → Tool failed,
    internal → Harness error. Other stops: max_tokens → Hit the output limit, max_rounds → Stopped at the round limit.
  - A full-width message line at 12.5 in `bad` 80%.
  - A full-width action row: **Retry** (`bad` fill, white, retry icon) and **Switch profile** (`card-hi`), which opens
    the profile switcher.
- When a loop was folded, the row adds `↻ N repeated replies folded` in `ask`.

### Runaway loop card (`acp-opencode-loop`)
- **Rule:** in one turn, a reply whose first line matches an earlier reply's first line opens the loop card where that
  reply was. Lines are compared lowercased, letters only, first 48 characters.
- Every later reply, thought, tool call and notice of that turn goes into the card. The first answer stays in the transcript.
- Card: radius 11, border `ask-30`, fill `ask-5`. The header is like a tool header: an ask-coloured `↻` kind tile,
  "The agent kept repeating itself" (650), then `N replies · N thoughts · N tool calls` in 12 `fg-dim`, a `folded` chip,
  and a chevron that rotates 90° when open.
- The open body has a dashed `ask` 25% top border and padding `10 12 12 22`, and shows the folded items in their normal form.

### ACP-agent sessions: fold rules native must match
- Strip one surrounding fence from tool output: ```` ```console\n…\n``` ```` → the inner text.
- `[terminal <id>]` and `(no output)` count as no output. An execute card with no output shows a dim line:
  "output stayed in the agent's own terminal" when the tool was named by its command, or "no output" otherwise.
- Tool naming:
  - `Terminal` with `{command, description}` → **Run** `command`, with the description as a tooltip.
  - A name that is a command line (`ls -la`, kind execute, no input) → **Run** `ls -la`.
  - A phrase (`List files`) → the phrase is the verb and there is no target.
- Reasoning summaries are trimmed. The gist drops `**`, `__` and backticks and stops at the first sentence or line break.
- `usage: null` → no meter and no token figures. The receipt shows duration and counts only.
- A `notice` at warn level (for example "the agent does not offer model …") is a 6px `ask` dot and 11.5 `fg-dim` text,
  never a card.

## Tight transcript (2026-11-03)

Operator: "output can be tighter and tidier, too noisy right now". This applies to every state. Same information, less chrome:

- **Tool head:** `exit N` only when N ≠ 0. No `auto` badge when a rule allowed the call; show `allowed`, `always` or `denied` only
  when a person or the unattended policy decided. The duration only from 1s. Head padding 6/10.
- **Tool body:** never repeat the command the head already names. No `$ cmd` line, and no `exit 0` line.
- **Fold by default:** a completed, successful command opens only when its output is 6 lines or fewer. Running, asking,
  failed and edit cards stay open. Reads stay folded as before. One click opens any of them.
- **Short thoughts (under 1s):** no "Thought" label, just the 12px dim gist line after the spinner icon. The chevron shows on
  hover only. "Thought for N" stays for 1s and over.
- **Receipt:** no card. A hairline rule above one 11.5px line: check mark, title, then the duration (from 1s, or always when the turn did not
  end ok), tools and edits. No rounds count. The failed receipt keeps its red rule, reason and actions.
- **Rhythm:** feed gap 8.

## What never reaches the transcript

Operator, 2026-11-03, on a screenful of "SYSTEM Native · agent_message_chunk · 129 bytes" cards: "this is very verbose".

- **Native records** (`Item::Native`, the raw protocol edge records the log keeps for fidelity) never draw. They feed the log
  and its tools only. The text they carry already arrives as agent message, reasoning and tool items.
- **Usage** draws nothing in the feed (see Refinement pass 2).
- **No generic fallback card.** The role-labelled StreamDock card (`SYSTEM` / `TOOL` + kind + status) is not part of this face.
  Every kind the face does not style draws as one quiet notice line (6px dot, 11.5 `fg-dim`):
  - Command and Hook: `Ran /name`, `Hook name`;
  - SystemMessage: its text;
  - Compaction: a hairline divider reading `Context compacted`;
  - anything unknown: `Unsupported item: <tag>`.
- **Witness:** replaying a log that holds N native chunk records draws the same rows as the same log with them removed.

## Standalone window (`standalone-window`)

The standalone app is the term's window, not an OS window.

- Create the window with `with_decorations(false)`, as `bboxy450-term` `main.rs` does. There is no OS title bar and no system-drawn chrome.
- Draw the term's own title bar, `bbt_ui::modules::title_bar::TitleBar` (web `C.titleBar`):
  - window dots, the bboxy mark, title `bboxy450-harness`, crumb `profile · cwd leaf`, and the theme button;
  - no sidebar toggle.
  - Dragging the bar calls `window.drag_window()`. A double click follows the system preference (Fill/Zoom/Minimize) through
    the `bboxy450-term-desktop` window helpers, as in term `desktop_window.rs`.
- The app menu comes from the term's platform-free `bboxy450-term-desktop-core` menu model and actions, installed through
  `bboxy450-term-desktop` `MenuBar` the way term `desktop_menu.rs` does. Shortcuts dispatch the same actions as the keyboard.
- **Global text zoom:**
  - Cmd+= or Cmd++ zooms in, Cmd+- zooms out, and Cmd+0 resets.
  - The scale runs 90% to 160% in 10% steps. It applies to every text size and line height in the face, and the layout reflows.
  - The scroll position stays anchored to the top visible block.
  - The scale persists in the harness settings.
  - A centred mono pill showing `110%` drops in at the top of the feed (170ms) and fades out after 900ms.

## Markdown everywhere

Every prose body uses the term's renderer, `bbt_doc` (`Markdown` / `StreamingMarkdown`, mermaid figures with Drawing/Source).
Only terminal output stays raw mono.

- Agent messages: already rendered this way.
- Expanded reasoning: markdown at 12.5px in `fg-dim`. While live, it is plain text with a caret.
- Read of a `.md`/`.markdown`/`.mdx` file: rendered markdown with line numbers stripped, in an inset panel
  (radius 8, `hair` border, max-height 420, scrolls).
- Any other tool output that is markdown renders the same way, unless the tool failed. Markdown means a ``` fence, or at least two
  lines that start with a heading, list item, table row or quote.
- Mermaid fences anywhere in these bodies draw as native figures.
- GFM tables draw full width: 12.5px, a `card-hi` header row at weight 650, cell padding 5/10, `hair` rules, radius 8.

## Refinement pass 2 (2026-11-02)

Found by comparing the native capture with the reference on the real ACP and mermaid logs (`parity-web.sh`, harness ca80cde),
and from the operator's screenshot.

- **Usage items never draw a row.** An `item_completed` with body `usage` adds to the header meter and nothing else.
  If the total is 0 in and 0 out, there is no meter. Never draw "0 in · 0 out" or any per-round token line in the feed.
- **The message source toggle is hover-only.** The `source` / `formatted` control on an agent message is hidden at rest.
  On row hover it fades to 0.6 over 150ms `std`, and to 1.0 when the control itself is hovered.
- **The exit status lives in the tool card header only** (`exit N` chip). The terminal body never gets an extra `exit 0` line.
- **Terminal body padding:** 8px top and 10px bottom. Do not add space under the last output line beyond that.
- **Message text rhythm** (13.5px body):
  - paragraphs 0 above, 6 below;
  - headings 15/700, 8 above, 6 below;
  - lists 2 above, 8 below, 20 indent, 2 between items;
  - list markers in `seat`;
  - inline code 0.88em mono, 1/5 padding, radius 5, `card-hi` fill, `hair` border.
- **Mermaid sequence:** lifeline heads 30px tall. Messages are 30px apart, arrows run lifeline to lifeline, and labels sit 4px above the arrow.
  Flowchart back edges route around the nodes, never through a node box.

## Empty session (`empty-session`)

Shown while the session has no blocks. It leaves when the first block mounts.

- Layout: centred column, 12% of the scroll height from the top, gap 8.
- Mark: `bboxy.png` 52px round. A 1px `seat-45` halo ring sits 6px outside it and breathes (see Motion).
- Title: "What are we building?", 17/600 `fg`.
- Sub-line: `profile · model in ~/<cwd leaf>`, 12 `fg-dim`. Omit the parts that are unknown.
- Chips: `/ commands` (opens the "/" menu), `@ a file` (inserts `@`), `Describe a change` (focuses the composer).
  Pill radius, 12px text, padding 5/11, `card` fill and a `hair` border. Hover: `card-hi` fill, `seat-55` border, `fg` text.
- Never draw the old top-left "Send a message to begin." line.

## Motion

Operator, 2026-11-02, on the native app: "almost good, more animation more smoothness and we would be good to go".
Native renders through bbt_ui on the GPU, so **native animates**. The flat column below is only for reduced motion and
for a missed frame budget. It is not the default.

### Clock rules

- One animation clock per window. Any active tween or loop calls `request_redraw` for the next frame.
  When every tween has settled and no loop is visible, stop redrawing. An idle window costs no frames.
- A tween is `(from, to, t0, duration, curve)` keyed by element id. A new target in mid-flight retargets
  from the current value. Never snap back to `from`.
- Every value that changes on screen goes through a tween: opacity, offset, scale, colour, height, scroll position
  and caret position. A value that changes without a row in this table is a defect.
- Curves:
  - `out` = cubic-bezier(.2,.8,.2,1), for entrances and glides;
  - `std` = cubic-bezier(.4,0,.2,1), for colour and state changes;
  - `pop` = cubic-bezier(.34,1.56,.64,1), for overshoot.
- Reduced motion (the OS setting, or `BBOXY_REDUCED_MOTION=1`) draws end states and keeps only the caret blink.

### Table

| Motion | Duration · curve | What moves | Reduced / flat |
|---|---|---|---|
| block enters | 320ms `out` | rise 6px, opacity 0 → 1, scale .995 → 1 | end state |
| streaming text | 140ms `out` per chunk | each new chunk's glyphs fade 0 → 1; settled text never re-fades | end state |
| tail follow | critically damped, ~180ms | scroll offset glides to the bottom; a new chunk retargets mid-glide. It stops following when the user scrolls up more than 40px | jump |
| block grows | 200ms `out` | a card's height tweens as content arrives or folds, and blocks below it slide | jump |
| fold / unfold | 200ms `out` | body height plus opacity; the chevron rotates 0 → 90° | jump |
| empty session | 420ms `out`, chips staggered 60ms | rise 6px plus fade; the halo breathes 3.2s ease-in-out (opacity .25–.8, scale .96–1.04) | static ring at .5 |
| composer focus | 200ms `std` | border `hair` → `seat`, ring opacity 0 → 1 | end state |
| composer height | 160ms `out` | the box grows and shrinks with its lines; the feed above shifts at the same time | jump |
| send button | 200ms `std` | fill and icon colour across idle → ready → stop; the icon crossfades arrow ↔ square | end state |
| send press | 120ms `std` | scale .92 and back | none |
| caret | 80ms `out` move, 1s step blink | slides to the new column instead of jumping; it stays solid while typing and blinks after 500ms idle | blink only |
| menu open / close | 170ms `out` / 120ms `std` | open: drop 6px, scale .98 → 1, fade in. Close: fade out | appear / vanish |
| menu selection | 120ms `std` | the `sel` highlight slides between rows, never jumps | jump |
| hover | 150ms `std` | row, chip and button fills | end state |
| header chip change | 200ms `std` | profile/model text crossfades; the width tweens | end state |
| phase change | 200ms `std` | the phase label crossfades and the dot colour tweens | end state |
| status disc | 350ms `pop` | scale .2 → 1 | appear |
| running sweep | 1.6s ease-in-out, loop | a 40%-wide `seat` highlight crosses the composer top edge | static `seat-55` line |
| tool run border | 1.6s linear, loop | conic orbit around the running tool card | static `seat-55` border |
| thinking | 1.4s, loop | sheen across the "Thinking" label; three dots bounce, staggered 160ms | static label |
| phase dot / ask badge | 1.1–1.2s pulse | opacity 1 → .35 → 1 | steady |
| receipt | 320ms `out` | the receipt line rises in; numbers count up over 400ms | end state |

The page plays block enter, tail follow, empty session, menus, hover, send, sweep, orbit and thinking. Streaming-chunk
fade, caret glide, height tweens and the sliding menu selection are specified here only; build them from this table.

### Frame budget

Hold 60fps (16.6ms) at 1000 blocks. Animate only what is visible. Offscreen blocks take their end state. Loops pause
while the window is occluded. If two frames in a row miss the budget, fall back to the flat column until the next idle.
