# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""The mechanical policy move cannot alter a frozen reference case or budget."""
import json
import subprocess

from corpus import BASE, digest
from references import policy

old_bytes = subprocess.check_output(["git", "show", "3d7706fd014b1e2dc4c86ac3faf6e861aecfa8b8:crates/bboxy450-svg/fixtures/advisories.json"])
assert digest(old_bytes) == "47f64fb8c4ae8f4f503bd9740814b183060b61beb631b9ddc38cb817a20d1974"
old = json.loads(old_bytes)
current = json.loads((BASE / "fixtures/advisories.json").read_text())
old["policy"].update(source=policy()["source"], sha256=policy()["sha256"])
assert old == current, "policy move changed frozen evidence or a numeric budget"
assert current["policy"] == policy(), "SVG loop differs from the shared comparator policy"
print(json.dumps({"unchanged_advisories": len(current["cases"]), "policy": policy()}, sort_keys=True))
