#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""The one script-side reader of a knowledge record's top-level fields, for
`.md` (YAML header) and `.den` (`specs/spec-language.md`) records alike.

Den: one-line record fields (`title "x"`, `status Running note`, `slow true`)
and the host's `managed { key = json"..." }` block, which overrides them, as
crates/bbt-mux/src/knowledge/den.rs reads it. Values are text, as the shell
and Python checks compare them; nested blocks, links and prose are not fields.
A machine spec has no header fields here.

    python3 -I scripts/kb_record.py FIELD FILE...   one value per line
    load(this file).fields(path) / .records(directory)
"""
import json
import re
import sys
from pathlib import Path

RECORD = re.compile(r"^\s*(spec|interface|invariant|epic|task|report|e2e|decision|proposal|bug|recipe)\s+\w+\s*\{\s*$")
FIELD = re.compile(r"^\s*([A-Za-z_]\w*)\s+(.+?)\s*$")
MANAGED = re.compile(r'^\s*([A-Za-z_]\w*)\s*=\s*(json"(?:[^"\\]|\\.)*"|\S+)\s*$')
ESCAPES = {"n": "\n", "r": "\r", "t": "\t", '"': '"', "\\": "\\"}


def records(directory):
    """Every record file directly in `directory`, Markdown and Den, sorted."""
    directory = Path(directory)
    return sorted(p for p in directory.glob("*") if p.suffix in (".md", ".den") and p.is_file())


def _text(value):
    """A JSON value as the YAML-header text the existing checks compare."""
    if isinstance(value, str):
        return value
    if isinstance(value, bool):
        return "true" if value else "false"
    return json.dumps(value) if isinstance(value, (list, dict)) else str(value)


def _unquote(literal):
    body = literal[1:-1]
    return re.sub(r"\\(.)", lambda m: ESCAPES.get(m.group(1), m.group(1)), body)


def den_fields(text):
    out, depth, kind, in_block, managed = {}, 0, None, False, False
    for line in text.splitlines():
        stripped = line.strip()
        if in_block:
            if stripped.endswith('"""'):
                in_block = False
            continue
        if stripped.startswith("//") or not stripped:
            continue
        if depth == 0:
            match = RECORD.match(line)
            if match:
                kind, depth = match.group(1), 1
                out["type"] = kind
            continue
        if stripped == "}":
            depth -= 1
            managed = managed and depth == 2
            continue
        if depth == 1 and stripped == "managed {":
            depth, managed = 2, True
            continue
        if stripped.endswith("{"):
            depth += 1
            continue
        if managed and depth == 2:
            match = MANAGED.match(line)
            if match:
                key, value = match.groups()
                if value.startswith('json"'):
                    decoded = json.loads(_unquote(value[4:]))
                    if key == "other" and isinstance(decoded, dict):
                        out.update({k: _text(v) for k, v in decoded.items()})
                        continue
                    if key != "body_tail":
                        out[key] = _text(decoded)
                else:
                    out[key] = value.lower() if key == "status" else value
            continue
        if depth != 1:
            continue
        match = FIELD.match(line)
        if not match:
            continue
        key, value = match.groups()
        if value.startswith('"""'):
            in_block = not value.endswith('"""') or value == '"""'
            continue
        if value.startswith('"') and value.endswith('"') and len(value) > 1:
            value = _unquote(value)
        if key == "status":
            value = value.split()[0].lower()
        elif value.startswith("$ "):
            # A command field (`verify`, an invariant's `check`) reads as its command.
            value = re.sub(r"\s*~>\s*exit\s+\d+$", "", value.removeprefix("$ "))
        elif key == "paths":
            key = "allowed_paths"
        out.setdefault(key, value)
    if kind == "spec" and "title" not in out:
        return {}
    return out


def markdown_fields(text):
    lines = text.splitlines()
    out = {}
    if lines and lines[0] == "---":
        for line in lines[1:]:
            if line == "---":
                break
            if ":" in line and not line[0].isspace():
                key, value = line.split(":", 1)
                out[key] = value.strip().strip("\"'")
    return out


def text_fields(text):
    """[`fields`] of a record's text: a YAML header is Markdown, anything else Den."""
    return markdown_fields(text) if text.startswith("---") else den_fields(text)


def fields(path):
    """The record's top-level fields as text, `{}` when it has none."""
    path = Path(path)
    text = path.read_text()
    return den_fields(text) if path.suffix == ".den" else markdown_fields(text)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        sys.exit("usage: kb_record.py FIELD FILE...")
    for name in sys.argv[2:]:
        print(fields(name).get(sys.argv[1], ""))
