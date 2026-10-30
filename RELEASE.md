# Release 0.1.0

All 24 workspace crates are released from the same signed monorepo commit. Crate directories, Rust library names and existing executable names remain unchanged. Every internal dependency has a public package name, version `0.1.0` and its development path.

## Publish order

1. `bboxy450-desktop-notifications-linux` — `crates/bboxy450-desktop-notifications-linux`
2. `bboxy450-desktop-notifications-macos` — `crates/bboxy450-desktop-notifications-macos`
3. `bboxy450-desktop-notifications-windows` — `crates/bboxy450-desktop-notifications-windows`
4. `bboxy450-keychain-windows` — `crates/bboxy450-keychain-windows`
5. `bboxy450-term-core` — `crates/bbt-core`
6. `bboxy450-mermaid` — `crates/bbt-mermaid`
7. `bboxy450-plugin` — `crates/bbt-plugin`
8. `bboxy450-rt` — `crates/bbt-rt`
9. `bboxy450-ui-core` — `crates/bbt-ui-model`
10. `bboxy450-yarn-macros` — `crates/bbt-yarn-macros`
11. `bboxy450-term-firehose` — `crates/bboxy-firehose`
12. `bboxy450-desktop-notifications` — `crates/bboxy450-desktop-notifications`
13. `bboxy450-harness-wake-up` — `crates/bboxy450-harness-wake-up`
14. `bboxy450-keychain-file` — `crates/bboxy450-keychain-file`
15. `bboxy450-term-proto` — `crates/bbt-proto`
16. `bboxy450-harness-host` — `crates/bbt-supervisor`
17. `bboxy450-ui` — `crates/bbt-ui`
18. `bboxy450-yarn` — `crates/bbt-yarn`
19. `bboxy450-keychain` — `crates/bboxy450-keychain`
20. `bboxy450-doc` — `crates/bbt-doc`
21. `bboxy450-render` — `crates/bbt-render`
22. `bboxy450-book` — `crates/bboxybook`
23. `bboxy450-term-mux` — `crates/bbt-mux`
24. `bboxy450-term` — `crates/bboxy450-term`

Core retains Apache-2.0 and its vendored notices; the other packages use MIT OR Apache-2.0. Each crate includes its README and licence texts.

## Release artifacts

Run `cargo package --workspace --locked` from a clean signed source. Cargo verifies the unpublished sibling cohort through its temporary registry. Bind every archive to its source commit and SHA-256, sign each `.crate` with the release key, and verify the signed release tag and detached signatures. Publishing, registry login and remote tag publication are performed by the release operator.

## Bundled inputs

The `release-inputs/` subtree keeps each canonical repository path. The fast
`packaged-inputs-current` invariant compares every copy byte for byte, and the
native-input derivation in `scripts/landing.py` selects each owning crate when
its canonical source changes.

| Crate | Canonical inputs under `release-inputs/` |
|---|---|
| `bboxy450-ui-core` | `design/tokens.toml`, `design/style.json` |
| `bboxy450-term` | `lab/web/components.js`, `lab/web/bboxy.png`, `.bboxy/specs/e2e-acceptance.md`, `crates/bbt-ui-model/tests/fixtures/scene15.json` |
| `bboxy450-book` | `lab/web/components.js`, `lab/web/bboxy.png`, `crates/bbt-proto/src/seat_names.json`, `crates/bbt-ui-model/tests/fixtures/scene15.json` |
