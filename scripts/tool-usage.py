#!/usr/bin/env python3
"""What seats call from python_execute: shell commands and MCP tools, totalled
across the persisted execution records (~/.bboxy450-harness/sessions/*/python/*.json).

Each record lists its calls in order; a command's label is the exact line run,
so the program names are read from the line itself. The "code reading" group
puts the shell habits (rg, grep, sed, head, cat ...) beside bboxy_code.*.

bboxy450-term — https://github.com/bboxy450/bboxy450-term
LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""
import argparse
import collections
import glob
import json
import os
import shlex
import time

SEPARATORS = {";", "&&", "||", "|", "&", "(", ")", "$(", "`", "\n"}
PREFIXES = {"env", "nohup", "time", "command", "exec", "sudo", "nice", "timeout"}
SHELL_WORDS = {"if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac", "!", "{", "}"}
CODE_READING = {"rg", "grep", "git grep", "find", "fd", "sed", "awk", "head", "tail", "cat", "ls", "wc"}


def programs(line):
    """The program names a shell line runs: the first word after each separator."""
    lexer = shlex.shlex(line, posix=True, punctuation_chars=";&|()`")
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        words = list(lexer)
    except ValueError:  # unbalanced quotes: count the first word only
        words = line.split()[:1]
    found, start = [], True
    for i, word in enumerate(words):
        if word in SEPARATORS or set(word) <= set(";&|()`"):
            start = True
            continue
        if not start:
            continue
        if word in SHELL_WORDS or "=" in word.split("/")[0] or word in PREFIXES or word.isdigit():
            continue
        if word == "cd":
            start = False
            continue
        name = os.path.basename(word)
        if name == "git" and i + 1 < len(words) and words[i + 1] == "grep":
            name = "git grep"
        found.append(name)
        start = False
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", default=os.path.expanduser("~/.bboxy450-harness/sessions"))
    parser.add_argument("--days", type=float, default=0, help="only records modified in the last N days (0: all)")
    parser.add_argument("--top", type=int, default=25)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    since = time.time() - args.days * 86400 if args.days else 0
    calls, sessions, records = collections.Counter(), set(), 0
    for path in glob.glob(os.path.join(args.root, "*", "python", "*.json")):
        try:
            if os.path.getmtime(path) < since:
                continue
            with open(path, encoding="utf-8") as handle:
                record = json.load(handle)
        except (OSError, ValueError):
            continue
        records += 1
        sessions.add(path.split(os.sep)[-3])
        for child in record.get("children", []):
            if child.get("kind") == "call":
                calls[f"{child.get('server')}.{child.get('tool')}"] += 1
            else:
                for name in programs(child.get("label") or ""):
                    calls[f"sh {name}"] += 1
    reading = {key: n for key, n in calls.items()
               if key.startswith("bboxy_code.") or key.removeprefix("sh ") in CODE_READING}
    shell = sum(n for key, n in reading.items() if key.startswith("sh "))
    code = sum(n for key, n in reading.items() if key.startswith("bboxy_code."))
    result = {"records": records, "sessions": len(sessions), "days": args.days or None,
              "top": calls.most_common(args.top),
              "code_reading": {"shell": shell, "bboxy_code": code,
                               "bboxy_code_share": round(code / (shell + code), 3) if shell + code else None,
                               "by_tool": sorted(reading.items(), key=lambda kv: -kv[1])}}
    if args.json:
        print(json.dumps(result, indent=2))
        return
    print(f"{records} executions, {len(sessions)} sessions" + (f", last {args.days:g} days" if args.days else ""))
    for key, n in result["top"]:
        print(f"{n:8d}  {key}")
    share = result["code_reading"]["bboxy_code_share"]
    print(f"\ncode reading: shell {shell}, bboxy_code {code}" + (f" ({share:.1%} bboxy_code)" if share is not None else ""))
    for key, n in result["code_reading"]["by_tool"]:
        print(f"{n:8d}  {key}")


if __name__ == "__main__":
    main()
