#!/bin/sh
# Check the licence map against actual workspace members, including optional Yarn crates.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
python3 - <<'PY'
import json
import re
from pathlib import Path
import subprocess
import sys

result = subprocess.run(
    ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
    capture_output=True, text=True,
)
if result.returncode:
    sys.stderr.write(result.stderr)
    raise SystemExit(result.returncode)
metadata = json.loads(result.stdout)
root = Path(metadata["workspace_root"]).resolve()
members = set(metadata["workspace_members"])
packages = [p for p in metadata["packages"] if p["id"] in members]
errors = []
policy = {}
in_map = False
# Read directory keys and SPDX terms from the authoritative map table.
for line in (root / ".bboxy/specs/licensing.md").read_text(encoding="utf-8").splitlines():
    cells = [cell.strip() for cell in re.split(r"(?<!\\)\|", line.strip().strip("|"))]  # split on unescaped pipes only; \| inside a wikilink alias stays in its cell
    if cells[:3] == ["Crate", "Publishes as", "License"]:
        in_map = True
        continue
    if not in_map:
        continue
    if not line.lstrip().startswith("|"):
        break
    if len(cells) < 4:
        continue
    names = re.findall(r"`([^`]+)`", cells[0])
    terms = cells[2].replace("*", "").replace("`", "")
    expression = re.match(r"^(MIT OR Apache-2\.0|Apache-2\.0)(?:\s|$)", terms)
    if names and expression:
        for name in names:
            if name in policy:
                errors.append(f"licensing map repeats {name!r}")
            policy[name] = expression.group(1)


def require_file(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        errors.append(f"{path.relative_to(root)}: required nonempty licence file is missing or not a regular copy")


for name in ("LICENSE-MIT", "LICENSE-APACHE", "NOTICE"):
    require_file(root / name)
for package in packages:
    directory = Path(package["manifest_path"]).parent.resolve()
    expected = policy.get(directory.name)
    if expected is None:
        errors.append(f"{package['name']}: directory {directory.name!r} is missing from the licensing map")
        continue
    if package["license"] != expected:
        errors.append(f"{package['name']}: expected SPDX {expected!r}, found {package['license']!r}")
    if package["license_file"] is not None:
        errors.append(f"{package['name']}: use the SPDX licence expression, not license-file")
    for name in (("LICENSE-APACHE", "NOTICE") if expected == "Apache-2.0" else ("LICENSE-MIT", "LICENSE-APACHE")):
        require_file(directory / name)
    if directory.name == "bbt-core":
        for name in ("alacritty/LICENSE-APACHE", "vte/LICENSE-MIT", "vte/LICENSE-APACHE"):
            require_file(directory / "src/vendor" / name)
if not packages:
    errors.append("workspace has no members to check")
if errors:
    print("\n".join(errors), file=sys.stderr)
    raise SystemExit(1)
print(f"licenses: {len(packages)} workspace crates, SPDX map and required root/crate files present")
PY
