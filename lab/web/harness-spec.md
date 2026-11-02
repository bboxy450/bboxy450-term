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
- **Running sweep:** a 2px line on the box's top edge. On web it is a 40%-wide seat highlight that slides left to right
  over 1.6s ease-in-out and repeats.
  **Flat fallback:** a 2px solid seat bar, 40% of the box width, moved by x-offset each frame. Without per-frame paint,
  use a static 2px `seat-55` line across the full top edge.
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

## Motion

Every motion has a still end state. Under reduced motion, or when native cannot animate, draw the end state.

| Motion | Web | Native flat |
|---|---|---|
| block enters | 0.32s rise 6px + fade, cubic-bezier(.2,.8,.2,1) | fade 0.2s, or none |
| menu opens | 0.16–0.18s lift/drop 6px + scale .98 → 1 | appear at once |
| status disc pops | 0.35s scale .2 → 1, overshoot | appear at once |
| running sweep | 1.6s ease-in-out, loop | stepped bar, or a static `seat-55` line |
| tool run border | conic orbit 1.6s linear | static `seat-55` border |
| phase dot / ask badge | opacity pulse 1.1–1.2s | steady |
| send press | scale .92 at 0.12s | none |
| caret | 1s step blink | 1s step blink |
