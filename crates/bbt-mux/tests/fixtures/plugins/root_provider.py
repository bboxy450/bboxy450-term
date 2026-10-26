# Credential-free root-backed fixture. The Rust fixture prepends SCHEMA as JSON.
import base64, hashlib, json, mimetypes, os, pathlib, re, sys
ROOT = None
TRACE = None
GATE = None
GATE_ROOT = None
GATE_ARMED = None
def trace(kind, **fields):
    if TRACE:
        with open(TRACE, "a") as log: log.write(json.dumps(dict(kind=kind, pid=os.getpid(), **fields))+"\n")
HISTORY = {}
METHODS = {
    "resolve": ("ResolveRequest", "RecordRef"), "fetch": ("FetchRequest", "Record"),
    "search": ("SearchRequest", "SearchPage"), "list": ("ListRequest", "RefPage"),
    "links": ("FetchRequest", "Links"), "graph": ("GraphRequest", "GraphPage"),
    "write": ("WriteRequest", "WriteResult"), "changes_since": ("ChangesRequest", "Changes"),
}
class Refused(Exception):
    def __init__(self, code, message, candidates=None):
        self.value = {"code": code, "message": message}
        if candidates: self.value["candidates"] = candidates

def schema(method, result=False):
    return {"$schema": SCHEMA["$schema"], "$defs": SCHEMA["$defs"], "$ref": "#/$defs/" + METHODS[method][int(result)]}

def shape():
    return {"name":"root fixture", "version":"1.0", "shape_version":"1.0", "health_command":"health",
        "systems":[{"contract":"data_source", "contract_version":"1.0", "commands":[
        {"name":m,"args":schema(m),"result":schema(m,True),"timeout_ms":30000} for m in METHODS]}]}

def confined(relative, write=False):
    path = pathlib.Path(relative)
    if path.is_absolute() or '..' in path.parts or not path.parts or path.suffix != '.md':
        raise Refused('path_outside_root','a root-relative Markdown path is required')
    target = ROOT / path
    for part in [target, *target.parents]:
        if part == ROOT: break
        if part.is_symlink(): raise Refused('path_outside_root','symlinks are refused')
    if not target.resolve().is_relative_to(ROOT): raise Refused('path_outside_root','path leaves root')
    return target

def frontmatter(text):
    if not text.startswith('---\n') or '\n---\n' not in text[4:]:
        raise Refused('representation_unavailable','fixture requires JSON or flat YAML frontmatter')
    raw, body = text[4:].split('\n---\n',1)
    try: value = json.loads(raw)
    except json.JSONDecodeError:
        value = {}
        for line in raw.splitlines():
            if not line.strip() or line.startswith('#'): continue
            match = re.fullmatch(r'([a-zA-Z_][a-zA-Z0-9_-]*):\s*(.*)',line)
            if not match: raise Refused('representation_unavailable','nested YAML is outside this fixture')
            key, scalar = match.groups()
            if scalar.startswith('"'):
                try: scalar=json.loads(scalar)
                except json.JSONDecodeError: raise Refused('representation_unavailable','invalid JSON scalar')
            elif scalar.startswith("'") and scalar.endswith("'"):
                scalar=scalar[1:-1].replace("''", "'")
            elif scalar in ('null','~'): scalar=None
            elif scalar in ('true','false'): scalar=scalar=='true'
            elif key=='number' and scalar.isdecimal(): scalar=int(scalar)
            elif scalar.startswith(('|','>','[','{','&','*','!')):
                raise Refused('representation_unavailable','complex YAML is outside this fixture')
            value[key]=scalar
    if not isinstance(value,dict): raise Refused('representation_unavailable','frontmatter must be an object')
    return value,body

def record(path, resources=False):
    relative=path.relative_to(ROOT).as_posix()
    confined(relative)
    raw=path.read_bytes()
    if len(raw)>2*1024*1024 or b'\r' in raw: raise Refused('representation_unavailable','fixture record size/newlines are unsupported')
    try: metadata,body=frontmatter(raw.decode('utf8'))
    except UnicodeDecodeError: raise Refused('representation_unavailable','record is not UTF-8')
    digest=hashlib.sha256(raw).hexdigest()
    result={"id":str(metadata.get('id') or 'fixture:'+relative),"number":metadata.get('number'),
        "slug":path.stem,"path":relative,"digest":digest,"frontmatter":metadata,"body":body,"etag":digest,"resources":[]}
    for name in (re.findall(r'!\[\[([^\]|]+)(?:\|[^\]]*)?\]\]',body) if resources else []):
        resource=ROOT/name if name.startswith('images/') else ROOT/'images'/name
        if pathlib.Path(name).is_absolute() or '..' in pathlib.Path(name).parts or resource.is_symlink() or not resource.resolve().is_relative_to(ROOT):
            raise Refused('resource_outside_data_dir','image leaves the selected root')
        fd=os.open(resource,os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
        with os.fdopen(fd,'rb') as source: data=source.read(64*1024*1024+1)
        if len(data)>64*1024*1024: raise Refused('resource_too_large','image exceeds the resource bound')
        directory=pathlib.Path.cwd()/'resources';directory.mkdir(mode=0o700,exist_ok=True)
        target=directory/(hashlib.sha256(data).hexdigest()+resource.suffix)
        if not target.exists(): target.write_bytes(data)
        result['resources'].append({"type":"resource_link","uri":target.as_uri(),"name":name,"mimeType":mimetypes.guess_type(name)[0] or 'application/octet-stream',"size":len(data)})
    return result

def refs(records):
    return [{key:r[key] for key in ('id','number','slug','path','digest')} for r in records]

def records():
    return [record(path) for path in sorted(ROOT.rglob('*.md')) if not path.is_symlink()]

def resolve(locator, rows):
    found=[r for r in rows if locator in (r['id'],r['slug'],r['path']) or locator.startswith('#') and str(r['number'])==locator[1:]]
    if len(found)!=1:
        raise Refused('ambiguous_reference' if found else 'not_found','record is ambiguous' if found else 'record was not found',refs(found))
    return found[0]

def exact(reference, rows):
    found=[r for r in rows if r['id']==reference['id'] and r['path']==reference['path']]
    if len(found)!=1: raise Refused('not_found','record was not found')
    if found[0]['digest']!=reference['digest']: raise Refused('stale_reference','record digest changed')
    return found[0]

def filtered(rows, filters):
    return [r for r in rows if all(not v or str(r['frontmatter'].get(k,''))==str(v) for k,v in filters.items() if k!='since')
        and (not filters.get('since') or str(r['frontmatter'].get('updated',r['frontmatter'].get('created','')))>=filters['since'])]

def page(items, args, identity):
    limit=args.get('limit',32)
    if not isinstance(limit,int) or isinstance(limit,bool) or not 1<=limit<=128: raise Refused('invalid_page','limit must be 1..128')
    offset=0
    if args.get('cursor'):
        try: token,offset=args['cursor'].rsplit(':',1);offset=int(offset)
        except (ValueError,AttributeError): raise Refused('invalid_cursor','invalid page cursor')
        if token!=identity or not 0<=offset<=len(items): raise Refused('invalid_cursor','cursor belongs to another query/root snapshot')
    end=offset+limit
    return {"items":items[offset:end],"next":identity+':'+str(end) if end<len(items) else None}

def edges(rows):
    result=[]
    for row in rows:
        for link in re.findall(r'(?<!!)\[\[([^\]]+)\]\]',row['body']):
            parts=link.split('|'); target=parts[0]
            try: target=resolve(target,rows)['id']
            except Refused: pass
            result.append({"from":row['id'],"to":target,"rel":parts[1] if len(parts)==3 else 'references',"weight":float(parts[2]) if len(parts)==3 else 1.0})
    return result

def call(method,args):
    if ROOT is None: raise Refused('not_initialized','initialize must bind a root first')
    if method=='health': return {"ok":True,"pid":os.getpid(),"cwd":os.getcwd()}
    if method=='write':
        if args.get('ref'):
            current=exact(args['ref'],records());path=confined(current['path'])
            if args.get('etag') and args['etag']!=current['etag']: raise Refused('etag_mismatch','etag does not match')
        else:
            path=confined(args.get('path',''),True)
            if path.exists(): raise Refused('path_exists','create path already exists')
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text('---\n'+json.dumps(args['frontmatter'],ensure_ascii=False)+'\n---\n'+args['body'],encoding='utf8')
        return {"record":record(path,True),"lossy":False}
    rows=records(); references=refs(rows)
    snapshot=hashlib.sha256(json.dumps(references,sort_keys=True).encode()).hexdigest()
    if method=='resolve': return refs([resolve(args['locator'],rows)])[0]
    if method=='fetch': return record(confined(exact(args['ref'],rows)['path']),True)
    if method=='list':
        selected=filtered(rows,args.get('filter',{})); key=hashlib.sha256((snapshot+json.dumps(args.get('filter',{}),sort_keys=True)).encode()).hexdigest()
        return page(refs(selected),args,key)
    if method=='search':
        filters={};words=[]
        for term in args['query'].split():
            if ':' in term and term.split(':',1)[0] in ['type','status','since']: key,value=term.split(':',1);filters[key]=value
            else: words.append(term.lower())
        selected=[r for r in filtered(rows,filters) if all(w in (str(r['frontmatter'].get('title',''))+' '+r['body']).lower() for w in words)]
        items=[{"ref":refs([r])[0],"title":str(r['frontmatter'].get('title',r['slug'])),"type":str(r['frontmatter'].get('type','note')),"number":r['number'],"snippet":r['body'][:512],"score":1.0} for r in selected]
        return page(items,args,hashlib.sha256((snapshot+args['query']).encode()).hexdigest())
    if method=='links':
        ref=exact(args['ref'],rows); all_edges=edges(rows)
        return {"outgoing":[e for e in all_edges if e['from']==ref['id']],"incoming":[e for e in all_edges if e['to']==ref['id']]}
    if method=='graph':
        scope=args['scope'];selected=rows
        if scope['graph']!='markdown': selected=[r for r in rows if r['frontmatter'].get('type')==scope['graph']]
        if scope['kind']=='type': selected=filtered(selected,{'type':scope['type']})
        elif scope['kind']=='swarm': selected=filtered(selected,{'swarm':scope['swarm']})
        elif scope['kind']=='neighborhood':
            center=exact(scope['ref'],rows)['id'];ids={center};all_edges=edges(rows)
            for _ in range(scope['depth']): ids|={e['to'] for e in all_edges if e['from'] in ids}|{e['from'] for e in all_edges if e['to'] in ids}
            selected=[r for r in selected if r['id'] in ids]
        nodes=[{"id":r['id'],"ref":refs([r])[0],"title":r['frontmatter'].get('title'),"type":r['frontmatter'].get('type'),"frontmatter":r['frontmatter']} for r in selected]
        paged=page(nodes,args,hashlib.sha256((snapshot+json.dumps(scope,sort_keys=True)).encode()).hexdigest());ids={n['id'] for n in paged['items']}
        outgoing=[e for e in edges(rows) if e['from'] in ids]
        return {"nodes":paged['items'],"edges":outgoing[:512],"next":paged['next'],"edges_truncated":len(outgoing)>512}
    if method=='changes_since':
        cursor=args['cursor'];previous={} if cursor=='0' else HISTORY.get(cursor)
        if previous is None: raise Refused('invalid_cursor','changes cursor is unknown to this instance')
        limit=args.get('limit',32)
        if not isinstance(limit,int) or isinstance(limit,bool) or not 1<=limit<=128: raise Refused('invalid_page','limit must be 1..128')
        current={r['id']:r for r in references}
        changed=[r for key,r in current.items() if previous.get(key)!=r]
        changed += [dict(r,number=None) for key,r in previous.items() if key not in current]
        changed=changed[:limit];progress=dict(previous)
        for row in changed:
            if row['id'] in current: progress[row['id']]=row
            else: progress.pop(row['id'],None)
        next_cursor=hashlib.sha256(json.dumps(progress,sort_keys=True).encode()).hexdigest()
        HISTORY[next_cursor]=progress
        return {"changed":changed,"next":next_cursor}
    raise Refused('unknown_method','method is not in the data_source contract')

def reply(request,result):
    print(json.dumps({"jsonrpc":"2.0","id":request['id'],"result":result},separators=(',',':')),flush=True)

if sys.argv[1:]==['--shape']:
    trace('shape')
    print(json.dumps(shape()),flush=True)
    raise SystemExit(0)
for line in sys.stdin:
    try:
        request=json.loads(line)
        if 'id' not in request: continue
        method=request['method'];params=request.get('params',{})
        if method=='initialize':
            if ROOT is not None: raise Refused('already_initialized','one root per instance')
            assert not any(k in params for k in ['author','signer','device','account'])
            ROOT=pathlib.Path(params['root']).resolve(strict=True)
            assert ROOT != pathlib.Path.cwd().resolve()
            trace('initialize',root=str(ROOT),cwd=os.getcwd())
            if not ROOT.is_dir(): raise Refused('invalid_root','root is not a directory')
            reply(request,{"protocolVersion":params['protocolVersion'],"serverInfo":{"name":"root fixture","version":"1.0"},"capabilities":{"tools":{},"experimental":{"bboxy450":{"shape_version":"1.0","contracts":{"data_source":"1.0"},"concurrent":False,"trace":False}}}})
        elif method=='tools/list':
            reply(request,{"tools":[{"name":"data_source_"+m,"inputSchema":schema(m),"outputSchema":schema(m,True)} for m in METHODS]+[{"name":"health","inputSchema":{"type":"object"}}]})
        elif method=='tools/call':
            name=params['name'];name=name.removeprefix('data_source_')
            assert not any(k in params.get('arguments',{}) for k in ['root','author','signer','device','account'])
            trace('call',method=name)
            if name=='fetch' and GATE and str(ROOT)==GATE_ROOT and (GATE_ARMED is None or os.path.exists(GATE_ARMED)):
                import socket
                with socket.socket(socket.AF_UNIX) as gate:
                    gate.connect(GATE);gate.recv(1)
            reply(request,{"structuredContent":call(name,params.get('arguments',{}))})
        else: reply(request,{"isError":True,"structuredContent":{"code":"unknown_method","message":"unknown MCP method"}})
    except Refused as reason: reply(request,{"isError":True,"structuredContent":reason.value})
    except Exception as reason: reply(request,{"isError":True,"structuredContent":{"code":"fixture_error","message":type(reason).__name__+': '+str(reason)}})
