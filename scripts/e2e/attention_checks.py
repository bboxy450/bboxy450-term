"""MA-9: real native wake inputs, deterministic loopback model, no provider secrets.

The installed Codex client receives the app's native deliveries. Its HTTP inputs,
the durable message IDs and receipts decide the verdict, never model prose.
"""
import http.server
import json
import os
from pathlib import Path
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
from messaging import App, operation, require
from native_checks import approve_codex_hooks, configure_codex_provider


MARKER = "M7_NATIVE_OK"


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)


class Model(http.server.ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self):
        super().__init__(("127.0.0.1", 0), Provider)
        self.requests = []
        self.errors = []
        self.lock = threading.Lock()

    def inputs(self):
        with self.lock:
            require(not self.errors, f"local model protocol failed: {self.errors}")
            return list(self.requests)


class Provider(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        try:
            require(self.path == "/v1/responses", "unexpected local model endpoint")
            size = int(self.headers["Content-Length"])
            require(0 < size <= 8 * 1024 * 1024, "unbounded local model request")
            body = json.loads(self.rfile.read(size))
            user = ["\n".join(strings(item.get("content", [])))
                    for item in body.get("input", []) if item.get("role") == "user"]
            require(user, "native model request has no user input")
            with self.server.lock:
                index = len(self.server.requests)
                self.server.requests.append(dict(index=index, latest_user=user[-1]))
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.end_headers()

            def event(name, **data):
                self.wfile.write(("event: " + name + "\ndata: " +
                                  json.dumps(dict(type=name, **data)) + "\n\n").encode())
                self.wfile.flush()

            part = dict(type="output_text", text=MARKER, annotations=[])
            item = dict(id=f"msg_m7_{index}", type="message", status="completed",
                        role="assistant", content=[part])
            response = dict(id=f"resp_m7_{index}", object="response", status="completed",
                            created_at=int(time.time()), model=body["model"], output=[item],
                            usage=dict(input_tokens=64, output_tokens=4, total_tokens=68))
            event("response.created", response=dict(response, status="in_progress", output=[]))
            event("response.output_item.added", output_index=0,
                  item=dict(item, status="in_progress", content=[]))
            event("response.content_part.added", item_id=item["id"], output_index=0,
                  content_index=0, part=dict(part, text=""))
            event("response.output_text.delta", item_id=item["id"], output_index=0,
                  content_index=0, delta=MARKER)
            event("response.output_text.done", item_id=item["id"], output_index=0,
                  content_index=0, text=MARKER)
            event("response.content_part.done", item_id=item["id"], output_index=0,
                  content_index=0, part=part)
            event("response.output_item.done", output_index=0, item=item)
            event("response.completed", response=response)
        except Exception as error:
            with self.server.lock:
                self.server.errors.append(repr(error))
            raise


def until(read, ready, description):
    # Completion guard only. No elapsed duration is an acceptance predicate.
    deadline = time.monotonic() + 60
    while True:
        value = read()
        if ready(value):
            return value
        require(time.monotonic() < deadline, f"{description}: {value}")
        time.sleep(.05)


def seat(app, receiver):
    return next(row for row in app.ctl("seat", "ls")["data"]
                if row["record"]["id"] == receiver)


def idle(app, receiver):
    row = until(lambda: seat(app, receiver),
                lambda r: r["state"]["activity"]["kind"] == "idle"
                and r["state"]["grade"] in ("shim", "reported")
                and not r["state"]["turn_open"], "native receiver did not become idle")
    require(row["record"]["role"] == {"name": "executor", "class": "worker"}
            and row["record"].get("attention") is None, "receiver is not on role defaults")
    return row["state"]


def receipt(app, message, receiver):
    rows = [r for r in app.receipts(message) if r["recipient"] == receiver]
    require(len(rows) == 1, f"missing or duplicate recipient receipt: {rows}")
    return rows[0]


def admitted(app, message, receiver):
    row = until(lambda: receipt(app, message, receiver),
                lambda r: r["evidence"] == "native-queued", "native admission not witnessed")
    require(row["attempt"] and row["native_id"] and row["held"] is None,
            "native receipt is missing its attempt or native ID")
    return row


def check(app, model):
    work = app.work
    configure_codex_provider(model.server_port)
    require((work.parent / "home/.l3-private-home").is_file(), "no private harness home")
    (work / "l3-native-provider.json").write_text(json.dumps({"port": model.server_port}) + "\n")
    sender, swarm = app.spawn("M7 routing fixture", "architect")
    status_name = "M7 status " + operation()
    status_seat, _ = app.spawn(status_name, swarm=swarm)
    receiver = app.ctl("seat", "spawn", "codex", "--role", "executor", "--place", "tab",
                       "--cwd", str(work), "--name", "M7 idle receiver", "--swarm", swarm)["data"]["seat"]
    hook_review = approve_codex_hooks(receiver, marker=MARKER, prefix="m7")
    app.ctl("seat", "wait", receiver, "--text", MARKER, "--timeout", "60000")
    initial_idle = idle(app, receiver)
    baseline = len(model.inputs())
    require(baseline > 0, "native bootstrap never reached the local model")

    # Typed Presence is the existing production status producer. Mute a different
    # seat so the receiver stays eligible and idle throughout the negative case.
    app.muted(status_seat, True)
    app.muted(status_seat, False)
    events = [m["envelope"] for m in app.inbox(sender)
              if m["envelope"]["kind"] == "presence"
              and m["envelope"]["body"].get("name") == status_name]
    require(len(events) == 2 and {m["body"]["event"] for m in events} == {"mute", "unmute"},
            "typed event producer did not store both status records")
    event_ids = [m["id"] for m in events]
    for mid in event_ids:
        row = receipt(app, mid, receiver)
        require(row["wake"] == "none" and row["evidence"] == "recipient-stored"
                and row["held"] is None and row["attempt"] is None
                and row["native_id"] is None, f"default event was not inbox-only: {row}")
    idle(app, receiver)
    require(len(model.inputs()) == baseline, "status event woke the idle native client")

    # The positive DM is also a native-pump barrier for those earlier events.
    direct = app.send(receiver, "M7_DM_" + operation())["data"]["message"]["envelope"]["id"]
    direct_receipt = admitted(app, direct, receiver)
    require(direct_receipt["wake"] == "now", "DM did not admit an immediate wake")
    after_dm = until(model.inputs, lambda rows: len(rows) >= baseline + 1, "DM model turn absent")
    require(len(after_dm) == baseline + 1 and direct in after_dm[-1]["latest_user"],
            "DM did not produce exactly one native input with its message ID")
    idle(app, receiver)

    destination = "channels/M7"
    app.routes(sender, destination, [receiver])
    calls = [("message_send", {"destination": destination + "/**", "text": f"M7 body {n}"})
             for n in range(49)]
    stored = app.agent(sender, *calls)
    require(all(r["kind"] == "stored" for r in stored), "digest messages were not stored")
    ids = [r["message"]["envelope"]["id"] for r in stored]
    deferred = [receipt(app, mid, receiver) for mid in ids]
    require(all(r["wake"] == "deferred" and r["until"] and r["held"] is None
                and r["attempt"] is None and r["evidence"] == "recipient-stored"
                for r in deferred), "the first 49 messages did not remain deferred")
    require(len({r["until"] for r in deferred}) == 1, "digest members changed their first deadline")
    require(len(model.inputs()) == baseline + 1, "digest arrived before its cap")
    last = app.agent(sender, ("message_send", {"destination": destination + "/**",
                                             "text": "M7 body 49"}))[0]
    require(last["kind"] == "stored", "fiftieth message was not stored")
    ids.append(last["message"]["envelope"]["id"])
    require(len(ids) == len(set(ids)) == 50, "not 50 distinct digest members")
    final_receipts = sorted([admitted(app, mid, receiver) for mid in ids],
                            key=lambda row: row["message_sequence"])
    # The durable arrival sequence, rather than concurrent MCP response order,
    # is the contract the native digest must preserve.
    require(len({r["message_sequence"] for r in final_receipts}) == 50,
            "digest member arrival sequence is not unique")
    ids = [r["message"] for r in final_receipts]
    attempts = {r["attempt"] for r in final_receipts}
    native_ids = {r["native_id"] for r in final_receipts}
    require(len(attempts) == len(native_ids) == 1, "cap did not produce one native delivery")
    require(all(r["wake"] == "deferred" and r["until"] == deferred[0]["until"]
                for r in final_receipts), "delivery changed the admitted digest decision")
    final_inputs = until(model.inputs, lambda rows: len(rows) >= baseline + 2,
                         "digest model turn absent")
    idle(app, receiver)
    final_inputs = model.inputs()
    require(len(final_inputs) == baseline + 2, "cap produced more than one model turn")
    digest = final_inputs[-1]["latest_user"]
    require(digest.count("Bboxy message digest") == 1 and "messages: 50\n" in digest,
            "native input is not one fifty-message summary")
    positions = [digest.find(mid) for mid in ids]
    require(all(digest.count(mid) == 1 for mid in ids)
            and positions == sorted(positions), "native digest omitted, duplicated or reordered IDs")
    require("M7 body" not in digest, "digest repasted message bodies")
    require(not any(mid in row["latest_user"] for mid in event_ids for row in final_inputs),
            "an event reached native model input")
    for mid in event_ids:
        row = receipt(app, mid, receiver)
        require(row["wake"] == "none" and row["attempt"] is None,
                "later traffic woke an archived event")

    proof = dict(function="M7", coverage="full", receiver=receiver, issued_by=app.label,
                 headless=True, custody="file", native_client="installed Codex",
                 provider="scripted loopback, no credentials", initial_idle=initial_idle,
                 approved_app_hooks=hook_review, events=event_ids, event_wakes=0,
                 dm=direct, dm_attempt=direct_receipt["attempt"], dm_turns=1,
                 receipt_evidence="native-queued; model input is checked independently",
                 digest_ids=ids, before_cap=49, deadline=deferred[0]["until"],
                 digest_attempt=next(iter(attempts)), digest_native_id=next(iter(native_ids)),
                 digest_turns=1, model_requests=final_inputs,
                 window="MA-4 injected-clock unit contract; no wall-clock wait")
    (work / "m7-result.json").write_text(json.dumps(proof, indent=2) + "\n")
    print("M7: event stored/no wake, DM one native turn, 49 deferred then one ordered native digest at cap50")
    print(json.dumps(proof, separators=(",", ":")))


if __name__ == "__main__":
    model = Model()
    thread = threading.Thread(target=model.serve_forever, daemon=True)
    thread.start()
    try:
        check(App(), model)
    finally:
        model.shutdown()
        model.server_close()
        thread.join()
        with model.lock:
            evidence = dict(requests=model.requests, errors=model.errors)
        (Path(os.environ["E2E_WORK"]) / "m7-native-inputs.json").write_text(
            json.dumps(evidence, indent=2) + "\n")
