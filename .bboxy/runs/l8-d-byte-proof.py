import hashlib, json, pathlib, sys, time
root=pathlib.Path(__file__).parent
state=json.loads((root/'d-ci-state.json').read_text())
assert state.get('exit')==0, 'clean full CI has not passed'
baseline=pathlib.Path('/Users/feral/mydev/bboxy450-term-exec6/target/f4-parity-3f97803')
after=pathlib.Path(state['parity_out'])
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as f:
  for chunk in iter(lambda:f.read(1024*1024),b''):h.update(chunk)
 return h.hexdigest()
def pngs(path):return {str(p.relative_to(path)):p for p in path.rglob('*.png')}
before=pngs(baseline/'native'); current=pngs(after/'native')
assert before.keys()==current.keys(), (len(before),len(current),set(before)^set(current))
def case_keys(path):
 return {(c['component'],c['slug'],c['density'],c['theme'],c['state']) for c in json.loads(path.read_text())}
assert case_keys(baseline/'cases.json')==case_keys(after/'cases.json')
rows=[]
for name,p in sorted(before.items()):
 a=digest(p);b=digest(current[name]);rows.append({'key':name,'before_sha256':a,'after_sha256':b,'equal':a==b})
geometry=[]
for name in ['layouts.json','surface-layouts.json']:
 a=baseline/'native'/name;b=after/'native'/name
 if a.exists() and b.exists():
  geometry.append({'path':name,'before_sha256':digest(a),'after_sha256':digest(b),'equal':json.loads(a.read_text())==json.loads(b.read_text())})
record={'source':state['source'],'before_source':'3f978038b522675c696728b72bb0dd63973c3e62','baseline':str(baseline.resolve()),'after':str(after.resolve()),'created':time.time(),'count':len(rows),'identical':sum(r['equal'] for r in rows),'changed':[r['key'] for r in rows if not r['equal']],'geometry':geometry,'per_file':rows}
(root/'d-byte-proof.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps({k:v for k,v in record.items() if k!='per_file'}))
assert record['identical']==record['count']
assert all(r['equal'] for r in geometry)
