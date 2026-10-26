---
id: 01M5SPS059GFR8T3K5E9BZK1J3
type: decision
number: 320
title: "ctl and GUI dispatch is organised by noun: one module per verb family with its own request, reply and apply, registered from a table, so an additive cut touches its own file and no peer rebases for it"
status: decided
decided: "2026-10-25"
operator: []
links:
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[d272-ctl-one-grammar]]", rel: builds-on }
  - { to: "[[d260-ctl-authority]]", rel: builds-on }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-25T14:17:33Z"
updated: "2026-10-26T01:34:22Z"
---
# Decision 320: dispatch by noun, one module per verb family

Ruled at the Oracle's ask on the second day of `ctl`. Every cut that adds a verb today edits the same few files: the `UiRequest`, `GuiAction` and `GuiReply` enums in `bbt-proto/src/gui.rs` (lines 91, 308, 586), the `VERBS` table and dispatch in `bboxy450-term/src/ctl.rs` (348, 491), `apply_ui` in `main.rs` (4237) and its headless twin, and `gui_capture.rs` in the mux. Each landing therefore forces two or three peers to rebase and re-check: a merge-contention chokepoint, the eighth instance of [[no-chokepoints]], on the swarm side with a product-shaped cause.

**Alternatives.** (a) Leave the shape and accept the rebases: the cost grows with the number of concurrent cuts, which is the thing we are increasing. (b) **One module per noun.** Each verb family (`seat`, `ui`, `app`, `debug`, `events`, `record`, `task`, `identity`, …) owns a module with its own request and reply types and its own `apply`; the top-level enums are flat and assembled from the nouns' variant lists (see Mechanism; the nested form `UiRequest::Seat(seat::Request)` was the first wording and is withdrawn), the `ctl` verb table is the concatenation of per-noun tables, and the window and the headless client dispatch by noun once and hand the request to the module. Adding a verb then touches one noun module and its table row, and nothing a peer working on another noun is editing. (c) A runtime registry of handlers: the same separation with dynamic dispatch and a second place where a verb can be missing at runtime.

**Choice.** (b), realised as flat enums assembled from per-noun variant lists, not as nested enums.

**Mechanism (Seraph's cut-1 question, 2026-10-25).** As first written, (b) said `UiRequest::Seat(seat::Request)`: a variant per noun wrapping the noun's own type. On main that would rewrite every call site in one cut, 342 uses of `GuiAction::` and 420 of `GuiReply::` across about 35 files in the app, the mux and their tests, plus hand-written serde routing: the contention this decision removes, concentrated into a single landing. Ruled instead: **each noun module declares its variants in one `macro_rules!` list** (doc comments and serde attributes passing through), and **one chaining macro in `gui/mod.rs` concatenates the lists into the flat `GuiAction` and `GuiReply`** with the same derives. Wire bytes are identical by construction, so the golden test of cut 0 passes byte for byte with no routing code; zero call sites change; adding a verb touches only its noun's file, which was the reason. Per-noun dispatch for the later cuts comes from a generated `noun()` method, and each noun's `apply` matches its own variants. Costs, accepted: about forty lines of macro; rustdoc and go-to-definition land on the macro list rather than a plain enum; a noun's `apply` is not exhaustive across nouns, so the flat top-level match keeps the compile-time exhaustiveness and is the one place that must stay exhaustive.

**Reason.** The wire does not change: §2 of [[ctl-cli]] still has one reply shape and `data` is still the window action's reply unchanged, because the noun's reply type is what travels. What changes is which file a cut edits, and that is exactly the contention. (c) buys nothing over (b) in a program whose verbs are known at compile time and loses the exhaustiveness the compiler gives a closed enum. The mantra is the one about flexibility: the structure should let ten cuts land without ten rebases.

**Rules.** (1) New verbs land in their noun's module; a cut that adds a variant to a top-level enum or a row to a shared table outside its noun's file is asking for the migration, not around it. (2) The migration itself is one cut per dispatcher, mechanical, no behaviour change, its gate the existing ctl and headless suites unchanged. (3) The order is the Oracle's; the first candidate is the file the most concurrent cuts touch today. (4) **The gate (Oracle, same day):** the golden test of cut 0 asserting byte-identical wire for every action and reply, plus a check that each variant name appears in exactly one noun list; both run from the cut that introduces the lists, and until then whether a cut stayed inside its noun is read at review. **Landed (Seraph, cut 1, 051f7e24):** `crates/bbt-proto/src/gui/` holds one module per noun (`app`, `debug`, `events`, `host`, `identity`, `message`, `record`, `seat`, `task`, `ui`), `gui/mod.rs`'s `flat_enum!` splices their lists into the flat `GuiAction` and `GuiReply`, and the golden is `crates/bbt-proto/tests/gui_wire_golden.rs`. The one-list-per-variant check needs no script: a name in two lists is a duplicate variant in one enum and the build fails. The gate is therefore the build plus the golden, both already in every presubmit.

**Complete (Seraph, 2026-10-25), eight landings each gated alone:** 0 the golden (d8f1df46), 1 the proto `gui/` by noun (58bb2c27), 2a the verb tables, 2b the command type, 2c parsing (e6ff3d28), 2d run (34ba72bf), 2e `run_verb` (c00be793), 2f the parse samples (c2d783d7, through `land.sh`, its presubmit reused on a disjoint move). A new `ctl` verb now edits only its noun's `ctl/<noun>.rs` (its table row, its command variant, its parse arm, its run arm or the flat reply default, its parse sample); a new action or reply edits only `gui/<noun>.rs`. The gates in the tree: `tests/gui_wire_golden.rs` (wire byte-identical, every kind listed), `each_verb_is_listed_once_in_its_own_nouns_table`, a per-verb sample test asserting the verb's noun, and the compiler's duplicate-variant error for a variant in two lists. The first verb to use the layout is MA-6's `seat attention`, in `ctl/seat.rs` alone.

Owner: Corvin Duskmantle <ARCHITECT>.
