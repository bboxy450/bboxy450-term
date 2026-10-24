---
id: "01M5N23RM0C6NNJCN2MPY8RXMQ"
type: "decision"
number: 39
title: "The suite is the product; the terminal is its first app"
status: "decided"
decided: "2026-10-23"
operator:
  - "all those libraries are reusable parts, all those crates are reusable part of #bboxy450-* suite, term is just a way we glue them together, later on we will build more apps using our GUI kit that all connects together!"
links:
  - { to: "[[2026-10-22-architecture-round-1#39]]", rel: "supersedes-as-index" }
  - { to: "[[layering]]", rel: "constrains" }
created: "2026-10-23T18:59:28Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 39: The suite is the product; the terminal is its first app

**The suite is the product; the terminal is its first app (operator,
2026-10-23, verbatim: "all those libraries are reusable parts, all those
crates are reusable part of #bboxy450-* suite, term is just a way we
glue them together, later on we will build more apps using our GUI kit
that all connects together!").** Consequences: the app crate is glue
(cockpit, live binding, main) and nothing else; when a module in the
app could serve a second app it moves down a layer in the same brief;
the kit is `rt`, `yarn`, `ui-core`, `render`, `ui`, `doc`, `mermaid`,
`term-core`, `pty`; the connective tissue between apps is `proto`, the
bus and messaging under the same identity chain, so a second app is
another client of a logic process, or its own logic on the same bus,
never a fork of the terminal. [[layering]] is the enforcement that
already exists; the app's line count against the crates is an advisory
drift signal, read at each architecture round, not a gate. Rejected: a
monolithic app with internal modules (every later app re-extracts
them); a plugin host as the suite's shape now ([[d12-audience-and-plugins|#12]] keeps plugins for
providers, later).
