# Canonical build/test/install/release interface. See `make help`.
# Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
SHELL := /bin/sh
.DEFAULT_GOAL := help

APP := bboxy450-term
PROFILE ?= release
TARGET_DIR ?= target
INSTALL_DIR ?= $(HOME)/Applications
UNSIGNED_INSTALL_DIR ?= $(TARGET_DIR)/install-unsigned
BUNDLE ?= $(TARGET_DIR)/$(PROFILE)/$(APP).app
DIST_DIR ?= $(TARGET_DIR)/dist
RELEASE_REPO := bboxy450/bboxy450-term-releases
SIGNING_IDENTITY := 3943AF130A015B36824B7FBC86AA672E7B25F3C4
GPG_KEY := 9B07E849480C68328CA20684844ECC0587DE7AB0
REQUIREMENT := identifier "com.bboxy450.term" and anchor apple generic and certificate leaf[subject.OU] = "UQB3368A84"
APP_VERSION = $(shell python3 -c 'import tomllib; a=tomllib.load(open("crates/bboxy450-term/Cargo.toml","rb"))["package"]["version"]; print(a if isinstance(a,str) else tomllib.load(open("Cargo.toml","rb"))["workspace"]["package"]["version"])')
ARCHIVE = $(DIST_DIR)/$(APP)-$(APP_VERSION)-macos-$(shell uname -m).zip
CI_LOG ?= $(abspath $(TARGET_DIR))/ci-local.log
CI_ARGS ?=
export CARGO_PROFILE_DEV_DEBUG ?= 0
export CARGO_PROFILE_TEST_DEBUG ?= 0
export CARGO_PROFILE_DEV_OPT_LEVEL ?= 1
export CARGO_PROFILE_TEST_OPT_LEVEL ?= 1

.PHONY: help build build-release compile lock test test-parity parity-swarm check test-boundaries test-swarm smoke bundle-files bundle bundle-unsigned install install-unsigned release package-crate publish-release push

help:
	@printf '%s\n' \
	  'make build / build-release       Build the app (debug / optimized)' \
	  'make lock                       Refresh workspace lock metadata offline' \
	  'make test                       Full local CI; no operator input' \
	  'make test-parity                Full local CI including visual parity' \
	  'make test-boundaries             Explicit shell/lifecycle process regressions' \
	  'make test-harness-startup        Launch identity, shell configuration and shared wake regressions' \
	  'make test-harness-native         Real Codex shared daemon, local model, warm and cold startup' \
	  'make test-codex-rendering        Large catalog relay, idle frames and resized logic recovery' \
	  'make test-checkpoint-resize      Both terminal screens survive bounded resize checkpoints' \
	  'make test-landing-phases         CI smoke fixtures use private account flags' \
	  'make check-harness-startup       Strict all-target Clippy for the changed harness path' \
	  'make test-swarm                  Swarm folder, persistence and quit regressions' \
	  'make test-swarm-switch           Empty swarm reads, inactive UI and real PTY dimming' \
	  'make test-navigation             Native clicks, Go to, split chords and launcher parameters' \
	  'make test-cross-focus             Directional focus glow and frozen-clock GPU regression' \
	  'make test-sidebar-resize          Sidebar drag, width bounds and retained session/restart' \
	  'make test-oracle-control         Seat MCP grants, spawn/split, focus and detached reattach' \
	  'make parity-swarm                Existing swarm/sidebar visual parity cases' \
	  'make smoke                      Unsigned private install + headless smoke' \
	  'make smoke-installed             Private smoke of the installed signed app' \
	  'make install                    Release app, Apple identity signed, ~/Applications' \
	  'make install-unsigned            Debug app, ad-hoc only, target/install-unsigned' \
	  'make release                    Signed app ZIP, SHA256 and detached GPG signature' \
	  'make package-crate CRATE=name    Package and GPG-sign one crate, without publishing' \
	  'make sync-release-inputs         Refresh packaged copies of canonical build inputs' \
	  'make publish-release            Signed tag + GitHub app release (authorization required)' \
	  'make push                       Push signed, tested HEAD to main (authorization required)' \
	  'Overrides: PROFILE=debug|release TARGET_DIR=... INSTALL_DIR=... UNSIGNED_INSTALL_DIR=... CI_LOG=...'

build:
	@$(MAKE) compile PROFILE=debug

build-release:
	@$(MAKE) compile PROFILE=release

lock:
	@set -eu; . ./scripts/cache-env.sh; cargo update --offline --workspace

compile:
	@set -eu; case '$(PROFILE)' in debug) flag='' ;; release) flag='--release' ;; *) echo 'PROFILE must be debug or release' >&2; exit 2 ;; esac; \
	. ./scripts/cache-env.sh; cargo build --locked -p $(APP) --target-dir '$(TARGET_DIR)' $$flag

test check:
	@set -eu; mkdir -p "$$(dirname '$(CI_LOG)')"; \
	BBOXY_GATE_SLOT=none BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))' sh scripts/ci-local.sh $(CI_ARGS) >'$(CI_LOG)' 2>&1 || { code=$$?; tail -60 '$(CI_LOG)'; exit $$code; }; \
	tail -12 '$(CI_LOG)'; printf 'Full gate log: %s\n' '$(CI_LOG)'

test-parity:
	@$(MAKE) test CI_ARGS=--parity

parity-swarm:
	@set -eu; . ./scripts/cache-env.sh; \
	export BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))'; \
	cargo build --locked -p bboxy450-book --target-dir '$(TARGET_DIR)'; \
	swarm_cases=$$('$(TARGET_DIR)/debug/bboxybook' manifest | python3 -c 'import json,sys; cases=json.load(sys.stdin); selected=["{component}/{slug}-{density}-{theme}.png".format(**row) for row in cases if (row.get("fixture") or {}).get("layer")=="swarms"]; assert selected; print(json.dumps(selected))'); \
	sh scripts/parity.sh --cases "$$swarm_cases"

test-swarm:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib every_verb_maps_to_exactly_one_window_action; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP) swarm_workdir; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib window_state; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) quit_and_hard_quit; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir -- --ignored

test-boundaries:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test seat_wiring; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test ctl --test native_signing; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib login_shell_exec -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib cockpit::shell_path::tests -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib refresh_replaces_a_confirmed_failed_process -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test cockpit -- --ignored --test-threads=1

bundle-files: compile
	@set -eu; test "$$(uname -s)" = Darwin || { echo 'macOS is required' >&2; exit 2; }; \
	case '$(BUNDLE)' in *.app) ;; *) echo 'BUNDLE must end in .app' >&2; exit 2 ;; esac; \
	mkdir -p '$(BUNDLE)/Contents/MacOS' '$(BUNDLE)/Contents/Resources'; \
	cp packaging/macos/Info.plist '$(BUNDLE)/Contents/Info.plist'; \
	cp '$(TARGET_DIR)/$(PROFILE)/$(APP)' '$(BUNDLE)/Contents/MacOS/$(APP)'; \
	cp NOTICE LICENSE-MIT LICENSE-APACHE packaging/macos/bboxy.icns '$(BUNDLE)/Contents/Resources/'; \
	plutil -replace CFBundleShortVersionString -string '$(APP_VERSION)' '$(BUNDLE)/Contents/Info.plist'; \
	plutil -replace CFBundleVersion -string "$$(git rev-list --count HEAD)" '$(BUNDLE)/Contents/Info.plist'; \
	revision=$$(git rev-parse HEAD); git diff --quiet HEAD -- || revision="$$revision-dirty"; \
	plutil -insert BBOXYSourceRevision -string "$$revision" '$(BUNDLE)/Contents/Info.plist'; \
	plutil -lint '$(BUNDLE)/Contents/Info.plist'

bundle: bundle-files
	@set -eu; security find-identity -v -p codesigning | grep -F '$(SIGNING_IDENTITY)' >/dev/null || { echo 'Approved Apple Development signing identity unavailable' >&2; exit 2; }; \
	codesign --force --sign '$(SIGNING_IDENTITY)' --requirements '=designated => $(REQUIREMENT)' --options runtime --timestamp=none '$(BUNDLE)'; \
	codesign --verify --strict --verbose=2 '$(BUNDLE)'; printf '%s\n' '$(BUNDLE)'

bundle-unsigned: PROFILE=debug
bundle-unsigned: bundle-files
	@set -eu; codesign --force --sign - --timestamp=none '$(BUNDLE)'; \
	codesign --verify --strict '$(BUNDLE)'; \
	printf '%s\n' 'Test bundle: ad-hoc signature only; no Apple identity or Keychain access.'

# Keep the outer .app inode (Dock/Finder aliases) and the old mapped executable.
# Previous Contents are retained in the printed private staging directory.
define install_bundle
set -eu; mkdir -p '$(1)'; \
stage=$$(mktemp -d '$(1)/.$(APP)-install.XXXXXX'); \
ditto '$(BUNDLE)' "$$stage/new.app"; codesign --verify --strict "$$stage/new.app"; \
destination='$(1)/$(APP).app'; mkdir -p "$$destination"; \
if [ -d "$$destination/Contents" ]; then mv "$$destination/Contents" "$$stage/previous-Contents"; fi; \
if ! mv "$$stage/new.app/Contents" "$$destination/Contents"; then \
  if [ -d "$$stage/previous-Contents" ]; then mv "$$stage/previous-Contents" "$$destination/Contents"; fi; exit 1; \
fi; \
codesign --verify --strict --verbose=2 "$$destination"; \
printf 'Installed: %s\nPrevious contents, if any: %s\n' "$$destination" "$$stage/previous-Contents"
endef

install: bundle
	@$(call install_bundle,$(INSTALL_DIR))

install-unsigned: PROFILE=debug
install-unsigned: bundle-unsigned
	@$(call install_bundle,$(UNSIGNED_INSTALL_DIR))

define smoke_binary
set -eu; run=$$(mktemp -d "$${TMPDIR:-/tmp}/bboxy-make-smoke.XXXXXX"); \
	mkdir "$$run/repository" "$$run/home"; \
	if env -i PATH="$$PATH" TERM="$${TERM:-xterm-256color}" LANG="$${LANG:-en_US.UTF-8}" TMPDIR="$${TMPDIR:-/tmp}" \
	  HOME="$$run/home" CODEX_HOME="$$run/home/.codex" CLAUDE_CONFIG_DIR="$$run/home/.claude" ZDOTDIR="$$run/home" \
	  '$(1)' \
	  --headless --smoke --ctl --unattended --test-account "$$run/account" --run-dir "$$run/run" --repository "$$run/repository" </dev/null; then \
	  rm -r -- "$$run"; \
	else code=$$?; printf 'Smoke failed; private evidence retained at %s\n' "$$run" >&2; exit $$code; fi
endef

smoke: PROFILE=debug
smoke: install-unsigned
	@$(call smoke_binary,$(abspath $(UNSIGNED_INSTALL_DIR))/$(APP).app/Contents/MacOS/$(APP))

.PHONY: smoke-installed
smoke-installed:
	@codesign --verify --strict '$(INSTALL_DIR)/$(APP).app'
	@$(call smoke_binary,$(abspath $(INSTALL_DIR))/$(APP).app/Contents/MacOS/$(APP))

release: bundle
	@set -eu; mkdir -p '$(DIST_DIR)'; \
	ditto -c -k --sequesterRsrc --keepParent '$(BUNDLE)' '$(ARCHIVE)'; \
	shasum -a 256 '$(ARCHIVE)' >'$(ARCHIVE).sha256'; \
	gpg --batch --yes --local-user '$(GPG_KEY)' --armor --detach-sign '$(ARCHIVE)'; \
	gpg --verify '$(ARCHIVE).asc' '$(ARCHIVE)'; printf 'Signed release artifacts: %s{,.sha256,.asc}\n' '$(ARCHIVE)'

.PHONY: sync-release-inputs
sync-release-inputs:
	@python3 -I scripts/release-inputs.py --sync

package-crate:
	@set -eu; test -n '$(CRATE)' || { echo 'Usage: make package-crate CRATE=package-name' >&2; exit 2; }; \
	. ./scripts/cache-env.sh; cargo package --locked -p '$(CRATE)' --target-dir '$(TARGET_DIR)'; \
	version=$$(cargo pkgid -p '$(CRATE)' --locked | sed 's/.*[@#]//'); \
	file='$(TARGET_DIR)/package/$(CRATE)-'"$$version"'.crate'; \
	gpg --batch --yes --local-user '$(GPG_KEY)' --armor --detach-sign "$$file"; gpg --verify "$$file.asc" "$$file"

publish-release: release
	@set -eu; test "$$(gh api user --jq .login)" = bboxy450; test -z "$$(git status --porcelain)"; \
	grep -E "^ci-local: result $$(git rev-parse HEAD) (normal|extended) 0$$" '$(CI_LOG)' >/dev/null; \
	tag='v$(APP_VERSION)'; if git rev-parse -q --verify "refs/tags/$$tag" >/dev/null; then \
	  test "$$(git rev-list -1 "$$tag")" = "$$(git rev-parse HEAD)"; git verify-tag "$$tag"; \
	else git tag -s "$$tag" -m '$(APP) $(APP_VERSION)'; fi; \
	GIT_TERMINAL_PROMPT=0 git push origin HEAD:main --follow-tags; \
	GIT_TERMINAL_PROMPT=0 git push 'https://github.com/$(RELEASE_REPO).git' "refs/tags/$$tag:refs/tags/$$tag"; \
	gh release create "$$tag" --repo '$(RELEASE_REPO)' '$(ARCHIVE)' '$(ARCHIVE).sha256' '$(ARCHIVE).asc' --title '$(APP) $(APP_VERSION)' --notes "Source: https://github.com/bboxy450/bboxy450-term/commit/$$(git rev-parse HEAD)"

push:
	@set -eu; test "$$(gh api user --jq .login)" = bboxy450; test -z "$$(git status --porcelain)"; \
	git fetch origin main; git merge-base --is-ancestor origin/main HEAD; \
	for revision in $$(git rev-list origin/main..HEAD); do git verify-commit "$$revision"; done; \
	grep -E "^ci-local: result $$(git rev-parse HEAD) (normal|extended) 0$$" '$(CI_LOG)' >/dev/null; \
	GIT_TERMINAL_PROMPT=0 git push origin HEAD:main

.PHONY: test-spawn-unit test-spawn check-spawn parity-launcher
test-spawn-unit:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test launchers; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib spawn_params

check-spawn:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked --all-targets -p bboxy450-mermaid -p bboxy450-proto -p bboxy450-ui-core -p bboxy450-ui -p bboxy450-mux -p bboxy450-term -p bboxy450-book --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-harness-startup check-harness-startup fmt-harness-startup
fmt-harness-startup:
	@rustfmt --edition 2024 --config skip_children=true crates/bboxy450-harness-wake-up/src/launch.rs crates/bboxy450-harness-wake-up/src/lib.rs crates/bboxy450-harness-wake-up/src/native.rs crates/bboxy450-harness-wake-up/tests/adapters.rs crates/bboxy450-harness-wake-up/tests/ready_exec.rs crates/bbt-mux/src/adapters/launch.rs crates/bbt-mux/src/adapters/mapping.rs crates/bbt-mux/src/logic.rs crates/bbt-mux/src/logic/ring_batch_tests.rs

test-harness-startup:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib login_shell_exec -- --include-ignored

check-harness-startup:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked --all-targets -p bboxy450-harness-wake-up -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-harness-native
test-harness-native: build
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)'
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)' --cold

.PHONY: test-codex-rendering
test-codex-rendering: build
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)' --test codex_proxy
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)' --render-check

.PHONY: test-checkpoint-resize
test-checkpoint-resize:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-term-core --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-harness-host --target-dir '$(TARGET_DIR)' --test checkpoint_resize

.PHONY: test-landing-phases
test-landing-phases:
	@python3 -I scripts/test-landing-recovery.py CiPhaseTests

.PHONY: preview-launcher
preview-launcher:
	@node crates/bboxybook/capture-launch-params.mjs '$(TARGET_DIR)/launch-preview'

test-spawn: test-spawn-unit
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' legacy_spawn_without_arguments_and_environment_round_trips; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir spawn_parameters_ctl_flow -- --ignored --nocapture

parity-launcher:
	@set -eu; . ./scripts/cache-env.sh; \
	export BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))'; \
	cargo build --locked -p bboxy450-book --target-dir '$(TARGET_DIR)'; \
	launcher_cases=$$('$(TARGET_DIR)/debug/bboxybook' manifest | python3 -c 'import json,sys; cases=json.load(sys.stdin); selected=["{component}/{slug}-{density}-{theme}.png".format(**row) for row in cases if (row.get("fixture") or {}).get("layer")=="launchers"]; assert selected; print(json.dumps(selected))'); \
	sh scripts/parity.sh --cases "$$launcher_cases"

.PHONY: fmt-spawn test-spawn-dependents
fmt-spawn:
	@set -eu; . ./scripts/cache-env.sh; { git diff HEAD --name-only --diff-filter=AM -- '*.rs'; git ls-files --others --exclude-standard -- '*.rs'; } | sort -u | while IFS= read -r source; do rustfmt --edition 2024 --config skip_children=true "$$source"; done

test-spawn-dependents:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p bboxy450-mermaid -p bboxy450-ui-core -p bboxy450-proto -p bboxy450-ui -p bboxy450-mux -p bboxy450-term -p bboxy450-book --target-dir '$(TARGET_DIR)'

.PHONY: test-spawn-e2e
test-spawn-e2e:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir spawn_parameters_ctl_flow -- --ignored --nocapture

.PHONY: test-spawn-wire
test-spawn-wire:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) launch_parameters_wire -- --nocapture

.PHONY: test-navigation check-navigation
.PHONY: test-new-swarm format-new-swarm
.PHONY: test-swarm-switch check-swarm-switch
test-swarm-switch:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test swarms --test seat_rows; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	BBOXY_SWITCH_EVIDENCE='$(abspath $(TARGET_DIR))/swarm-switch' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir swarm_switches_wait_for_membership_and_dim_real_inactive_seats -- --ignored --exact --nocapture

check-swarm-switch:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-ui -p bboxy450-book -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-sidebar-resize check-sidebar-resize
test-sidebar-resize:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui-core --target-dir '$(TARGET_DIR)' --lib input::sidebar; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test composer sidebar; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP) sidebar; \
	BBOXY_SIDEBAR_EVIDENCE='$(abspath $(TARGET_DIR))/sidebar-resize' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir sidebar_drag_reflows_and_persists_without_restarting_harnesses -- --ignored --exact

check-sidebar-resize:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-ui-core -p bboxy450-ui -p bboxy450-proto -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-cross-focus format-cross-focus check-cross-focus
format-cross-focus: fmt-spawn

test-cross-focus:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui-core --target-dir '$(TARGET_DIR)' --test input; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP) --test documents; \
	BBOXY_FOCUS_EVIDENCE='$(abspath $(TARGET_DIR))/cross-focus' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test headless test_account_ui_clock_freezes_pixels_steps_focus_and_resumes_live_time -- --ignored --exact
	@node --check lab/web/showcase.js

check-cross-focus:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-ui-core -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

test-new-swarm:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) new_swarm_workdir; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir new_swarm_cwd -- --ignored

format-new-swarm:
	@rustfmt --edition 2024 --config skip_children=true crates/bboxy450-term/src/main.rs crates/bboxy450-term/src/navigation.rs crates/bboxy450-term/src/scene.rs crates/bboxy450-term/src/swarm_folder.rs crates/bboxy450-term/tests/swarm_workdir.rs

test-navigation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin bboxy450-term; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir -- --ignored --nocapture

check-navigation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-ui -p bboxy450-proto -p bboxy450-mux -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-oracle-control test-oracle-ui check-oracle-control format-oracle-control
test-oracle-control:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto -p bboxy450-mux --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp mcp_oracle -- --include-ignored --nocapture; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	$(MAKE) test-oracle-ui

test-oracle-ui:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir oracle_mcp -- --ignored --nocapture

check-oracle-control:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-ui -p bboxy450-proto -p bboxy450-mux -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

format-oracle-control:
	@set -eu; . ./scripts/cache-env.sh; rustfmt --edition 2024 --config skip_children=true crates/bbt-mux/src/logic.rs crates/bbt-mux/src/mcp/seats.rs crates/bbt-mux/src/mcp/bridge.rs crates/bbt-mux/src/window_journal.rs crates/bbt-proto/src/registry.rs crates/bboxy450-term/src/model.rs crates/bboxy450-term/src/cockpit.rs crates/bboxy450-term/src/live.rs crates/bbt-mux/tests/mcp.rs crates/bboxy450-term/tests/swarm_workdir.rs

.PHONY: test-large-gui-frame format-large-gui-frame
test-large-gui-frame:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib gui_reply_budget_accepts_large_frames -- --nocapture; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir large_terminal_snapshot -- --ignored --nocapture

format-large-gui-frame:
	@rustfmt --edition 2024 --config skip_children=true crates/bbt-proto/src/gui/mod.rs crates/bbt-mux/src/seat_socket.rs crates/bboxy450-term/src/cockpit.rs crates/bboxy450-term/src/ctl.rs crates/bboxy450-term/tests/swarm_workdir.rs
