"""T1's assertions over the real UI owner's ctl receipts and PTYs."""
import json
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ui import App, frame, pane, require


def roster(app):
    return {row["record"]["id"]: row for row in app.ctl("seat", "ls")}


def terminal(app, seat, row, kind):
    pid = row["state"]["process"].get("pid")
    require(row["state"]["process"]["kind"] == "running" and type(pid) is int,
            "launched seat has no running process")
    if kind == "zsh":
        app.line(seat, "printf '\\nT1_READY zsh %s %s %s\\n' " + seat + ' "$$" "$(tty)"')
    app.wait(seat, rf"(?m)^T1_READY {kind} {seat} {pid} /dev/[^\s]+$")
    nonce = "input-" + seat
    if kind == "zsh":
        app.line(seat, "printf '\\nT1_REPLY " + nonce + "\\n'")
    else:
        app.line(seat, "T1_PING " + nonce)
    app.wait(seat, "(?m)^T1_REPLY " + re.escape(nonce) + "$")


def placed(before, after, seat, place):
    actual = pane(after, seat)
    x, y, w, h = actual["rect"]
    require(w > 0 and h > 0 and actual["cols"] > 1 and actual["rows"] > 0,
            "new pane has no usable solved rectangle")
    require(actual["focused"] and after["focused"] == seat, "new pane is not focused")
    if place == "tab":
        require(after["tab"] != before["tab"], "new-tab pick reused the old tab")
        require({p["seat"] for p in after["panes"]} == {seat}, "new tab has another pane")
        return
    require(after["tab"] == before["tab"] and after["swarm"] == before["swarm"],
            "split escaped its captured tab or swarm")
    anchor = pane(after, before["focused"])
    ax, ay, aw, ah = anchor["rect"]
    if place == "right":
        require(x >= ax + aw and abs(y - ay) < 1 and abs(h - ah) < 1,
                "right pick was not solved to the right of its captured anchor")
    else:
        require(y >= ay + ah and abs(x - ax) < 1 and abs(w - aw) < 1,
                "down pick was not solved below its captured anchor")


def main():
    app = App()
    pool = set(json.loads((Path(__file__).resolve().parents[2] /
                          "crates/bbt-proto/src/seat_names.json").read_text())["pool"])
    app.ctl("ui", "clock", "freeze")
    observations = []
    known_names = {}
    for kind, index in (("zsh", 4), ("claude", 0), ("codex", 1)):
        for place, depth in (("tab", 0), ("right", 1), ("down", 2)):
            old = roster(app)
            before = app.ctl("ui", "layout")
            keys = ["cmd-t"] + ["down"] * index
            if place != "tab":
                keys += ["right"] * (2 if kind == "claude" else 1)
                keys += ["down"] * depth
            opened = app.ctl("ui", "key", *keys)
            require(opened["launcher_open"], "Cmd-T did not open the real launcher")
            after = app.ctl("ui", "key", "enter", label=f"t1-{kind}-{place}-launch")
            new = roster(app)
            added = set(new) - set(old)
            require(len(added) == 1 and set(old) <= set(new), "launcher did not add exactly one seat")
            seat = added.pop()
            record = new[seat]["record"]
            require(record["harness_kind"] == kind, "launcher row started the wrong harness")
            require(record["name"] in pool, "default name is outside the shared furry pool")
            names = known_names.setdefault(record["swarm"], set())
            require(record["name"] not in names, "default names collide within the swarm")
            names.add(record["name"])
            require(not after["launcher_open"], "picked launcher did not close")
            placed(before, after, seat, place)
            receipt = after.get("spawned") or {}
            first = receipt.get("starting")
            require(receipt.get("seat") == seat and isinstance(first, dict),
                    "spawn lacks its correlated, actually produced starting-frame receipt")
            require(first["admitted_frame_seq"] >= frame(opened),
                    "starting receipt predates the launch input")
            require(first["name"] == record["name"], "chosen name changed without an actual-name reply")
            require(first["rect"] == pane(after, seat)["rect"],
                    "real pane moved out of its starting slot")
            require(after["seats"][0]["record"]["name"] == record["name"],
                    "spawn reply does not report the actual assigned name")
            terminal(app, seat, new[seat], kind)
            app.capture(seat, f"t1-{kind}-{place}", frame(after))
            observation = {"harness": kind, "place": place, "seat": seat,
                           "opened_frame": frame(opened), "ready_frame": frame(after),
                           "admitted_frame": first["admitted_frame_seq"],
                           "starting_frame": first["frame_seq"],
                           "frames": first["frame_seq"] - first["admitted_frame_seq"]}
            observations.append(observation)
            print(json.dumps(observation), flush=True)
    custom = app.ctl("seat", "spawn", "zsh", "--place", "tab", "--cwd", app.work,
                     "--name", "T1 Explicit Name")["seat"]
    require(roster(app)[custom]["record"]["name"] == "T1 Explicit Name",
            "explicit name was changed")
    (app.work / "t1-frame-observations.json").write_text(json.dumps(observations, indent=2) + "\n")
    require(all(row["frames"] == 1 for row in observations),
            "T1 one-frame launch bound failed: " + json.dumps(observations))
    print("T1: launcher rows/placements, real PTYs/input, pool names, captures and one-frame startup held")


if __name__ == "__main__":
    main()
