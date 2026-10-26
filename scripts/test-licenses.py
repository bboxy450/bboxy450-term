"""Exercise the licence gate against a disposable metadata workspace."""
from pathlib import Path
from tempfile import TemporaryDirectory
import json
import os
import shutil
import subprocess

temporary=TemporaryDirectory(prefix='bboxy-licences-');root=Path(temporary.name);fixture=root/'script-fixtures';bindir=root/'fixture-bin';fixture.mkdir();bindir.mkdir();(fixture/'.bboxy/specs').mkdir(parents=True);workspace=Path(__file__).resolve().parent.parent;script=workspace/'scripts/licenses.sh'
(fixture/'.bboxy/specs/licensing.md').write_bytes((workspace/'.bboxy/specs/licensing.md').read_bytes())
for name in ['LICENSE-MIT','LICENSE-APACHE','NOTICE']:(fixture/name).write_bytes((workspace/name).read_bytes())
metadata={'workspace_root':str(fixture),'workspace_members':[],'packages':[]}

def member(name,directory,license):
 path=fixture/directory;path.mkdir(parents=True);(path/'Cargo.toml').write_text('# disposable metadata fixture\n')
 for filename in (['LICENSE-APACHE','NOTICE']if name=='bboxy450-term-core'else['LICENSE-MIT','LICENSE-APACHE']):(path/filename).write_bytes((workspace/filename).read_bytes())
 package={'id':name,'name':name,'manifest_path':str(path/'Cargo.toml'),'license':license,'license_file':None};metadata['packages'].append(package);metadata['workspace_members'].append(name);return package,path

core,corepath=member('bboxy450-term-core','crates/bbt-core','Apache-2.0');ours,ourpath=member('bboxy450-ui','members/bbt-ui','MIT OR Apache-2.0')
for name in ['alacritty/LICENSE-APACHE','vte/LICENSE-MIT','vte/LICENSE-APACHE']:
 file=corepath/'src/vendor'/name;file.parent.mkdir(parents=True,exist_ok=True);file.write_bytes((workspace/'crates/bbt-core/src/vendor'/name).read_bytes())
meta=root/'fixture-metadata.json';cargo=bindir/'cargo';cargo.write_text('''#!/usr/bin/env python3
import os,sys
from pathlib import Path
assert sys.argv[1:]==["metadata","--no-deps","--format-version","1","--locked"],sys.argv
sys.stdout.write(Path(os.environ["LIC1_FIXTURE_METADATA"]).read_text())
''');cargo.chmod(0o700)
env=os.environ.copy();env['PATH']=str(bindir)+os.pathsep+env['PATH'];env['LIC1_FIXTURE_METADATA']=str(meta)
with(root/'script-fixtures.log').open('w')as log:
 def check(name,expected,reason=None):
  meta.write_text(json.dumps(metadata));p=subprocess.run(['sh',str(script)],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);log.write(f'CASE {name}\n{p.stdout}EXIT {p.returncode}; expected {expected}\n');log.flush();print(name,p.returncode,flush=True);assert p.returncode==expected,(name,p.stdout)
  if reason:assert reason in p.stdout,(name,p.stdout)
 check('Yarn absent',0)
 yarn,yarnpath=member('bboxy450-yarn','members/bbt-yarn','MIT OR Apache-2.0');macros,macrospath=member('bboxy450-yarn-macros','members/bbt-yarn-macros','MIT OR Apache-2.0');check('Yarn and macros present',0)
 file=yarnpath/'LICENSE-MIT';saved=file.read_bytes();file.unlink()
 try:check('present Yarn missing MIT',1,'bbt-yarn/LICENSE-MIT')
 finally:file.write_bytes(saved)
 file=corepath/'NOTICE';saved=file.read_bytes();file.unlink()
 try:check('core missing NOTICE',1,'crates/bbt-core/NOTICE')
 finally:file.write_bytes(saved)
 core['license']='MIT OR Apache-2.0'
 try:check('core wrong SPDX',1,"expected SPDX 'Apache-2.0'")
 finally:core['license']='Apache-2.0'
 ours['license']='MIT'
 try:check('dual crate wrong SPDX',1,"expected SPDX 'MIT OR Apache-2.0'")
 finally:ours['license']='MIT OR Apache-2.0'
 ours['license_file']='licence.txt'
 try:check('license-file declaration refused',1,'not license-file')
 finally:ours['license_file']=None
 file=fixture/'LICENSE-MIT';saved=file.read_bytes();file.write_bytes(b'')
 try:check('empty root licence refused',1,'LICENSE-MIT')
 finally:file.write_bytes(saved)
 backup=root/'fixture-saved-MIT';file.rename(backup);file.symlink_to(backup)
 try:check('symlink root licence refused',1,'regular copy')
 finally:file.unlink();backup.rename(file)
 file=corepath/'src/vendor/vte/LICENSE-MIT';saved=file.read_bytes();file.unlink()
 try:check('missing vendored licence refused',1,'src/vendor/vte/LICENSE-MIT')
 finally:file.write_bytes(saved)
 app,apppath=member('bboxy450-term','members/bboxy450-term','MIT OR Apache-2.0');check('app follows current dual map',0)
 mapfile=fixture/'.bboxy/specs/licensing.md';original=mapfile.read_text();lines=original.splitlines(True)
 for i,line in enumerate(lines):
  if line.startswith('| `bboxy450-term`'):
   cells=line.split('|');cells[3]=' Apache-2.0 ';lines[i]='|'.join(cells);break
 else:raise AssertionError('app map row not found')
 mapfile.write_text(''.join(lines))
 try:
  check('one-line map change changes expected app licence',1,"expected SPDX 'Apache-2.0'")
  app['license']='Apache-2.0';(apppath/'NOTICE').write_bytes((workspace/'NOTICE').read_bytes());check('app satisfying changed map accepted',0)
 finally:
  mapfile.write_text(original);app['license']='MIT OR Apache-2.0';(apppath/'NOTICE').unlink()
 unknown,unknownpath=member('bbt-unmapped','members/bbt-unmapped','MIT OR Apache-2.0')
 try:check('unmapped member refused',1,'missing from the licensing map')
 finally:
  metadata['packages'].remove(unknown);metadata['workspace_members'].remove('bbt-unmapped');shutil.rmtree(unknownpath)
 check('all restored',0)
 proto,protopath=member('bboxy450-proto','members/bbt-proto','MIT OR Apache-2.0')
 mux,muxpath=member('bboxy450-mux','members/bbt-mux','MIT OR Apache-2.0')
 check('escaped aliases before SPDX keep the license column',0)
 original=mapfile.read_text()
 linked=original.replace(r"internal until decision [[d8-open-source\|#8]]'s plateau",r"internal [[d8-open-source\|#8]] then [[d47-decision-nodes\|nodes]] with [[d48-every-record-has-a-number\|decides\|2]]")
 assert linked!=original
 mapfile.write_text(linked)
 check('multiple escaped links and pipes retain columns',0)
 proto['license']='MIT'
 check('linked row still enforces SPDX',1,"expected SPDX 'MIT OR Apache-2.0'")
 proto['license']='MIT OR Apache-2.0';mapfile.write_text(original)
 check('linked rows restored',0)
shutil.rmtree(fixture);shutil.rmtree(bindir);meta.unlink();print('Owned disposable fixture data removed0')

temporary.cleanup()
