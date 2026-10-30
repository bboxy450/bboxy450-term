---
id: "01M5XPZNF7E38KE7Q5V1GAAPQA"
type: "spec"
number: 363
title: "Spawn parameters — ↵ launches as is, → shows them, every row prefilled"
created: "2026-10-27T03:38:09Z"
updated: "2026-10-27T21:54:45Z"
---
# Spawn parameters — ↵ launches as is, → shows them, every row prefilled

2026-10-26 · Vesper Greywind <ORACLE> · decided with the operator ([[d364-launch-defaults]]).

[[ux-prototype]] §5.4 · [[ux-flow]] · [[gui-components]] · [[c1-live-seats]] · [[plugins]] · [[networking]] · [[cloud-and-mosh]]

## 0. The operator's process, verbatim

> "i open new shell, cd ....., codex, or open new shell, ssh bla bla; cd bla bla; codex, and sometimes ssh is different maybe new network, or maybe i want to use mosh or something"

> "split -> codex -> defaults if i just press enter, or i can press -> (right arrow) and edit defaults, that can be part of cmd+T dialog, can be editable things!"

> "we should also have defaults per swarm, so each swarm has default pwd that can be changed for example"

> "we can just select or change defaults or they just go as is"

> "here i shoud see role -> select (prefilled with default from settings), pwd -> select (prefilled with last choice)" · "thats the model of harness spawn customization"

## 1. A launch is the manual sequence, written down

```
via (local | ssh | mosh | et | cloud | a transport plugin) → connect (route, port, key) → cd (directory) → command (harness, role, arguments, environment)
```

Every launch from ⌘T resolves to this chain, and the launcher never hides it: under the parameters it shows **the configured command line** (`ssh -t -i ~/.ssh/id_ed25519 feral@rig.local 'cd ~/src && codex --sandbox workspace-write'`), updated as a parameter changes. What the operator would have typed is what runs. The operator clarified on 2026-10-27: "Show configured command as in web preview". Runs includes the configured executable, user arguments, directory and environment; generated per-seat MCP and hook flags added by the harness adapter are not displayed.

## 2. The model: one more level of the existing launcher

The launcher keeps its nesting (`Split right ›`, `SSH › rig ›`, `Codex › Split right`). On a row that would launch:

- **↵ launches it as is**, with every parameter at its prefilled value. The common path stays one key.
- **→ opens its spawn parameters**, one row each, every row prefilled, in the same list style as every other level (crumb `Split right › Claude ›`):

| Parameter | Prefilled with | → offers |
|---|---|---|
| Role | the harness's default **from Settings** (Claude: oracle; Codex and others: executor, fixed, decision #262) | the role list, each with its one-line purpose |
| Via (remote) | the last choice for the machine | `ssh`, `mosh`, `et`, transport plugins (§4) |
| Connect (remote) | the last route used for the machine | the machine's routes, one per network (`feral@rig.local`, `feral@rig.tail`); typing a new one takes it |
| Port, Key (remote) | the last choice for the machine | typed in place |
| Directory | **the last choice**: the swarm's working folder locally (`set_swarm_workdir`), the machine's last directory remotely | recent directories, most recent first; typing a path takes it |
| Command, Arguments | the last choice for the harness | typed in place |
| Environment | the last choice for the swarm | typed in place (`KEY=value …`) |

- **↵ on any parameter row launches** with all of them as shown. **→ or a click** changes one; ↵ in a value list takes the value and returns; esc keeps the old value; ← goes back.
- Placement presets (⌘D, ⌘⇧D) drop the placement sublist, so `Split right › codex` is itself the launch row; Claude's role is a parameter, not a separate level.

## 3. Last choice, remembered on launch

There is no save key. A launch remembers what it used, each parameter at its **home**: Directory and Environment on the **swarm** (the existing swarm working folder, plus a short recent list), Command and Arguments on the **harness** (Settings › Harnesses), Via, Connect, Port, Key and the remote Directory on the **machine** (Settings › Machines; a new route joins its routes). The **Role is not remembered**: its default is a setting, changed in Settings. Settings shows and edits the same records; the launcher never keeps a copy.

## 4. Transports are the extension point

A transport turns `(fields, inner command)` into argv. `ssh`, `mosh` and `et` are built in; anything else is a plugin fulfilling a new **`transport` contract** ([[plugins]] §1): its shape declares its fields (name, label, kind, default) and a `wrap` method returning argv. It is named `bboxy450-term-transport-<name>` (decision 362). A transport plugin never sees credentials it did not declare.

## 5. The spawn carries what the form resolved

`SpawnSpec` ([[c1-live-seats]]) gains, additively and `#[serde(default)]`: `args: Vec<String>`, `env: Vec<(String, String)>`, `via: Option<TransportSpec>` (`kind`, `route`, `port`, `key`, plugin fields). `cwd` keeps its meaning (the directory after `cd`, local or remote). Resolution happens once, at the pick, in the GUI; logic receives the resolved chain and runs it, never re-resolving from layers that may have changed since.

## 6. Verification

| Requirement | Evidence |
|---|---|
| ↵ launches as is; → opens the parameters; → on a parameter changes it; ↵ on a parameter launches; last choices are remembered at their homes, the role is not | e2e row in [[e2e-acceptance]] (launcher group), driven by `ctl ui key`; unit tests on the resolver |
| The shown command line matches the configured launch | test: compose(fields) equals the directory, environment, executable and user argv supplied to the harness adapter; generated MCP/hook flags are verified separately |
| Prefill sources and homes | resolver unit test over a table of (harness, local or remote, swarm, machine) cases |
| Old spawns still decode | `SpawnSpec` without the new fields round-trips (serde default) |
| Preview | `lab/web` picker: → on a launch row (components.js `launchFields`, `composeLaunch`); scripted flow ⌘D → claude → → → Role › Architect → Directory typed → ↵ |

## Remote cut 2 implementation — 2026-10-30

`SpawnSpec.via` carries the resolved ssh/mosh/et transport (absent for old/local
spawns). These are local PTY clients owned by the existing local supervisor;
`cwd` remains the remote directory and the client's local cwd is its private
launch home. Settings `[machines."ssh:<host>"]` owns Via, Connect, SSH bootstrap
Port, Key, remote Directory, routes and recent remote directories. Native
Settings and the launcher edit the same records; a successful spawn remembers
the captured values. A remote launch never replaces the swarm's local folder.
Remote `~` is expanded by the remote shell. Key paths expand on the local host.

The pure composer is shared by Runs and logic. SSH uses `-t`; mosh passes port
and key through `--ssh`; et uses `--ssh-option` and `--command` long flags.
Remote harnesses receive the configured command/arguments/environment, without
local-only MCP sockets, app-server companions or generated local hook paths.
Transport plugins (cut 3), remote MCP installation and real-server validation
remain outside this cut. `make test-remote-launch` uses private fake executables
and the real Settings store without networking or operator input. The
TransportCapture model and broken control cover capture/remember/restart safety;
exact CLI bytes remain implementation-tested.

Saved machine records also supply the native Machines and launcher catalogues
when no remote provider snapshot exists. Add a machine accepts a host or
user@host route; availability still filters its launcher row. No connected
provider state is fabricated or required before attempting the local client.
