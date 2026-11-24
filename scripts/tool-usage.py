#!/usr/bin/env python3
"""Count shell programs and MCP tools in bboxy Python and Codex code-mode records.

Code-reading share compares bboxy_code calls with source-reading shell
programs at the start of a pipeline. The top list includes downstream programs.

bboxy450-term — https://github.com/bboxy450/bboxy450-term
LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""
import argparse
import collections
import glob
import json
import os
import re
import shlex
import time

PUNCTUATION = ";&|()`"
PREFIXES = {"env", "nohup", "time", "command", "exec", "sudo", "nice", "timeout"}
SHELL_WORDS = {"if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac", "!", "{", "}"}
CODE_READING = {"rg", "grep", "git grep", "sed", "cat", "nl", "find", "fd"}

# Match literal calls in Codex exec input, skipping strings and comments in JS.
JS_STRING = r'''"(?:\\[\s\S]|[^"\\])*"|'(?:\\[\s\S]|[^'\\])*'|`(?:\\[\s\S]|[^`\\])*`'''
CODEX_CALL = re.compile(
    rf"\btools\.(?:mcp__bboxy_code__(?P<tool>\w+)\s*\("
    rf"|exec_command\s*\(\s*\{{\s*cmd\s*:\s*(?P<command>{JS_STRING})(?=\s*[,}}]))"
    rf"|{JS_STRING}|//[^\n]*|/\*[\s\S]*?\*/"
)
JS_ESCAPE = re.compile(r"\\(\r\n|[\s\S])|\$\{")
JS_ESCAPES = {"n": "\n", "r": "\r", "t": "\t", "\n": "", "\r": "", "\r\n": ""}


def programs(line):
    """Yield each shell program and whether it starts its pipeline."""
    lexer = shlex.shlex(line, posix=True, punctuation_chars=PUNCTUATION)
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        words = list(lexer)
    except ValueError:  # unbalanced quotes: count the first word only
        words = line.split()[:1]
    start, pipeline_head = True, True
    for i, word in enumerate(words):
        if word and all(char in PUNCTUATION for char in word):
            start = True
            pipeline_head = word not in {"|", "|&"}
            continue
        if not start:
            continue
        if word in SHELL_WORDS or "=" in word.split("/")[0] or word in PREFIXES or word.isdigit():
            continue
        start = False
        if word == "cd":
            continue
        name = os.path.basename(word)
        if name == "git" and i + 1 < len(words) and words[i + 1] == "grep":
            name = "git grep"
        yield name, pipeline_head


def count_shell(calls, reading, line):
    for name, pipeline_head in programs(line):
        key = f"sh {name}"
        calls[key] += 1
        if pipeline_head and name in CODE_READING:
            reading[key] += 1


def recent_files(root, pattern, since):
    for path in glob.iglob(os.path.join(os.path.expanduser(root), pattern), recursive=True):
        try:
            if os.path.getmtime(path) >= since:
                yield path
        except OSError:
            continue


def python_usage(root, since):
    calls, reading = collections.Counter(), collections.Counter()
    sessions, records = set(), 0
    for path in recent_files(root, "*/python/*.json", since):
        try:
            with open(path, encoding="utf-8") as handle:
                record = json.load(handle)
        except (OSError, ValueError):
            continue
        records += 1
        sessions.add(path.split(os.sep)[-3])
        for child in record.get("children", []):
            if child.get("kind") == "call":
                key = f"{child.get('server')}.{child.get('tool')}"
                calls[key] += 1
                if key.startswith("bboxy_code."):
                    reading[key] += 1
            else:
                count_shell(calls, reading, child.get("label") or "")
    return calls, reading, len(sessions), records


def literal_command(literal):
    value = literal[1:-1]
    # A template with interpolation has no recoverable command string here.
    if literal[0] == "`" and any(match.group(1) is None for match in JS_ESCAPE.finditer(value)):
        return ""
    return JS_ESCAPE.sub(
        lambda match: JS_ESCAPES.get(match.group(1), match.group(1)) if match.group(1) else match[0],
        value,
    )


def codex_usage(root, since):
    calls, reading = collections.Counter(), collections.Counter()
    sessions, records = 0, 0
    for path in recent_files(root, "**/*.jsonl", since):
        before = records
        try:
            with open(path, encoding="utf-8", errors="replace") as handle:
                for line in handle:
                    try:
                        row = json.loads(line)
                    except ValueError:
                        continue
                    payload = row.get("payload") if isinstance(row, dict) else None
                    if not isinstance(payload, dict) or payload.get("name") != "exec":
                        continue
                    source = payload.get("input")
                    if not isinstance(source, str):
                        continue
                    records += 1
                    for match in CODEX_CALL.finditer(source):
                        if tool := match["tool"]:
                            key = f"bboxy_code.{tool}"
                            calls[key] += 1
                            reading[key] += 1
                        elif command := match["command"]:
                            count_shell(calls, reading, literal_command(command))
        except OSError:
            pass
        sessions += records > before
    return calls, reading, sessions, records


def summarize(calls, reading, sessions, records, top):
    shell = sum(n for key, n in reading.items() if key.startswith("sh "))
    code = sum(reading.values()) - shell
    return {
        "records": records,
        "sessions": sessions,
        "top": calls.most_common(top),
        "code_reading": {
            "shell": shell,
            "bboxy_code": code,
            "bboxy_code_share": round(code / (shell + code), 3) if shell + code else None,
            "by_tool": reading.most_common(),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", default="~/.bboxy450-harness/sessions")
    parser.add_argument("--codex-root", default="~/.codex/sessions")
    parser.add_argument("--days", type=float, default=0, help="only records modified in the last N days (0: all)")
    parser.add_argument("--top", type=int, default=25)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    since = time.time() - args.days * 86400 if args.days else 0
    python = python_usage(args.root, since)
    codex = codex_usage(args.codex_root, since)
    total = tuple(a + b for a, b in zip(python, codex))
    result = summarize(*total, args.top)
    result["days"] = args.days or None
    result["sources"] = {"bboxy python": summarize(*python, args.top), "codex": summarize(*codex, args.top)}
    if args.json:
        print(json.dumps(result, indent=2))
        return
    window = f", last {args.days:g} days" if args.days else ""
    print(f"total: {result['records']} executions, {result['sessions']} sessions{window}")
    for key, n in result["top"]:
        print(f"{n:8d}  {key}")
    for name, summary in [("total", result), *result["sources"].items()]:
        reading = summary["code_reading"]
        share = reading["bboxy_code_share"]
        share_text = f" ({share:.1%} bboxy_code)" if share is not None else ""
        print(f"\n{name} code reading: shell {reading['shell']}, bboxy_code {reading['bboxy_code']}{share_text}")
        for key, n in reading["by_tool"]:
            print(f"{n:8d}  {key}")


if __name__ == "__main__":
    main()
