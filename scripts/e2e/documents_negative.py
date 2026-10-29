#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Finite production mutations; real native assertions must go red, then green.

Only the three named document owners are temporarily edited. Their exact bytes
are restored in finally; no assertion, deadline, identity or runner is mutated.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


MARKDOWN = "crates/bbt-doc/src/markdown/layout.rs"
MERMAID = "crates/bbt-doc/src/mermaid.rs"
SCENE = "crates/bboxy450-term/src/scene.rs"


def replace_once(source, old, new):
    assert source.count(old) == 1, "mutation anchor changed"
    return source.replace(old,new,1)


def mutation(name, source):
    if name == "heading-level":
        anchor = re.search(r"size: Some\(match level \{.*?\} as f32\)",source,re.S)
        assert anchor, "heading mutation anchor changed"
        return replace_once(source,anchor[0],"size: Some({ let _ = level; STYLE.r#type.doc_heading as f32 })")
    if name == "task-glyph":
        return replace_once(source,'if *done { "☑ ".into() } else { "☐ ".into() }','if *done { "☐ ".into() } else { "☐ ".into() }')
    if name == "edge-endpoint":
        start,end = source.index("fn edge_points("),source.index("fn arrow(")
        block = replace_once(source[start:end],"c::PathCommand::Move(points[0])","c::PathCommand::Move([points[0][0] + 6., points[0][1]])")
        return source[:start]+block+source[end:]
    if name == "glyph-paint-scale":
        start,end = source.index("impl DocumentLayout {"),source.index("pub struct LocalPageLayout")
        body = '''{
            fn primitives(items: &mut [bbt_ui_model::draw::Prim]) {
                for item in items {
                    match item {
                        bbt_ui_model::draw::Prim::Group { primitives: children, .. } => primitives(children),
                        bbt_ui_model::draw::Prim::Text { text, style, .. } if text.as_ref() == "BodyGlyphProbeHgMQ" => {
                            style.scale = bbt_ui_model::draw::TypeScale::Custom(style.pixels() * 0.5);
                        }
                        _ => {}
                    }
                }
            }
            fn paints(ops: &mut [c::Paint]) {
                for op in ops {
                    match op {
                        c::Paint::Group { ops, .. } => paints(ops),
                        c::Paint::Draw(list) | c::Paint::Underlay(list) => primitives(&mut list.primitives),
                        _ => {}
                    }
                }
            }
            let mut ops = c::paint(&self.layout, self.context);
            paints(&mut ops);
            ops
        }'''
        block = replace_once(source[start:end],"c::paint(&self.layout, self.context)",body)
        return source[:start]+block+source[end:]
    raise AssertionError("unknown finite document mutation")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target",type=Path,required=True)
    parser.add_argument("--out",type=Path,required=True)
    args = parser.parse_args()
    root,target,out = Path.cwd().resolve(),args.target.resolve(),args.out.resolve()
    assert target.is_relative_to(root/"target") and out.is_relative_to(target)
    out.mkdir(parents=True,exist_ok=False)
    binary = target/"debug/bboxy450-term"
    original = {name:(root/name).read_bytes() for name in [MARKDOWN,MERMAID,SCENE]}
    env = os.environ.copy()
    env.update(CARGO_PROFILE_DEV_DEBUG="0",CARGO_PROFILE_TEST_DEBUG="0",
        CARGO_PROFILE_DEV_OPT_LEVEL="1",CARGO_PROFILE_TEST_OPT_LEVEL="1",
        CARGO_PROFILE_DEV_DEBUG_ASSERTIONS="true",CARGO_PROFILE_TEST_DEBUG_ASSERTIONS="true")
    def run(command,log):
        with log.open("w") as output:
            return subprocess.run(command,cwd=root,env=env,stdin=subprocess.DEVNULL,
                stdout=output,stderr=subprocess.STDOUT,timeout=900).returncode
    def build(directory):
        command = ["make","build",f"TARGET_DIR={target}"]
        code = run(command,directory/"build.log")
        assert code == 0, f"mutation build failed: {directory}/build.log ({code})"
        shutil.copy2(binary,directory/"bboxy450-term")
        return {"command":command,"exit":code,"sha256":digest(binary)}
    def night(directory):
        command = [sys.executable,"-I","-B",str(root/"scripts/e2e/documents_night.py"),
            "--bin",str(binary),"--source",str(root),"--out",str(directory/"night")]
        code = run(command,directory/"night.log")
        return code,json.loads((directory/"night/verdict.json").read_text())
    # Retain the actual pre-mutation bytes and every governing notice once.
    shutil.copy2(binary,out/"before-bboxy450-term")
    (out/"source.diff").write_bytes(subprocess.check_output(["git","diff","--binary"]))
    changed = subprocess.check_output(["git","diff","--name-only"],text=True).splitlines()
    added = subprocess.check_output(["git","ls-files","--others","--exclude-standard","--","crates","scripts"],text=True).splitlines()
    for name in changed+added+["design/style.json"]:
        destination = out/"source"/name
        destination.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(root/name,destination)
    for name in ["D1.sh","D2.sh","documents.py","documents_night.py","documents_negative.py","ui.py","lib.sh"]:
        destination = out/"helpers"/name
        destination.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(root/"scripts/e2e"/name,destination)
    tracked = subprocess.check_output(["git","ls-files"],text=True).splitlines()
    for name in tracked:
        if Path(name).name.upper().startswith(("LICENSE","NOTICE","COPYING")) or name == "ATTRIBUTION.md":
            destination = out/"licenses"/name
            destination.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(root/name,destination)
    checks,failure = [],None
    cases = [("heading-level",MARKDOWN,"D1","did not paint its baselined named style"),
        ("task-glyph",MARKDOWN,"D1","native modal omitted required fixture text"),
        ("edge-endpoint",MERMAID,"D2","starts away from"),
        ("glyph-paint-scale",SCENE,"D1","painted glyphs were scaled")]
    try:
        for name,path,promise,reason in cases:
            for restored,data in original.items(): (root/restored).write_bytes(data)
            changed = mutation(name,original[path].decode())
            (root/path).write_text(changed)
            directory = out/name
            directory.mkdir()
            (directory/"mutation.diff").write_bytes(subprocess.check_output(["git","diff","--",path]))
            built = build(directory)
            code,verdict = night(directory)
            detail = json.loads((directory/"night"/promise/"document-verdict.json").read_text())
            other = "D2" if promise == "D1" else "D1"
            assert code != 0 and verdict["script_verdicts"][promise]["exit"] != 0 and reason in (detail["failure"] or ""), f"{name} was not rejected for its production signal: {detail}"
            assert verdict["script_verdicts"][other]["verdict"] == "pass", f"{name} also broke independent {other}"
            checks.append({"mutation":name,"path":path,"source_sha256":digest(root/path),
                "build":built,"night_exit":code,"promise":promise,"failure":detail["failure"],"other":other,"other_verdict":"pass"})
    except Exception as error:
        failure = f"{type(error).__name__}: {error}"
    finally:
        for path,data in original.items(): (root/path).write_bytes(data)
        restored = out/"restored"
        restored.mkdir()
        try:
            built = build(restored)
            code,verdict = night(restored)
            assert code == 0 and verdict["pass"], "restored native document checks failed"
            checks.append({"restored":True,"build":built,"night_exit":code})
        except Exception as error:
            failure = failure or f"restored {type(error).__name__}: {error}"
    result = {"pass":failure is None and len(checks)==5,"failure":failure,"checks":checks,
        "source_files":{path:hashlib.sha256(data).hexdigest() for path,data in original.items()},
        "helper_sha256":digest(Path(__file__))}
    (out/"verdict.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps(result,indent=2))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
