"""G3: real SDK hook timings and Settings, driven through a private app's ctl.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
"""
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from settings import SettingsApp, negative, texts
from ui import frame, nodes, require


def labels(page):
    return [n.get("text", "") for n in nodes(page)]


def rows(page):
    return [n for n in nodes(page) if re.fullmatch(r"named:card0/plugin\d+", n["kind"])]


def warning_visible(page):
    require(any("Slow hook" in text and "budget" in text for text in labels(page)),
            "produced Settings frame has no measured slow-hook warning")


def main():
    app = SettingsApp()
    repository = app.root / "repository"
    fixture = Path(os.environ.get("BBOXY_G3_FIXTURE", Path(app.binary).resolve().parent.parent / "g3-plugins"))
    require(all((fixture / name).is_file() for name in ("bboxy450-term-repo_graph-reference", "bboxy450-term-repo_graph-untrusted")),
            "SDK fixtures missing: make test-settings-plugins builds the canonical prebuilt night inputs")
    destination = repository / ".bboxy/plugins"
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("bboxy450-term-repo_graph-reference", "bboxy450-term-repo_graph-untrusted"):
        shutil.copy2(fixture / name, destination / name)
    seat = app.ctl("seat", "spawn", "zsh", "--place", "tab", "--cwd", repository,
                   "--name", "G3 private hook")['seat']
    app.ctl("ui", "focus", seat)
    initial = app.ctl("plugin", "list")["entries"]
    require(len(initial) == 2 and all(e["state"] == "untrusted" for e in initial),
            "fixture discovery did not return exactly two untrusted candidates")
    require(not list(app.root.rglob("untrusted-executed")), "discovery executed the untrusted SDK fixture")
    plugin = repository / ".bboxy/plugins/bboxy450-term-repo_graph-reference"
    trusted = app.ctl("plugin", "trust", plugin)["entries"]
    trusted_entry = next(e for e in trusted if e["candidate"]["source"] == str(plugin))
    candidate = trusted_entry["candidate"]["id"]
    hook_dir = Path(trusted_entry["trust"]["data_dir"])
    require(hook_dir.resolve().is_relative_to(app.root), "fixture hook log is outside the private account")
    def hook_count():
        logs = list(hook_dir.rglob("hooks.jsonl"))
        require(len(logs) <= 1, "fixture unexpectedly ran hooks on multiple workspace roots")
        return sum(len(path.read_text().splitlines()) for path in logs)
    checked = app.ctl("plugin", "check", candidate)["entries"]
    require(next(e for e in checked if e["candidate"]["id"] == candidate)["conformance"]["ok"],
            "the genuine SDK fixture failed conformance")
    app.ctl("plugin", "enable", candidate)
    event = json.dumps({"hook_event_name": "SessionStart", "session_id": "g3-private"})
    command = "printf %s " + shlex.quote(event) + " | BBOXY_HOOK_SOCKET_ONLY=1 " + shlex.quote(app.binary) + " hook --agent claude"
    observed = []
    response_seconds = []
    initial_hooks = hook_count()
    for count in range(1, 4):
        app.line(seat, command)
        until = time.monotonic() + 20
        while hook_count() < initial_hooks + count:
            require(time.monotonic() < until, "SDK hook never entered its ten-second work")
            time.sleep(.01)
        started = time.monotonic()
        page = app.section("Plugins")
        elapsed = time.monotonic() - started
        require(elapsed < .5, f"Settings stalled during the active hook: {elapsed:.3f}s")
        response_seconds.append(elapsed)
        until = time.monotonic() + 20
        while True:
            entries = app.ctl("plugin", "list")["entries"]
            entry = next(e for e in entries if e["candidate"]["id"] == candidate)
            warnings = entry.get("hook_warnings", [])
            if warnings and warnings[0]["consecutive_overruns"] == count:
                break
            require(time.monotonic() < until, f"hook {count} has no registry overrun")
            time.sleep(.05)
        warning = warnings[0]
        require(warning["elapsed_ms"] > warning["budget_ms"], "warning has no real overrun")
        require(warning["elapsed_ms"] < warning["budget_ms"] + 500,
                "host did not cut the ten-second hook at its declared budget")
        require(warning["skipped"] == (count == 3), "skip is not exactly three consecutive overruns")
        require(any(s.get("hook") == "SessionStart" and s["ms"] > 0
                    for s in entry.get("measurements", [])), "host hook timeline is empty")
        observed.append(warning)
        until = time.monotonic() + 10
        while True:
            page = app.section("Plugins")
            if any("Slow hook" in text and f"({count} consecutive" in text for text in labels(page)):
                break
            require(time.monotonic() < until, "Settings did not publish the current overrun")
            time.sleep(.05)
        warning_visible(page)
        require(len(rows(page)) == 3, "Settings must list two discovered plugins and the bundled provider exactly once")
        require(sum("bboxy450-term-data_source-bboxy (bundled)" in texts(r) for r in rows(page)) == 1,
                "Settings duplicated or omitted the bundled provider")
        trusted_rows = [r for r in rows(page) if "G3 slow hook" in texts(r)]
        require(len(trusted_rows) == 1, "trusted shape name is missing or duplicated")
        content = " ".join(texts(trusted_rows[0])).lower()
        for expected in ["trusted", "read", "repo_graph", "1.0", "degraded", "hook", "ms"]:
            require(expected in content, f"Settings is missing {expected}: {content}")
        untrusted_rows = [r for r in rows(page) if "bboxy450-term-repo_graph-untrusted" in texts(r)]
        require(len(untrusted_rows) == 1 and "never run" in " ".join(texts(untrusted_rows[0])),
                "Settings does not report the untrusted plugin's actual health")
        require(any("timeline" in n["kind"] for n in nodes(page)), "Settings has no actual timeline")
        app.capture("window", f"g3-overrun-{count}", frame(page))
    before = len([s for s in entry["measurements"] if s.get("hook") == "SessionStart"])
    app.line(seat, command)
    until = time.monotonic() + 10
    while True:
        entries = app.ctl("plugin", "list")["entries"]
        entry = next(e for e in entries if e["candidate"]["id"] == candidate)
        if any(s.get("code") == "hook_budget_skipped" for s in entry.get("measurements", [])):
            break
        require(time.monotonic() < until, "fourth hook was not recorded as a named skip")
        time.sleep(.05)
    require(entry["hook_warnings"][0]["consecutive_overruns"] == 3, "skip counter changed")
    require(hook_count() == initial_hooks + 3, "the skipped SDK hook executed")
    untrusted = [e for e in entries if e["candidate"]["id"] != candidate]
    require(len(untrusted) == 1 and untrusted[0]["state"] == "untrusted"
            and untrusted[0]["activation"] is None and not list(app.root.rglob("untrusted-executed")),
            "untrusted plugin executed or acquired authority")
    page = app.section("Plugins")
    require(any("skipped until" in text for text in labels(page)), "Settings has no latched-skip warning")
    app.capture("window", "g3-skipped", frame(page))
    negative(app.work, "missing-slow-hook-warning", lambda: warning_visible({"tree": []}))
    if os.environ.get("BBOXY_TRANSPORT_PLUGIN"):
        transport = repository / ".bboxy/plugins/bboxy450-term-transport-example"
        shutil.copy2(os.environ["BBOXY_TRANSPORT_PLUGIN"], transport)
        entries = app.ctl("plugin", "trust", transport)["entries"]
        candidate = next(e["candidate"]["id"] for e in entries if e["candidate"]["source"] == str(transport))
        app.ctl("plugin", "check", candidate)
        app.ctl("plugin", "enable", candidate)
        until = time.monotonic() + 10
        while True:
            page = app.section("Machines")
            content = " ".join(labels(page))
            if "Local wrapper" in content and "plugin:example" in content:
                break
            require(time.monotonic() < until, "Settings Machines omitted the enabled SDK transport")
            time.sleep(.05)
        require("Plugin transports" in content and "Prefix" in content and "Delay (ms)" in content,
                "Settings omitted the declared transport parameter fields")
        app.capture("window", "transport-machines", frame(page))
    (app.work / "g3-evidence.json").write_text(json.dumps({"overruns": observed, "entries": entries,
        "attempted_hook_work_ms": 10000, "ui_response_seconds_during_hook": response_seconds, "operator_input": False, "before_skip_samples": before}, indent=2) + "\n")


if __name__ == "__main__":
    main()
