"""L4: native clients execute scripted local tool calls against the real bridge.

Model judgement is advisory (spec 0177a9cb). The verdict comes from ctl's read-only
control journal and task records, combined with the native API result ordering.
No model response contains a verdict, and the provider never calls MCP itself.
"""
import hashlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

# Isolated Python excludes the script directory; load only our committed helpers.
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from native_checks import approve_codex_hooks, configure_codex_provider, until
from task_checks import ctl, operation, seats, status


WORK = Path(os.environ["E2E_WORK"])
MARKER = "L4_NATIVE_OK"
SUMMARY = "L4 native first-turn protocol fixture"


def objects(value):
    if isinstance(value, dict):
        yield value
        for item in value.values():
            yield from objects(item)
    elif isinstance(value, list):
        for item in value:
            yield from objects(item)


class Provider(http.server.BaseHTTPRequestHandler):
    plans = {}
    requests = []

    def log_message(self, *_):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path.endswith("/count_tokens"):
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"input_tokens":64}')
            return
        kind = "codex" if self.path.endswith("/responses") else "claude"
        assert self.path.endswith("/responses") or "/messages" in self.path
        # The native client's initial request can precede spawn's produced-frame
        # receipt. Only this fixture model waits for the real task delegation;
        # neither the app nor another seat is blocked by this local request.
        deadline = time.monotonic() + 20
        while kind not in self.plans:
            assert time.monotonic() < deadline, "fixture delegation was not admitted"
            time.sleep(.01)
        plan = self.plans[kind]
        results = {x.get("tool_use_id", x.get("call_id")) for x in objects(body)
                   if x.get("type") in ("tool_result", "function_call_output")}
        stem = "call_l4_" + kind + "_"
        stage = ("orient" if stem + "orient" not in results else
                 "report" if stem + "report" not in results else "done")
        args = (dict(op_id=plan["orient_op"]) if stage == "orient" else dict(
            op_id=plan["report_op"], task_path=plan["task_path"], summary=SUMMARY,
            verify_result=dict(command="true", exit_code=0, evidence=[dict(
                criterion="L4", reference="independent true command exit 0")])) )
        self.requests.append(dict(
            kind=kind, stage=stage, model=body.get("model"),
            seen_results=sorted(str(x) for x in results),
            request_sha256=hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()))
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()

        def event(name, **data):
            self.wfile.write(("event: " + name + "\ndata: " +
                             json.dumps(dict(type=name, **data)) + "\n\n").encode())
            self.wfile.flush()

        if kind == "claude":
            self.messages(event, body, stage, stem, args)
        else:
            self.send_responses(event, body, stage, stem, args)

    @staticmethod
    def messages(event, body, stage, stem, args):
        event("message_start", message=dict(
            id="msg_" + stage, type="message", role="assistant", model=body["model"],
            content=[], stop_reason=None, stop_sequence=None,
            usage=dict(input_tokens=64, output_tokens=0)))
        if stage == "done":
            event("content_block_start", index=0, content_block=dict(type="text", text=""))
            event("content_block_delta", index=0, delta=dict(type="text_delta", text=MARKER))
        else:
            event("content_block_start", index=0, content_block=dict(
                type="tool_use", id=stem + stage,
                name="mcp__bboxy__" + ("orient" if stage == "orient" else "task_report"), input={}))
            event("content_block_delta", index=0,
                  delta=dict(type="input_json_delta", partial_json=json.dumps(args)))
        event("content_block_stop", index=0)
        event("message_delta", delta=dict(
            stop_reason="end_turn" if stage == "done" else "tool_use", stop_sequence=None),
            usage=dict(output_tokens=4))
        event("message_stop")

    @staticmethod
    def send_responses(event, body, stage, stem, args):
        # Codex 0.163 advertises namespaced functions. Use those declared names,
        # not the unsupported pre-namespace mcp__bboxy__orient alias.
        # https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/protocol/src/models.rs#L1028
        if stage == "done":
            item = dict(id="msg_l4", type="message", status="completed", role="assistant",
                        content=[dict(type="output_text", text=MARKER, annotations=[])])
        else:
            item = dict(id="fc_" + stage, type="function_call", namespace="mcp__bboxy",
                        name="orient" if stage == "orient" else "task_report",
                        arguments=json.dumps(args), call_id=stem + stage, status="completed")
        response = dict(id="resp_" + stage, object="response", status="completed",
                        created_at=int(time.time()), model=body["model"], output=[item],
                        usage=dict(input_tokens=64, output_tokens=4, total_tokens=68))
        event("response.created", response=dict(response, status="in_progress", output=[]))
        event("response.output_item.added", output_index=0, item=dict(
            item, status="in_progress", **(dict(content=[]) if stage == "done" else dict(arguments=""))))
        if stage == "done":
            event("response.content_part.added", item_id=item["id"], output_index=0,
                  content_index=0, part=dict(type="output_text", text="", annotations=[]))
            event("response.output_text.delta", item_id=item["id"], output_index=0,
                  content_index=0, delta=MARKER)
            event("response.output_text.done", item_id=item["id"], output_index=0,
                  content_index=0, text=MARKER)
            event("response.content_part.done", item_id=item["id"], output_index=0,
                  content_index=0, part=item["content"][0])
        else:
            event("response.function_call_arguments.delta", item_id=item["id"], output_index=0,
                  delta=item["arguments"])
            event("response.function_call_arguments.done", item_id=item["id"], output_index=0,
                  arguments=item["arguments"])
        event("response.output_item.done", output_index=0, item=item)
        event("response.completed", response=response)


def native_result(kind, seat):
    if kind == "claude":
        path = WORK / (seat + ".native.jsonl")

        def read():
            if not path.exists():
                return []
            return [json.loads(line) for line in path.read_text().splitlines()
                    if line.startswith("{") and line.endswith("}")]

        rows = until(read, lambda xs: any(x.get("type") == "result" for x in xs),
                     "native Claude completion")
        result = next(x for x in rows if x.get("type") == "result")
        assert not result["is_error"] and result["result"] == MARKER, result
        return dict(session=result["session_id"], api_turns=result["num_turns"])
    screen = until(lambda: ctl("seat", "screen", seat)["data"],
                   lambda x: json.dumps(x).count(MARKER) >= 2, "native Codex completion")
    return dict(screen_sequence=screen["sequence"])


def case(kind):
    created = ctl("task", "create", "L4 " + kind + " first-turn fixture", "--verify", "true",
                  "--brief", "Private protocol fixture only: orient, report this task with its "
                  "declared verification result, then stop.")["data"]["record"]
    seat = ctl("seat", "spawn", kind, "--role", "executor", "--place", "tab",
               "--cwd", str(WORK), "--name", "L4 native " + kind)["data"]["seat"]
    ctl("task", "delegate", "#" + str(created["number"]), seat)
    initial = status(created["path"])
    assert initial["metadata"]["status"] == "claimed"
    assert initial["metadata"]["assignee"] == "seat:" + seat
    assert initial["metadata"]["verify"] == "true"
    verified = subprocess.run(["true"], capture_output=True, text=True, timeout=5)
    assert verified.returncode == 0, verified
    plan = dict(task_path=created["path"], orient_op=operation(), report_op=operation())
    Provider.plans[kind] = plan
    if kind == "codex":
        assert approve_codex_hooks(seat, marker=MARKER, prefix="l4"), "fresh hook review absent"
    item = until(lambda: status(created["path"]),
                 lambda x: x["metadata"]["status"] == "reported", "native task report")
    metadata = item["metadata"]
    assert metadata["verify_result"]["command"] == metadata["verify"] == "true"
    assert metadata["verify_result"]["exit_code"] == verified.returncode == 0
    assert metadata["report"]["reported_by"] == "seat:" + seat
    assert metadata["report"]["op_id"] == plan["report_op"]
    assert item["body"].count(SUMMARY) == 1, "native report duplicated"
    generation = seats()[seat]["record"]["generation"]
    assert generation == 1, "fixture resumed an existing seat"
    journals = []
    for op, tool in [(plan["orient_op"], "orient"), (plan["report_op"], "task_report")]:
        row = ctl("debug", "journal", "seat/" + seat + "/" + op)["data"]
        assert row["mode"] == "ro" and row["query_only"] is True, row
        entry = row["entry"]
        assert entry["tool"] == tool and entry["result"]["isError"] is False, entry
        assert entry["result"]["structuredContent"]["op_id"] == op, entry
        assert entry["actor"]["seat_id"] == seat, entry
        assert entry["actor"]["process_generation"] == generation, entry
        assert entry["actor"]["role"] == dict(name="executor", **{"class": "worker"}), entry
        journals.append(row)
    orient = journals[0]["entry"]["result"]["structuredContent"]
    assert created["id"] in {x["id"] for x in orient["tasks"]["items"]}, orient
    assert orient["tasks"]["scope"]["seat"] == seat, orient
    requests = until(lambda: [x for x in Provider.requests if x["kind"] == kind],
                     lambda xs: [x["stage"] for x in xs[:3]] == ["orient", "report", "done"]
                     and all(x["stage"] == "done" for x in xs[3:]), "native tool-result order")
    result = native_result(kind, seat)
    proof = dict(kind=kind, seat=seat, generation=generation, task=created["id"],
                 item=item, journals=journals, requests=requests, native=result)
    (WORK / ("l4-" + kind + "-proof.json")).write_text(json.dumps(proof, indent=2) + "\n")
    print(json.dumps(dict(kind="L4-case", harness=kind, model="scripted localhost",
                          seat=seat, generation=generation, task=created["id"],
                          reported_by=metadata["report"]["reported_by"],
                          calls=[x["entry"]["tool"] for x in journals], native=result)), flush=True)


def main():
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Provider)
    configure_codex_provider(server.server_port)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    config = WORK / "l3-native-provider.json"
    config.write_text(json.dumps(dict(port=server.server_port, first_turn=True)))
    try:
        for kind in ("claude", "codex"):
            case(kind)
    finally:
        config.unlink(missing_ok=True)
        server.shutdown()
        server.server_close()
        thread.join(timeout=3)
        (WORK / "l4-native-requests.json").write_text(json.dumps(Provider.requests, indent=2) + "\n")
    print("L4 PASS: installed clients, scripted localhost; first orient/report routed by the "
          "real MCP bridge, seat/generation attributed, declared verification exit 0", flush=True)


if __name__ == "__main__":
    main()
