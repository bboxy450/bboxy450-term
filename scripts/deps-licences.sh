#!/bin/sh
# Dependency licence invariant (.bboxy/invariants/no-copyleft-deps.md, decision #50):
# every dependency in the lockfile offers a permissive licence; copyleft-only or
# unlicensed crates never enter the tree.
set -eu
cd "$(dirname "$0")/.."
cargo metadata --format-version 1 --locked | python3 -I -c '
import json, sys
m = json.load(sys.stdin); ws = {p["id"] for p in m["packages"] if p["source"] is None}
perm = ("MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0", "Unicode-DFS-2016",
        "CC0-1.0", "Unlicense", "0BSD", "BSL-1.0", "MPL-2.0", "OpenSSL", "MIT-0", "CDLA-Permissive-2.0", "NCSA")
bad = 0
for p in m["packages"]:
    if p["id"] in ws: continue
    lic = p.get("license")
    if lic is None:
        print(p["name"] + " " + p["version"] + ": no license field (scripts/deps-licences.sh)"); bad += 1; continue
    alts = [a.strip() for a in lic.replace("/", " OR ").split(" OR ")]
    if not any(a.startswith(perm) for a in alts):
        print(p["name"] + " " + p["version"] + ": " + lic + " offers no permissive licence (scripts/deps-licences.sh)"); bad += 1
print("deps-licences: " + ("VIOLATED" if bad else "held"))
sys.exit(1 if bad else 0)
'
