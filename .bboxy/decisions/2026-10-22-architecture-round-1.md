---
id: 01M5JFWDVF32QNCDMXC9K0T49T
number: 131
type: note
title: architecture decisions, round 1 (operator + Architect, 2026-10-22)
links:
  - { to: "[[vision]]", rel: direction }
  - { to: "[[open-source]]", rel: changes }
  - { to: "[[layering]]", rel: invariant }
  - { to: "[[2026-10-22-c-cockpit]]", rel: epic }
created: 2026-10-22T19:02:24Z
updated: 2026-10-23T23:56:02Z
---
# Architecture decisions, round 1

**This file is the index.** From decision 47 on, every decision is its own node, `d<N>-<slug>.md` in this folder ([[d47-decision-nodes]], operator 2026-10-23); entries 1 to 46 move out in a sweep that leaves a one-line pointer under an anchor per number here, so every old link keeps landing.

Asked by Corvin Duskmantle <ARCHITECT>, answered by the operator on 2026-10-22,
from the six items Vesper Greywind <ORACLE> flagged as needing an architect.
Design principles restated by the operator the same day: **LESS IS MORE, KISS,
super good enough, not perfect.** Boundaries and crates may be imperfect; the
gate catches direction, not purity.
Each decision: chosen, rejected, why. Specs change on next touch; the Oracle
briefs Executors from the specs, never from this note.

## 1

1. [[#1]] — [[d1-crate-topology]]: Crate topology: split by process boundary.

## 2

2. [[#2]] — [[d2-c1-io-binding]]: C1 binding: app IO thread + double buffer.

## 3

3. [[#3]] — [[d3-mcp-server]]: MCP server inside logic, stdio bridge per seat.

## 4

4. [[#4]] — [[d4-seat-identity]]: Seat identity: full GPG chain in v0.1.

## 5

5. [[#5]] — [[d5-theme-data]]: Theme is data.

## 6

6. [[#6]] — [[d6-shared-protocol]]: Domain types go to a shared protocol crate.

## 7

7. [[#7]] — [[d7-provider-supervisor-link]]: Provider yields a supervisor link.

## 8

8. [[#8]] — [[d8-open-source]]: Everything goes open-source eventually.

## 9

9. [[#9]] — [[d9-layering-gate]]: Layering gate now.

## 10

10. [[#10]] — [[d10-thousand-seat-gates]]: 1000 seats: four measured gates before C1 ships.

## 11

11. [[#11]] — [[d11-supervisor-death-survivable]]: Supervisor death is survivable, not prevented.

## 12

12. [[#12]] — [[d12-audience-and-plugins]]: Audience and the plugin seam (operator).

## 13

13. [[#13]] — [[d13-document-types]]: Document types: the seven in work-items plus decision.

## 14

14. [[#14]] — [[d14-native-document-viewer]]: The app reads its own .bboxy: the viewer is cockpit work (operator).

## 15

15. [[#15]] — [[d15-local-ci]]: CI is local (operator, 2026-10-22).

## 16

16. [[#16]] — [[d16-task-invariants]]: The Architect's four loads bind the task system (operator: "embody it", 2026-10-22; ~/.agents/AGENTS.md Four roles).

## 17

17. [[#17]] — [[d17-device-key-custody]]: Device-key custody has three tiers (Architect, from ID1a evidence).

## 18

18. [[#18]] — [[d18-executor-harness-override]]: Claude Code may be an Executor when the operator launches it so (operator, 2026-10-22: "i gave you one more executor that is claude code").

## 19

19. [[#19]] — [[d19-pty-ceiling]]: The PTY ceiling is a product fact, not a bug (from Seraph's C1d measurement).

## 20

20. [[#20]] — [[d20-seats-and-jobs]]: A seat is a PTY; a pipe-mode harness run is a job (from the measured PTY cap).

## 21

21. [[#21]] — [[d21-ring-durability]]: Ring durability has two tiers (from the C1d measurement: logic dies at ~45 seats).

## 22

22. [[#22]] — [[d22-tick-journal]]: One tick journal per machine, not barrier threads (from Rook's measurement: cost is per file, two files per seat per tick; Busy at seat 58 after group commit).

## 23

23. [[#23]] — [[d23-recipient-batching]]: Deliveries batch per recipient (from Seraph's gate 2: a 30-message burst to 1000 seats took about 120 s because the pump moves 32 deliveries per 100 ms with an inbox batch of one, 30 000 deliveries).

## 24

24. [[#24]] — [[d24-defer-code-signing]]: No code signing until we distribute (operator, 2026-10-22; velocity first).

## 25

25. [[#25]] — [[d25-fake-provider-latency]]: The fake provider delays events, not requests (from D1b).

## 26

26. [[#26]] — [[d26-terminal-key-ownership]]: Terminals own every plain key (2026-10-23, Oracle finding in gui-components §4.4).

## 27

27. [[#27]] — [[d27-close-and-history-wire]]: Two C1c-2 wire rulings (2026-10-23, Oracle).

## 28

28. [[#28]] — [[d28-structured-task-verification]]: verify_result is structured and task_accept checks it field by field (2026-10-23, Oracle, W2.5d).

## 29

29. [[#29]] — [[d29-explicit-seat-placement]]: Explicit seat placement on the GUI wire (2026-10-23, Nyx's read-only seam proposal, accepted with two simplifications).

## 30

30. [[#30]] — [[d30-artifact-provenance-scope]]: Three W2.6 rulings (2026-10-23, Oracle, recorded in provenance and events).

## 31

31. [[#31]] — [[d31-device-bound-operator-journal]]: GUI operations are journaled under an operator principal bound to the device, in the account object (2026-10-23; unblocks L8 D / decision [[d29-explicit-seat-placement|#29]]).

## 32

32. [[#32]] — [[d32-kitty-keyboard]]: The kitty keyboard protocol is a core terminal invariant (operator, 2026-10-23, verbatim: "also for terminals we should adotp Kitty keyboard protocol!", "document that as a core invariant of our terminal implementation").

## 33

33. [[#33]] — [[d33-yarn-macro]]: Vendor the operator's reactor! macro as bbt-yarn (operator, 2026-10-23: "if yes just create this as bboxy450-crate, name it somehow sexy").

## 34

34. [[#34]] — [[d34-licensing]]: Licensing: dual MIT OR Apache-2.0 by default, bbt-core Apache-2.0 only (operator, 2026-10-23: "lets do dual license, be smart about this, ask architect as well what project will have what license").

## 35

35. [[#35]] — [[d35-alive-rim]]: The pane glow is the alive rim inside the border (operator, 2026-10-23, verbatim: "I ACTUALLY LOVE RIM EFFECT", "8px alive rim is the thing we want", "go with it").

## 36

36. [[#36]] — [[d36-apache-code-placement]]: Code adapted from Apache-only sources lives in bbt-core, never in a dual crate (2026-10-23; from LIC1 and K1).

## 37

37. [[#37]] — [[d37-gui-messaging-tasks]]: Messaging and tasks reach the app through two wrapper variants on the GUI wire and one events lane (2026-10-23, Oracle's ask for L9-M).

## 38

38. [[#38]] — [[d38-messaging-task-rulings]]: L9-M implementation rulings (2026-10-23; Kestrel's five, Nyx's four).

## 39

39. [[#39]] — [[d39-suite-is-the-product]]: The suite is the product; the terminal is its first app (operator, 2026-10-23, verbatim: "all those libraries are reusable parts, all those crates are reusable part of #bboxy450-* suite, term is just a way we glue them together, later on we will build more apps using our GUI kit that all connects together!").

## 40

40. [[#40]] — [[d40-scrollback-bounds]]: Scrollback replay bounds (2026-10-23, C1c-3, Sable's two questions).

## 41

41. [[#41]] — [[d41-plugins]]: Plugins: planned and logged, not built (operator, 2026-10-23, verbatim in plugins §0).

## 42

42. [[#42]] — [[d42-focus-swell]]: The focus swell replaces the locate ripple (operator, 2026-10-23, verbatim via Wren: "can we change focus circle effert to just using silk border glow, and pulsing it to say 40pxs and then back, you know? without changing spacing between blocks, as an indication of me focusing it", "40px swell is fucking perfect!").

## 43

43. [[#43]] — [[d43-kit-alive-borders]]: Alive borders are a kit capability (operator, 2026-10-23, verbatim: "each block in our UI GPUGUI system should have ability to define this type of dynamic border and border swell and such! thats important, our blocks should be sexy out of the box!").

## 44

44. [[#44]] — [[d44-waiting-attention]]: Attention: a waiting seat swells, pulses and notifies; the notification focuses it (operator, 2026-10-23, verbatim: "if harness has a question for the user - its name in the tab left bar and its borde should autoswell and pulsate a lot, and user should get desktop notification that this harness needs attention - and clicking on that notification would focus the harness terminal").

## 45

45. [[#45]] — [[d45-desktop-notifications]]: Desktop notifications are a facade over one crate per OS (operator via the Oracle, 2026-10-23: an OS-agnostic facade bboxy450-desktop-notifications with per-OS sub-crates, macOS first and real, Windows and Linux dummies marked "DO RESEARCH BEFORE IMPLEMENTING").

## 46

46. [[#46]] — [[d46-cross-os-facades]]: Everything cross-OS is a facade over per-OS crates (operator, same message: "as a GENERAL RULE, everything cross-OS is done this way, with an eye on eventually running in wasm, on iPad and on Android", "we are a pure GPU rendering library", "unless an existing crate already does this well enough to use").

## 47

47. [[#47]] — [[d47-decision-nodes]]: one file per decision; records are objects, the git tree is the first store (operator, 2026-10-23).

## 48

48. [[#48]] — [[d48-every-record-has-a-number]]: every record carries a number, one global sequence continuing from the decisions (operator, 2026-10-23: "everything needs to have a # to it").

## 49

49. [[#49]] — [[d49-swarm-canvas]]: the swarm canvas: an endless zoom view of every graph, structured like the Oracle's board, drawn from the store; logged for later (operator, 2026-10-23).

## 50

50. [[#50]] — [[d50-licence-when-mit-when-apache]]: when MIT, when Apache: dual for what we own, Apache alone where upstream forces it, never copyleft; identity is not licensed (operator, 2026-10-23).

## 51

51. [[#51]] — [[d51-frontmatter-is-the-gui-kit]]: the GUI kit publishes as bboxy450-frontmatter; internal names stay until the first release.

## 52

52. [[#52]] — [[d52-brand-and-face]]: everything is branded bboxy450-, the fox is the face everywhere, records carry images by wikilink; two checks.

## 53

53. [[#53]] — [[d53-evolve-and-restart]]: the two first mantras: be flexible and always evolve; a restart loses nothing, state lives in frontmatter.

## Specs to touch (Architect, in this order)

[[architecture]] drawn from the code ✔ → [[c1-live-seats]] ✔ (2, 6, 10, 11) → [[w2-5-mcp-and-seat-wiring]] ✔ (3) → [[d1-providers]] ✔ (7) → [[id1-seat-chain]] ✔ (4) → [[mux-daemon]] crate
split per 1 → [[open-source]] §1 per 8.

## Operator action pending

`.bboxy/runs/osc7501-codex*.bin` are empty: Codex status is proven by fixtures
only until the trust chooser is authorised in Codex's own pane.
