"""Named Settings controls and produced-frame assertions for R2/G1/G2.

Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
Named Appearance navigation adapted from parked candidate fd0a8b52971660b30cb4154b278750640e9349e0.
"""
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time
import tomllib

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ui import App, frame, nodes, require


def descendants(node):
    return nodes({"tree": [node]})


def texts(node):
    return [n["text"] for n in descendants(node) if n.get("kind") == "text"]


def one(items, reason):
    items = list(items)
    require(len(items) == 1, f"{reason}: expected one observed node, got {len(items)}")
    return items[0]


def control(page, label):
    name = one((n for n in nodes(page)
                if re.fullmatch(r"named:card\d+/r\d+/label/text", n["kind"])
                and texts(n) == [label]), f"Settings label {label}")["kind"]
    key = name.removesuffix("/label/text") + "/ctl"
    return one((n for n in nodes(page) if n["kind"] == key), f"named control {label}")


def rim_value(page):
    value = one((s for s in texts(control(page, "Rim size"))
                 if re.fullmatch(r"\d+ px", s)), "Rim size value")
    return int(value.split()[0])


def toggle_value(page, label):
    ctl = control(page, label)
    knob = one((n for n in descendants(ctl) if n["kind"].endswith("/knob")), label + " knob")
    x, _, w, _ = ctl["rect"]
    kx, _, kw, _ = knob["rect"]
    require(x < kx < kx + kw < x + w, label + " knob is outside its control")
    require(abs((kx - x) - 2) < .1 or abs((x + w - kx - kw) - 2) < .1,
            label + " has no observed on/off endpoint")
    return kx + kw / 2 > x + w / 2


def negative(work, name, assertion):
    try:
        assertion()
    except AssertionError as error:
        result = {"name": name, "rejected": True, "reason": str(error)}
        with (work / "negative-controls.jsonl").open("a") as out:
            out.write(json.dumps(result) + "\n")
        return result
    raise AssertionError(f"negative control {name} was accepted")


class SettingsApp(App):
    def __init__(self):
        super().__init__()
        self.run = Path(os.environ["BBOXY_RUN_DIR"]).resolve()
        self.root = self.work.parent.resolve()
        require(self.run.parent == self.root, "Settings fixture does not own its run directory")
        self.account = self.root / "account"
        self.app_pid = int(os.environ["E2E_APP"])
        require(Path(os.environ["HOME"]).resolve().is_relative_to(self.root), "Settings HOME is not private")
        self.config = self.account / "config.toml"
        self.sequence = 0

    def ctl(self, *args, label=None, refused=False):
        self.sequence += 1
        label = label or f"settings-ctl-{self.sequence:03}"
        value = super().ctl("--run-dir", self.run, *args, label=label, refused=refused)
        with (self.work / "commands.jsonl").open("a") as out:
            out.write(json.dumps({"args": list(map(str, args)), "receipt": label + ".jsonl",
                                  "refused": refused}) + "\n")
        return value

    def click(self, node):
        x, y, w, h = node["rect"]
        require(w > 0 and h > 0, "control has no visible hit rectangle")
        return self.ctl("ui", "click", round(x + w / 2), round(y + h / 2))

    def section(self, name):
        page = self.ctl("ui", "open", "settings")
        nav = one((n for n in nodes(page)
                   if re.fullmatch(r"named:nav\d+/text", n["kind"])
                   and texts(n) == [name]), "Settings navigation " + name)
        page = self.click(nav)
        require(page["page"] == "settings" and name in [n.get("text") for n in nodes(page)],
                "named navigation did not reach " + name)
        return page

    def rim(self, value):
        page = self.section("Appearance")
        require(any("2 to 8 px" in n.get("text", "") for n in nodes(page)),
                "visible Rim size control has no supported range")
        slider = control(page, "Rim size")
        while len(slider.get("children", [])) == 1:
            slider = slider["children"][0]
        slider = slider["children"][0]
        self.click(slider)
        page = self.ctl("ui", "key", *("left",) * 6)
        require(rim_value(page) == 2, "named Rim size did not clamp to its minimum")
        if value > 2:
            page = self.ctl("ui", "key", *("right",) * (value - 2))
        require(rim_value(page) == value, f"Rim size applied a different value than {value}")
        return page

    def toggle(self, label, expected):
        page = self.section("Notifications")
        if toggle_value(page, label) != expected:
            page = self.click(control(page, label))
        require(toggle_value(page, label) == expected, "named toggle did not apply: " + label)
        return page

    def persisted(self):
        require(self.config.is_file(), "actual Settings did not save private TOML")
        return tomllib.loads(self.config.read_text())

    def stop(self):
        def identity():
            result = subprocess.run(["/bin/ps", "-p", str(self.app_pid), "-o", "lstart=,args="],
                                    capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=3)
            require(result.returncode in (0, 1), "owned app exit lookup failed: " + result.stderr)
            return result.stdout.strip() if result.returncode == 0 else None
        owned = identity()
        require(owned is not None and self.root.name in owned and "--headless" in owned,
                "restart PID is not this fixture's private app")
        self.ctl("app", "stop")
        # The stop receipt acknowledges session shutdown, before GUI-owner reap.
        # Join that exact process; never retry a refused start or kill a PID.
        deadline = time.monotonic() + 10
        while identity() == owned:
            require(time.monotonic() < deadline, "private app did not exit after orderly stop")

    def restart(self, recording=True, verified=True):
        require(self.account.is_dir(), "runner private file account is missing")
        self.stop()
        args = ["app", "start", "--headless", "--ctl"]
        if verified:
            self.run = self.root / "run"
            os.environ.pop("BBOXY_TEST_CUSTODY", None)
            args += ["--unattended", "--test-account", self.account]
        else:
            require(not recording, "ordinary test fixture cannot select recording")
            os.environ["BBOXY_TEST_CUSTODY"] = "memory"
            self.run = self.root / "ordinary-run"
            require(not self.run.exists(), "ordinary boundary fixture must have a fresh run")
        if recording:
            args += ["--notifications", "record"]
        started = self.ctl(*args)
        self.app_pid = started["pid"]
        require(started["windows"] == 0 and started["custody_backend"] == ("file" if verified else "memory"),
                "restart escaped the private headless fixture")
        return started

    def record(self, phase, predicate=lambda _: True, revision=0):
        deadline = time.monotonic() + 30
        while True:
            state = self.ctl("debug", "notifications")
            require(state.get("available") and state.get("backend") == "recording"
                    and state.get("os_presentation") is False, "not the private recording backend")
            if (state["settled"] and state["processed_revision"] >= revision
                    and predicate(state)):
                require(state["plan_revision"] == state["processed_revision"], "unsettled policy plan")
                require(state["omitted_outstanding"] == state["omitted_records"] == 0,
                        "recording omitted evidence")
                (self.work / f"g2-{phase}.json").write_text(json.dumps(state, indent=2) + "\n")
                return state
            require(time.monotonic() < deadline, f"{phase}: policy completion timed out: {state}")

    def foreground(self, expected):
        layout = self.ctl("ui", "notification-foreground", str(expected).lower())
        require(layout["notification_frontmost"] is expected, "focus reply has another applied state")
        frame(layout)
        # Identical desired plans are coalesced by the real worker. Read the
        # actual post-frame submission fence instead of inventing revision + 1.
        applied = self.ctl("debug", "notifications")
        require(applied["frontmost"] is expected, "recording readback disagrees with applied frame")
        return self.record("foreground-" + str(expected).lower(),
                           lambda s: s["frontmost"] is expected,
                           applied["plan_revision"])


def geometry(layout, rim, chrome, previous=None):
    require(layout["page"] is None and len(layout["panes"]) == 2,
            "Settings did not return to the original two panes")
    require([node["rect"] for node in layout["tree"][:2]] == chrome,
            "Rim size moved titlebar or sidebar")
    outer = sorted(n["rect"] for n in nodes(layout) if n["kind"] == "named:pane")
    require(len(outer) == 2, "expected two actual pane borders")
    left, right = outer
    gap = right[0] - left[0] - left[2]
    sidebar_right = chrome[1][0] + chrome[1][2]
    title_bottom = chrome[0][1] + chrome[0][3]
    insets = [left[0] - sidebar_right, layout["size"][0] - right[0] - right[2]]
    insets += [rect[1] - title_bottom for rect in outer]
    insets += [layout["size"][1] - rect[1] - rect[3] for rect in outer]
    require(abs(gap - rim) < .01 and all(abs(v - rim / 2) < .01 for v in insets),
            f"Rim {rim}: gutter={gap}, insets={insets}")
    offsets = []
    for pane in layout["panes"]:
        center = pane["rect"][0] + pane["rect"][2] / 2
        border = next(r for r in outer if r[0] < center < r[0] + r[2])
        offsets.append([pane["rect"][0] - border[0], pane["rect"][1] - border[1]])
    require(previous is None or offsets == previous, "Rim size changed content padding")
    return {"rim": rim, "gutter": gap, "insets": insets, "content_offsets": offsets}


def two_panes(app, promise):
    first = app.spawn(promise + " left")
    second = app.ctl("seat", "spawn", "zsh", "--place", "right", "--cwd", app.work,
                     "--name", promise + " right")["seat"]
    for seat in (first, second):
        app.line(seat, "printf '\\033[?25l\\nSETTINGS_READY\\n'; exec cat")
        app.wait(seat, "(?m)^SETTINGS_READY$")
    return first


def g1():
    app = SettingsApp()
    require(not app.config.exists(), "G1 needs fresh Appearance defaults")
    page = app.section("Appearance")
    require(rim_value(page) == 4, "fresh Rim size default is not 4")
    require(not any("depth" in n.get("text", "").lower() for n in nodes(page)),
            "retired depth setting is still visible")
    negative(app.work, "wrong-fresh-default", lambda: require(rim_value(page) == 5, "default differs"))
    app.capture("window", "g1-fresh-appearance", frame(page))
    app.ctl("ui", "key", "esc")
    first = two_panes(app, "G1")
    initial = app.ctl("ui", "layout")
    chrome = [n["rect"] for n in initial["tree"][:2]]
    observations = []
    for value in range(2, 9):
        page = app.rim(value)
        if value in (2, 8):
            page = app.ctl("ui", "key", "left" if value == 2 else "right")
            require(rim_value(page) == value, "rim accepted an out-of-range edit")
        require(app.persisted()["appearance"]["rim"] == value, "saved rim differs from applied control")
        app.ctl("ui", "key", "esc")
        layout = app.ctl("ui", "focus", first)
        observations.append(geometry(layout, value, chrome))
    saved = app.persisted()
    require("depth" not in json.dumps(saved).lower(), "retired depth persisted")
    app.restart()
    restored = app.section("Appearance")
    require(rim_value(restored) == 8 and app.persisted() == saved, "Appearance did not persist across restart")
    app.capture("window", "g1-restarted-appearance", frame(restored))
    (app.work / "g1-result.json").write_text(json.dumps({"default": 4, "range": observations,
        "boundary_clamps": [2, 8], "saved": saved, "restart_value": 8, "depth_absent": True}, indent=2) + "\n")
    print("G1: named Appearance default 4; all seven rim values 2–8 applied/saved; bounds clamp; no depth; restart retains 8")


def waiting_fixture(app):
    """Two real PTYs, including a private Claude executable with no provider/model."""
    producer = app.work / "settings-waiting-producer.py"
    producer.write_text("import base64,sys\nprint('G2READY',flush=True)\n"
        "for raw in sys.stdin:\n"
        " text=raw.strip()\n"
        " if text=='IDLE':print('\\033]7501;state=idle\\007',end='',flush=True)\n"
        " else:\n"
        "  detail=base64.b64encode(text.encode()).decode()\n"
        "  print('\\033]7501;state=blocked:kind=question:msg='+detail+'\\007',end='',flush=True)\n"
        " print('G2ACK:'+text,flush=True)\n")
    binary = app.root / "bin"
    binary.mkdir(exist_ok=True)
    fake = binary / "claude"
    fake.write_text("#!/bin/sh\nexec " + shlex.quote(sys.executable) + " -I -u "
                    + shlex.quote(str(producer)) + "\n")
    fake.chmod(0o700)
    os.environ["PATH"] = str(binary) + ":/usr/bin:/bin:/usr/sbin:/sbin"
    zdotdir = app.root / "zsh-config"
    zdotdir.mkdir(exist_ok=True)
    (zdotdir / ".zprofile").write_text("export PATH=" + shlex.quote(os.environ["PATH"]) + "\n")
    os.environ["ZDOTDIR"] = str(zdotdir)
    return producer


def waiting_seats(app, producer):
    shell = app.spawn("G2 shell")
    app.line(shell, "exec " + shlex.quote(sys.executable) + " -I -u " + shlex.quote(str(producer)))
    app.wait(shell, "(?m)^G2READY$")
    claude = app.ctl("seat", "spawn", "claude", "--place", "tab", "--cwd", app.work,
                     "--name", "G2 private Claude producer")["seat"]
    app.wait(claude, "(?m)^G2READY$")
    neutral = app.spawn("G2 focused nonwaiting shell")
    roster = {r["record"]["id"]: r for r in app.ctl("seat", "ls")}
    require(roster[shell]["record"]["harness_kind"] == "zsh"
            and roster[claude]["record"]["harness_kind"] == "claude", "fixture does not have two harness types")
    return shell, claude, neutral


def questions(app, seats, phase):
    for seat in seats:
        app.line(seat, "IDLE")
        app.wait(seat, "(?m)^G2ACK:IDLE$")
        app.ctl("seat", "wait", seat, "--state", "idle", "--timeout", "20000")
    app.record(phase + "-idle", lambda s: s["outstanding_count"] == 0)
    for seat in seats:
        text = phase + "-" + seat
        app.line(seat, text)
        app.wait(seat, "(?m)^G2ACK:" + re.escape(text) + "$")
        app.ctl("seat", "wait", seat, "--state", "waiting", "--timeout", "20000")
    return app.record(phase, lambda s: {r["seat"] for r in s["waiting"]} >= set(seats))


def admitted(state, seats):
    require({r["id"] for r in state["outstanding"]} == set(seats),
            f"notification policy admitted {[r['id'] for r in state['outstanding']]}, expected {list(seats)}")


def control_contrast(path, node):
    from rim_clock import png, region
    x, y, w, h = node["rect"]
    data = region(png(path), (x, y, x + w, y + h))
    brightness = [sum(data[i:i + 3]) for i in range(0, len(data), 4)]
    return max(brightness) - min(brightness)


def g2():
    app = SettingsApp()
    require(not app.config.exists(), "G2 needs fresh Notifications defaults")
    page = app.section("Notifications")
    require(toggle_value(page, "Desktop notifications"), "notifications are not enabled by default")
    require(not toggle_value(page, "Only in the background"), "fresh policy unexpectedly requires background")
    for harness in ("zsh", "claude"):
        require(toggle_value(page, harness), harness + " disabled by default")
    app.capture("window", "g2-defaults", frame(page))
    producer = waiting_fixture(app)
    app.restart()
    shell, claude, neutral = waiting_seats(app, producer)
    app.foreground(False)
    default = questions(app, (shell, claude), "default-on")
    admitted(default, (shell, claude))
    require(default["posts"] == 2, "fresh default did not post once for each harness")
    app.toggle("zsh", False)
    off = questions(app, (shell, claude), "zsh-disabled")
    admitted(off, (claude,))
    require(off["posts"] - default["posts"] == 1, "disabled harness posted")
    app.toggle("Only in the background", True)
    page = app.section("Notifications")
    enabled_image = app.capture("window", "g2-rows-enabled", frame(page))
    enabled_contrast = control_contrast(enabled_image, control(page, "Only in the background"))
    page = app.toggle("Desktop notifications", False)
    before_disabled_clicks = app.persisted()
    for label, expected in (("Only in the background", True), ("zsh", False), ("claude", True)):
        require(toggle_value(page, label) is expected, "disabled row lost value: " + label)
        page = app.click(control(page, label))
        require(toggle_value(page, label) is expected, "disabled row accepted edit: " + label)
    require(app.persisted() == before_disabled_clicks, "disabled row changed saved values")
    disabled_image = app.capture("window", "g2-rows-disabled", frame(page))
    disabled_contrast = control_contrast(disabled_image, control(page, "Only in the background"))
    require(0 < disabled_contrast < enabled_contrast * .8, "disabled Settings rows were not visually faded")
    global_off = questions(app, (shell, claude), "globally-disabled")
    admitted(global_off, ())
    require(global_off["posts"] == off["posts"], "globally disabled notifications posted")
    app.foreground(True)
    app.toggle("Desktop notifications", True)
    app.ctl("ui", "key", "esc")
    app.ctl("ui", "focus", neutral)
    foreground = questions(app, (shell, claude), "foreground-suppressed")
    admitted(foreground, ())
    require(foreground["posts"] == global_off["posts"], "only-background policy posted while frontmost")
    app.foreground(False)
    background = questions(app, (shell, claude), "background-admitted")
    admitted(background, (claude,))
    negative(app.work, "inverted-background-policy", lambda: admitted(background, ()))
    negative(app.work, "disabled-harness-admitted", lambda: admitted(background, (shell, claude)))
    app.foreground(True)
    app.toggle("Only in the background", False)
    app.ctl("ui", "key", "esc")
    app.ctl("ui", "focus", neutral)
    unrestricted = questions(app, (shell, claude), "foreground-unrestricted")
    admitted(unrestricted, (claude,))
    negative(app.work, "inverted-foreground-policy", lambda: admitted(unrestricted, ()))
    app.toggle("Only in the background", True)
    saved = app.persisted()
    require(saved["notifications"] == {"enabled": True, "only_when_not_frontmost": True,
                                        "disabled_harnesses": ["zsh"]}, "saved policy does not match real controls")
    require(set(saved) == {"notifications"}, "Notifications changed an unrelated setting")
    app.restart()
    page = app.section("Notifications")
    for label, expected in (("Desktop notifications", True), ("Only in the background", True),
                            ("zsh", False), ("claude", True)):
        require(toggle_value(page, label) is expected, "restart lost " + label)
    require(app.persisted() == saved, "restart changed saved policy")
    shell, claude, neutral = waiting_seats(app, producer)
    app.foreground(True)
    restarted_foreground = questions(app, (shell, claude), "restart-foreground")
    admitted(restarted_foreground, ())
    app.foreground(False)
    restarted_background = questions(app, (shell, claude), "restart-background")
    admitted(restarted_background, (claude,))
    app.restart(recording=False)
    refused = app.ctl("ui", "notification-foreground", "true", refused=True)
    require("notification_foreground_refused" in refused["message"], "non-recording call had another refusal")
    layout = app.ctl("ui", "layout")
    require("notification_frontmost" not in layout, "non-recording app exposed private focus state")
    try:
        app.restart(recording=False, verified=False)
        ordinary = app.ctl("ui", "notification-foreground", "true", refused=True)
        require("notification_foreground_refused" in ordinary["message"], "ordinary account had another refusal")
        require("notification_frontmost" not in app.ctl("ui", "layout"),
                "ordinary account exposed private focus state")
    finally:
        if app.run.name == "ordinary-run":
            # Restore the runner's original file-account endpoint for its cleanup.
            app.restart()
    report = {"defaults": {"enabled": True, "only_when_not_frontmost": False},
              "saved": saved, "disabled_harness": "zsh", "enabled_harness": "claude",
              "os_presentation": False, "boundary_refusal": refused, "ordinary_refusal": ordinary,
              "row_contrast": {"enabled": enabled_contrast, "disabled": disabled_contrast},
              "phases": ["default-on", "zsh-disabled", "globally-disabled", "foreground-suppressed",
                         "background-admitted", "foreground-unrestricted", "restart-foreground", "restart-background"]}
    (app.work / "g2-result.json").write_text(json.dumps(report, indent=2) + "\n")
    print("G2: actual named Notifications defaults, per-harness off, faded disabled rows, foreground/background policy and persistence; private recording only")


if __name__ == "__main__":
    {"G1": g1, "G2": g2}[sys.argv[1]]()
