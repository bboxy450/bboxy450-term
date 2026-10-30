#!/bin/sh
# Brand invariant (.bboxy/invariants/bboxy450-prefix.md, decision #52): every
# workspace crate is either named bboxy450-* or has a row in the licensing map
# (.bboxy/specs/licensing.md §1) whose "Publishes as" column names a bboxy450-*
# crate or says internal. A new crate adds its row in the same commit. An internal
# crate must also set publish = false: anything that reaches crates.io is bboxy450-*
# (bboxy-firehose 0.1.0 shipped under the internal exemption).
# Decision 362 classifications live in licensing.md; never guess app coupling.
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
set -eu
cd "$(dirname "$0")/.."
cargo metadata --no-deps --format-version 1 | python3 -I -c '
import json, re, sys
rows = {}
for line in open(".bboxy/specs/licensing.md"):
    if not line.startswith("| `"): continue
    cols = [c.strip() for c in re.split(r"(?<!\\)\|", line.strip().strip("|"))]
    if len(cols) < 2: continue
    for crate in re.findall(r"`([^`]+)`", cols[0]):
        rows[crate] = cols[1]
bad = 0
for p in json.load(sys.stdin)["packages"]:
    n = p["name"]
    if n in {"bboxy450-mux", "bboxy450-proto"}:
        print(n + ": app-specific package needs bboxy450-term-* (decision 362)"); bad += 1
    for target in p["targets"]:
        if "bin" in target["kind"] and target["name"] != n:
            print(n + ": installed binary " + target["name"] + " must match package (decision 362)"); bad += 1
    if n.startswith("bboxy450-"): continue
    pub = rows.get(n)
    if pub is None:
        print(n + ": no row in licensing.md §1 (scripts/brand.sh)"); bad += 1; continue
    if "internal" in pub.lower() and p.get("publish") == []:
        continue
    if "internal" in pub.lower():
        print(n + ": internal but publishable; rename to bboxy450-* or set publish = false (scripts/brand.sh)"); bad += 1; continue
    if any(t.startswith("bboxy450-") for t in re.findall(r"`([^`]+)`", pub)):
        continue
    print(n + ": publishes as " + pub + ", not bboxy450-* (scripts/brand.sh)"); bad += 1
print("brand: " + ("VIOLATED" if bad else "held"))
sys.exit(1 if bad else 0)
'
