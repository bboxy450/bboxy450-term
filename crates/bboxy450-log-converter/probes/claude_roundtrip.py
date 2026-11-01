# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https:#github.com/bboxy450/bboxy450-term
# License: https:#github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https:#github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md


"""P3 F2: real Claude resumes a parse/lower output from private staging."""
import hashlib
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parent))
from support import Case, Mock, available, claude_session

def main(binary):
    if not available("claude"):
        return
    with Case() as case, Mock() as mock:
        case.env["ANTHROPIC_BASE_URL"]=mock.url
        _,source=claude_session(case,external=True)
        before=source.read_bytes()
        target=case.root/"private-p3-lowered.jsonl"
        converted=case.run([str(binary),str(source),str(target)])
        assert converted["exit"]==0 and not converted["timed_out"],converted
        assert target.read_bytes()==before,"ClaudeSameProviderNativeIdentity"
        assert source.read_bytes()==before,"ClaudeNativeSourceUntouched"
        result=case.run(case.claude_resume(target))
        mock.witness(result,"p3-claude-lowered-external-path",source_unchanged=source.read_bytes()==before,
                     same_provider_byte_identity=True,source_sha256=hashlib.sha256(before).hexdigest())
if __name__=="__main__":
    main(Path(sys.argv[1]).resolve())
