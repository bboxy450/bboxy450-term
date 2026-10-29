# Canonical build/test/install/release interface. See `make help`.
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Retain accurate source credit on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
SHELL := /bin/sh
.DEFAULT_GOAL := help

APP := bboxy450-term
PROFILE ?= release
TARGET_DIR ?= target
# Each fixture owns real app/PTY/provider processes. Leave fixed provider deadlines
# for its own scenario rather than unrelated GUI fixture startup.
SWARM_TEST_THREADS ?= 2
INSTALL_DIR ?= $(HOME)/Applications
UNSIGNED_INSTALL_DIR ?= $(TARGET_DIR)/install-unsigned
BUNDLE ?= $(TARGET_DIR)/$(PROFILE)/$(APP).app
DIST_DIR ?= $(TARGET_DIR)/dist
RELEASE_REPO := bboxy450/bboxy450-term-releases
PUSH_IDENTITY ?= bboxy450
CODEX_BIN ?= codex
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
	  'make lock-online                Resolve added dependencies with the registry' \
	  'make test                       Full local CI; no operator input' \
	  'make test-task-migration        Private authenticated lifecycle/import and independent 250-record audit' \
	  'make test-signatures             Pinned public-only history and signature refusal fixtures' \
	  'make test-landing                Private author-landing fixtures; no operator key' \
	  'make test-task-import-transport  Focused authenticated import transport and original-operation diagnostics' \
	  'make test-clipboard-isolation   Concurrent explicitly owned private pasteboards; no general clipboard' \
	  'make check-clipboard-isolation  Strict all-target lint for the private pasteboard fixtures' \
	  'make test-parity                Full local CI including visual parity' \
	  'make test-boundaries             Explicit shell/lifecycle process regressions' \
	  'make test-gui-events             Durable GUI restart and event socket regressions' \
	  'make test-make-isolation         Private signing fixtures ignore outer Make target overrides' \
	  'make test-signed-mentions        Signed names, receiver admission, replay and composer boundaries' \
	  'make test-fifty-seat-startup     Real fifty-seat startup and bounded failure diagnostics' \
	  'make diagnose-fifty-seat-pty     Bounded Darwin syscall probe; nonzero if an allocation fails' \
	  'make test-documents-night        Full native D1/D2 private document assertions' \
	  'make test-documents-activation   Independent document wire refresh with retained native sessions' \
	  'make parity-documents           Existing strict document gallery geometry gate' \
	  'make test-harness-startup        Launch identity, shell configuration and shared wake regressions' \
	  'make test-adapter-startup        Retained real-zsh startup and prompt/command boundary diagnostics' \
	  'make test-harness-native         Real Codex shared daemon, local model, direct and forwarding shim' \
	  'make test-harness-proxy          Codex proxy hook composition, seat policy and large catalogs' \
	  'make test-wake-causes            Typed wake causes, legacy replay, MCP refusal and retained-logic refresh' \
	  'make test-harness-fixture        Run an optional local executable HARNESS_FIXTURE against a private build' \
	  'make test-codex-rendering        Large catalog relay, idle frames and resized logic recovery' \
	  'make test-checkpoint-resize      Both terminal screens survive bounded resize checkpoints' \
	  'make test-terminal-replay        Busy idle saves, bounded replay recovery and real ctl input' \
	  'make test-input-backend          Admission wake, watched echo latency and replay safety' \
	  'make test-input-ui               Wheel bursts, pointer motion, selection and sidebar regressions' \
	  'make profile-input-ui            Repeatable CPU host-layout profile (no GPU latency claim)' \
	  'make check-terminal-replay       Strict all-target Clippy for mux and app' \
	  'make test-landing-phases         CI smoke fixtures use private account flags' \
	  'make check-harness-startup       Strict all-target Clippy for the changed harness path' \
	  'make test-swarm                  Swarm folder, persistence and quit regressions' \
	  'make test-swarm-switch           Empty swarm reads, inactive UI and real PTY dimming' \
	  'make test-navigation             Native clicks, Go to, split chords and launcher parameters' \
	  'make test-terminal-selection     Terminal drag selection, private clipboard and native Copy' \
	  'make test-terminal-font          Global terminal font chords, native menu and retained PTY geometry' \
	  'make test-terminal-colors        Private PTY/login-shell color defaults and explicit opt-outs' \
	  'make test-terminal-colors-native Real private Codex shared-daemon TUI colors under NO_COLOR' \
	  'make test-color-emoji            Native chrome/terminal color glyphs and Unicode clusters' \
	  'make test-color-emoji-swarm      Private Unicode swarm rename, storage and UI restart' \
	  'make check-color-emoji-all       Strict Clippy for color rendering and its dependents' \
	  'make check-color-emoji           Strict all-target Clippy for shared color rendering' \
	  'make check-terminal-font         Strict all-target Clippy for terminal font paths' \
	  'make test-close-shortcut         Cmd+W focused-seat close, retained PTYs and undo' \
	  'make test-seat-mute              Seat buttons, Cmd+M, muted swarm isolation and operator control' \
	  'make test-swarm-metadata         Persistent swarm names/cwd, legacy import and operator admission' \
	  'make test-workspace-boundary     Scoped wire, root-bound replay and read-completion fences' \
	  'make test-ctl-workspaces         Bounded same-op workspace preparation and real operator CLI' \
	  'make test-workspace-roots        Canonical cwd mounts, scoped records and stale-root fences' \
	  'make test-cursors                Shared cursor intent, enabled controls and native overlay/resize routing' \
	  'make test-role-identity          ROLE-first seat, pane and message identity; check-role-identity / parity-role-identity' \
	  'make test-app-overlays           Native/ctl menus, dialog input and persistent swarm rename' \
	  'make test-seat-actions           Captured seat menus, scoped task assignment and exact-child kill' \
	  'make test-task-contract          Exact native task catalogue and plugin schema alignment' \
	  'make test-scoped-workspaces      Captured native docs/tasks/graph and cross-project ctl fences' \
	  'make test-overlay-captures       Private headless light/dark menu and dialog evidence' \
	  'make test-ui-overlays            Pure context-menu/dialog interactions and strict shared UI Clippy' \
	  'make test-desktop               Portable actions and real native macOS menu/window probe' \
	  'make test-desktop-app           Private desktop admission, titlebar gestures and strict lint' \
	  'make test-desktop-lifecycle     Owned daemon/UI detach and same-harness reopen boundary' \
	  'make test-image-paste             Private clipboard, atomic markers, custody and local recipient restart' \
	  'make check-image-paste            Strict all-target Clippy for image-paste crates and dependents' \
	  'make test-chat-ux                Chat controls, local times, quiet status and saved vertical splits' \
	  'make test-chat-sections          Independent Chat/Tasks, incoming tail follow and expiring notices' \
	  'make test-cross-focus             Directional focus glow and frozen-clock GPU regression' \
	  'make test-sidebar-resize          Sidebar drag, width bounds and retained session/restart' \
	  'make test-sidebar-accordions      Swarm/SSH/cloud collapse, focus independence and restart' \
	  'make test-oracle-control         Seat MCP grants, spawn/split, focus and detached reattach' \
	  'make test-settings-role-picker-ui Pure Settings pointer release and retained picker regressions' \
	  'make test-settings-night          Private full R2/G1/G2 Settings proofs and negative controls' \
	  'make test-notification-copy       Actual native request fields, real waiting/mute and copy mutations' \
	  'make test-settings-role-picker   Private complete-click Settings default role and launch regression' \
	  'make check-settings-role-picker  Strict all-target Clippy for Settings picker crates and dependents' \
	  'make parity-swarm                Existing swarm/sidebar visual parity cases' \
	  'make smoke                      Unsigned private install + headless smoke' \
	  'make smoke-installed             Private smoke of the installed signed app' \
	  'make install                    Release app, Apple identity signed, ~/Applications' \
	  'make install-unsigned            Debug app, ad-hoc only, target/install-unsigned' \
	  'make release                    Signed app ZIP, SHA256 and detached GPG signature' \
	  'make package-crate CRATE=name    Package and GPG-sign one crate, without publishing' \
	  'make sync-release-inputs         Refresh packaged copies of canonical build inputs' \
	  'make sync-licenses               Refresh local crate LICENSE/NOTICE/ATTRIBUTION copies' \
	  'make sync-source-notices         Attach scoped source comments; preserve upstream files' \
	  'make test-licenses               License metadata, copies and unattended negative fixtures' \
	  'make check-license-packages      Inspect all crate package lists; no compilation' \
	  'make publish-release            Signed tag + GitHub app release (authorization required)' \
	  'make push                       Push signed, tested HEAD to main (authorization required)' \
	  'Overrides: PROFILE=debug|release TARGET_DIR=... INSTALL_DIR=... UNSIGNED_INSTALL_DIR=... CI_LOG=... CODEX_BIN=... PUSH_IDENTITY=... SWARM_TEST_THREADS=2'

build:
	@$(MAKE) compile PROFILE=debug

build-release:
	@$(MAKE) compile PROFILE=release

lock:
	@set -eu; . ./scripts/cache-env.sh; cargo update --offline --workspace

.PHONY: lock-online
lock-online:
	@set -eu; . ./scripts/cache-env.sh; cargo update --workspace

compile:
	@set -eu; case '$(PROFILE)' in debug) flag='' ;; release) flag='--release' ;; *) echo 'PROFILE must be debug or release' >&2; exit 2 ;; esac; \
	. ./scripts/cache-env.sh; cargo build --locked -p $(APP) --target-dir '$(TARGET_DIR)' $$flag

test check: test-signatures
	@set -eu; mkdir -p "$$(dirname '$(CI_LOG)')"; \
	SIGNED_RANGE= BBOXY_GATE_SLOT=none BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))' sh scripts/ci-local.sh $(CI_ARGS) >'$(CI_LOG)' 2>&1 || { code=$$?; tail -60 '$(CI_LOG)'; exit $$code; }; \
	tail -12 '$(CI_LOG)'; printf 'Full gate log: %s\n' '$(CI_LOG)'

test-parity:
	@$(MAKE) test CI_ARGS=--parity

.PHONY: test-task-migration
test-task-migration:
	@set -eu; . ./scripts/cache-env.sh; \
	python3 scripts/test-task-migration.py prepare '$(abspath $(TARGET_DIR))'; \
	export BBOXY_TASK_MIGRATION_INPUT='$(abspath $(TARGET_DIR))/task-migration/input'; \
	export BBOXY_TASK_MIGRATION_OUTPUT='$(abspath $(TARGET_DIR))/task-migration'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib migration -- --nocapture; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp task_migration -- --nocapture; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib task_contract_properties_derive_from_the_current_native_tool_catalogue; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp mcp_channel_tools_keep_worker_links_and_recheck_revoked_reads_before_cached_results; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' --test messaging; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib task_migration_verbs; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir task_migration_catalogue_upgrade -- --ignored --nocapture --test-threads=1; \
	python3 scripts/test-task-migration.py verify '$(abspath $(TARGET_DIR))'

.PHONY: test-signatures test-landing
test-signatures:
	@python3 -I scripts/test-verify-signatures.py

test-landing:
	@python3 -I scripts/test-landing.py
.PHONY: test-task-import-transport
TASK_IMPORT_FILTER ?= task_migration
test-task-import-transport:
	@set -eu; . ./scripts/cache-env.sh; \
	python3 scripts/test-task-migration.py prepare '$(abspath $(TARGET_DIR))'; \
	export BBOXY_TASK_MIGRATION_INPUT='$(abspath $(TARGET_DIR))/task-migration/input'; \
	export BBOXY_TASK_MIGRATION_OUTPUT='$(abspath $(TARGET_DIR))/task-migration'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp $(TASK_IMPORT_FILTER) -- --nocapture; \
	python3 scripts/test-task-migration.py verify '$(abspath $(TARGET_DIR))'

.PHONY: test-clipboard-isolation check-clipboard-isolation
CLIPBOARD_TEST_FILTER ?= clipboard::tests
test-clipboard-isolation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-term-desktop-macos --target-dir '$(TARGET_DIR)' --lib $(CLIPBOARD_TEST_FILTER) -- --nocapture

check-clipboard-isolation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-term-desktop-macos --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-role-identity check-role-identity parity-role-identity
test-role-identity:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	cargo test --locked -p bboxy450-book --target-dir '$(TARGET_DIR)' --test fixtures

check-role-identity:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked --all-targets -p bboxy450-ui -p bboxy450-book -p $(APP) --target-dir '$(TARGET_DIR)' -- -D warnings

parity-role-identity:
	@set -eu; . ./scripts/cache-env.sh; \
	export BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))'; \
	cargo build --locked -p bboxy450-book --target-dir '$(TARGET_DIR)'; \
	identity_out='$(TARGET_DIR)/parity-role-identity'; \
	if [ -n '$(ROLE_IDENTITY_CASES_FILE)' ]; then \
	  identity_cases=$$(cat '$(ROLE_IDENTITY_CASES_FILE)'); identity_out="$$identity_out-recheck"; \
	else \
	  identity_cases=$$('$(TARGET_DIR)/debug/bboxybook' manifest | python3 -c 'import json,sys; xs=json.load(sys.stdin); selected=["{component}/{slug}-{density}-{theme}.png".format(**row) for row in xs if row["component"] in ("seat-row","title-strip","sidebar","message-row","stream")]; assert selected; print(json.dumps(selected))'); \
	fi; \
	PARITY_OUT="$$identity_out" sh scripts/parity.sh --cases "$$identity_cases"

.PHONY: test-cursors
test-cursors:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) pointer_cursor

.PHONY: test-chat-recent test-chat-sections test-chat-sections-e2e test-chat-notices check-chat-sections test-chat-ux parity-chat
test-chat-recent:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' --test gui_wire_golden; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib recent; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test gui_events retained_message_and_receipt_pages_hydrate_rows_without_replaying_seat_state -- --exact; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib quiet_history_tests; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test messaging_ui

test-chat-sections-e2e:
	@set -eu; . ./scripts/cache-env.sh; \
	BBOXY_CHAT_SECTIONS_EVIDENCE='$(abspath $(TARGET_DIR))/chat-sections' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test chat_sections --test chat_notices -- --ignored --nocapture --test-threads=$(SWARM_TEST_THREADS)

test-chat-notices:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test chat_notices -- --ignored --nocapture

test-chat-sections:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' --test gui_wire_golden --test ui_sidebar_sections; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test ctl; \
	$(MAKE) test-chat-sections-e2e

check-chat-sections:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-ui -p bboxy450-proto -p bboxy450-doc -p bboxy450-mux -p bboxy450-book -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

test-chat-ux:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib local_time; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib quiet_history; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib sidebar_split_preferences; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) composer_host_tests; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) composer_storage_success; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) pointer_cursor; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test messaging_ui; \
	BBOXY_VERTICAL_EVIDENCE='$(abspath $(TARGET_DIR))/sidebar-vertical' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir sidebar_vertical_splits_resize_save_and_restore_without_restarting_harnesses -- --ignored --exact --nocapture

parity-chat:
	@set -eu; . ./scripts/cache-env.sh; \
	export BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))'; \
	cargo build --locked -p bboxy450-book --target-dir '$(TARGET_DIR)'; \
	chat_cases=$$('$(TARGET_DIR)/debug/bboxybook' manifest | python3 -c 'import json,sys; xs=json.load(sys.stdin); selected=["{component}/{slug}-{density}-{theme}.png".format(**row) for row in xs if row["component"] in ("compose","stream","message-row")]; assert selected; print(json.dumps(selected))'); \
	PARITY_OUT='$(TARGET_DIR)/parity-chat' sh scripts/parity.sh --cases "$$chat_cases"

.PHONY: test-settings-window
test-settings-window:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) settings_; \
	BBOXY_SETTINGS_WINDOW_EVIDENCE='$(abspath $(TARGET_DIR))/settings-window' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir settings_uses_full_window_height_after_resizes -- --ignored --exact --nocapture

.PHONY: parity-settings
parity-settings:
	@set -eu; . ./scripts/cache-env.sh; \
	export BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))'; \
	cargo build --locked -p bboxy450-book --target-dir '$(TARGET_DIR)'; \
	settings_cases=$$('$(TARGET_DIR)/debug/bboxybook' manifest | python3 -c 'import json,sys; xs=json.load(sys.stdin); selected=["{component}/{slug}-{density}-{theme}.png".format(**row) for row in xs if row["component"] in ("settings","harness-icon","launcher")]; assert selected; print(json.dumps(selected))'); \
	sh scripts/parity.sh --cases "$$settings_cases"

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
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir -- --ignored --skip settings_codex_service_fixture --test-threads=$(SWARM_TEST_THREADS); \
	$(MAKE) test-chat-sections-e2e

.PHONY: test-make-isolation
test-make-isolation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test native_signing

.PHONY: test-signed-mentions
test-signed-mentions:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --tests --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --lib mentions --target-dir '$(TARGET_DIR)' -- --nocapture; \
	cargo test --locked -p bboxy450-mux --lib first_held_operator_delivers_an_authorless_intent_but_cannot_mute --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --test identity --test identity_local --test identity_io --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --test messaging --test mcp mention --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --test attention_policy --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --test messaging_flow --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --test swarm_workdir signed_mentions_refresh_retains_sessions_and_shell_state --target-dir '$(TARGET_DIR)' -- --ignored --nocapture; \
	cargo clippy --locked -p bboxy450-proto -p bboxy450-mux -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-gui-events
.PHONY: test-document-protocol
test-document-protocol:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --test ui_document_geometry --test gui_wire_golden --target-dir '$(TARGET_DIR)'

.PHONY: test-documents-night
.PHONY: test-documents-activation
test-documents-activation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --lib private_document_identity_tests --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --test swarm_workdir document_observations_refresh_independent_markers_and_retain_native_sessions --target-dir '$(TARGET_DIR)' -- --ignored --exact --nocapture

test-documents-night:
	@set -eu; . ./scripts/cache-env.sh; \
	export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_OPT_LEVEL=1 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true; \
	cargo build --locked -p $(APP) --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-proto --test ui_document_geometry --test gui_wire_golden --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-doc --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-ui --test documents --test document_host --test flow --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p $(APP) --test documents --test documents_observation --target-dir '$(TARGET_DIR)'; \
	python3 -I -B scripts/e2e/documents_night.py --bin '$(abspath $(TARGET_DIR))/debug/$(APP)' --source '$(CURDIR)' --out '$(abspath $(TARGET_DIR))/documents-runs/'$$(date -u +%Y%m%dT%H%M%SZ); \
	$(MAKE) check-documents

.PHONY: check-documents
check-documents:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-doc -p bboxy450-ui -p bboxy450-proto -p bboxy450-mux -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: parity-documents
parity-documents:
	@set -eu; . ./scripts/cache-env.sh; \
	export BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))' BBOXYBOOK_GALLERY=documents; \
	node scripts/e2e/test_document_headings.mjs '$(TARGET_DIR)/document-heading-reference'; \
	python3 -I -B scripts/release-inputs.py; \
	PARITY_OUT='$(TARGET_DIR)/parity-documents' sh scripts/parity.sh

.PHONY: test-documents-controls
test-documents-controls:
	@python3 -I -B scripts/e2e/documents_negative.py --target '$(TARGET_DIR)' --out '$(TARGET_DIR)/document-controls/'$$(date -u +%Y%m%dT%H%M%SZ)

test-gui-events:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib bus::gui_tests -- --nocapture; \
	BBOXY_GUI_EVENTS_EVIDENCE=/tmp cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test gui_events -- --nocapture; \
	BBOXY_GUI_EVENTS_EVIDENCE=/tmp cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test gui_events -- --ignored --exact retained_membership_over_64_keeps_events_chat_and_tasks_live --nocapture --test-threads=1

test-boundaries:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test seat_wiring; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test ctl --test native_signing; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib login_shell_exec -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib cockpit::shell_path::tests -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib refresh_replaces_a_confirmed_failed_process -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test cockpit -- --ignored --test-threads=1

.PHONY: test-fifty-seat-startup
test-fifty-seat-startup: build-fifty-seat-probe
	@set -eu; . ./scripts/cache-env.sh; \
	'$(TARGET_DIR)/night-fifty-seat-startup/pty-syscall-probe' --check-accounting; \
	cargo test --locked -p bboxy450-harness-host --target-dir '$(TARGET_DIR)' --test supervisor spawn_errors_name_the_failed_paths_without_arguments_or_environment -- --ignored --exact --nocapture; \
	cargo test --locked -p bboxy450-harness-host --target-dir '$(TARGET_DIR)' --test supervisor fifty_seats_share_one_supervisor_poller -- --ignored --exact --nocapture

.PHONY: build-fifty-seat-probe diagnose-fifty-seat-pty
build-fifty-seat-probe:
	@set -eu; test "$$(uname -s)" = Darwin; mkdir -p '$(TARGET_DIR)/night-fifty-seat-startup'; \
	xcrun clang -std=c11 -O2 -Wall -Wextra -Werror -pthread crates/bbt-supervisor/tests/fixtures/pty_syscall_probe.c -o '$(TARGET_DIR)/night-fifty-seat-startup/pty-syscall-probe'

diagnose-fifty-seat-pty: build-fifty-seat-probe
	@set -eu; \
	python3 -c 'import subprocess,sys; sys.exit(subprocess.run([sys.argv[1]], timeout=30).returncode)' '$(TARGET_DIR)/night-fifty-seat-startup/pty-syscall-probe'

bundle-files: check-licenses compile
	@set -eu; test "$$(uname -s)" = Darwin || { echo 'macOS is required' >&2; exit 2; }; \
	case '$(BUNDLE)' in *.app) ;; *) echo 'BUNDLE must end in .app' >&2; exit 2 ;; esac; \
	mkdir -p '$(BUNDLE)/Contents/MacOS' '$(BUNDLE)/Contents/Resources'; \
	cp packaging/macos/Info.plist '$(BUNDLE)/Contents/Info.plist'; \
	cp '$(TARGET_DIR)/$(PROFILE)/$(APP)' '$(BUNDLE)/Contents/MacOS/$(APP)'; \
	cp LICENSE NOTICE ATTRIBUTION.md LICENSE-MIT LICENSE-APACHE packaging/macos/bboxy.icns '$(BUNDLE)/Contents/Resources/'; \
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

.PHONY: sync-licenses sync-source-notices check-licenses test-licenses test-licenses-fixtures test-license-integration check-license-packages
sync-licenses:
	@python3 -I scripts/license-packages.py --sync

sync-source-notices:
	@python3 -I scripts/source-notices.py --sync

check-licenses:
	@sh scripts/licenses.sh

test-licenses-fixtures:
	@python3 -I scripts/test-licenses.py

test-licenses: test-licenses-fixtures check-licenses
	@python3 -I scripts/test-source-notices.py

test-license-integration:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-doc --target-dir '$(TARGET_DIR)' --test corpus; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test native_signing

check-license-packages:
	@python3 -I scripts/license-packages.py --packages

package-crate: check-licenses
	@set -eu; test -n '$(CRATE)' || { echo 'Usage: make package-crate CRATE=package-name' >&2; exit 2; }; \
	python3 -I scripts/license-packages.py --packages --package '$(CRATE)'; \
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
	@set -eu; test "$$(gh api user --jq .login)" = '$(PUSH_IDENTITY)'; test -z "$$(git status --porcelain)"; \
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

.PHONY: test-adapter-startup test-harness-startup check-harness-startup fmt-harness-startup
ADAPTER_STARTUP_TEST ?= adapters_real_zsh_overlay_reports_prompt_133_b_and_command_boundary
test-adapter-startup:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)' --lib process_start_ -- --nocapture; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters adapters_zsh_startup_ -- --nocapture; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters '$(ADAPTER_STARTUP_TEST)' -- --exact --nocapture

fmt-harness-startup:
	@rustfmt --edition 2024 --config skip_children=true crates/bboxy450-harness-wake-up/src/launch.rs crates/bboxy450-harness-wake-up/src/lib.rs crates/bboxy450-harness-wake-up/src/native.rs crates/bboxy450-harness-wake-up/tests/adapters.rs crates/bboxy450-harness-wake-up/tests/ready_exec.rs crates/bboxy450-harness-wake-up/tests/codex_proxy.rs crates/bbt-mux/src/adapters/launch.rs crates/bbt-mux/src/adapters/mapping.rs crates/bbt-mux/src/logic.rs crates/bbt-mux/src/logic/ring_batch_tests.rs

test-harness-startup:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib login_shell_exec -- --include-ignored

.PHONY: test-terminal-colors test-terminal-colors-native
test-terminal-colors:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters terminal_colors_ -- --nocapture

test-terminal-colors-native: build
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)' --codex '$(CODEX_BIN)' --color-check

check-harness-startup:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked --all-targets -p bboxy450-harness-wake-up -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-cli-error-isolation
test-cli-error-isolation:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test diagnostics --test ctl; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test headless notification_recording_requires_a_test_account_before_startup -- --exact

.PHONY: test-harness-native
test-harness-native: build
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)' --codex '$(CODEX_BIN)'
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)' --codex '$(CODEX_BIN)' --cold --shim

.PHONY: test-harness-fixture
test-harness-fixture: build
	@set -eu; test -n '$(HARNESS_FIXTURE)' && test -x '$(HARNESS_FIXTURE)' || { echo 'Set HARNESS_FIXTURE to a local executable driver' >&2; exit 2; }; \
	'$(HARNESS_FIXTURE)' '$(abspath $(TARGET_DIR))/debug/$(APP)'

.PHONY: test-harness-proxy fmt-harness-proxy
fmt-harness-proxy:
	@rustfmt --edition 2024 --config skip_children=true crates/bboxy450-harness-wake-up/src/launch.rs crates/bboxy450-harness-wake-up/src/native.rs crates/bboxy450-harness-wake-up/tests/adapters.rs crates/bboxy450-harness-wake-up/tests/codex_proxy.rs

test-harness-proxy:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)' --lib codex_proxy_relay_tests; \
	cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)' --test codex_proxy

.PHONY: test-wake-causes test-wake-journal-failure
test-wake-journal-failure:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters wake_causes_failed_result_journal_fences_same_id_native_and_pty -- --exact

test-wake-causes:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-harness-wake-up --target-dir '$(TARGET_DIR)' --lib --test adapters; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib adapters::; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib wake_causes_; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test adapters; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test messaging wake_causes_; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp mcp_seat_controls_enforce_catalogue_scope_generation_and_strict_arguments -- --exact; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir task_migration_catalogue_upgrade_retains_all_seats_and_shell_state -- --ignored --exact --nocapture --test-threads=1; \
	cargo clippy --locked -p bboxy450-harness-wake-up -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' --all-targets -- -D warnings

.PHONY: test-codex-rendering
test-codex-rendering: build test-harness-proxy
	@python3 -I -B scripts/e2e/shared_codex.py '$(TARGET_DIR)/debug/$(APP)' --render-check

.PHONY: test-checkpoint-resize
test-checkpoint-resize:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-term-core --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-harness-host --target-dir '$(TARGET_DIR)' --test checkpoint_resize

.PHONY: test-input-latency test-input-backend test-input-ui test-input-layout profile-input-ui check-input test-terminal-replay check-terminal-replay
test-input-latency:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test input_latency -- --nocapture

test-input-backend:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib logic::idle_command_tests; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib seat_socket::input_check_tests; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib seat_socket::gui_tests::busy_gui_poll_and_locked_cursor_keep_the_session_and_queued_events -- --exact
	$(MAKE) test-input-latency test-terminal-replay

test-input-ui:
	$(MAKE) test-cursors test-terminal-selection test-sidebar-resize test-chat-sections
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib chat_scroll::tests; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) sidebar_chat::tests
	$(MAKE) test-input-layout

test-input-layout:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) conversation_cache::tests -- --nocapture

check-input:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked --all-targets -p bboxy450-harness-wake-up -p bboxy450-ui -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' -- -D warnings

profile-input-ui:
	@set -eu; . ./scripts/cache-env.sh; BBOXY_INPUT_PROFILE=1 cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) lazy_projection_tests::input_frame_cpu_profile -- --exact --nocapture

test-terminal-replay:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test checkpoint_scheduling -- --nocapture; \
	R5G_SEATS=4 cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test recovery checkpoint_bounded_recovery -- --exact --nocapture; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test ctl ctl_screen_input_and_wait_drive_a_seat -- --exact --nocapture

check-terminal-replay:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked --all-targets -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' -- -D warnings

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
.PHONY: test-swarm-metadata check-swarm-metadata format-swarm-metadata
test-swarm-metadata:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib window_journal; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test swarm_metadata --test storage_bus_index; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP); \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir oracle_mcp_splits_focuses_and_reattaches_without_restarting_harnesses -- --ignored --exact --nocapture

check-swarm-metadata:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-proto -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' --all-targets -- -D warnings

format-swarm-metadata:
	@rustfmt --edition 2024 --config skip_children=true crates/bbt-proto/src/registry.rs crates/bbt-proto/src/gui/swarm.rs crates/bbt-proto/src/gui/mod.rs crates/bbt-mux/src/registry.rs crates/bbt-mux/src/window_journal.rs crates/bbt-mux/src/logic.rs crates/bbt-mux/src/seat_socket.rs crates/bbt-mux/tests/swarm_metadata.rs crates/bboxy450-term/src/live.rs crates/bboxy450-term/src/cockpit.rs crates/bboxy450-term/tests/swarm_workdir.rs

test-spawn-wire:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) launch_parameters_wire -- --nocapture

.PHONY: test-navigation check-navigation
.PHONY: test-new-swarm format-new-swarm
.PHONY: test-swarm-switch check-swarm-switch
.PHONY: test-sidebar-accordions check-sidebar-accordions
.PHONY: test-settings-role-picker-ui test-settings-role-picker check-settings-role-picker
.PHONY: test-settings-night
.PHONY: test-notification-copy
test-notification-copy:
	@$(MAKE) compile PROFILE=debug
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-desktop-notifications-macos --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings; \
	python3 -I -B scripts/e2e/notification_copy_night.py --binary '$(TARGET_DIR)/debug/$(APP)' --target-dir '$(TARGET_DIR)' --out '$(TARGET_DIR)/night-notification-copy'

test-settings-night:
	@$(MAKE) compile PROFILE=debug
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' --test ui_notification_foreground; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib ctl::tests::every_verb_maps_to_exactly_one_window_action -- --exact; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) notification_foreground_tests; \
	python3 -I -B scripts/e2e/settings_night.py --binary '$(TARGET_DIR)/debug/$(APP)' --out '$(TARGET_DIR)/night-settings-acceptance/verdicts'

test-settings-role-picker-ui:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test settings_roles; \
	cargo clippy --locked -p bboxy450-ui --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

test-settings-role-picker: test-settings-role-picker-ui
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir settings_harness_default_role_survives_complete_clicks_and_model_publication -- --ignored --exact --nocapture

check-settings-role-picker:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-ui -p bboxy450-doc -p bboxy450-book -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-settings-roles check-settings-roles test-settings-roles-e2e
.PHONY: test-settings-role-runtime
test-settings-role-runtime:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib role_catalog; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp --test knowledge_provider

test-settings-roles:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib role_catalog; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test settings --test settings_roles --test settings_destinations --test provider_settings --test cursor; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP) --test settings

check-settings-roles:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-proto -p bboxy450-mux -p bboxy450-ui -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

test-settings-roles-e2e:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test provider_gui gui_spawn_roles_follow_the_catalog_and_default_shells_to_worker -- --ignored --exact --nocapture; \
	BBOXY_SETTINGS_EVIDENCE='$(abspath $(TARGET_DIR))/settings-roles' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir settings_roles_and_harness_policies_persist_and_reach_native_launches -- --ignored --exact --nocapture

test-sidebar-accordions:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib window_journal; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	BBOXY_ACCORDION_EVIDENCE='$(abspath $(TARGET_DIR))/sidebar-accordions' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir sidebar_accordions_toggle_without_focus_changes_and_survive_ui_restart -- --ignored --exact --nocapture

check-sidebar-accordions:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-proto -p bboxy450-mux -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

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

.PHONY: test-seat-mute
test-seat-mute:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test seat_rows; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test messaging mute; \
	BBOXY_SWARM_EVIDENCE='$(abspath $(TARGET_DIR))/seat-mute' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir seat_mute_buttons_and_shortcut_keep_operator_control_and_survive_ui_restart -- --ignored --exact --nocapture

.PHONY: test-close-shortcut
.PHONY: test-workspace-roots
test-workspace-roots:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test workspace_roots --test knowledge --test knowledge_provider; \
	cargo clippy --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --all-targets -- -D warnings

test-close-shortcut:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir command_w_closes_only_the_focused_seat_and_undo_keeps_the_same_process -- --ignored --exact --nocapture

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

.PHONY: test-desktop
test-desktop:
	@set -eu; . ./scripts/cache-env.sh; \
	sh scripts/layering.sh; \
	sh scripts/cross-os.sh; \
	cargo test --locked -p bboxy450-term-desktop-core -p bboxy450-term-desktop-macos -p bboxy450-term-desktop --target-dir '$(TARGET_DIR)'; \
	cargo clippy --locked -p bboxy450-term-desktop-core -p bboxy450-term-desktop-macos -p bboxy450-term-desktop --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings; \
	if [ "$$(uname -s)" = Darwin ]; then cargo run --locked -p bboxy450-term-desktop-macos --example native_probe --target-dir '$(TARGET_DIR)'; fi

.PHONY: test-desktop-app
test-desktop-app:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib cockpit::desktop; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) desktop_window; \
	cargo clippy --locked -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-desktop-lifecycle
test-desktop-lifecycle:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test desktop daemon_survives_ui_detach_and_reopens_the_same_running_harness -- --ignored --exact --nocapture

.PHONY: test-ui-overlays
test-ui-overlays:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test overlays_interaction; \
	cargo clippy --locked -p bboxy450-ui --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-app-overlays
test-app-overlays:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' --test ui_layout_text; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir context_menu_rename_is_trapped_validated_and_persistent -- --ignored --exact --nocapture; \
	cargo clippy --locked -p bboxy450-proto -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-seat-actions test-seat-actions-backend test-seat-actions-reconnect check-seat-actions check-seat-actions-backend test-task-contract
test-task-contract:
	@set -eu; . ./scripts/cache-env.sh; cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib plugins::check::tests::task_contract_properties_derive_from_the_current_native_tool_catalogue -- --exact

test-seat-actions-backend: test-task-contract
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-plugin --target-dir '$(TARGET_DIR)' --test data_source; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib task_role_tests; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test provider_gui seat_controls_fence_generations_scope_and_kill_only_the_target_child -- --ignored --exact --nocapture; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib seat_socket::gui_tests::seat_control_role_snapshot_precedes_receipt_without_events -- --ignored --exact --nocapture

test-seat-actions-reconnect:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) live::tests::disconnected_mutations_wait_for_explicit_retry_while_pane_controls_survive -- --exact; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) live::tests::mute_snapshot_preserves_its_receipt_and_the_next_operation -- --exact

test-seat-actions: test-seat-actions-backend test-seat-actions-reconnect
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) app_overlays; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir seat_context_actions_ -- --ignored --nocapture

check-seat-actions:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-plugin -p bboxy450-proto -p bboxy450-mux -p $(APP) -p bboxy450-book --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

check-seat-actions-backend:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-plugin -p bboxy450-proto -p bboxy450-mux --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-workspace-boundary check-workspace-boundary format-workspace-boundary
test-workspace-boundary:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib mcp::journal; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib knowledge::seat_reads; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib knowledge::responsiveness_tests; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test mcp --test knowledge_provider; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP); \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test knowledge_e2e --test documents --test ctl_records --test messaging_flow; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir oracle_mcp_splits_focuses_and_reattaches_without_restarting_harnesses -- --ignored --exact --nocapture

check-workspace-boundary:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-proto -p bboxy450-mux -p $(APP) --target-dir '$(TARGET_DIR)' --all-targets -- -D warnings

format-workspace-boundary:
	@rustfmt --edition 2024 --config skip_children=true crates/bbt-proto/src/gui/swarm.rs crates/bbt-proto/src/gui/mod.rs crates/bbt-proto/src/gui/record.rs crates/bbt-proto/src/gui/task.rs crates/bbt-proto/tests/gui_wire_golden.rs crates/bbt-mux/src/mcp/journal.rs crates/bbt-mux/src/knowledge/seat_reads.rs crates/bbt-mux/src/knowledge/responsiveness_tests.rs crates/bbt-mux/src/logic.rs crates/bbt-mux/src/seat_socket.rs crates/bbt-mux/tests/mcp.rs crates/bboxy450-term/src/cockpit.rs crates/bboxy450-term/src/ctl.rs crates/bboxy450-term/src/documents.rs crates/bboxy450-term/src/knowledge.rs crates/bboxy450-term/src/live.rs crates/bboxy450-term/tests/knowledge_e2e.rs crates/bboxy450-term/tests/messaging_flow.rs crates/bboxy450-term/tests/swarm_workdir.rs

.PHONY: test-workspace-routing
# Private cwd roots, stale completions/replays, provider responsiveness and scoped wakes.
test-workspace-routing:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib workspace_; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test workspace_roots --test knowledge --test knowledge_provider --test mcp --test messaging_suppression --test recovery --test logic --test provenance; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib knowledge_responsiveness_tests; \
	cargo clippy --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --all-targets -- -D warnings

.PHONY: test-scoped-workspace-frontend
# Pure frontend checks do not require a running app or provider.
test-scoped-workspace-frontend:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib --bin $(APP); \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test documents --test messaging_ui --test knowledge_e2e; \
	cargo clippy --locked -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: check-scoped-workspace-frontend
check-scoped-workspace-frontend:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-scoped-workspaces test-overlay-captures
test-scoped-workspaces: test-scoped-workspace-frontend
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir native_documents_capture_swarm_roots_and_drop_a_held_reply_after_cwd_change -- --ignored --exact --nocapture

test-overlay-captures:
	@set -eu; . ./scripts/cache-env.sh; \
	for theme in light dark; do \
	  BBOXY_UI_OVERLAY_THEME="$$theme" BBOXY_UI_OVERLAY_EVIDENCE='$(abspath $(TARGET_DIR))/overlay-evidence/'"$$theme" cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir context_menu_rename_is_trapped_validated_and_persistent -- --ignored --exact --nocapture; \
	done

.PHONY: test-ctl-workspaces
test-ctl-workspaces:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib ctl::workspace; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test ctl; \
	cargo clippy --locked -p $(APP) --target-dir '$(TARGET_DIR)' --all-targets -- -D warnings

.PHONY: test-terminal-font check-terminal-font format-terminal-font
format-terminal-font: fmt-spawn

test-terminal-font:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-term-desktop-core --target-dir '$(TARGET_DIR)' terminal_font; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib terminal_font; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) terminal_font; \
	BBOXY_FONT_EVIDENCE='$(abspath $(TARGET_DIR))/terminal-font' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir terminal_font_ -- --ignored --nocapture; \
	if [ "$$(uname -s)" = Darwin ]; then cargo run --locked -p bboxy450-term-desktop-macos --example native_probe --target-dir '$(TARGET_DIR)'; fi

check-terminal-font:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-proto -p bboxy450-ui -p bboxy450-term-desktop-core -p bboxy450-term-desktop-macos -p bboxy450-term-desktop -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-image-paste test-image-paste-wire test-image-paste-desktop test-image-paste-ui test-image-paste-app test-image-paste-e2e check-image-paste check-image-paste-wire
test-image-paste: test-image-paste-wire test-image-paste-desktop test-image-paste-ui test-image-paste-app test-image-paste-e2e

test-image-paste-wire:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-proto --target-dir '$(TARGET_DIR)' --test ui_image_paste --test gui_wire_golden

check-image-paste-wire:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-proto --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

test-image-paste-desktop:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-term-desktop-core --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-term-desktop-macos --target-dir '$(TARGET_DIR)' clipboard_

test-image-paste-ui:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-ui --target-dir '$(TARGET_DIR)' --test composer image_markers

test-image-paste-app:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib composer_images; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib ctl::ui::tests::image_paste; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test messaging_ui image_attachment; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) image_paste

test-image-paste-e2e:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir image_paste_ctl_owns_atomic_marker_and_recipient_png_survives_ui_and_logic_restart -- --ignored --exact --nocapture

check-image-paste:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-proto -p bboxy450-mux -p bboxy450-term-desktop-core -p bboxy450-term-desktop-macos -p bboxy450-term-desktop -p bboxy450-ui -p bboxy450-doc -p bboxy450-book -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-color-emoji check-color-emoji
test-color-emoji:
	@set -eu; . ./scripts/cache-env.sh; \
	BBOXY_EMOJI_EVIDENCE='$(abspath $(TARGET_DIR))/emoji-evidence' cargo test --locked -p bboxy450-render --target-dir '$(TARGET_DIR)'

check-color-emoji:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-render --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-color-emoji-swarm check-color-emoji-all
test-color-emoji-swarm:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test swarm_metadata; \
	BBOXY_UI_OVERLAY_EVIDENCE='$(abspath $(TARGET_DIR))/emoji-evidence/app' cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir context_menu_rename_is_trapped_validated_and_persistent -- --ignored --exact --nocapture

check-color-emoji-all:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo clippy --locked -p bboxy450-render -p bboxy450-ui-core -p bboxy450-ui -p bboxy450-mux -p bboxy450-term -p bboxy450-book --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings
.PHONY: test-terminal-selection check-terminal-selection
test-terminal-selection:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-term-desktop-core --target-dir '$(TARGET_DIR)'; \
	cargo test --locked -p bboxy450-term-desktop-macos --target-dir '$(TARGET_DIR)' clipboard_; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib terminal_selection; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib keyboard_adapter_uses_neutral_core_and_live_modes; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib every_verb_maps_to_exactly_one_window_action; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --bin $(APP) selection; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test swarm_workdir terminal_selection_ -- --ignored --nocapture

check-terminal-selection:
	@set -eu; . ./scripts/cache-env.sh; cargo clippy --locked -p bboxy450-proto -p bboxy450-term-desktop-core -p bboxy450-term-desktop-macos -p bboxy450-term-desktop -p $(APP) --all-targets --target-dir '$(TARGET_DIR)' -- -D warnings

.PHONY: test-harness-vocabulary
test-harness-vocabulary:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-harness-host --target-dir '$(TARGET_DIR)' --test vocabulary
