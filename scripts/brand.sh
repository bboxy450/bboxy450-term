#!/bin/sh
# Brand invariant (.bboxy/invariants/bboxy450-prefix.md, decision #52): every
# workspace crate is either named bboxy450-* or has a row in the licensing map
# (.bboxy/specs/licensing.md §1) whose "Publishes as" column names a bboxy450-*
# crate or says internal. A new crate adds its row in the same commit.
set -eu
cd "$(dirname "$0")/.."
cargo metadata --no-deps --format-version 1 | python3 -I -c '
import json, re, sys
rows = {}
for line in open(".bboxy/specs/licensing.md"):
    if not line.startswith("| `"): continue
    cols = [c.strip() for c in line.strip().strip("|").split("|")]
    if len(cols) < 2: continue
    for crate in re.findall(r"`([^`]+)`", cols[0]):
        rows[crate] = cols[1]
bad = 0
for p in json.load(sys.stdin)["packages"]:
    n = p["name"]
    if n.startswith("bboxy450-"): continue
    pub = rows.get(n)
    if pub is None:
        print(n + ": no row in licensing.md §1 (scripts/brand.sh)"); bad += 1; continue
    if "internal" in pub.lower() or any(t.startswith("bboxy450-") for t in re.findall(r"`([^`]+)`", pub)):
        continue
    print(n + ": publishes as " + pub + ", not bboxy450-* (scripts/brand.sh)"); bad += 1
print("brand: " + ("VIOLATED" if bad else "held"))
sys.exit(1 if bad else 0)
'
