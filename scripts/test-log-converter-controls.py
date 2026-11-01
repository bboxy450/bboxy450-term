# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md


"""P3 controls on synthetic data; source is restored before every green run."""
import hashlib,json,os,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
CRATE=ROOT/'crates/bboxy450-log-converter'
CASES=[
 ('sanitize','src/sanitize/mod.rs','self.sanitize_event(event);','let _ = event;','sanitize','planted_secrets_and_personal_identifiers_are_absent_from_kind_and_raw','SanitizePlantedSecretsRemoved'),
 ('unknown','src/parse/claude.rs','fn native(typ: &str) -> EventKind {','fn native(typ: &str) -> EventKind { if typ == "future" { panic!("deliberate unknown control"); }','roundtrip','claude_unknown_malformed_and_truncated_are_safe','ClaudeUnknownNeverPanics'),
 ('identity','src/parse/claude.rs','key, tag)','key, "")','roundtrip','claude_metadata_vector_and_raw_placement','ClaudeDistinctDerivedIds'),
 ('raw','src/parse/claude.rs','raw.take(),','raw.clone(),','roundtrip','claude_metadata_vector_and_raw_placement','ClaudeOneRawPerRecord'),
]
def main():
 logs=ROOT/'target/log-converter-controls';logs.mkdir(parents=True,exist_ok=True);receipts=[]
 env={k:v for k,v in os.environ.items() if not k.startswith('BBOXY_CAPABILITY')}
 for name,relative,old,new,binary,test,assertion in CASES:
  source=CRATE/relative;original=source.read_bytes();text=original.decode();assert text.count(old)==1,(name,text.count(old))
  command=['cargo','test','--offline','--locked','-p','bboxy450-log-converter','--target-dir',env.get('BBOXY_CARGO_TARGET_DIR',str(ROOT/'target')),'--test',binary,test,'--','--exact','--nocapture']
  try:
   source.write_text(text.replace(old,new))
   with (logs/(name+'-red.log')).open('wb') as output:red=subprocess.run(command,cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT)
  finally:source.write_bytes(original)
  output=(logs/(name+'-red.log')).read_text()
  assert red.returncode==101 and 'running 1 test' in output and 'test result: FAILED' in output and assertion in output,(name,'not a named runtime failure')
  with (logs/(name+'-green.log')).open('wb') as output:green=subprocess.run(command,cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT)
  assert green.returncode==0,(name,'restored test failed')
  receipt=dict(name=name,assertion=assertion,command=command,red_exit=red.returncode,green_exit=green.returncode,restored_source_sha256=hashlib.sha256(original).hexdigest(),synthetic_only=True)
  receipts.append(receipt);print(json.dumps(receipt),flush=True)
 (logs/'receipt.json').write_text(json.dumps(receipts,indent=2)+'\n')
if __name__=='__main__':main()
