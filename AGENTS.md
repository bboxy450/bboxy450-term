# bboxy450-term: working instructions

Read `~/.agents/AGENTS.md` first. Codex is an Executor. A direct operator request is the brief; implement its scope in your own worktree.

## Use the Makefile

**The Makefile is the canonical interface for building, tests, installation and release work. Run `make help`. Do not invent Cargo/codesign/copy/publish sequences or keep a second installation recipe in a script or handoff. Add a missing workflow to Make.**

| Command | Result |
|---|---|
| `make build` | Debug app |
| `make build-release` | Optimized app |
| `make test` | Full local CI, with exact source/result lines in `target/ci-local.log` |
| `make test-parity` | Full local CI plus the existing visual parity gate |
| `make test-boundaries` | Explicit shell PATH and real process/lifecycle regressions |
| `make test-swarm` | Swarm working folder, override, restart and quit-key regressions |
| `make smoke` | Private unsigned install, throwaway identity, headless smoke; no operator |
| `make install` | Release app signed with the approved Apple Development identity, installed in `~/Applications` |
| `make install-unsigned` | Ad-hoc-only debug app in `target/install-unsigned`, for tests; never replaces the trusted app by default |
| `make release` | Identity-signed macOS ZIP, SHA256 and detached GPG signature |
| `make package-crate CRATE=name` | Package/sign one crate without publishing |
| `make publish-release` | Signed tag and GitHub app release; only on an authorized release brief |
| `make push` | Verify the exact-HEAD CI result and signed commits, then fast-forward GitHub main; only when push is authorized |

Overrides live in `make help`: `PROFILE`, `TARGET_DIR`, `INSTALL_DIR`, `UNSIGNED_INSTALL_DIR`, `CI_LOG`. Build caches are selected by `scripts/cache-env.sh`; Make sources it. Keep two Cargo jobs and a worktree-local target. Never change global Cargo configuration for a task.

Signed installation never silently falls back to unsigned. `install-unsigned` uses an ad-hoc executable signature needed by arm64 macOS, with no Apple identity/Keychain access. It is a test install, not distribution signing. Notification authorization tests must use the appropriate signed build; ordinary smoke uses file custody and no OS prompt.

## Scope and evidence

- Read the task, `.bboxy/vision.md`, relevant specs/decisions/interfaces and the source/Git state before editing. `.bboxy/tasks/README.md` is the historical task board; GitHub issues track current release findings.
- One writing worktree per Executor. Do not edit the shared checkout, stage broadly, stash, reset, or change someone else's work. Stage exact paths. Every commit is signed as Boxy with the approved machine key.
- Run the Make targets appropriate to the change. Before handoff/push, `make test` must pass on the exact committed source. A real process-boundary change also runs `make test-boundaries`; UI geometry/reference changes require the existing parity check.
- Report commands and actual exits, paths/lines and mechanisms, what was skipped, the commit and worktree status. An agent report or dashboard label is not proof.
- Keep raw logs and fixture accounts outside Git. In `.bboxy/runs`, commit only a `.log`/`.json` command/exit/hash summary. Markdown under `.bboxy` is a numbered record and must follow its frontmatter format.
- Preserve live work: window close/Cmd-Q detach; Cmd-Option-Q and `ctl app stop` explicitly stop backend owners. Cmd-Shift-Q belongs to macOS logout. Supervisor/PTYS remain independent of ordinary UI restart.

## Unattended operation

Every unit, smoke, e2e and agent test runs with nobody at the machine. Use private accounts, homes, sockets and scripted controls. No operator Keychain prompt, login, click, pane authorization or physical probe may be required.

Drive this app through its flags, `ctl`, sockets and captures. Never inject OS keystrokes/clicks or screen-capture the operator's desktop. Kill only exact PIDs you started and recorded; delete only directories you created. Never use broad process kills or clean up peers' targets/worktrees.

## Coordination

Use repository-local task records and GitHub handoffs. **GitKB, ATC and ICQ are retired; never invoke them, including through historical scripts.** `scripts/land.sh` runs the signed landing gates and records its receipt locally, without messaging. Use it when the brief requires it; use the Make targets above for build/test/install work. Do not recruit, assign, merge or publish outside that brief.

The Oracle verifies landed code and logs; the Architect owns interface/spec consistency; Creativity owns the reference lab. Design gaps are written into the relevant spec/interface, not answered only in a message. Operator instructions outrank defaults.
