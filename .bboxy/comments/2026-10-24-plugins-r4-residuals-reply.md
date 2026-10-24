---
id: 01M5NVTYNA5NFMAJC18JFR91H7
type: comment
number: 251
title: "reply: revision 4 inconsistencies"
status: resolved
author: name:Corvin key:6b172639511c3998
on: "[[plugins]]"
on_digest: "sha256:a1b592b7e34c42b63f7acc007ff99610b9b1263c3deace7aca4fa75a08bd65bb"
anchor: "revision 4a"
kind: approval
links:
  - { to: "[[plugins]]", rel: on }
  - { to: "[[2026-10-24-plugins-r4-residuals]]", rel: resolves }
created: "2026-10-24T02:29:01Z"
updated: "2026-10-24T02:29:01Z"
---
# reply: revision 4 inconsistencies

All four accepted into [[plugins]] (digest above): (1) the resource-link read is the one plugin file the app reads, scoped to a `realpath` inside the plugin's data directory, opened `O_NOFOLLOW`, capped at 64 MiB, anything else `resource_outside_data_dir`, and the "never reads" sentence now says so; (2) the diagram says an MCP server over stdio, fixture updated; (3) §1 Isolation says resource links and cursor pages; (4) §3 keeps only P-0's written acceptance and defers P-1 to P-4. Author grade inferred: written outside a cockpit seat, machine-key commit only ([[work-items]] RFC loop).
