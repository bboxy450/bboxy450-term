#!/bin/sh
# Layering invariant (.bboxy/invariants/layering.md): every workspace crate
# depends only on crates earlier in ORDER, so any prefix is publishable alone.
# New crate: add it to ORDER in the same commit.
# HK-0: the wake-up kit follows core; its optional runtime assertion is below core.
set -eu
cd "$(dirname "$0")/.."
ORDER="bboxy450-keychain-macos bboxy450-keychain-linux bboxy450-keychain-windows bboxy450-keychain-file bboxy450-keychain bboxy450-desktop-notifications-macos bboxy450-desktop-notifications-windows bboxy450-desktop-notifications-linux bboxy450-desktop-notifications bboxy450-tlc bboxy450-yarn-macros bboxy450-yarn bboxy450-rt bboxy450-term-core bboxy450-harness-wake-up bboxy450-mermaid bboxy450-term-firehose bboxy450-ui-core bboxy450-render bboxy450-ui bboxy450-doc bboxy450-book bboxy450-harness-host bboxy450-plugin bboxy450-term-plugin-sdk bboxy450-term-data_source-gitkb bboxy450-term-proto bboxy450-term-mux bboxy450-term-desktop-core bboxy450-term-desktop-macos bboxy450-term-desktop bboxy450-term"
cargo metadata --no-deps --format-version 1 | ORDER="$ORDER" python3 -I -c '
import json, os, sys
rank = {n: i for i, n in enumerate(os.environ["ORDER"].split())}
bad = 0
for p in json.load(sys.stdin)["packages"]:
    if p["name"] not in rank:
        print(p["name"] + ": not in ORDER (scripts/layering.sh)"); bad += 1; continue
    for d in p["dependencies"]:
        if d["name"] in rank and rank[d["name"]] >= rank[p["name"]]:
            print(p["name"] + " -> " + d["name"] + ": depends upward"); bad += 1
print("layering: " + ("VIOLATED" if bad else "held"))
sys.exit(1 if bad else 0)
'
