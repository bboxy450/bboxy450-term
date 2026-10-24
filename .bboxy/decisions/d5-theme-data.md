---
id: "01M5JFXASG4VAB3PJJ87CTPY5R"
type: "decision"
number: 5
title: "Theme is data"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#5]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 5: Theme is data

**Theme is data.** One source, `design/tokens.toml`, becomes a Theme value at
startup (hot-reload in the lab), passed by reference to layout and draw.
bbt-ui holds a neutral default for tests; the bboxy theme is an asset of the
app. Rejected: compile-time consts; trait object.
