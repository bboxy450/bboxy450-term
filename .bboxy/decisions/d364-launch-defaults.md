---
id: 01M5XPZNG0VW1MYGR9VAWZ54DB
type: decision
number: 364
title: "spawn parameters: ↵ launches as is, → shows them prefilled (role from Settings, the rest the last choice); transports are plugins"
status: decided
decided: 2026-10-26
operator:
  - "split -> codex -> defaults if i just press enter, or i can press -> (right arrow) and edit defaults, that can be part of cmd+T dialog, can be editable things!"
  - "we should also have defaults per swarm, so each swarm has default pwd that can be changed for example"
  - "think about my manual process, i open new shell, cd ....., codex, or open new shell, ssh bla bla; cd bla bla; codex, and sometimes ssh is different maybe new network, or maybe i want to use mosh or something"
  - "we can just select or change defaults or they just go as is, i love this"
  - "here i shoud see role -> select (prefilled with default from settings), pwd -> select (prefilled with last choice)"
  - "thats the model of harness spawn customization"
links:
  - { to: "[[launch-defaults]]", rel: decides }
  - { to: "[[ux-prototype]]", rel: constrains }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[plugins]]", rel: constrains }
created: 2026-10-27T03:38:09Z
updated: 2026-10-27T03:38:09Z
---
# Decision 364: launch defaults

Decided 2026-10-26. Spec [[launch-defaults]].

**What.** In the ⌘T launcher, ↵ on a launch row launches it as is. → opens its spawn parameters as one more level of the existing nesting: one row per parameter (role, via, connect, port, key, directory, command, arguments, environment), every row prefilled, the role from Settings and the rest with the last choice, and under them the exact command line it runs. ↵ on any row launches; → or a click changes one. A launch remembers its choices at each parameter's home: swarm (directory, the existing swarm working folder; environment), harness (command, arguments), machine (via, route, port, key, remote directory). The role is a setting, not remembered.

**Why.** It is the operator's manual process (new shell, ssh, cd, codex) with the typing removed and nothing hidden; remote machines differ by network, so a machine keeps routes; swarms differ by project, so the directory lives on the swarm; the role is a policy, so it comes from Settings.

Rejected: a form of input fields (the operator: "just more options, next to role"; the list style is the launcher's own); a save key (last choice is remembered on launch, less to learn); a separate settings dialog before launch (an extra stop on the common path, where ↵ must stay one key); defaults stored per launcher row (a second copy of Settings records); built-in transports only (mosh, et and future tunnels are exactly what plugins are for).

Owner: Vesper Greywind <ORACLE>.
