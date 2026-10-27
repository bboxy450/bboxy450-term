# Canonical build/test/install/release interface. See `make help`.
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
export CARGO_PROFILE_DEV_DEBUG ?= 0
export CARGO_PROFILE_TEST_DEBUG ?= 0
export CARGO_PROFILE_DEV_OPT_LEVEL ?= 1
export CARGO_PROFILE_TEST_OPT_LEVEL ?= 1

.PHONY: help build build-release compile test check test-boundaries smoke bundle-files bundle bundle-unsigned install install-unsigned release package-crate publish-release push

help:
	@printf '%s\n' \
	  'make build / build-release       Build the app (debug / optimized)' \
	  'make test                       Full local CI; no operator input' \
	  'make test-boundaries             Explicit shell/lifecycle process regressions' \
	  'make smoke                      Unsigned private install + headless smoke' \
	  'make install                    Release app, Apple identity signed, ~/Applications' \
	  'make install-unsigned            Debug app, ad-hoc only, target/install-unsigned' \
	  'make release                    Signed app ZIP, SHA256 and detached GPG signature' \
	  'make package-crate CRATE=name    Package and GPG-sign one crate, without publishing' \
	  'make publish-release            Signed tag + GitHub app release (authorization required)' \
	  'make push                       Push signed, tested HEAD to main (authorization required)' \
	  'Overrides: PROFILE=debug|release TARGET_DIR=... INSTALL_DIR=... UNSIGNED_INSTALL_DIR=... CI_LOG=...'

build:
	@$(MAKE) compile PROFILE=debug

build-release:
	@$(MAKE) compile PROFILE=release

compile:
	@set -eu; case '$(PROFILE)' in debug) flag='' ;; release) flag='--release' ;; *) echo 'PROFILE must be debug or release' >&2; exit 2 ;; esac; \
	. ./scripts/cache-env.sh; cargo build --locked -p $(APP) --target-dir '$(TARGET_DIR)' $$flag

test check:
	@set -eu; mkdir -p "$$(dirname '$(CI_LOG)')"; \
	BBOXY_GATE_SLOT=none BBOXY_CARGO_TARGET_DIR='$(abspath $(TARGET_DIR))' sh scripts/ci-local.sh >'$(CI_LOG)' 2>&1 || { code=$$?; tail -60 '$(CI_LOG)'; exit $$code; }; \
	tail -12 '$(CI_LOG)'; printf 'Full gate log: %s\n' '$(CI_LOG)'

test-boundaries:
	@set -eu; . ./scripts/cache-env.sh; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --test seat_wiring; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --test ctl --test native_signing; \
	cargo test --locked -p bboxy450-mux --target-dir '$(TARGET_DIR)' --lib login_shell_exec -- --include-ignored; \
	cargo test --locked -p $(APP) --target-dir '$(TARGET_DIR)' --lib cockpit::shell_path::tests -- --include-ignored; \
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

smoke: PROFILE=debug
smoke: install-unsigned
	@set -eu; run=$$(mktemp -d "$${TMPDIR:-/tmp}/bboxy-make-smoke.XXXXXX"); \
	mkdir "$$run/repository" "$$run/home"; \
	if env -i PATH="$$PATH" TERM="$${TERM:-xterm-256color}" LANG="$${LANG:-en_US.UTF-8}" TMPDIR="$${TMPDIR:-/tmp}" \
	  HOME="$$run/home" CODEX_HOME="$$run/home/.codex" CLAUDE_CONFIG_DIR="$$run/home/.claude" ZDOTDIR="$$run/home" \
	  '$(abspath $(UNSIGNED_INSTALL_DIR))/$(APP).app/Contents/MacOS/$(APP)' \
	  --headless --smoke --ctl --unattended --test-account "$$run/account" --run-dir "$$run/run" --repository "$$run/repository" </dev/null; then \
	  rm -r -- "$$run"; \
	else code=$$?; printf 'Smoke failed; private evidence retained at %s\n' "$$run" >&2; exit $$code; fi

release: bundle
	@set -eu; mkdir -p '$(DIST_DIR)'; \
	ditto -c -k --sequesterRsrc --keepParent '$(BUNDLE)' '$(ARCHIVE)'; \
	shasum -a 256 '$(ARCHIVE)' >'$(ARCHIVE).sha256'; \
	gpg --batch --yes --local-user '$(GPG_KEY)' --armor --detach-sign '$(ARCHIVE)'; \
	gpg --verify '$(ARCHIVE).asc' '$(ARCHIVE)'; printf 'Signed release artifacts: %s{,.sha256,.asc}\n' '$(ARCHIVE)'

package-crate:
	@set -eu; test -n '$(CRATE)' || { echo 'Usage: make package-crate CRATE=package-name' >&2; exit 2; }; \
	. ./scripts/cache-env.sh; cargo package --locked -p '$(CRATE)' --target-dir '$(TARGET_DIR)'; \
	version=$$(cargo pkgid -p '$(CRATE)' --locked | sed 's/.*[@#]//'); \
	file='$(TARGET_DIR)/package/$(CRATE)-'"$$version"'.crate'; \
	gpg --batch --yes --local-user '$(GPG_KEY)' --armor --detach-sign "$$file"; gpg --verify "$$file.asc" "$$file"

publish-release: release
	@set -eu; test "$$(gh api user --jq .login)" = bboxy450; test -z "$$(git status --porcelain)"; \
	grep -F "ci-local: result $$(git rev-parse HEAD) normal 0" '$(CI_LOG)' >/dev/null; \
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
	grep -F "ci-local: result $$(git rev-parse HEAD) normal 0" '$(CI_LOG)' >/dev/null; \
	GIT_TERMINAL_PROMPT=0 git push origin HEAD:main
