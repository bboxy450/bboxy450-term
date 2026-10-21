# Monorepo with weavie — what the tool does, how we lay out bboxy450-term

2026-10-21 · research subagent for Vesper Greywind <ORACLE>. Facts are `file:line — mechanism` or `command — result`. Unknowns are **Gap:**. [[vision]] · [[terminal-core]] · [[roadmap]]

Source read: the local checkout `/Users/feral/mydev/graph-repo` (commit `a500b44`, 2026-10-19, "Implement Nix-backed Rust source workspaces and live input overrides"). `gh repo clone bboxy450/weavie --depth 1` into the scratchpad succeeded and `diff -rq` against the local checkout reported no differences, so paths below are the local ones. `W/` = `/Users/feral/mydev/graph-repo/crates/weavie/`.

Machine state: `which nix` — not found. `weavie --version` — `weavie 0.2.0` (`~/.cargo/bin/weavie`). `cargo --version` — 1.99.0. `cargo search weavie` — 0.2.0 on crates.io. **Gap: no Nix on this Mac.** Every weavie command except `graph`/`why`/`nix-lib` shells out to `nix` (below), so nothing in the proposal was executed here.

## 1. What weavie is

A graph-repo tool: a Nix flake composes several independent Git repositories; weavie opens a per-task **session** that shows only that target's repositories as symlinks under `src/`, gives the ones you are editing an isolated Git worktree, and runs Cargo and Nix against the same selected sources. Directory layout is a view; the flake and its lock are the dependency authority (`graph-repo/docs/nix-graph-repo.md:16-33`).

### Commands (`W/src/main.rs`)

| Command | Mechanism |
|---|---|
| `open [DIR#TARGET] --name N --edit a,b --directory D` | `main.rs:32-43`. `Target::resolve` canonicalizes the local flake dir and runs `nix eval --impure --raw --expr builtins.currentSystem` (`W/src/backend.rs:67-94`). Evaluates `DIR#graphRepo.targets.<system>.<target>` as JSON (`backend.rs:100-123`, schema 1) and `nix flake metadata --json` (`backend.rs:125-131`). Creates `<DIR>/.weavie/sessions/<N>` by default (`main.rs:99-101`). With no `--edit`, the root repository is editable (`W/src/session.rs:246-250`). |
| `edit ALIAS` | Promotes a read-only snapshot to a worktree at its locked commit (`main.rs:45`, `session.rs:559-569`). |
| `status` | Per alias: edit/read, revision, path, `git status --short` for worktrees (`session.rs:732-774`). |
| `build [-- nix args]` | `nix build <root>#<target> --no-link` plus `--override-input` per non-self repo (`session.rs:776-787`, `backend.rs:139-151`). |
| `cargo [--native] ARGS` | Default `build`. Runs `nix develop <root>#<target> --command cargo` or plain `cargo` with `--native`; always `--config <session>/cargo-config.toml`, `CARGO_HOME=<session>/cargo-home`, `CARGO_TARGET_DIR=<session>/target`, cwd = root crate's `cargoRoot` (`session.rs:790-812`). Before running, it takes `cargo metadata` and fails if any mapped package resolves outside its selected directory (`session.rs:814-871`). |
| `nix-lib` | Prints the embedded helper `W/nix/lib.nix` (`main.rs:171-176`); you commit it as `weavie.nix`. |
| `graph [--from PATH]`, `why INPUT [--from PATH]` | Offline `flake.lock` graph read; no fetch (`W/src/lib.rs:1-5`, `main.rs:179-247`). |

Global flags: `--file`, `--json`, `--session DIR` (`main.rs:16-25`). Session discovery walks up from cwd looking for `weavie-session.json` (`session.rs:212-218`).

### Config: there is no weavie.toml

Configuration is the attrset passed to `import ./weavie.nix` in the composition's `flake.nix`. Arguments (`W/nix/lib.nix:3-12`): `pkgs`, `inputs`, `repositories`, `rootRepository`, `cargoLock`, `cargoRoot ? "."`, `pname ? rootRepository`, `version ? "0.1.0"`, `buildArgs ? {}`.

`repositories.<alias>` = `{ input ? alias; crates = { <cargo-package-name> = "<dir relative to that input>"; }; patchSources ? ["crates-io"]; }` (`lib.nix:21-34`). `input = "self"` means the composition's own Git repo (`lib.nix:18-20`). Each `crates` entry is checked at eval time: the `Cargo.toml` at that path must have `package.name == <key>` (`lib.nix:25-27`). Alias must match `[a-zA-Z0-9_-]+` (`lib.nix:22`).

The helper returns `{ metadata, package, devShell }` (`lib.nix:95-101`). The flake must export `graphRepo.targets.<system>.<target> = graph.metadata` (`graph-repo/docs/workspaces.md:5-9`, example `graph-repo/README.md:74-76`); `packages`/`devShells` are the same `graph.package` / `graph.devShell`.

### How Nix inputs become workspace members

1. `metadata.repositories.<alias>.storePath = toString repo.source` where `source = inputs.<input>` or `inputs.self` (`lib.nix:38-45`). That is the Nix store copy of each repository.
2. `Session::create` binds every alias: resolves `input` through the lock graph (`session.rs:289-303`, `lib.rs:219-235` for slash paths like `app/transport`), rejects two aliases on one lock node (`session.rs:326-333`), registers a GC root per store source via `nix-store --realise --add-root <session>/gcroots/<alias> --indirect` (`session.rs:350-368`), and symlinks `worktrees/<alias> -> store path` and `src/<alias> -> worktrees/<alias>` (`session.rs:369-376`).
3. Cargo mapping: for every alias, crate and patch source it writes `[patch.<source>.<crate>] path = <session>/worktrees/<alias>/<dir>` into `cargo-config.toml`, and merges the same patches into `cargo-home/config.toml` so child Cargo processes inherit them (`session.rs:636-730`). The Nix build does the same with `@weavieRoot@/<alias>/<dir>` substituted inside the assembled source derivation (`lib.nix:46-72`).
4. The Nix package is one `rustPlatform.buildRustPackage` whose `src` is a `runCommand` that `cp -R`s every repository under `$out/<alias>/`, writes the patch config into `<root>/<cargoRoot>/.cargo/config.toml`, copies `cargoLock` to `<root>/<cargoRoot>/Cargo.lock`, and sets `sourceRoot` to `<root>/<cargoRoot>` (`lib.nix:62-79`). `preBuild` asserts every mapped crate has `source == null` in `cargo metadata` (`lib.nix:81-89`).

### How worktrees are used

- Promotion (`session.rs:405-557`): bare object cache at `<composition>/.weavie/cache/git/<sha256(url)>.git` shared by all sessions (`session.rs:276`, `416-442`); the locked commit is fetched into `refs/weavie/pins/<rev>` (`session.rs:451-464`); `git worktree add -b weavie/<sha256(session dir)[..16]>/<alias> <dest> <rev>` (`session.rs:471-482`). On failure the store symlink is restored (`session.rs:483-488`).
- Only Git-backed inputs can be edited: `git`, `github`, `gitlab`, `sourcehut` (`session.rs:141-165`); a full 40/64-hex `rev` or `dirtyRev` is required (`session.rs:167-181`).
- For `self` with a dirty checkout, `git diff --binary HEAD` from the composition is applied into the new worktree; the original is untouched (`session.rs:414`, `501-539`). Untracked files are not carried (`workspaces.md:96-98`).
- Non-self editable repos are passed to Nix as `--override-input <input> git+file:///<worktree>?dir=…` so `weavie build` and `nix develop` see the live worktree; read-only ones are frozen as `path:/nix/store/…` (`session.rs:583-619`). `self` is handled by pointing the flake root at the worktree instead (`session.rs:571-581`).
- Session state is absolute and must not move (`session.rs:220-234`). Nothing deletes worktrees, branches or sessions (`W/README.md:57-58`).

### Proof status upstream

`graph-repo/docs/workspaces.md:141-162` — live Nix tests ran on aarch64 Linux with Nix 2.34.9 / Rust 1.98.1, sandbox on; native tests on macOS with Rust 1.99.0. CI (`graph-repo/.github/workflows/ci.yml`) runs fmt/clippy/test on ubuntu and macos and the Nix proof on ubuntu. No evidence of a macOS Nix run.

## 2. Proposed layout for bboxy450-term (PROPOSED, not executed)

Lens: weavie in its degenerate case, one repository `self` with six crates. Weavie buys us (a) the Nix build and dev shell, (b) one isolated worktree per Executor (`weavie open --name <executor> --edit term`), which is the CLAUDE.md "one writing worktree per Executor" rule as a command, (c) the growth path: when `term-core` or `mux` deserves its own repository, it becomes another `repositories` alias and a flake input with no change to crate code.

### Tree

```text
bboxy450-term/
├── flake.nix              composition; repositories map and two targets
├── flake.lock
├── weavie.nix             output of `weavie nix-lib`, committed (README.md:35-36, 85-91)
├── Cargo.toml             [workspace] members = ["crates/*"]; shared [workspace.dependencies]
├── Cargo.lock             the lock Nix installs (lib.nix:70); regenerate inside a session (workspaces.md:37-47)
├── .gitignore             /.weavie/ /target/ (workspaces.md:76)
├── crates/
│   ├── term-core/         alacritty_terminal fork: VTE, grid, viewport-grid + byte ring ([[terminal-core]])
│   ├── mux/               daemon: seats, PTYs (portable-pty), messaging, MCP server; depends on term-core
│   ├── ui-model/          state enums shared with the lab; no deps beyond serde
│   ├── renderer/          raw wgpu, text, shaders; depends on ui-model; include_str!("../../design/tokens.toml")
│   ├── app/               bin `bboxy450-term`: GUI and --daemon; depends on mux, renderer, ui-model
│   └── lab/               bin `lab`: component gallery; depends on renderer, ui-model
├── design/tokens.toml     already present; single token source for lab/web and renderer
├── lab/web/               already present; not a Cargo member, invisible to weavie, carried by the self worktree
├── .bboxy/
└── .weavie/               ignored. cache/git/<hash>.git, sessions/<name>/{src,worktrees,gcroots,target,cargo-home}
```

Crate dependency direction: `term-core <- mux <- app`; `ui-model <- renderer <- {app, lab}`; `mux <- app`. `lab` never links `mux` or `term-core`.

Package names: `term-core`, `mux`, `ui-model`, `renderer`, `app`, `lab` are too generic for crates.io; [[vision]] says crates publish under bboxy450. **Gap: final package names** (`bboxy450-term-core`, …?) — the `crates` map keys must equal `package.name` (`lib.nix:25-27`, `session.rs:646-653`), so this is decided once in `Cargo.toml` and mirrored in the flake. Directory names can stay short.

Fork naming: if the fork keeps `package.name = "alacritty_terminal"`, every `[patch.crates-io]` weavie writes would redirect any transitive `alacritty_terminal` to our fork. Use `patchSources = []` for the self repository (as the upstream example does, `README.md:61`) and rename the fork; then no patch entries are emitted at all (`lib.nix:35-37`, `session.rs:654-664`) and Cargo resolves members by path like any workspace.

### weavie config (lives in `flake.nix`)

```nix
{
  description = "bboxy450-term: GPU terminal cockpit for agent swarms";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = inputs@{ self, nixpkgs, ... }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAll = nixpkgs.lib.genAttrs systems;
      repositories = {
        term = {
          input = "self";
          crates = {
            term-core = "crates/term-core";
            mux       = "crates/mux";
            ui-model  = "crates/ui-model";
            renderer  = "crates/renderer";
            app       = "crates/app";
            lab       = "crates/lab";
          };
          patchSources = [ ];
        };
      };
      mkGraph = system: bin:
        let pkgs = nixpkgs.legacyPackages.${system};
        in (import ./weavie.nix) {
          inherit pkgs inputs repositories;
          rootRepository = "term";
          cargoLock = inputs.self + "/Cargo.lock";
          pname = "bboxy450-term-${bin}";
          version = "0.1.0";
          buildArgs = {
            cargoBuildFlags = [ "-p" bin ];
            cargoTestFlags  = [ "--workspace" ];
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.apple-sdk_14 ];
          };
        };
      graphs = forAll (system: { app = mkGraph system "app"; lab = mkGraph system "lab"; });
    in {
      packages  = builtins.mapAttrs (_: g: { app = g.app.package; lab = g.lab.package; default = g.app.package; }) graphs;
      devShells = builtins.mapAttrs (_: g: { app = g.app.devShell; lab = g.lab.devShell; default = g.app.devShell; }) graphs;
      graphRepo.targets = builtins.mapAttrs (_: g: { app = g.app.metadata; lab = g.lab.metadata; }) graphs;
    };
}
```

Why two graph calls: one helper call is one `buildRustPackage` (`lib.nix:73`), so `app` and `lab` are separate targets with identical `metadata`; a session opened on either sees the same six crates. `apple-sdk_14` matches the macOS 14 floor in [[vision]]. **Gap: exact nixpkgs attribute for the Metal/AppKit SDK on current `nixos-unstable`** — unverifiable here without Nix.

### Bootstrap (in order)

```sh
weavie nix-lib > weavie.nix
git add flake.nix weavie.nix Cargo.toml Cargo.lock crates design
nix flake lock && git add flake.lock && git commit
weavie open .#app --name prepare            # self is editable by default (session.rs:246-250)
cd .weavie/sessions/prepare
weavie cargo --native generate-lockfile     # workspaces.md:42-47
weavie cargo --native test --locked
weavie build                                # nix build .#app with the worktree as flake root
```

Open and edit must happen in a repository with a commit: `self` needs `rev` or `dirtyRev` from `nix flake metadata` (`session.rs:289-295`, `167-181`). bboxy450-term has commits (`git log` — `ad9e1e5` HEAD), so this holds.

### Executor flow

```sh
weavie open .#app --name rook-text-rendering --edit term
cd .weavie/sessions/rook-text-rendering
weavie cargo --native test -p renderer
weavie status
```

Each Executor gets `src/term` as a Git worktree on branch `weavie/<hash16>/term` at the commit the composition was at (`session.rs:471-481`), its own `target/` and `cargo-home` (`session.rs:806-808`). Two Executors never share a working tree; objects are shared through `.weavie/cache/git`. The Oracle verifies from the worktree path reported by `weavie status --json`.

### Growth path (when a crate leaves the monorepo)

Add `inputs.term-core.url = "github:bboxy450/term-core"`, add alias `term-core = { crates.term-core = "."; patchSources = [ "https://github.com/bboxy450/term-core" ]; }`, point `crates/mux/Cargo.toml` at `git = "https://github.com/bboxy450/term-core"`, remove the crate dir. Sessions then `--edit term,term-core` and `weavie build` overrides the input with the worktree (`session.rs:583-619`). The Cargo source URL in `patchSources` must equal the `git =` URL without `git+` (`workspaces.md:26-29`).

## 3. What weavie lacks for this use — improvement candidates

Ordered by how soon it bites.

1. **Nix is mandatory for `open`.** `Target::resolve` runs `nix eval` (`backend.rs:82-88`) and `open` evaluates the flake (`backend.rs:100-131`). No Nix on this Mac, so no sessions at all today. Candidate: `weavie open --native` that derives the descriptor from `cargo metadata` of the root workspace (`self` only, every member crate) and skips GC roots; or at minimum a clear preflight error naming the Nix install.
2. **Crates must be listed by hand and asserted one by one** (`lib.nix:25-27`). For a `self` monorepo the map duplicates `[workspace] members`. Candidate: `crates = "workspace"` sentinel that reads `Cargo.toml` members and each `package.name` at eval time.
3. **One package per helper call** (`lib.nix:73-90`). Two binaries means two graph calls and duplicated metadata. Candidate: `bins = [ "app" "lab" ]` producing `packages.<bin>` from one source derivation, one `metadata`.
4. **Whole repository is the Nix source** (`lib.nix:64-66` `cp -R`). Edits to `.bboxy/` or `lab/web` rebuild the Rust package. Candidate: `lib.fileset` filter per repository (`crates/`, `Cargo.*`, `design/`), or honour `include` from each crate manifest.
5. **No dependency-artifact caching.** `buildRustPackage` rebuilds every registry crate per change; `docs/nix-graph-repo.md:227-231` already names crane as the reference. wgpu plus a VTE fork is a heavy dependency closure. Candidate: crane backend behind the same `metadata`.
6. **Dev shell is `cargo rustc git` only** (`lib.nix:97-100`); no clippy, rustfmt, rust-analyzer, no `rust-toolchain.toml`, no fenix/rust-overlay pin. Nixpkgs rustc version drives `edition`/MSRV. Candidate: `toolchain` argument and extra `devPackages`.
7. **No session lifecycle.** No `list`, no `close`/`remove`, no worktree or branch deletion (`W/README.md:57-58`, `workspaces.md:86-87`). Branch names are hash-based `weavie/<hash16>/<alias>` (`session.rs:471-474`), unreadable in `git branch`. Candidates: `weavie sessions`, `weavie close NAME [--delete-branch]`, branch `weavie/<session-name>/<alias>`.
8. **Sessions are pinned to absolute paths** (`session.rs:229-232`) and live inside the composition under `.weavie/sessions`. Fine locally; blocks moving a session to another machine.
9. **`version ? "0.1.0"`** (`lib.nix:10`) is not read from the root crate. Candidate: default from `Cargo.toml`.
10. **Untracked files are invisible to `weavie build`** (`workspaces.md:59-61`, `96-98`) because Nix snapshots the Git index. Executors adding files will see Cargo pass and Nix fail. Candidate: `weavie build` warns on `git status --porcelain` untracked paths under mapped crates.
11. **No commit, push or PR from a session** (`workspaces.md:139`). Executors report; the Oracle merges by hand. Acceptable under the no-auto-merge rule, but a `weavie status --json` field with the branch name and upstream would help the handoff.
12. **`weavie cargo` without `--native` spawns `nix develop` every invocation** (`session.rs:798-803`) — slow per command. Candidate: cached `nix print-dev-env` profile.
13. **Unix only** (`session.rs:97-100`). Irrelevant for v0.1 (macOS first), noted for Linux-then-mobile.

## 4. Verified vs proposed

Verified by reading source: everything in §1 (`file:line` above), `weavie 0.2.0` installed, Nix absent, local checkout equals GitHub HEAD. Proposed and unexecuted: §2 tree, flake, bootstrap, Executor flow. The flake follows `graph-repo/README.md:41-78` and `crates/weavie/tests/common/mod.rs:303-321, 359-374` (the `self` variant) but has not been evaluated. **Gap: first `weavie open` on this project** — needs a Nix install (Determinate or upstream installer) on this Mac, or improvement candidate 1.
