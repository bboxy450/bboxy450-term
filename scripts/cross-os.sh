#!/bin/sh
# Cross-OS facade invariant (.bboxy/invariants/cross-os-facade.md, decision #46):
# raw OS bindings live only in per-OS crates (*-macos, *-windows, *-linux, *-ios,
# *-android, *-web) behind an OS-agnostic facade crate. OS-agnostic crates that
# are facades themselves (winit, wgpu) are not bindings. ALLOW lists the bindings
# that predate the rule, by crate:dep; each leaves when its facade arrives
# (bboxy450-mux's core-foundation and security-framework leave with bboxy450-keychain-macos, decision #281).
set -eu
cd "$(dirname "$0")/.."
BINDINGS="block2 core-foundation core-foundation-sys core-graphics core-text cocoa security-framework windows windows-sys zbus dbus web-sys wasm-bindgen jni"
PREFIXES="objc2 windows- winrt ndk tauri-winrt"
ALLOW="bboxy450-term:block2 bboxy450-term:objc2 bboxy450-term:objc2-app-kit bboxy450-term:objc2-foundation bboxy450-term:objc2-metal bboxy450-term:objc2-quartz-core bboxy450-render:objc2-core-foundation bboxy450-render:objc2-core-graphics bboxy450-render:objc2-core-text bboxy450-mux:core-foundation bboxy450-mux:security-framework"
cargo metadata --no-deps --format-version 1 | BINDINGS="$BINDINGS" PREFIXES="$PREFIXES" ALLOW="$ALLOW" python3 -I -c '
import json, os, re, sys
bindings = set(os.environ["BINDINGS"].split()); prefixes = tuple(os.environ["PREFIXES"].split())
allow = set(os.environ["ALLOW"].split()); per_os = re.compile(r"-(macos|windows|linux|ios|android|web)$")
bad = 0
for p in json.load(sys.stdin)["packages"]:
    if per_os.search(p["name"]): continue
    for d in p["dependencies"]:
        n = d["name"]
        if (n in bindings or n.startswith(prefixes)) and p["name"] + ":" + n not in allow:
            print(p["name"] + " -> " + n + ": OS binding outside a per-OS crate (scripts/cross-os.sh)"); bad += 1
print("cross-os: " + ("VIOLATED" if bad else "held"))
sys.exit(1 if bad else 0)
'
