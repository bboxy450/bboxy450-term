#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Check source-attached notices; --sync changes comments only, never upstream files."""
import argparse
from collections import Counter
from pathlib import Path
import re
import subprocess
import sys

PROJECT = "https://github.com/bboxy450/bboxy450-term"
POLICY = PROJECT + "/blob/main/ATTRIBUTION.md"
LICENSE = PROJECT + "/blob/main/LICENSE"
SPDX = "LicenseRef-Respectful-Slop-1.0-Draft"
YARN_SOURCE = "https://github.com/Gonzih/kittens-old/tree/293e134526bb2deee96f1197d6268aee64495482"
SOURCE_SUFFIXES = {".rs", ".wgsl", ".py", ".sh", ".js", ".mjs", ".cjs", ".ts", ".tsx", ".css", ".html", ".c", ".h", ".cc", ".cpp", ".m", ".mm"}
UPSTREAM_PREFIXES = (
    "crates/bbt-core/src/vendor/alacritty/",
    "crates/bbt-core/src/vendor/vte/",
    "crates/bboxy-firehose/reference/vendor/",
    "crates/bbt-render/tests/fonts/",
    "crates/bbt-mermaid/tests/fixtures/docs/",
    "crates/bbt-mermaid/tests/fixtures/state/",
)
UPSTREAM_FILES = {"crates/bbt-core/src/keyboard.rs"}
FORK_PREFIXES = ("crates/bbt-yarn/", "crates/bbt-yarn-macros/")
OWNED_FORK_FILES = {"crates/bbt-yarn/src/model.rs", "crates/bbt-yarn/examples/verify.rs", "crates/bbt-yarn/tests/model_projection.rs"}
STRICT_PREFIXES = ("crates/bbt-yarn/tests/ui/",)
CHECKER_FILES = {"scripts/source-notices.py", "scripts/test-source-notices.py"}
# The packaged web source is edited canonically; release-inputs copies are synced
# by their existing owner. Strict JSON/image inputs remain data, never code headers.
PACKAGED_SOURCES = {"lab/web/components.js"}
# Explicit generated companions: their line-one source markers must remain first.
# They are reference outputs, not independently owned handwritten sources. The
# Rust OUT_DIR tokens receive the attached notice in bbt-ui-model/build.rs.
GENERATED_OUTPUTS = {"lab/web/tokens.js", "lab/web/logs.js", "lab/web/e2e.js", "site/real.js"}


def inventory(root):
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root, capture_output=True, check=True,
    )
    return sorted(set(result.stdout.decode().split("\0")) - {""})


def category(name, text):
    """An explicit boundary is stronger than an inferred project ownership claim."""
    if not name.startswith("crates/") and name not in CHECKER_FILES | PACKAGED_SOURCES | GENERATED_OUTPUTS:
        return None
    if name in UPSTREAM_FILES or name.startswith(UPSTREAM_PREFIXES):
        return "upstream"
    if "/release-inputs/" in name:
        return "mirror"
    if name.startswith(STRICT_PREFIXES):
        return "fixture"
    if Path(name).suffix not in SOURCE_SUFFIXES:
        return None
    if name in GENERATED_OUTPUTS:
        return "generated"
    if name.startswith("crates/bbt-core/"):
        return "core"
    if name in OWNED_FORK_FILES:
        return "owned"
    if name.startswith(FORK_PREFIXES):
        return "fork"
    return "owned"


def notice(kind):
    if kind == "owned":
        return [
            "Copyright 2026 Boxy (bboxy450) and contributors.",
            "SPDX-License-Identifier: " + SPDX,
            "Project: bboxy450-term — " + PROJECT,
            "License: " + LICENSE,
            "Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.",
            "Policy: " + POLICY,
        ]
    terms = "Existing Apache-2.0 crate terms and retained upstream notices apply."
    source = []
    if kind == "fork":
        terms = "Existing MIT OR Apache-2.0 terms and Kittens contributor notices apply."
        source = ["Fork source: " + YARN_SOURCE]
    return [
        "Project integration: bboxy450-term — " + PROJECT,
        *source,
        terms,
        "Credit bboxy450 integration accurately; independent inspiration credit requested.",
        "Policy: " + POLICY,
    ]


def header(name, kind):
    lines = notice(kind)
    suffix = Path(name).suffix
    if suffix in {".html", ".css"}:
        start, end = ("<!--", "-->") if suffix == ".html" else ("/*", "*/")
        return start + "\n" + "\n".join(lines) + "\n" + end + "\n"
    prefix = "# " if suffix in {".py", ".sh"} else "// "
    return "".join(prefix + line + "\n" for line in lines)


def split_prefix(text):
    """Keep BOMs, executable shebangs and Python encoding declarations in place."""
    prefix = "\ufeff" if text.startswith("\ufeff") else ""
    rest = text[len(prefix):]
    lines = rest.splitlines(keepends=True)
    count = int(bool(lines and lines[0].startswith("#!") and not lines[0].startswith("#![")))
    if len(lines) > count and re.search(r"coding[:=]\s*[-\w.]+", lines[count]) and lines[count].lstrip().startswith("#"):
        count += 1
    return prefix + "".join(lines[:count]), "".join(lines[count:])


def migrate_owned_notice(name, text):
    # Only our old project-level notice is replaced. This is never called for
    # forks, upstream files or unknown copyright/license statements.
    lines = text.splitlines(keepends=True)
    if name == "crates/bboxy450-term/src/keyboard.rs" and lines and lines[0].strip() == "// SPDX-License-Identifier: MIT OR Apache-2.0":
        lines.pop(0)
    result = []
    skip_policy = False
    for index, line in enumerate(lines):
        if index < 40 and re.fullmatch(r"\s*//!? Origin: bboxy450-term, MIT OR Apache-2\.0\. Retain credit on reuse:\s*", line):
            skip_policy = True
            continue
        if skip_policy and line.strip().lstrip("/! ") == POLICY:
            skip_policy = False
            continue
        skip_policy = False
        result.append(line)
    return "".join(result)


def foreign_notice(text):
    for line in text.splitlines()[:40]:
        if re.match(r"\s*(?://!?|#|\*)\s*SPDX-License-Identifier:", line) and SPDX not in line:
            return line.strip()
        if re.match(r"\s*(?://!?|#|\*)\s*Copyright\b", line, re.I) and "Boxy (bboxy450)" not in line:
            return line.strip()
    return None


def check(root, synchronize=False):
    counts = Counter()
    changed = 0
    errors = []
    for name in inventory(root):
        path = root / name
        if not path.is_file() or path.is_symlink():
            continue
        # Source inventory uses text; excluded binary data is never decoded.
        if Path(name).suffix not in SOURCE_SUFFIXES:
            continue
        original = path.read_bytes()
        try:
            text = original.decode("utf-8")
        except UnicodeDecodeError:
            errors.append(f"{name}: source is not UTF-8; requires an explicit boundary")
            continue
        kind = category(name, text)
        if kind is None:
            continue
        counts[kind] += 1
        if kind in {"upstream", "mirror", "fixture", "generated"}:
            if kind == "upstream" and SPDX in "\n".join(text.splitlines()[:40]):
                errors.append(f"{name}: project draft SPDX must not replace upstream terms")
            continue
        prefix, body = split_prefix(text)
        expected = header(name, kind)
        if body.startswith(expected):
            if kind != "owned" and SPDX in "\n".join(body.splitlines()[:40]):
                errors.append(f"{name}: scoped integration must retain upstream terms, not project draft SPDX")
            continue
        candidate = migrate_owned_notice(name, body) if kind == "owned" else body
        if kind == "owned" and (foreign := foreign_notice(candidate)):
            errors.append(f"{name}: unclassified license/copyright notice ({foreign}); refusing ownership claim")
            continue
        if kind != "owned" and SPDX in "\n".join(candidate.splitlines()[:40]):
            errors.append(f"{name}: scoped integration must retain upstream terms, not project draft SPDX")
            continue
        if synchronize:
            path.write_bytes((prefix + expected + candidate).encode("utf-8"))
            changed += 1
        else:
            errors.append(f"{name}: missing or stale {kind} source notice")
    for error in errors:
        print("source-notices: " + error, file=sys.stderr)
    summary = ", ".join(f"{kind}={counts[kind]}" for kind in sorted(counts))
    print(f"source-notices: {'VIOLATED' if errors else 'held'} ({summary}; changed={changed})")
    return bool(errors)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="check coverage without changing files (default)")
    mode.add_argument("--sync", action="store_true", help="synchronize project comment notices only")
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    try:
        return check(args.root.resolve(), args.sync)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"source-notices: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
