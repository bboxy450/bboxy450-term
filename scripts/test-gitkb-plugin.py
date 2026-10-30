#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Real GitKB fixtures, never a mock Markdown store or the operator's KB."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import sys
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--plugin", required=True)
parser.add_argument("--gitkb", required=True)
parser.add_argument("--retain", action="store_true")
parser.add_argument("--evidence")
parser.add_argument("--schema", default=str(Path(__file__).resolve().parent.parent / "crates/bbt-plugin/schemas/data_source-1.0.schema.json"))
args = parser.parse_args()
PLUGIN = Path(args.plugin).resolve()
BACKEND = str(Path(shutil.which(args.gitkb) or args.gitkb).resolve())
RUN = Path(tempfile.mkdtemp(prefix="bboxy-gitkb-fixture-")).resolve()
HOME = RUN / "home"
HOME.mkdir()
(HOME / ".gitconfig").write_text('[user]\nname = Private GitKB Fixture\nemail = fixture@example.invalid\n')
ENV = {"PATH": os.environ["PATH"], "HOME": str(HOME), "TMPDIR": str(RUN), "LANG": "en_US.UTF-8", "TERM": "dumb"}
COMMANDS = []
CLIENTS = []


def run(command, cwd, env=ENV, text=None, json_output=True):
    result = subprocess.run(command, cwd=cwd, env=env, input=text, text=True,
                            capture_output=True, timeout=20)
    COMMANDS.append({"command": command, "cwd": str(cwd), "exit": result.returncode})
    if result.returncode:
        raise AssertionError(str(command) + " exit " + str(result.returncode) + "\n" + result.stdout + result.stderr)
    return json.loads(result.stdout) if json_output else result.stdout


def cli(root, *arguments, text=None, json_output=True):
    return run([BACKEND, "--no-pager", "--color", "never", *arguments], root,
               dict(ENV, GITKB_ROOT=str(root), GITKB_WORKSPACE="main"), text, json_output)


def fixture(name):
    root = RUN / name
    root.mkdir()
    (root / ".bboxy").mkdir()
    run(["git", "init", "--initial-branch=main"], root, json_output=False)
    run(["git", "config", "user.name", "Private GitKB Fixture"], root, json_output=False)
    run(["git", "config", "user.email", "fixture@example.invalid"], root, json_output=False)
    cli(root, "init", "--name", "Private bboxy GitKB conformance", json_output=False)
    return root


class Client:
    def __init__(self, root, binary=PLUGIN, env=ENV, fixture_mode=False):
        self.data = RUN / ("data-" + str(len(CLIENTS)))
        self.data.mkdir()
        self.process = subprocess.Popen([str(binary)], cwd=self.data, env=env,
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, text=True, bufsize=1)
        CLIENTS.append(self)
        self.sequence = 0
        params = {"protocolVersion": "2025-06-18", "root": str(root), "unknown_future": True}
        if fixture_mode:
            params["fixture"] = {"version": 1, "kind": "data_source_conformance"}
        self.initialization = self.rpc("initialize", params)
        self.notify("notifications/initialized", {})

    def notify(self, method, params):
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "method": method, "params": params}) + "\n")
        self.process.stdin.flush()

    def rpc(self, method, params, error=None):
        self.sequence += 1
        self.process.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.sequence, "method": method, "params": params}) + "\n")
        self.process.stdin.flush()
        fixture_init = method == "initialize" and params.get("fixture") == {"version": 1, "kind": "data_source_conformance"}
        commit_call = method == "tools/call" and params.get("name") == "data_source_commit"
        wait = 16 if fixture_init or commit_call else 8
        if not select.select([self.process.stdout], [], [], wait)[0]:
            raise AssertionError("provider failed to answer " + method)
        line = self.process.stdout.readline()
        assert line, "provider exited during " + method
        value = json.loads(line)
        assert value["id"] == self.sequence, value
        if error:
            assert value.get("error", {}).get("data", {}).get("code") == error, value
            return value["error"]
        assert "error" not in value, value
        return value["result"]

    def call(self, method, arguments, error=None):
        result = self.rpc("tools/call", {"name": "data_source_" + method, "arguments": dict(arguments, unknown_future=True)})
        value = result["structuredContent"]
        if error:
            assert result.get("isError") and value["code"] == error, result
        else:
            assert not result.get("isError"), result
        return value

    def create(self, slug, title="Fixture", kind="note", **fields):
        fm = dict(type=kind, title=title, **fields)
        return self.call("write", {"path": slug + ".md", "frontmatter": fm,
                                   "body": "# " + title + "\n", "signed_record": "fixture opaque host proof " + slug})["record"]

    def collect(self, method, arguments):
        items = []
        request = dict(arguments)
        for _ in range(100):
            page = self.call(method, request)
            items.extend(page["items"])
            if not page.get("next"):
                return items
            request["cursor"] = page["next"]
        raise AssertionError("pagination failed to terminate")

    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            self.process.wait(timeout=5)
        assert self.process.returncode == 0, self.process.stderr.read()


def main():
    shape = json.loads(run([str(PLUGIN), "--shape"], RUN, json_output=False))
    bare = run([str(PLUGIN)], RUN, text="")
    assert shape == bare and shape["name"] == "GitKB" and shape["signed_writer"] is True
    skill_bytes = (Path(__file__).resolve().parent.parent / "plugins/gitkb/SKILL.md").read_bytes()
    assert len(shape["skills"]) == 1 and shape["skills"][0]["name"] == "gitkb"
    assert shape["skills"][0]["content"].encode("utf-8") == skill_bytes
    assert "description: " + shape["skills"][0]["description"] + "\n" in skill_bytes.decode("utf-8")
    assert {m["name"] for m in shape["systems"][0]["commands"]} == {"resolve", "fetch", "search", "list", "links", "graph", "write", "changes_since", "commit"}
    a, b = fixture("root-a"), fixture("root-b")
    checker = RUN / "checker-knowledge/.bboxy"
    checker.mkdir(parents=True)
    (checker / "p0-fixture.md").write_text("---\nid: 00000000000000000000000007\ntype: note\nnumber: 7\ntitle: Plugin conformance\n---\nPlugin conformance fixture.\n")
    bootstrap = Client(checker, fixture_mode=True)
    assert bootstrap.call("resolve", {"locator": "#7"})["number"] == 7
    assert bootstrap.call("write", {"path": "p0-conformance.md", "frontmatter": {"type": "note", "title": "Plugin conformance"},
                                     "body": "P-0 conformance\n"})["record"]["slug"] == "p0-conformance"
    assert bootstrap.call("write", {"path": "p0-conformance-minor.md", "frontmatter": {"type": "note", "title": "Plugin conformance"},
                                     "body": "P-0 conformance\n", "p0_future_minor_field": {"ignored": True}})["record"]["slug"] == "p0-conformance-minor"
    repeat = Client(checker, fixture_mode=True)
    assert repeat.call("resolve", {"locator": "#7"})["number"] == 7
    client, other = Client(a / ".bboxy"), Client(b / ".kb")
    assert client.initialization["capabilities"]["resources"] == {}
    assert "extensions" not in client.initialization["capabilities"]
    uri = "gitkb://skills/gitkb/SKILL.md"
    resources = client.rpc("resources/list", {})["resources"]
    assert resources == [{"uri": uri, "name": "gitkb", "description": shape["skills"][0]["description"],
                          "mimeType": "text/markdown"}]
    for connection in (client, other):
        contents = connection.rpc("resources/read", {"uri": uri})["contents"]
        assert len(contents) == 1 and contents[0]["uri"] == uri and contents[0]["mimeType"] == "text/markdown"
        assert contents[0]["text"].encode("utf-8") == skill_bytes
    for unknown in ("gitkb://skills/other/SKILL.md", "gitkb://skills/gitkb/../SKILL.md",
                    "gitkb://skills/gitkb/%2e%2e/SKILL.md", "gitkb://skills/gitkb/%53KILL.md",
                    uri + "?path=../../outside", "file:///etc/passwd"):
        assert client.rpc("resources/read", {"uri": unknown}, error="unknown_resource")["code"] == -32002
    assert len(client.rpc("tools/list", {})["tools"]) == 9
    client.call("write", {"path": "notes/unsigned.md", "frontmatter": {}, "body": ""}, "signed_writer_unavailable")
    one = client.create("notes/shared", "Needle fixture", id="01ARZ3NDEKTSV4RRFFQ69G5FAV",
                        number=77, status="active", nested={"preserved": [1, True, "x"]},
                        due_date="2026-11-01T00:00:00Z")
    assert one["id"] == "01ARZ3NDEKTSV4RRFFQ69G5FAV" and one["number"] == 77
    assert one["external_id"] == "01563e3a-b5d3-d676-4c61-efb99302bd5b"
    raw = cli(a, "show", "notes/shared", "--source", "workspace", "--raw-content", "--json")
    assert raw["document"]["id"] == one["external_id"]
    assert raw["document"]["frontmatter"]["nested"] == {"preserved": [1, True, "x"]}
    assert raw["document"]["frontmatter"]["bboxy_signed_record"] == one["signed_record"]
    assert "bboxy_signed_record" not in one["frontmatter"]
    for locator in ("notes/shared", "notes/shared.md", one["id"], one["external_id"], "#77"):
        assert client.call("resolve", {"locator": locator})["id"] == one["id"]
    reference = client.call("resolve", {"locator": "notes/shared"})
    fetched = client.call("fetch", {"ref": reference})
    assert fetched["frontmatter"]["due_date"] == "2026-11-01T00:00:00Z"
    assert fetched["body"] == "# Needle fixture\n"
    sibling = other.create("notes/shared", "Other reality")
    assert sibling["id"] != one["id"]
    other.call("fetch", {"ref": reference}, "stale_ref")
    assert other.call("resolve", {"locator": "notes/shared"})["id"] == sibling["id"]

    baseline = client.call("changes_since", {"cursor": "0"})["next"]
    unchanged = client.call("changes_since", {"cursor": baseline})
    assert unchanged["changed"] == [], unchanged
    baseline = unchanged["next"]
    task = client.create("tasks/harmony-7", "Lifecycle", "task", id="01ARZ3NDEKTSV4RRFFQ69G5FAW", status="open", swarm="fixture-a", assignee="seat:fixture")
    assert task["number"] == 7
    changes = client.call("changes_since", {"cursor": baseline})
    assert [r["slug"] for r in changes["changed"]] == [task["slug"]], changes
    assert client.call("changes_since", {"cursor": changes["next"]})["changed"] == []
    other.call("changes_since", {"cursor": changes["next"]}, "stale_cursor")
    for status in ("claimed", "running", "reported", "accepted"):
        ref = client.call("resolve", {"locator": task["slug"]})
        task = client.call("fetch", {"ref": ref})
        fm = dict(task["frontmatter"], status=status)
        proof = "fixture admitted intent " + status
        update = client.call("write", {"ref": ref, "etag": task["etag"], "frontmatter": fm,
                                       "body": task["body"], "signed_record": proof})
        task = update["record"]
        assert update["signed_record"] == task["signed_record"] == proof
        assert task["frontmatter"]["status"] == status and not update["lossy"]
        assert task["external_id"] == "01563e3a-b5d3-d676-4c61-efb99302bd5c"
        actual = cli(a, "show", task["slug"], "--source", "workspace", "--raw-content", "--json")
        assert actual["document"]["frontmatter"]["bboxy_status"] == status
        assert actual["document"]["frontmatter"]["status"] == ("completed" if status == "accepted" else "active")
        assert actual["document"]["frontmatter"]["bboxy_signed_record"] == proof
    assert len(client.call("list", {"filter": {"type": "task", "status": "accepted"}})["items"]) == 1
    assert len(client.call("list", {"filter": {"type": "task", "status": "accepted", "swarm": "fixture-a", "assignee": "seat:fixture"}})["items"]) == 1
    assert client.call("list", {"filter": {"type": "task", "swarm": "fixture-b"}})["items"] == []
    assert client.call("list", {"filter": {"type": "task", "assignee": "seat:other"}})["items"] == []
    assert client.collect("search", {"query": "Lifecycle type:task", "filter": {"swarm": "fixture-b"}}) == []
    assert client.collect("search", {"query": "Lifecycle type:task", "filter": {"assignee": "seat:other"}}) == []
    assert [item["ref"]["slug"] for item in client.collect("search", {
        "query": "Lifecycle type:task", "filter": {"swarm": "fixture-a", "assignee": "seat:fixture"}})] == [task["slug"]]
    absent_graph = client.call("graph", {"scope": {"kind": "swarm", "graph": "task", "swarm": "fixture-b"}})
    assert absent_graph["nodes"] == [] and absent_graph["edges"] == [], absent_graph
    scoped_graph = client.call("graph", {"scope": {"kind": "swarm", "graph": "task", "swarm": "fixture-a"}})
    assert [node["id"] for node in scoped_graph["nodes"]] == [task["id"]], scoped_graph
    # Explicit native commit writes only A; pending B and the other root survive.
    commit = client.call("commit", {"refs": [client.call("resolve", {"locator": task["slug"]})], "message": "Commit only selected fixture task"})
    assert commit["commit"] and [r["slug"] for r in commit["committed"]] == [task["slug"]]
    committed_native = cli(a, "show", task["slug"], "--source", "committed", "--raw-content", "--json")
    assert committed_native["exists"] and committed_native["document"]["frontmatter"]["bboxy_status"] == "accepted"
    pending_native = cli(a, "show", "notes/shared", "--source", "committed", "--raw-content", "--json")
    assert not pending_native["exists"]
    assert other.call("resolve", {"locator": "notes/shared"})["id"] == sibling["id"]
    other.call("commit", {"refs": commit["committed"], "message": "Cannot cross roots"}, "not_found")
    client.create("notes/other-7", "Duplicate family number")
    assert len(client.call("resolve", {"locator": "#7"}, "ambiguous_locator")["candidates"]) == 2
    first = client.call("list", {"filter": {"type": "note"}, "limit": 1})
    assert first["next"]
    second = client.call("list", {"filter": {"type": "note"}, "limit": 1, "cursor": first["next"]})
    assert first["items"][0]["id"] != second["items"][0]["id"]
    other.call("list", {"limit": 1, "cursor": first["next"]}, "stale_cursor")
    client.call("list", {"filter": {"type": "task"}, "cursor": first["next"]}, "stale_cursor")
    assert client.collect("search", {"query": "Needle type:note", "limit": 1})[0]["ref"]["slug"] == "notes/shared"
    client.create("notes/one-term", "Needle only")
    strict = client.collect("search", {"query": "Needle fixture type:note", "limit": 1})
    assert [hit["ref"]["slug"] for hit in strict] == ["notes/shared"]
    client.call("commit", {"refs": [client.call("resolve", {"locator": slug}) for slug in ("notes/shared", "notes/one-term")],
                           "message": "Private native strict-search fixture"})
    strict = client.collect("search", {"query": "Needle fixture type:note", "limit": 1})
    assert [hit["ref"]["slug"] for hit in strict] == ["notes/shared"]

    old = client.call("resolve", {"locator": "notes/shared"})
    record = client.call("fetch", {"ref": old})
    source = cli(a, "show", "notes/shared", "--source", "workspace", "--raw-content", "--json")
    cli(a, "workspace", "write", "notes/shared", "--stdin", "--if-generation", source["generation"],
        "--origin-id", "external-fixture-editor", "--json", text=source["rawContent"] + "\nexternal edit\n")
    client.call("fetch", {"ref": old}, "stale_ref")
    client.call("write", {"ref": old, "etag": record["etag"], "frontmatter": record["frontmatter"],
                           "body": "must not overwrite", "signed_record": "fixture stale"}, "stale_ref")
    client.call("list", {"filter": {"type": "note"}, "limit": 1, "cursor": first["next"]}, "stale_cursor")
    assert "external edit" in cli(a, "show", "notes/shared", "--source", "workspace", "--raw-content", "--json")["rawContent"]

    current = client.call("fetch", {"ref": client.call("resolve", {"locator": "notes/shared"})})
    linked = client.call("write", {"ref": {k: current[k] for k in ("id", "external_id", "number", "slug", "path", "digest")},
                                    "etag": current["etag"], "frontmatter": current["frontmatter"],
                                    "body": current["body"] + "\n[[tasks/harmony-7]]\n", "signed_record": "fixture link"})["record"]
    linked_ref = client.call("resolve", {"locator": linked["slug"]})
    assert any(e["to"] == task["id"] and e["rel"] == "references" for e in client.call("links", {"ref": linked_ref})["outgoing"])
    assert any(e["from"] == linked["id"] for e in client.call("links", {"ref": client.call("resolve", {"locator": task["slug"]})})["incoming"])
    graph = client.call("graph", {"scope": {"kind": "all", "graph": "markdown"}, "limit": 1})
    assert len(graph["nodes"]) == 1 and graph["next"] and not graph["edges_truncated"]
    nodes = []
    request = {"scope": {"kind": "all", "graph": "markdown"}, "limit": 32}
    for _ in range(100):
        page = client.call("graph", request)
        assert len(page["nodes"]) <= 1, "graph must reserve budget for native edges"
        nodes.extend(node["id"] for node in page["nodes"])
        if not page["next"]:
            break
        request["cursor"] = page["next"]
    else:
        raise AssertionError("graph continuation did not finish")
    expected = {ref["id"] for ref in client.collect("list", {"limit": 32})}
    assert len(nodes) == len(set(nodes)) and set(nodes) == expected
    client.call("graph", {"scope": {"kind": "all", "graph": "execution"}}, "unsupported_graph")
    client.call("resolve", {"locator": "../outside"}, "path_outside_root")
    client.call("list", {"limit": 0}, "invalid_limit")
    client.call("resolve", {"locator": "notes/shared", "root": str(b)}, "host_field_in_arguments")
    client.call("write", {"path": "../escape.md", "frontmatter": {"title": "x", "type": "note"},
                           "body": "x", "signed_record": "fixture"}, "path_outside_root")
    client.call("write", {"path": "notes/shared.md", "frontmatter": {"title": "x", "type": "note"},
                           "body": "x", "signed_record": "fixture"}, "path_exists")
    (a / "images").mkdir()
    (a / "images/pixel.png").write_bytes(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZQmcAAAAASUVORK5CYII="))
    image = client.create("notes/image", "Image", id="01ARZ3NDEKTSV4RRFFQ69G5FAX")
    image = client.call("write", {"ref": client.call("resolve", {"locator": "notes/image"}),
                                  "etag": image["etag"], "frontmatter": image["frontmatter"],
                                  "body": "![[images/pixel.png]]\n", "signed_record": "fixture image"})["record"]
    resource = client.call("fetch", {"ref": client.call("resolve", {"locator": image["slug"]})})["resources"][0]
    assert resource["uri"].startswith(client.data.as_uri() + "/resources/") and resource["size"] > 0
    (a / "images/pixel.png").unlink()
    (a / "images/pixel.png").symlink_to(b / "outside.png")
    client.call("fetch", {"ref": client.call("resolve", {"locator": image["slug"]})}, "resource_outside_data_dir")

    # Canonical committed source materializes through native generation CAS.
    cli(a, "commit", "--json", "-m", "Private conformance fixture", "notes/shared")
    cli(a, "clear", "notes/shared", "--force", json_output=False)
    committed = client.call("fetch", {"ref": client.call("resolve", {"locator": "notes/shared"})})
    assert committed["gitkb"]["source"]["kind"] == "committed"
    client.call("write", {"ref": client.call("resolve", {"locator": "notes/shared"}), "etag": committed["etag"],
                           "frontmatter": committed["frontmatter"], "body": committed["body"], "signed_record": "fixture committed update"})
    # Instrument a real backend invocation to prove child environment/cwd.
    trace = RUN / "backend-trace.jsonl"
    wrapper = RUN / "traced-git-kb"
    wrapper.write_text("#!" + sys.executable + " -I\nimport os,sys,json\n"
                       "with open(" + repr(str(trace)) + ",'a') as f: f.write(json.dumps({'cwd':os.getcwd(),'env':sorted(os.environ),'root':os.environ.get('GITKB_ROOT'),'args':sys.argv[1:]})+'\\n')\n"
                       "os.execv(" + repr(BACKEND) + ", [" + repr(BACKEND) + ", *sys.argv[1:]])\n")
    wrapper.chmod(0o700)
    traced_plugin = RUN / "bboxy450-term-data_source-gitkb"
    run([sys.executable, "-I", str(Path(__file__).resolve().parent / "build-gitkb-plugin.py"),
         "--gitkb", str(wrapper), "--schema", args.schema, "--output", str(traced_plugin)], RUN)
    traced = Client(a, binary=traced_plugin, env=dict(ENV, GITKB_ROOT=str(b), GITKB_WORKSPACE="wrong",
                                                    BBOXY_CAPABILITY="fixture-never-forward", API_TOKEN="fixture-never-forward"))
    traced.call("resolve", {"locator": "notes/shared"})
    rows = [json.loads(line) for line in trace.read_text().splitlines() if "--version" not in line]
    assert rows and all(row["cwd"] == str(traced.data) and row["root"] == str(a) for row in rows)
    # macOS adds __CF_USER_TEXT_ENCODING when the Python backend wrapper starts.
    assert all(set(row["env"]) <= {"PATH", "HOME", "LANG", "TMPDIR", "TERM", "LC_CTYPE", "GITKB_ROOT", "GITKB_WORKSPACE", "__CF_USER_TEXT_ENCODING"} for row in rows)
    wrapper.write_text(wrapper.read_text() + "\n# dependency changed\n")
    before_refusal = trace.read_bytes()
    traced.call("resolve", {"locator": "notes/shared"}, "backend_changed")
    assert trace.read_bytes() == before_refusal, "changed backend executed through the existing client"
    try:
        Client(a, binary=traced_plugin)
        raise RuntimeError("changed backend unexpectedly admitted")
    except AssertionError as error:
        assert "backend_changed" in str(error)

    # Reusable installer writes only explicit local artifacts/notices.
    config = b / ".bboxy/settings.toml"
    config.write_text("peer_config = 'preserve'\n")
    run([sys.executable, "-I", str(Path(__file__).resolve().parent / "install-gitkb-plugin.py"), str(PLUGIN), "--workspace", str(b)], RUN)
    assert config.read_text() == "peer_config = 'preserve'\n"
    assert sorted(p.name for p in (b / ".bboxy/plugins").iterdir()) == [PLUGIN.name]
    assert (b / ".bboxy/plugin-notices/gitkb/NOTICE").is_file()
    for c in CLIENTS:
        c.close()
    return {"result": "passed", "checks": ["shape", "canonical embedded skill and fixed MCP resource", "unknown and escaped skill URIs refused", "all eight methods", "real source-aware create/update",
            "UUID/ULID and numbers", "host intent storage and lifecycle projection (signature verification belongs to host)",
            "two isolated roots with same slug", "query/root/snapshot cursors", "native incoming/outgoing graph",
            "stale external edit rejection", "confined resource copy", "committed/workspace source transition",
            "real selected-only commit, unrelated pending source preserved", "disposable native bootstrap",
            "scrubbed native process environment and private cwd", "backend hash pin on existing and new clients", "local installation preserves config"],
            "plugin_sha256": hashlib.sha256(PLUGIN.read_bytes()).hexdigest()}


try:
    report = main()
    evidence = Path(args.evidence).resolve() if args.evidence else RUN / "commands.json"
    evidence.parent.mkdir(parents=True, exist_ok=True)
    evidence.write_text(json.dumps(dict(report, commands=COMMANDS), indent=2))
    report["command_exits"] = str(evidence) if args.evidence or args.retain else "all exit 0 (temporary fixture removed)"
    print(json.dumps(report, indent=2))
except BaseException:
    print("Private fixture retained: " + str(RUN), file=sys.stderr)
    raise
finally:
    for client in CLIENTS:
        if client.process.poll() is None:
            client.process.kill()
            client.process.wait()
    if not args.retain and sys.exc_info()[0] is None:
        shutil.rmtree(RUN)
