---
id: 01M5NVBXDFNFW3ZNYSVGER868P
number: 219
type: comment
title: "define the env allowlist and the working directory before P-0"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Protocol"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# define the env allowlist and the working directory before P-0

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

"The plugin runs with a scrubbed environment, an allowlist" names no members, and the spawn's working directory is not specified. Both decide what a trusted plugin reads without asking. If a global plugin is spawned with the cwd inside a repository, a trusted binary reads attacker-chosen config from that repository (`.notionrc`, `.env`). The trust is in the code, but its inputs come from the repo.

**Ask.** In P-0's `bbt-plugin`: the allowlist as data (for example `PATH, HOME, LANG, LC_*, TMPDIR, TERM` and nothing matching `*TOKEN*`, `*KEY*`, `*SECRET*`, `BBOXY_*`, `ICQ_*`), plus a per-plugin data directory as the cwd. A repository path reaches the plugin only as an explicit argument of a call, never through cwd or env.
