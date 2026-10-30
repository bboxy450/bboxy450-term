#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Run the standard private night; only complete D1/D2/D3 script verdicts pass.

D4 remains an independent unimplemented promise. Their outer red/amber status
does not decide this cut, and is retained verbatim with all native runner rows.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin",type=Path,required=True)
    parser.add_argument("--source",type=Path,required=True)
    parser.add_argument("--out",type=Path,required=True)
    args = parser.parse_args()
    binary,source,out = args.bin.resolve(),args.source.resolve(),args.out.resolve()
    out.mkdir(parents=True,exist_ok=False)
    # The runner itself clears each app/script environment and authenticates its
    # generated file-custody account. No installed client is spawned by D1/D2.
    command = [str(binary),"e2e","night","--group","documents","--scripts",str(source/"scripts/e2e")]
    env = {key:os.environ[key] for key in ["PATH","TERM","LANG","TMPDIR"] if key in os.environ}
    home = out/"runner-home"
    home.mkdir(mode=0o700)
    env.update(HOME=str(home),CLAUDE_CONFIG_DIR=str(home/".claude"),CODEX_HOME=str(home/".codex"))
    log_path = out/"night.jsonl"
    failure = None
    with log_path.open("w") as output:
        child = subprocess.Popen(command,cwd=source,env=env,stdin=subprocess.DEVNULL,stdout=output,stderr=subprocess.STDOUT)
        try:
            code = child.wait(timeout=480)
        except subprocess.TimeoutExpired:
            child.terminate()
            child.wait(timeout=15)
            code,failure = child.returncode,"standard document night exceeded its existing finite script bounds"
    rows = []
    for line in log_path.read_text().splitlines():
        try:
            rows.append(json.loads(line))
        except ValueError:
            pass
    verdicts = {row["function"]:row for row in rows if row.get("kind")=="e2e"}
    apps = [row for row in rows if row.get("kind")=="app"]
    exits = []
    for app in apps:
        if app.get("mode") != "headless" or app.get("windows") != 0 or app.get("custody_backend") != "file" or not str(app.get("issued_by","")).startswith("test-account:"):
            failure = failure or "runner did not use a verified private headless/file account"
        # AppStopping acknowledges an orderly stop before final process teardown.
        # Observe exit within a finite cleanup bound; do not call stop again.
        start,deadline,alive = time.monotonic(),time.monotonic()+3,True
        while True:
            try:
                os.kill(app["pid"],0)
            except ProcessLookupError:
                alive = False
                break
            if time.monotonic() >= deadline:
                failure = failure or "standard night retained an active owned app after orderly teardown"
                break
            time.sleep(0.02)
        exits.append({"pid":app["pid"],"gone":not alive,"observed_seconds":time.monotonic()-start})
    for promise in ["D1","D2","D3"]:
        row = verdicts.get(promise,{})
        if row.get("verdict") != "pass" or row.get("exit") != 0 or row.get("coverage") != "full" or row.get("missing"):
            failure = failure or f"{promise} lacks a complete successful script verdict: {row}"
    summaries = [row for row in rows if row.get("kind")=="summary"]
    if not summaries or summaries[-1].get("unwritten"):
        failure = failure or "standard night omitted its summary or failed to write its private records"
    for row in rows:
        if row.get("kind")=="e2e" and row.get("function") in ["D1","D2","D3"]:
            # The complete driver evidence survives the native runner's account
            # retirement under the build target; copy it to this unique run.
            for line in str(row.get("evidence","")).splitlines():
                try:
                    record = json.loads(line)
                except ValueError:
                    continue
                if record.get("promise") == row["function"] and record.get("evidence"):
                    evidence = Path(record["evidence"]).resolve()
                    assert evidence.is_relative_to(binary.parent.parent/"document-evidence")
                    shutil.copytree(evidence,out/row["function"])
    result = {"command":command,"standard_night_exit":code,"source":subprocess.check_output(["git","rev-parse","HEAD"],cwd=source,text=True).strip(),
        "binary":str(binary),"binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
        "helper_sha256":{name:hashlib.sha256((source/"scripts/e2e"/name).read_bytes()).hexdigest() for name in ["D1.sh","D2.sh","D3.sh","documents.py","documents_night.py","ui.py","lib.sh"]},
        "script_verdicts":{name:verdicts.get(name) for name in ["D1","D2","D3"]},"outer_summary":summaries[-1] if summaries else None,
        "apps":apps,"app_exits":exits,"pass":failure is None,"failure":failure}
    (out/"verdict.json").write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({"pass":failure is None,"failure":failure,"evidence":str(out),"D1":verdicts.get("D1"),"D2":verdicts.get("D2"),"D3":verdicts.get("D3")},indent=2))
    return 0 if failure is None else 1


if __name__ == "__main__":
    raise SystemExit(main())
