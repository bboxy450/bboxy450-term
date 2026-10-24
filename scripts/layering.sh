#!/bin/sh
# Layering invariant (.bboxy/invariants/layering.md): every workspace crate
# depends only on crates earlier in ORDER, so any prefix is publishable alone.
# New crate: add it to ORDER in the same commit.
set -eu
cd "$(dirname "$0")/.."
ORDER="bboxy450-desktop-notifications-macos bboxy450-desktop-notifications-windows bboxy450-desktop-notifications-linux bboxy450-desktop-notifications bbt-yarn-macros bbt-yarn bbt-rt bbt-core bboxy450-harness-wake-up bbt-mermaid bboxy-firehose bbt-ui-model bbt-render bbt-ui bbt-doc bboxybook bbt-supervisor bbt-plugin bbt-proto bbt-mux bboxy450-term"
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
