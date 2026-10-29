#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Private migration fixture and independent byte/link/disposition verifier; stdlib only."""
import hashlib
from collections import Counter
import json
import os
from pathlib import Path
import re
import shutil
import sys


def sha(data):
    return hashlib.sha256(data).hexdigest()


def prepare(base):
    dest = base / "input"
    if dest.exists():
        shutil.rmtree(dest)  # Only this target's previously generated private fixture.
    dest.mkdir(parents=True)
    for name in ("published", "proof.json", "verification.json"):
        product = base / name
        if product.is_dir(): shutil.rmtree(product)
        elif product.exists(): product.unlink()
    supplied = os.environ.get("TASK_MIGRATION_INPUT")
    if supplied:
        source = Path(supplied).resolve()
        for name in ("lossless-import-manifest.json", "checked-audit.json"):
            shutil.copyfile(source / name, dest / name)
        shutil.copytree(source / "dependency-source", dest / "dependency-source")
        return
    # Portable 250-record proof when the historical audit packet is unavailable.
    seeds = [59,60,61,62,88,85,70,76,63,99,101,102,116,103,100,109,114,119,123,121,
             108,128,127,125,129,130,137,204,203,277,332,329,330,336,337,338,339,360,342,343,64]
    rows, links, decisions = [], [], []
    for i in range(250):
        number = seeds[i] if i < 41 else (171 if i == 41 else 1000+i)
        ident = f"{i+1:026d}"
        path = f"{'tasks' if i < 41 else 'notes'}/fixture-{i}.md"
        kind = "task" if i < 40 else "note"
        raw = (f"---\nid: '{ident}'\nnumber: {number}\ntype: {kind}\nstatus: running\n"
               f"title: Private source {i}\nassignee: old-historical-owner\nverify: true\n---\n"
               f"# Fixture {i}\n\n[[#171|collision]] [[#999|missing]]\n").encode()
        output = dest / "dependency-source" / path
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(raw)
        rows.append(dict(source_id=ident,source_number=number,source_type=kind,source_path=".bboxy/"+path,
                         source_sha256=sha(raw),destination_path=path,dedupe_key=f"fixture:record/{ident}",seed_task_folder_record=i<41))
        for locator, targets, result in [("#171|collision",[".bboxy/notes/fixture-41.md"],"resolved"),("#999|missing",[],"unresolved")]:
            links.append({"from":".bboxy/"+path,"raw":locator,"source_locator":locator.split('|')[0],"targets":targets,"result":result})
        if i < 41:
            disposition = {108:"retain_deferred",128:"retain_narrowed",125:"retain_narrowed",129:"retain_narrowed",
                           329:"retain_active",330:"retain_active",342:"retain_followup",336:"already_done",64:"archive_note"}.get(number,"close_completed")
            decisions.append(dict(index=i,disposition=disposition,reason=f"Audited private source {number}",commits=["fixture"],evidence=[{"path":path,"line":1,"mechanism":"private fixture evidence"}]))
    (dest / "lossless-import-manifest.json").write_text(json.dumps(dict(source_revision="a"*40,seed_records=41,dependency_records=209,all_records=250,rows=rows,links=links)))
    (dest / "checked-audit.json").write_text(json.dumps(dict(decisions=decisions)))


def field(text, key):
    text = frontmatter(text)
    match = re.search(r"^"+re.escape(key)+r": (.*)$", text, re.M)
    assert match, key
    return match[1].strip("'\"")


def frontmatter(text):
    return text.removeprefix("---\n").split("\n---\n",1)[0] if text.startswith("---\n") else text


def verify(base):
    source = base / "input"
    proof = json.loads((base / "proof.json").read_text())
    root = base / "published"
    manifest_bytes = (source / "lossless-import-manifest.json").read_bytes()
    audit_bytes = (source / "checked-audit.json").read_bytes()
    manifest, audit = json.loads(manifest_bytes), json.loads(audit_bytes)
    digest = hashlib.sha256(b"bboxy450/task-import/v1\0")
    for blob in (manifest_bytes,audit_bytes)+tuple((source/"dependency-source"/r["destination_path"]).read_bytes() for r in manifest["rows"]):
        digest.update(len(blob).to_bytes(8,"big")); digest.update(blob)
    batch = digest.hexdigest()
    directory = root / "imports" / batch
    receipt = json.loads((directory / "receipt.json").read_text())
    assert receipt["bundle_sha256"] == batch
    assert manifest["all_records"] == len(receipt["records"]) == 250
    assert manifest["seed_records"] == 41 and manifest["dependency_records"] == 209
    assert (directory / "manifest.json").read_bytes() == manifest_bytes
    assert (directory / "audit.json").read_bytes() == audit_bytes
    mapping = {r["source_path"]:r for r in receipt["records"]}
    assert len(mapping) == len({r["dedupe_key"] for r in receipt["records"]}) == 250
    assert len({r["id"] for r in receipt["records"]}) == len({r["number"] for r in receipt["records"]}) == 250
    source_ids = {r["source_id"] for r in manifest["rows"]}
    all_dest_ids = set()
    docs = {}
    seed = 0
    for row in manifest["rows"]:
        dest = mapping[row["source_path"]]
        assert dest["id"] not in source_ids and dest["number"] > 999
        original = (source / "dependency-source" / row["destination_path"]).read_bytes()
        retained = directory / "raw" / (dest["id"]+".source")
        assert retained.read_bytes() == original and sha(original) == row["source_sha256"]
        assert retained.stat().st_mode & 0o222 == 0
        docpath = root / dest["path"]
        raw = docpath.read_bytes()
        assert sha(raw) == receipt["published_sha256"][docpath.name]
        text = raw.decode()
        assert field(text,"id") == dest["id"] and int(field(text,"number")) == dest["number"]
        assert not re.search(r"^(assignee|report|verify_result):",frontmatter(text),re.M)
        assert row["source_sha256"] in text and row["dedupe_key"] in text
        assert row["source_path"] in text and manifest["source_revision"] in text
        if row["seed_task_folder_record"]:
            decision = next(d for d in audit["decisions"] if d["index"] == seed)
            seed += 1
            live = decision["disposition"].startswith("retain_") and decision["disposition"] != "retain_followup"
            assert field(text,"status") == ("open" if live else "done")
            assert "fresh_accepted: false" in text and decision["disposition"] in text
            if live: assert "# Remaining scope" in text
        elif row["source_type"] not in ("task","bug","epic") and re.search(r"^status:",frontmatter(original.decode()),re.M):
            assert field(text,"status") == field(original.decode(),"status"), "documentary source status changed"
        all_dest_ids.add(dest["id"])
        docs[row["source_path"]] = text
    # Every declared resolved link uses the complete identity mapping. Missing
    # references are plain labelled text and can never resolve to live task #999.
    for link in manifest["links"]:
        text = docs[link["from"]]
        if link["result"] == "resolved":
            assert "[["+mapping[link["targets"][0]]["id"] in text, link
        else:
            assert "[["+link["raw"]+"]]" not in text, link
    paths = list(directory.glob("*.md"))
    assert len(paths) == 251
    followups = [p for p in paths if field(p.read_text(),"id") not in all_dest_ids]
    assert len(followups) == 1 and "github_issue: 40" in followups[0].read_text()
    all_ids=all_dest_ids|{field(p.read_text(),"id") for p in followups}
    for text in docs.values():
        assert all(raw.split('|')[0].strip() in all_ids for raw in re.findall(r"\[\[([^\]]+)\]\]",text)), "unmapped source reference survived"
    assert sum(field(p.read_text(),"type") == "task" and field(p.read_text(),"status") == "open" for p in paths) == 7
    assert proof["task171_before"] == proof["task171_after"] == sha((root / proof["task171_path"]).read_bytes())
    assert proof["unrelated_before"] == proof["unrelated_after"]
    assert proof["historical_wakes"] == 0 and proof["replay_unchanged"] is True
    controls = proof["transport_controls"]
    assert len(controls) == 2 and len({row["op_id"] for row in controls}) == 2
    for row in controls:
        assert row["actual"]["isError"] is False
        assert row["actual"]["structuredContent"]["status"] == "imported"
        assert row["actual"]["structuredContent"]["source_records"] == 250
        assert row["read_only_original"] == dict(tool="task_import",status="imported",
                                                error_code=None,source_records=250)
        assert row["unknown_retried"] is False
        if row["phase"] == "completed_reply_lost":
            error = row["observed"]["error"]
            assert error["code"] == -32001
            assert error["data"]["op_id"] == row["op_id"] and error["data"]["status"] == "unknown"
            assert error["data"]["transport"]["phase"] == "response_eof"
        else:
            assert row["phase"] == "completed_reply_delivered"
            assert row["observed"]["result"] == row["actual"]
    summary = dict(command="make test-task-migration",records=250,dependencies=209,seeds=41,published=251,
                   live_tasks=7,source_revision=manifest["source_revision"],bundle_sha256=batch,
                   dispositions="independently checked",raw_hashes="250/250",links=len(manifest["links"]),
                   disposition_counts=dict(Counter(d["disposition"] for d in audit["decisions"])),
                   task171_sha256=proof["task171_after"],historical_wakes=0,duplicate_prevention=True)
    summary["known_no_effect_attempts"]=proof.get("known_no_effect_attempts",0)
    summary["transport_controls"] = [row["phase"] for row in controls]
    (base / "verification.json").write_text(json.dumps(summary,indent=2)+"\n")
    print(json.dumps(summary,sort_keys=True))


if __name__ == "__main__":
    mode, target = sys.argv[1:]
    base = Path(target) / "task-migration"
    base.mkdir(parents=True,exist_ok=True)
    (prepare if mode == "prepare" else verify)(base)
