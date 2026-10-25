"""Messaging night assertions over ctl and real seat-MCP fixture responses.

The runner owns every app and private file account. Receipt states, raw envelope
signatures and addressed inbox reads decide the result; a fixture prints no verdict.
"""
import base64
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import time


def require(ok, reason):
    if not ok:
        raise AssertionError(reason)


def operation():
    value = (int(time.time() * 1000) << 80) | int.from_bytes(os.urandom(10), "big")
    return "".join("0123456789ABCDEFGHJKMNPQRSTVWXYZ"[(value >> (5 * i)) & 31]
                   for i in range(25, -1, -1))


class App:
    def __init__(self):
        self.binary = os.environ["BBT"]
        self.work = Path(os.environ["E2E_WORK"])
        self.index = 0
        self.label = os.environ["BBOXY_CTL_ISSUER"]
        require(self.label.startswith("test-account:"), "not the runner's test identity")
        status = self.ctl("app", "status")["data"]
        require(status["windows"] == 0 and status["custody_backend"] == "file",
                "not the runner's private headless app")

    def ctl(self, *args, exit=0):
        self.index += 1
        result = subprocess.run([self.binary, "ctl", *map(str, args)],
                                stdin=subprocess.DEVNULL, capture_output=True,
                                text=True, timeout=40)
        rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        with (self.work / "message-calls.jsonl").open("a") as log:
            log.write(json.dumps({"args": args, "exit": result.returncode,
                                  "replies": rows, "stderr": result.stderr}) + "\n")
        require(result.returncode == exit and all(r.get("ok") == (exit == 0) for r in rows),
                f"{args}: exit {result.returncode}: {rows}")
        require(rows, f"{args}: no structured reply")
        return rows[0] if len(rows) == 1 else rows

    def spawn(self, name, role="shell", swarm=None, place="tab"):
        args = ["seat", "spawn", "zsh", "--place", place, "--role", role,
                "--name", name, "--cwd", str(self.work)]
        if swarm is not None:
            args += ["--swarm", swarm]
        data = self.ctl(*args)["data"]
        require(data["frame_seq"] > 0, "spawn has no placed-pane frame")
        return data["seat"], data["swarm"]

    def agent(self, seat, *calls):
        tag = operation()
        request, output = self.work / (tag + "-request.json"), self.work / (tag + "-mcp.json")
        request.write_text(json.dumps([dict(tool=tool, args=args, op=operation())
                                       for tool, args in calls]) + "\n")
        driver = Path(__file__).with_name("message_agent.py").resolve()
        command = " ".join(shlex.quote(str(v)) for v in
                           ("python3", "-I", driver, self.binary, request, output, tag))
        self.ctl("seat", "type", seat, command)
        self.ctl("seat", "key", seat, "enter")
        self.ctl("seat", "wait", seat, "--text", "(?m)^MESSAGE_FIXTURE_DONE " + tag + "$",
                 "--timeout", "35000")
        record = json.loads(output.read_text())
        require(record["exit"] == 0, f"seat MCP exited: {record}")
        replies = {r["id"]: r for r in record["responses"] if "id" in r}
        require(set(replies) == set(range(len(calls) + 1)), "missing MCP responses")
        results = []
        for number in range(1, len(calls) + 1):
            response = replies[number]
            require("error" not in response and not response["result"].get("isError"),
                    f"seat MCP refused {calls[number-1][0]}: {response}")
            results.append(response["result"]["structuredContent"])
        return results

    def routes(self, oracle, folder, recipients):
        calls = [("ch_link", {"node": {"path": folder, "kind": "folder"}})]
        calls += [("ch_link", {"node": {"path": folder + "/" + seat,
                                       "kind": "link", "target": f"seats/{seat}/inbox"}})
                  for seat in recipients]
        self.agent(oracle, *calls)

    def send(self, destination, text, swarm=None, op=None):
        args = ["message", "send", destination, text]
        if swarm:
            args += ["--swarm", swarm]
        if op:
            args += ["--op", op]
        reply = self.ctl(*args)
        require(reply["data"]["kind"] == "stored", f"send was not stored: {reply}")
        return reply

    def receipts(self, message):
        rows = self.ctl("message", "receipts", message)["data"]
        return [r.get("receipt", r) for r in rows]

    def received(self, message, expected, held=None):
        deadline = time.monotonic() + 30
        while True:
            rows = self.receipts(message)
            seats = [r for r in rows if not r["recipient"].startswith("operator")
                     and r["evidence"] != "sender-stored"]
            require(len({r["recipient"] for r in seats}) == len(seats), "duplicate recipient receipt")
            require({r["recipient"] for r in seats} == set(expected), f"recipient leak: {seats}")
            if all((held in (r["held"] or "") if held else r["held"] is None) for r in seats):
                return seats
            require(time.monotonic() < deadline, f"receipts never reached held={held}: {seats}")
            time.sleep(0.05)

    def inbox(self, seat):
        # `channel` filters the original signed context, not an arrival sink.
        # Default List is the admitted seat's inbox/outbox visibility.
        result = self.agent(seat, ("message_list", {"limit": 128}))[0]
        return result["page"]["items"]

    def acknowledge(self, seat, messages):
        self.agent(seat, *[("message_ack", {"message": m}) for m in messages])
        for message in messages:
            rows = self.receipts(message)
            row = next(r for r in rows if r["recipient"] == seat)
            require(row["evidence"] == "application-acknowledged", f"no explicit app ACK: {row}")

    def muted(self, seat, muted):
        self.ctl("ui", "focus", seat)
        layout = self.ctl("ui", "key", "cmd-m")["data"]
        require(layout["frame_seq"] > 0, "mute has no produced frame")
        row = next(r for r in self.ctl("seat", "ls")["data"] if r["record"]["id"] == seat)
        require(row["record"]["muted"] == muted, "mute did not reach retained seat state")


def verify_device(app, stored):
    envelope = stored["envelope"]
    require(envelope["seat"] == "0" * 26 and envelope["author"] == envelope["device"],
            "sender is not the operator device")
    require(envelope["role"] == {"name": "operator", "class": "oracle"}
            and envelope["door"] == "ctl" and envelope["issued_by"] == app.label,
            "operator attribution is not the admitted test device")
    fields = [[envelope[k] for k in ("version", "id", "issuer_kind", "issuer", "account", "device",
                                    "seat", "author", "swarm_id", "process_generation", "context")]
              + [{"name": envelope["role"]["name"], "class": envelope["role"]["class"]},
                 envelope["role_grant_id"]],
              [envelope[k] for k in ("role_epoch", "priority_class", "sender_sequence", "thread_id",
                                    "reply_to", "timestamp", "priority", "kind")]
              + [{"type": "text", "text": envelope["body"]["text"]}, envelope["attachments"]]]
    payload = json.dumps([fields, [envelope["door"], envelope["issued_by"]]],
                         ensure_ascii=False, separators=(",", ":")).encode()
    signed = b"bboxy450/message/v1\0" + len(payload).to_bytes(8, "big") + payload
    stem = app.work / envelope["id"]
    public = stem.with_suffix(".public.der")
    public.write_bytes(bytes.fromhex("302a300506032b6570032100" + envelope["author"]))
    signature = stem.with_suffix(".signature")
    signature.write_bytes(base64.urlsafe_b64decode(envelope["signature"] + "=="))
    body = stem.with_suffix(".signed-bytes")
    body.write_bytes(signed)
    openssl = shutil.which("openssl")
    require(openssl is not None, "fixture_dependency_unavailable: openssl Ed25519 verification")
    checked = subprocess.run([openssl, "pkeyutl", "-verify", "-pubin", "-keyform", "DER",
                              "-inkey", str(public), "-rawin", "-in", str(body),
                              "-sigfile", str(signature)], capture_output=True, text=True, timeout=15)
    require(checked.returncode == 0, f"operator signature invalid: {checked.stderr}")
    return envelope


def m1(app):
    oracle, swarm = app.spawn("M1 channel fixture", "architect")
    member, _ = app.spawn("M1 channel member", "executor", swarm)
    nonmember, _ = app.spawn("M1 nonmember", swarm=swarm)
    outside, _ = app.spawn("M1 other swarm", place="space")
    folder = "channels/M1"
    app.routes(oracle, folder, [oracle, member])
    sends = [app.send(member, "M1_DM_" + operation()),
             app.send(folder + "/**", "M1_CHANNEL_" + operation(), swarm),
             app.send("swarms/" + swarm, "M1_SWARM_" + operation(), swarm)]
    expected = [{member}, {oracle, member}, {oracle, member, nonmember}]
    ids = []
    for reply, recipients in zip(sends, expected):
        stored = reply["data"]["message"]
        envelope = verify_device(app, stored)
        require(envelope["id"] == reply["op"], "message and control id differ")
        ids.append(envelope["id"])
        app.received(envelope["id"], recipients)
    retried = app.send(member, sends[0]["data"]["message"]["envelope"]["body"]["text"], op=ids[0])
    require(retried["data"] == sends[0]["data"], "same-op retry changed the stored send")
    for seat in (oracle, member, nonmember, outside):
        messages = app.inbox(seat)
        actual = {row["envelope"]["id"] for row in messages} & set(ids)
        wanted = {mid for mid, recipients in zip(ids, expected) if seat in recipients}
        require(actual == wanted, f"wrong addressed inbox {seat}: {actual} != {wanted}")
        if wanted:
            app.acknowledge(seat, sorted(wanted))
    journal = app.ctl("debug", "journal", "account/" + ids[0])["data"]
    require(journal["mode"] == "ro" and journal["query_only"] is True
            and journal["entry"]["actor"]["door"] == "ctl", "not the operator control journal")
    stream = "swarm-" + swarm
    events = app.ctl("events", "tail", "--stream", stream, "--since", "0", "--timeout", "30000")
    events = events if isinstance(events, list) else [events]
    require(events[-1]["done"] and events[-1]["next_since"] >= 0, "no retained-event cursor")
    text = json.dumps(events)
    require(all(mid in text for mid in ids), "retained stream omitted a tested message")
    (app.work / "m1-result.json").write_text(json.dumps({"messages": ids,
        "operator_signatures_verified": True, "recipient_sets": [sorted(x) for x in expected],
        "outside_swarm": outside, "same_op_replay": True,
        "composer": "not exercised; ctl send uses the same admitted messaging service"}, indent=2) + "\n")
    print("M1: DM/channel/swarm exact inboxes, receipts, explicit app ACK, device signatures, replay and retained events; composer unproven")


def m2(app):
    first, swarm = app.spawn("M2 role fixture", "executor")
    second, _ = app.spawn("M2 same role", "executor", swarm)
    other, _ = app.spawn("M2 other role", "creativity", swarm)
    rows = app.ctl("seat", "ls")["data"]
    members = {r["record"]["id"] for r in rows if r["record"]["role"]["name"] == "executor"}
    require(members == {first, second} and other not in members, "fixture role grants differ")
    (app.work / "m2-unavailable.json").write_text(json.dumps({
        "fixture_members": sorted(members), "nonmember": other,
        "missing": "composer role-chip selection/send/group receipt and above-32 refusal are not exposed by ctl",
        "substituted_send_loops": False}, indent=2) + "\n")
    raise AssertionError("missing_composer_role_control: exact role fixture is ready; role fanout, grouped send and role_recipient_limit_32 cannot be driven by existing ctl")


def m6(app):
    oracle, swarm = app.spawn("M6 routing fixture", "architect")
    recipient, _ = app.spawn("M6 muted recipient", swarm=swarm)
    worker, _ = app.spawn("M6 channel recipient", swarm=swarm)
    group, channel = "groups/M6", "channels/M6"
    app.routes(oracle, group, [recipient])
    app.routes(oracle, channel, [worker])
    app.agent(oracle, ("ch_grant", {"path": group, "seat": worker,
                                  "rights": ["read", "write"]}))
    grouped = app.send(group + "/**", "M6_GROUP_" + operation(), swarm)
    channeled = app.send(channel + "/**", "M6_CHANNEL_" + operation(), swarm)
    for sent, target, prefix in ((grouped, recipient, group), (channeled, worker, channel)):
        envelope = sent["data"]["message"]["envelope"]
        require(envelope["context"].startswith(prefix + "/"), "group/channel destination rerouted")
        app.received(envelope["id"], {target})
    app.muted(recipient, True)
    queued = []
    for sender, label in ((oracle, "ORACLE"), (worker, "WORKER")):
        for priority in (0, 9):
            result = app.agent(sender, ("message_send", {"destination": group + "/**",
                "text": f"M6_{label}_{priority}_" + operation(), "priority": priority}))[0]
            require(result["kind"] == "stored", f"priority send not durable: {result}")
            envelope = result["message"]["envelope"]
            require(envelope["priority"] == priority, "requested priority changed")
            queued.append(envelope)
            app.received(envelope["id"], {recipient}, "recipient muted")
    # The operator's explicit direct-control exception is not evidence that queued
    # group traffic may bypass mute. Check the two paths independently.
    direct = app.send(recipient, "M6_DIRECT_EXCEPTION_" + operation())
    app.received(direct["data"]["message"]["envelope"]["id"], {recipient})
    for envelope in queued:
        app.received(envelope["id"], {recipient}, "recipient muted")
    app.muted(recipient, False)
    for envelope in queued:
        app.received(envelope["id"], {recipient})
    inbox = {row["envelope"]["id"]: row["envelope"] for row in app.inbox(recipient)}
    for envelope in queued:
        require(inbox.get(envelope["id"]) == envelope, "a held priority message was deleted/changed")
    require(channeled["data"]["message"]["envelope"]["id"] not in inbox,
            "channel traffic leaked into the group recipient")
    app.acknowledge(recipient, [e["id"] for e in queued])
    (app.work / "m6-result.json").write_text(json.dumps({"group": group, "channel": channel,
        "queued": [e["id"] for e in queued], "priorities": [e["priority"] for e in queued],
        "classes": [e["priority_class"] for e in queued], "retained_after_unmute": True,
        "priority_dispatch_order": "unproven; explicit inbox ACK is not native scheduling"}, indent=2) + "\n")
    print("M6: groups stay distinct, recipient mute holds group traffic, direct operator exception is separate; all priority/class messages survive unmute and explicit ACK; automatic priority delay unproven")


if __name__ == "__main__":
    {"M1": m1, "M2": m2, "M6": m6}[sys.argv[1]](App())
