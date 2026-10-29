"""Join actual private-app copy fields to the real native request submission port.

Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
Private process setup adapted from this project's Settings fixtures.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from settings import SettingsApp, negative, toggle_value, waiting_fixture
from ui import frame, require


app = SettingsApp()
require(not app.config.exists(), "copy proof requires fresh default preferences")
page = app.section("Notifications")
require(toggle_value(page, "Desktop notifications")
        and not toggle_value(page, "Only in the background"), "notification defaults changed")
for harness in ("zsh", "claude"):
    require(toggle_value(page, harness), "fresh harness is disabled: " + harness)
app.capture("window", "copy-defaults", frame(page))
producer = waiting_fixture(app)
producer.write_text("import base64,json,sys\nprint('COPY_READY',flush=True)\n"
    "for raw in sys.stdin:\n"
    " row=json.loads(raw)\n"
    " detail=base64.b64encode(row['question'].encode()).decode()\n"
    " print('\\033]7501;state=blocked:kind=question:msg='+detail+'\\007',end='',flush=True)\n"
    " print('COPY_ACK:'+row['nonce'],flush=True)\n")
app.restart()
names = {"zsh": "Quill 🦊 café", "claude": "Säble 月"}
seats = {}
for harness, name in names.items():
    seat = app.ctl("seat", "spawn", harness, "--place", "tab" if harness == "zsh" else "right",
                   "--cwd", app.work, "--name", name)["seat"]
    seats[harness] = seat
    if harness == "zsh":
        app.line(seat, "exec " + shlex.quote(sys.executable) + " -I -B -u " + shlex.quote(str(producer)))
    app.wait(seat, "(?m)^COPY_READY$")
neutral = app.ctl("seat", "spawn", "zsh", "--place", "down", "--cwd", app.work,
                  "--name", "Copy focused nonwaiting shell")["seat"]
layout = app.ctl("ui", "focus", neutral)
require(len(layout["panes"]) == 3, "copy fixture needs three visible real PTYs")
app.foreground(False)
roster = {r["record"]["id"]: r["record"] for r in app.ctl("seat", "ls")}
for harness, seat in seats.items():
    require(roster[seat]["name"] == names[harness] and roster[seat]["harness_kind"] == harness,
            "actual seat display/harness differs from the copy fixture")


def ask(seat, nonce, question):
    # Both nonempty lines reach the actual PTY/detail parser. Only the first
    # nonblank question line belongs in the notification body (attention §3).
    payload = {"nonce": nonce, "question": " \n" + question + "\nPrivate second line stays out of the body"}
    app.line(seat, json.dumps(payload, ensure_ascii=False))
    app.wait(seat, "(?m)^COPY_ACK:" + re.escape(nonce) + "$")
    app.ctl("seat", "wait", seat, "--state", "waiting", "--timeout", "20000")


def copies(state, expected):
    observed = {r["id"]: r for r in state["outstanding"]}
    require(set(observed) == set(expected), "wrong notification identities: " + str(observed))
    for seat, fields in expected.items():
        require(observed[seat] == {"id": seat, **fields}, "wrong title/body at recording boundary: " + str(observed[seat]))
    return observed


questions = {"zsh": "¿Dónde está el café?", "claude": "この一行を確認しますか？"}
expected = {seat: {"title": names[harness] + " · " + harness, "body": questions[harness]}
            for harness, seat in seats.items()}
for harness, seat in seats.items():
    ask(seat, "initial-" + harness, questions[harness])
initial = app.record("copy-initial", lambda s: s["outstanding_count"] == 2)
observed = copies(initial, expected)
require(initial["posts"] == 2 and initial["replacements"] == 0, "default question episodes did not post once")
requests = [observed[seats[harness]] for harness in ("zsh", "claude")]
seat = seats["claude"]
replacement_question = "¿Revisas esta segunda pregunta, 🦊?"
ask(seat, "replacement", replacement_question)
expected[seat]["body"] = replacement_question
replacement = app.record("copy-replacement", lambda s: s["posts"] == initial["posts"] + 1)
observed = copies(replacement, expected)
require(replacement["replacements"] == 1 and replacement["outstanding_count"] == 2,
        "replacement stacked or changed notification identity")
requests.append(observed[seat])
layout = app.ctl("ui", "layout", label="copy-before-mute")
key = "pane-mute:" + seat
require(key in layout["regions"], "actual pane mute control is absent")
layout = app.click({"rect": layout["regions"][key]})
require(layout["focused"] == neutral, "mute control changed focus and handled the waiting episode")
row = next(row for row in app.ctl("seat", "ls") if row["record"]["id"] == seat)
require(row["record"]["muted"] is True, "actual mute control did not mute its captured seat")
muted = app.record("copy-muted", lambda s: s["outstanding_count"] == 1
                   and any(r["seat"] == seat and r["muted"] for r in s["waiting"]))
remaining = {seats["zsh"]: expected[seats["zsh"]]}
copies(muted, remaining)
require(muted["posts"] == replacement["posts"] and any(
    r.get("id") == seat and r["operation"] == "remove" and r["stage"] == "completion"
    for r in muted["records"]), "mute did not withdraw the matching notice")
ask(seat, "muted-question", "A muted question must not post")
app.foreground(False)
suppressed = app.record("copy-muted-suppressed")
copies(suppressed, remaining)
require(suppressed["posts"] == muted["posts"] and suppressed["replacements"] == muted["replacements"],
        "muted seat submitted another notice")
negative(app.work, "wrong-native-title", lambda: require(requests[0]["title"] == "wrong · zsh", "wrong title rejected"))
negative(app.work, "body-with-seat-header", lambda: require(requests[0]["body"] == names["zsh"] + ": " + questions["zsh"], "body header rejected"))
negative(app.work, "muted-notice-admitted", lambda: require(seat in {r["id"] for r in suppressed["outstanding"]}, "muted notice rejected"))
witness = app.work / "native-copy-input.bin"
witness.write_bytes(b"".join(row[key].encode() + b"\0" for row in requests for key in ("id", "title", "body")))
capture = app.work / "native-copy-capture.bin"
test_binary = Path(os.environ["BBOXY_NATIVE_COPY_TEST_BIN"]).resolve(strict=True)
native_env = dict(os.environ, BBOXY_NOTIFICATION_COPY_INPUT=str(witness),
                  BBOXY_NOTIFICATION_COPY_CAPTURE=str(capture))
with (app.work / "native-copy-submission.log").open("w") as log:
    result = subprocess.run([str(test_binary), "--exact", "tests::native_copy_requests_from_private_app",
                             "--ignored", "--nocapture"], env=native_env, cwd=app.work,
                            stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, timeout=30)
require(result.returncode == 0, "actual native request submission assertion failed")
require(capture.read_bytes() == witness.read_bytes(), "native request objects changed the app's exact fields")
report = {"requests": requests, "source": "actual private app attention/notification worker",
          "native_test_binary": str(test_binary), "native_test_sha256": hashlib.sha256(test_binary.read_bytes()).hexdigest(),
          "native_exit_code": result.returncode, "native_capture_sha256": hashlib.sha256(capture.read_bytes()).hexdigest(),
          "stable_replacement_id": seat, "mute_withdrawn_id": seat, "muted_new_posts": suppressed["posts"] - muted["posts"],
          "os_presentation": False, "native_submission_port": "private test-thread observer"}
(app.work / "notification-copy-result.json").write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
print("Native request submission observed exact Unicode seat/harness titles, question-only bodies and stable replacement IDs; real worker mute withdrew/suppressed the notice", flush=True)
