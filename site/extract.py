#!/usr/bin/env python3
"""Build site/real.js: real transcripts, tasks and ICQ messages for the public showcase.

  python3 -I site/extract.py --claude SESSION.jsonl --codex ROLLOUT.jsonl [--since 2026-10-26T21:00:00Z]

Sources: a Claude Code session transcript, a Codex rollout, this repository's GitHub issues (gh) and the
current identity's ICQ inbox and outbox (icq). The output is refused if it contains any local secret.
"""
import argparse, datetime, json, os, re, subprocess, sys

HOME = os.path.expanduser("~")
COHORT = {  # key prefix -> (name, role); the repository's cohort record
    "726afa3b": ("Vesper Greywind", "oracle"), "6b4e9bf6": ("Nyx Copperwing", "executor"),
    "bfae37c6": ("Rook Ashwhisker", "executor"), "b04a6091": ("Sable Embertail", "executor"),
    "b2a1d2db": ("Aster Cinderwing", "executor"), "2868e75c": ("Kestrel Voidwing", "executor"),
    "99a17c42": ("Velvet Vulpine", "executor"), "6b172639": ("Corvin Duskmantle", "architect"),
}
SEATS = {  # showcase fixture seat -> the real seat it stands for
    "Vesper Greywind": {"title": "release 0.1.0"},
    "Nyx Copperwing": {"title": "0.1.0 release cut", "log": "codex-Nyx"},
    "Rook Ashwhisker": {"title": "#56 error journal", "log": "codex-Rook",
                        "waitLine": "Please rule on the narrow Codex hook subscription seam"},
    "Sable Quill": {"name": "Sable Embertail", "title": "#42 ctl BrokenPipe", "log": "codex-Sable"},
    "Pip Hollow": {"name": "Kestrel Voidwing", "harness": "codex", "title": "#54 Finder cwd", "log": "codex-Kestrel"},
    "Marrow Finch": {"name": "Aster Cinderwing", "harness": "codex", "title": "release smoke", "log": "codex-Aster"},
    "Dax Riven": {"name": "Velvet Vulpine", "title": "MA-10 mentions", "log": "codex-Velvet"},
    "zsh": {"title": "~/mydev/bboxy450-term"},
}
ASSIGNEE = {54: "Kestrel Voidwing", 55: "Sable Embertail", 42: "Sable Embertail", 56: "Rook Ashwhisker",
            51: "Velvet Vulpine", 53: "Nyx Copperwing", 62: "Vesper Greywind", 65: "Vesper Greywind"}
RUNNING = {56}
W = 110  # widest command or output line kept


def tidy(s):
    s = re.sub(r"/private/tmp/claude-501/[^/\s]+/[0-9a-f-]{36}/scratchpad", "$SCRATCH", s)
    return s.replace(HOME, "~")


def cut(s, n=W):
    return s if len(s) <= n else s[: n - 1] + "…"


def ts(s):
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))


def claude_lines(path, since):
    """Claude Code's own look: ❯ prompt bar, prose, dim 'Ran N shell commands', dim '✻ Worked for'."""
    out, ran, t0, t1 = [], 0, None, None

    def flush_ran():
        nonlocal ran
        if ran:
            out.extend([f"Ran {ran} shell command{'s' if ran > 1 else ''}", ""])
            ran = 0

    def worked():
        if t0 and t1 and t1 > t0:
            m, s = divmod(int((t1 - t0).total_seconds()), 60)
            out.extend([f"✻ Worked for {m}m {s}s" if m else f"✻ Worked for {s}s", ""])

    for raw in open(path, encoding="utf-8"):
        try:
            d = json.loads(raw)
        except ValueError:
            continue
        if d.get("type") not in ("user", "assistant") or not d.get("timestamp") or ts(d["timestamp"]) < since:
            continue
        c = (d.get("message") or {}).get("content")
        if (d["type"] == "user" and isinstance(c, str) and not c.startswith(("<", "Another Claude session", "[SYSTEM", "This session is being continued"))
                and d.get("userType") == "external"):
            flush_ran(); worked()
            out.extend(["❯ " + cut(" ".join(c.split()), 200), ""])
            t0, t1 = ts(d["timestamp"]), None
        elif d["type"] == "assistant" and isinstance(c, list):
            for b in c:
                if b.get("type") == "tool_use":
                    ran += 1
                elif b.get("type") == "text" and b["text"].strip():
                    flush_ran()
                    out.extend(tidy(b["text"]).strip().split("\n") + [""])
            t1 = ts(d["timestamp"])
    flush_ran(); worked()
    return out


def codex_lines(path, events):
    """Codex's own look: › prompt, • Ran <cmd>, └ output with '+ N lines (^t to expand)', • prose."""
    recs = []
    with open(path, "rb") as f:
        f.seek(max(0, os.path.getsize(path) - 4_000_000))
        f.readline()
        for raw in f:
            try:
                recs.append(json.loads(raw)["payload"])
            except (ValueError, KeyError):
                continue
    out = []
    for p in recs:
        t = p.get("type")
        if t == "message" and p.get("role") == "user":
            text = "".join(x.get("text", "") for x in p.get("content", []))
            if text.startswith("<") or "context: channel" in text:
                continue
            m = re.search(r"--- peer content ---\n(.*?)\n--- end peer content ---", text, re.S)
            if "ICQ peer message" in text and m:
                k = re.search(r"from: ([0-9a-f]{8})", text)
                who = COHORT.get(k.group(1), ("a peer", ""))[0] if k else "a peer"
                text = f"ICQ from {who}: " + m.group(1)
            out.extend(["› " + cut(" ".join(text.split()), 160), ""])
        elif t == "custom_tool_call" and p.get("name") == "exec":
            m = re.search(r'cmd:"((?:[^"\\]|\\.)*)"', p.get("input", ""))
            if not m:
                continue
            cmd = json.loads('"' + m.group(1) + '"').strip().split("\n")
            out.append("• Ran " + cut(tidy(cmd[0]), 90) + (" …" if len(cmd) > 1 else ""))
        elif t == "custom_tool_call_output":
            text = "".join(x.get("text", "") for x in p.get("output", []) if isinstance(x, dict))
            body = [l for l in tidy(text.split("Output:\n", 1)[-1]).rstrip().split("\n") if l.strip() and not l.startswith('{"chunk_id"')]
            if not body:
                out.extend(["  └ (no output)", ""])
                continue
            out.append("  └ " + cut(body[0]))
            out.extend("    " + cut(l) for l in body[1:3])
            if len(body) > 3:
                out.append(f"    + {len(body) - 3} lines (^t to expand)")
            out.append("")
        elif t == "message" and p.get("role") == "assistant":
            text = tidy("".join(x.get("text", "") for x in p.get("content", []))).strip()
            if len(text.split()) >= 3:
                ls = text.split("\n")
                out.extend(["• " + ls[0]] + ["  " + l for l in ls[1:]] + [""])
    # keep the last `events` blocks (a block ends at a blank line)
    blocks, cur = [], []
    for l in out:
        cur.append(l)
        if l == "":
            blocks.append(cur); cur = []
    return [l for b in blocks[-events:] for l in b]


def epics():
    issues = json.loads(subprocess.run(["gh", "issue", "list", "-R", "bboxy450/bboxy450-term", "--state", "all",
                                        "--limit", "100", "--json", "number,title,state"], capture_output=True,
                                       text=True, check=True).stdout)
    by = {i["number"]: i for i in issues}

    def item(n):
        i = by[n]
        st = "done" if i["state"] == "CLOSED" else "running" if n in RUNNING else "open"
        x = {"id": f"gh{n}", "type": "task", "title": f"#{n} {i['title']}", "status": st}
        if n in ASSIGNEE:
            x["assignee"] = ASSIGNEE[n]
        return x
    groups = [("0.1.0 release", [54, 55, 42, 53, 56, 51, 65]), ("Scale budgets", [46, 47, 48, 49, 50]),
              ("Acceptance coverage", [1, 5, 9, 11, 15, 21])]
    return [{"id": f"e{k}", "title": t, "open": k == 0, "items": [item(n) for n in ns if n in by]}
            for k, (t, ns) in enumerate(groups)]


def icq_stream(since, keep):
    msgs = []
    for cmd in (["inbox"], ["outbox"]):
        after = 0
        while True:
            args = ["--after", str(after), "--limit", "50"] if cmd == ["inbox"] else []
            r = subprocess.run(["icq", *cmd, *args], capture_output=True, text=True)
            try:
                items = json.loads(r.stdout).get("items", [])
            except ValueError:
                break
            if not items:
                break
            for m in items:
                e = m.get("envelope", m)
                a = (e.get("author") or "")[:8]
                if a not in COHORT or not e.get("text") or e.get("timestamp", 0) < since.timestamp():
                    continue
                msgs.append(e)
            if cmd == ["outbox"] or "seq" not in items[-1]:
                break
            after = int(items[-1]["seq"])
    seen, out = set(), []
    for e in sorted(msgs, key=lambda e: e["timestamp"]):
        if re.match(r"SLOT \d+ (TAKEN|RELEASED)|ACK ", e["text"]) or (e.get("context") or {}).get("kind") != "dm" and not e["text"].startswith("LANDED"):
            continue
        text = re.sub(r"^(ROLE: \w+ — |RULING \(Oracle\): )?[\w ]{0,40}<\w+> ?— ", "", e["text"].strip())
        text = re.sub(r"\b([0-9a-f]{8})[0-9a-f]{32}\b", r"\1", text)
        text = re.sub(r"\s*— Vesper Greywind <ORACLE>\s*$", "", text)
        text = cut(" ".join(tidy(text).split()), 150)
        if e["id"] in seen or text in seen:
            continue
        seen.update([e["id"], text])
        name, role = COHORT[e["author"][:8]]
        out.append({"author": {"kind": "seat", "id": e["author"][:8], "name": name}, "role": role,
                    "issuer_kind": "gpg-root", "kind": "text", "priority_class": "oracle" if role == "oracle" else "worker",
                    "priority": 0, "ts": int(e["timestamp"] * 1000), "text": text, "evidence": "recipient-stored"})
    return out[-keep:]


def secrets():
    vals = []
    for p in ("~/feral.txt", "~/swarm-passphrase.txt"):
        try:
            v = open(os.path.expanduser(p), "rb").read().strip()
            if len(v) >= 6:
                vals.append(v)
        except OSError:
            pass
    try:
        vals += re.findall(rb'token\s*=\s*"([^"]+)"', open(os.path.expanduser("~/.cargo/credentials.toml"), "rb").read())
    except OSError:
        pass
    return vals


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--claude", required=True)
    ap.add_argument("--codex", action="append", required=True, help="NAME=ROLLOUT.jsonl, one per Executor; the first is the default")
    ap.add_argument("--since", default="2026-10-26T21:00:00Z")
    ap.add_argument("--out", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "real.js"))
    a = ap.parse_args()
    since = ts(a.since)
    git_log = subprocess.run(["git", "log", "-3", "--stat", "--format=commit %h %G?%nAuthor: %an <%ae>%nDate:   %ad%n%n    %s%n"],
                             capture_output=True, text=True, check=True).stdout
    logs = {"claude-session": "\n".join(claude_lines(a.claude, since)),
            "git-log": git_log}
    for k, spec in enumerate(a.codex):
        name, path = spec.split("=", 1)
        logs["codex-" + name] = "\n".join(codex_lines(os.path.expanduser(path), 40))
        if k == 0:
            logs["codex-session"] = logs["codex-" + name]
    body = ("// Generated by site/extract.py from real sessions, issues and ICQ messages. Do not edit.\n"
            "Object.assign(window.LOGS, " + json.dumps(logs, ensure_ascii=False) + ");\n"
            "window.REAL = " + json.dumps({"seats": SEATS, "epics": epics(), "stream": icq_stream(since, 12)},
                                          ensure_ascii=False) + ";\n")
    raw = body.encode()
    if any(v in raw for v in secrets()):
        sys.exit("extract: refusing to write: output contains a local secret")
    open(a.out, "w", encoding="utf-8").write(body)
    print(f"{a.out}: {len(raw)} bytes, claude {logs['claude-session'].count(chr(10))} lines, "
          f"codex {len(a.codex)} sessions")


if __name__ == "__main__":
    main()
