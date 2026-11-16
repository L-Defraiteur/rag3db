#!/usr/bin/env python3
"""Proposed COPY-only qualification. No runtime without exact C7 GO and fresh Embar receipt."""
from pathlib import Path
import argparse,collections,hashlib,importlib.util,io,json,math,os,re,shutil,signal,stat,subprocess,sys,time
import audit,typed_rows
HOME=Path.home();R=HOME/'.cache/rag3weaver-build/bascule-live-20261116';HERE=Path(__file__).resolve().parent
P=R/'debt-acquittement-copy-v1';Q=R/'debt-acquittement-control-v2';S=R/'debt-acquittement-scopes-v1'
SOURCE=R/'migration-rehearsal-service-corrective-02/code-principal/post-copy';BASE=P/'copy/code-principal.rag3db'
PROOFS=R/'vector-debt-replay-01';GEO=R/'vector-debt-geometry-pc-01'
CLI=HOME/'.local/bin/rag3weaver-backend';IDS=R/'id-sets-pc-01/snapshot-files';BLAKE=PROOFS/'replay-chunker'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PINS={LIB/'rag3db.hpp':'4e25a659ce070ec38023ec90c1ced5a806fdcdaeda5eb96eda799feafa5806ba',CLI:'0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288',IDS:'d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7',BLAKE:'ac861f7f6d5da390a8b171429656f121497dfc79ed2e60d5e5bb3451819a37f0',LIB/'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',LIB/'libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
REPLAY_MANIFEST='6d4667936fc876e4a415d5cd12698486f0d231b7f8ebd1cca57250b475868cc6'
GEOMETRY_MANIFEST='dd5157b51fac6d4ed1877585655039ac30ca41b4f46f3841a8192f37088c2568'
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(path,obj,deadline=None):
 with path.open('x') as f:
  if deadline is not None and time.monotonic()>=deadline:raise ValueError('deadline immediately before publication')
  json.dump(obj,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
def first_red(reason):
 try:save(P/'first-red.json',dict(reason=str(reason),observed=time.monotonic()))
 except FileExistsError:pass
def loadmod(name):
 spec=importlib.util.spec_from_file_location(name,Q/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def verify_manifest(root,expected):
 if sha(root/'proof-manifest.json')!=expected:raise ValueError('evidence manifest differs')
 for row in json.loads((root/'proof-manifest.json').read_text()):
  p=root/row['name']
  if p.stat().st_size!=row['size'] or sha(p)!=row['sha256']:raise ValueError('evidence bytes differ')
def source_files():
 prefix='docs/arbre-principal/16-11-2026/debt-copy-a/'
 result={n:prefix+n for n in ['qualify.py','typed_rows.py','audit.py','read-typed.cpp','backend.json','payer-retard-dense.mmd','provider-go.schema.json']}
 result['inspect-fts.py']='docs/arbre-principal/16-11-2026/bascule/inspect-fts.py'
 for name in ['confiner_validation.py','poste']:result[name]='extension/rag3weaver/scripts/'+name
 return result

def guard(cfg):
 if git(cfg['source_root'],'rev-parse','HEAD')!=cfg['pilot'] or git(cfg['source_root'],'status','--porcelain'):raise ValueError('pilot source is not exact and clean')
 if set(cfg['files'])!=set(source_files()):raise ValueError('controller source set differs')
 for name,h in cfg['files'].items():
  actual_git=subprocess.check_output(['git','-C',cfg['source_root'],'show',cfg['pilot']+':'+source_files()[name]])
  if sha(Q/name)!=h or hashlib.sha256(actual_git).hexdigest()!=h:raise ValueError('controller/Git pin differs '+name)
 for path,h in PINS.items():
  if sha(path)!=h:raise ValueError('tool/SDK pin differs')
 if sha(cfg['compiler'])!=cfg['compiler_sha'] or sha(cfg['timeout'])!=cfg['timeout_sha']:raise ValueError('compiler/timeout differs')
def directories(root):
 rows=[]
 for path in sorted(root.rglob('*')):
  info=path.lstat()
  if info.st_uid!=os.getuid() or path.is_symlink():raise ValueError('foreign owner or symlink in bundle')
  if path.is_dir():rows.append(dict(path=str(path.relative_to(root)),mode=stat.S_IMODE(info.st_mode),uid=info.st_uid))
 return rows

def require_time(end,margin=15):
 if time.monotonic()>=end-margin:raise ValueError('allocation deadline')
def process_identity(pid):
 p=Path('/proc')/str(pid);raw=(p/'stat').read_text().rsplit(') ',1)[1].split()
 return dict(pid=pid,start=raw[19],state=raw[0],exe=os.readlink(p/'exe'),exe_sha256=sha(p/'exe'))
def collect_process_paths(proc,observe):
 for role in ['exe','cwd']:observe(role,os.readlink(proc/role))
 for fd in (proc/'fd').iterdir():
  try:path=os.readlink(fd)
  except (FileNotFoundError,ProcessLookupError):continue
  observe('fd',path)
def copy_quiet():
 found=[];unknown=[]
 for entry in Path('/proc').iterdir():
  if not entry.name.isdigit():continue
  pid=int(entry.name);attributed=False;start=None
  try:
   if entry.stat().st_uid!=os.getuid():continue
   fields=(entry/'stat').read_text().rsplit(') ',1)[1].split();start=fields[19]
   if fields[0]=='Z':continue
   def observe(field,path):
    nonlocal attributed
    if path==str(BASE.parent) or path.startswith(str(BASE.parent)+'/'):
     attributed=True;found.append(dict(pid=pid,start=start,field=field,path=path))
   # Existing DOUBLE collect_process_paths primitive, byte-identical port.
   collect_process_paths(entry,observe)
  except (FileNotFoundError,ProcessLookupError):continue
  except OSError as error:unknown.append(dict(pid=pid,start=start,attributed=attributed,error=str(error)))
 if found or any(x['attributed'] for x in unknown):raise ValueError('private COPY occupied or attributed UNKNOWN')
 return dict(found=found,unknown=unknown,meaning='unattributed global UNKNOWN retained; not negative observations')

def provider_identity(grant):
 actual=process_identity(grant['pid'])
 if actual['start']!=grant['start'] or actual['exe_sha256']!=grant['exe_sha256'] or actual['state'] in ['Z','T']:raise ValueError('provider identity changed')
 return actual

def validate_grant(grant,now,max_chars):
 if grant['provider']!='127.0.0.1:7878' or grant['model']!='granite-278m' or grant['dimension']!=768:raise ValueError('coordination targets another provider')
 if not grant.get('c7_reference') or not grant.get('embar_reference') or grant.get('restart_authorized') is not False:raise ValueError('coordination references absent or restart allowed')
 if any(type(grant.get(k)) not in [int,float] or not math.isfinite(grant[k]) for k in ['observed_monotonic','valid_until_monotonic']):raise ValueError('nonfinite coordination clock')
 if not 0<=now-grant['observed_monotonic']<=120 or now>=grant['valid_until_monotonic']:raise ValueError('coordination is not fresh')
 expected=grant['identity']
 if set(expected)!=set(['service','modele','dim','factice','dual','sparse','precision','executable','max_chars_per_text']):raise ValueError('provider identity must contain only the nine fixed fields')
 if expected.get('service')!='rag3weaver-embeddings' or expected.get('modele')!='granite-278m' or expected.get('dim')!=768 or expected.get('factice') is not False or expected.get('dual') is not False or expected.get('sparse') is not False:raise ValueError('not the expected real dense model')
 if type(expected.get('max_chars_per_text')) is not int or expected['max_chars_per_text']<max_chars or not expected.get('executable') or not expected.get('precision'):raise ValueError('provider cap or provenance insufficient')
 return expected

def health(grant,label,end,*,context=None):
 # The common worker owns HTTP; cycle policy is pinned and checked separately.
 require_time(end,0)
 if context is None:raise ValueError('explicit pinned health policy required')
 context=dict(context);now=time.monotonic()
 context['contact_deadline']=float(min(now+5.,end,grant['valid_until_monotonic']))
 trace=dict(no_embedding_calls_authorized=True,contact_deadline=context['contact_deadline'])
 try:
  policy=loadmod('lifecycle_health_policy');transport=loadmod('provider_health_transport')
  policy.worker_gate(context)
  response=transport.run_health_worker(context,trace)
 except BaseException as error:
  trace.update(error_type=type(error).__name__)
  save(P/(label+'.transport.json'),trace)
  raise
 raw=response['body'];trace.update(status=response['status'],body_hex=raw.hex(),body_sha256=hashlib.sha256(raw).hexdigest())
 # Immutable raw transport evidence exists before status/JSON/type assertions.
 save(P/(label+'.transport.json'),trace)
 if type(response['status']) is not int or response['status']!=200:raise ValueError('provider health refused')
 data=transport.validate_health(raw,grant['identity'],status=response['status'])
 policy.worker_gate(context)
 if time.monotonic()>=context['contact_deadline']:raise ValueError('whole health observation exceeded effective deadline')
 save(P/(label+'.json'),data)
 policy.worker_gate(context)
 if time.monotonic()>=context['contact_deadline']:raise ValueError('health publication exceeded effective deadline')
 return data

class Reader:
 def __init__(self,binary,end,env,timeout):self.binary=binary;self.end=end;self.env=env;self.timeout=timeout;self.counter=0
 def query(self,sql,collect,callback=None):
  require_time(self.end);self.counter+=1;label='read-'+str(self.counter);values=[]
  duration=min(90,self.end-time.monotonic()-15)
  with (P/(label+'.stderr')).open('xb') as err:
   proc=subprocess.Popen([self.timeout,'--signal=TERM','--kill-after=2s',str(duration)+'s',str(self.binary),str(BASE),sql],stdout=subprocess.PIPE,stderr=err,env=self.env)
   try:result=typed_rows.consume(proc.stdout,values.append if collect else callback)
   except BaseException:proc.stdout.close();proc.wait();raise
   finally:proc.stdout.close()
   rc=proc.wait()
  save(P/(label+'.json'),dict(query=sql,rc=rc,rows=result['count'],columns=result['columns']))
  if rc or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in (P/(label+'.stderr')).read_bytes():raise ValueError('native read failed')
  require_time(self.end);return values if collect else result
 def collect(self,sql):return self.query(sql,True)
 def digest(self,sql):return self.query(sql,False)
 def visit(self,sql,callback):return self.query(sql,False,callback)

def fts(reader,fs,env,end,offsets,label):
 root=BASE.parent/(BASE.name+'.fts');observed=fs.inspect(root);rows=[]
 for index in observed['indexes']:
  require_time(end);name=index['index_key'];table=name.removeprefix('Lucivy_');audit.identifier(table)
  if table not in offsets:raise ValueError('FTS index has no physical graph table')
  result=subprocess.run([str(IDS),str(root/name),'--live-ids'],env=env,capture_output=True,timeout=min(60,end-time.monotonic()-15))
  (P/(label+'-'+name+'.stdout')).write_bytes(result.stdout);(P/(label+'-'+name+'.stderr')).write_bytes(result.stderr)
  if result.returncode:raise ValueError('immutable FTS decoding failed')
  lines=result.stdout.decode().splitlines()
  if not lines or not re.fullmatch(r'IMMUTABLE_SNAPSHOT\tshards=[1-9][0-9]*\tdocuments=[0-9]+\tbytes=[0-9]+\tcommit=REFUSED',lines[0]):raise ValueError('FTS protocol differs')
  ids=[]
  for line in lines[1:]:
   if not re.fullmatch(r'LIVE_ID\t[0-9]+',line):raise ValueError('FTS ID protocol differs')
   ids.append(int(line.split('\t')[1]))
  count=int(re.search(r'documents=([0-9]+)',lines[0])[1])
  if len(ids)!=count or len(set(ids))!=count or sorted(ids)!=offsets[table]:raise ValueError('FTS/physical ID sets differ')
  generation=reader.collect("MATCH (m:_catalog_meta {_key:'fts_generation:"+name+"'}) RETURN m._value")
  if len(generation)!=1 or int(typed_rows.text(generation[0][0]))!=index['generation_file']:raise ValueError('FTS generation differs from DB')
  rows.append(dict(index=name,ids=ids,generation=index['generation_file']))
 if fs.file_inventory(root)!=observed['files']:raise ValueError('FTS snapshot read changed files')
 return dict(indexes=rows,files=observed['files'])

def verify_meta(response):
 if response.get('ok') is not True:raise ValueError('tool response refused')
 meta=response['result']['result'];meta=json.loads(meta) if isinstance(meta,str) else meta
 # SearchMeta (search.rs) serializes camelCase. No legacy alias or ambiguous pair.
 if not isinstance(meta,dict) or 'pending_count' in meta or type(meta.get('pendingCount')) is not int or meta['pendingCount']!=0 or meta.get('warnings') or meta.get('partial') is not False or meta.get('signals')!=[]:raise ValueError('source.meta refused or partial; not a debt-zero proof')
 return meta

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);common=loadmod('confiner_validation');authority=common.autorite(common.processus());end=authority['echeance'];start=time.monotonic()
 recipe=json.loads((Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).parent/'recette.json').read_text())
 if recipe['delai']!=1730:raise ValueError('unexpected common budget')
 cg=Path('/sys/fs/cgroup')/authority['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 if limits['memory.max']!='34359738368' or limits['memory.high']!='max' or limits['memory.swap.max']!='0':raise ValueError('memory contract differs')
 quota,period=limits['cpu.max'].split()
 if quota=='max' or not 0<int(quota)<=8*int(period):raise ValueError('CPU contract differs')
 save(P/'authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,limits=limits))
 fs=loadmod('inspect-fts');verify_manifest(PROOFS,REPLAY_MANIFEST);verify_manifest(GEO,GEOMETRY_MANIFEST)
 expected=json.loads((PROOFS/'source-before.json').read_text());source_before=fs.file_inventory(SOURCE)
 if source_before!=expected:raise ValueError('COPY02 private source changed')
 save(P/'source-before.json',source_before);source_dirs=directories(SOURCE);save(P/'source-directories-before.json',source_dirs)
 geometry=json.loads((GEO/'observations.json').read_text());cohort=sorted(row.split('|')[0] for row in geometry['cohort-new-geometry'])
 if len(cohort)!=469 or len(set(cohort))!=469:raise ValueError('invalid exact debt cohort')
 save(P/'DEBT_PENDING.json',dict(count=469,ids=cohort,paid=False,source=str(SOURCE)))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for name in ['tmp','cache','config','data','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env.update(LD_LIBRARY_PATH=str(LIB),TMPDIR=str(P/'tmp'),XDG_CACHE_HOME=str(P/'cache'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=LIB.name,RAG3DB_LIBS_DIR=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',CARGO_BUILD_JOBS='8',RUST_TEST_THREADS='4',RAYON_NUM_THREADS='4',RAG3WEAVER_SERVICE_EMBED='127.0.0.1:7878')
 def command(args,cap):
  require_time(end);duration=min(cap,end-time.monotonic()-15)
  if str(args[0])==cfg['compiler']:label='compile'
  elif str(args[0])==str(binary) and args[1:]==['--self-test']:label='typed-selftest'
  elif args[0]=='cp':label='copy'
  else:raise ValueError('not an approved corpus-free preflight command')
  began=time.monotonic();save(P/(label+'-command.json'),dict(argv=[str(x) for x in args],cap=duration,started=began))
  result=None;timed_out=None
  try:result=subprocess.run(args,capture_output=True,env=env,timeout=duration)
  except subprocess.TimeoutExpired as error:timed_out=error
  stdout=(timed_out.stdout or b'') if timed_out else result.stdout
  stderr=(timed_out.stderr or b'') if timed_out else result.stderr
  # These three commands cannot emit database corpus: preserve even failed/partial logs.
  for suffix,data in [('stdout',stdout),('stderr',stderr)]:
   with (P/(label+'.'+suffix)).open('xb') as stream:stream.write(data);stream.flush();os.fsync(stream.fileno())
  save(P/(label+'-process.json'),dict(rc=None if timed_out else result.returncode,timeout=timed_out is not None,elapsed=time.monotonic()-began,stdout_bytes=len(stdout),stderr_bytes=len(stderr)))
  if timed_out:raise RuntimeError(label+' timed out; partial outputs preserved') from timed_out
  if result.returncode:raise RuntimeError(label+' refused rc='+str(result.returncode)+'; outputs preserved')
  return result
 binary=P/'read-typed';build=[cfg['compiler'],'-std=c++20','-O2','-I'+str(LIB),str(Q/'read-typed.cpp'),'-L'+str(LIB),'-Wl,-rpath,'+str(LIB),'-lrag3db','-pthread','-o',str(binary)]
 save(P/'prebuild-ledger.json',dict(command=build,source_sha256=sha(Q/'read-typed.cpp'),compiler_sha256=sha(cfg['compiler']),pins={str(p):h for p,h in PINS.items()},prepared_sha256=sha(Q/'prepared.json')))
 result=command(build,90)
 golden=command([str(binary),'--self-test'],15);decoded=[];typed_rows.consume(io.BytesIO(golden.stdout),decoded.append)
 if decoded!=[[('STRING',b'a\0|\nb'),('FLOAT',1.25),('STRING',b'')]]:raise ValueError('native typed selftest differs')
 save(P/'typed-before-db.json',dict(elf_sha256=sha(binary),golden_sha256=hashlib.sha256(golden.stdout).hexdigest()))
 command(['cp','-a','--reflink=always','--',str(SOURCE),str(P/'copy')],120)
 if fs.file_inventory(BASE.parent)!=expected:raise ValueError('new private copy is not faithful')
 reader=Reader(binary,end,env,cfg['timeout']);before=audit.snapshot(reader,cohort);save(P/'graph-before.json',before)
 before_fts=fts(reader,fs,env,end,before['offsets'],'before');save(P/'fts-before.json',before_fts)
 for query in ["MATCH (n:ScopeContent) WHERE n._chunked_hash IS NULL OR n._chunked_hash<>n._content_hash RETURN count(*)","MATCH (v:ScopeContent)-[r:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE r._render_hash IS NULL OR r._render_hash='' RETURN count(*)"]:
  rows=reader.collect(query)
  if len(rows)!=1 or audit.number(rows[0][0])!=0:raise ValueError('additional chunk/render debt present')
 def blake(body):
  require_time(end)
  result=subprocess.run([str(BLAKE),'1500','0','30','5','lines'],input=body,capture_output=True,env=env,timeout=min(5,end-time.monotonic()-15))
  if result.returncode:raise ValueError('actual stored input hashing failed')
  head=result.stdout.splitlines()[0].decode()
  if not re.fullmatch(r'INPUT\t'+str(len(body))+r'\t[0-9a-f]{64}',head):raise ValueError('input hash witness differs')
  return head.split('\t')[2]
 payload=audit.inputs(reader,cohort,blake);save(P/'payload-before-provider.json',dict(records=payload,embedding_input='_text; _title separately fingerprinted but not prepended',model='granite-278m',dimension=768,storage='suffixed',count=469,cache_reuse_qualified=0,minimum_calculations_claimed=False))
 # Publication of this pause precedes every provider request and the writer.
 save(P/'READY_FOR_PROVIDER_COORDINATION.json',dict(observed=time.monotonic(),deadline=end,max_chars=max(x['chars'] for x in payload),payload_sha256=sha(P/'payload-before-provider.json'),coordination_path=str(Q/'PROVIDER_GO.json')))
 pause_end=min(end-60,time.monotonic()+120)
 while not (Q/'PROVIDER_GO.json').exists():
  if time.monotonic()>=pause_end:raise ValueError('fresh provider coordination not received')
  time.sleep(.2)
 grant_path=Q/'PROVIDER_GO.json';mode=grant_path.lstat()
 if grant_path.is_symlink() or not stat.S_ISREG(mode.st_mode) or mode.st_uid!=os.getuid():raise ValueError('invalid coordination file')
 grant=json.loads(grant_path.read_text());validate_grant(grant,time.monotonic(),max(x['chars'] for x in payload));save(P/'provider-coordination.json',grant)
 profile={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'}
 save(P/'profile-before-provider.json',dict(expected_cli_environment=profile,manifest_sha256=sha(Q/'backend.json'),payload_sha256=sha(P/'payload-before-provider.json'),embedding_input='_text exactly, no title prefix',provider_contact_started=False))
 health_before=health(grant,'provider-before',end);guard(cfg)
 if fs.file_inventory(SOURCE)!=source_before or directories(SOURCE)!=source_dirs:raise ValueError('private source changed before writer')
 copy_now=fs.file_inventory(BASE.parent)
 if copy_now!=expected or directories(BASE.parent)!=source_dirs:raise ValueError('read-only preflight changed new copy or directories')
 save(P/'copy-before-writer-quiet.json',copy_quiet())
 save(P/'ledger-before-writer.json',dict(cli_sha256=sha(CLI),manifest_sha256=sha(Q/'backend.json'),graph_sha256=sha(Q/'payer-retard-dense.mmd'),source_inventory_sha256=sha(P/'source-before.json'),payload_sha256=sha(P/'payload-before-provider.json'),computed_count=None,reused_count=None))
 require_time(end);call_end=min(end-60,time.monotonic()+900,grant['valid_until_monotonic']);require_time(call_end);captured=False
 with (P/'writer.stdout').open('xb') as out,(P/'writer.stderr').open('xb') as err:
  proc=subprocess.Popen([str(CLI),str(Q/'backend.json')],stdin=subprocess.PIPE,stdout=out,stderr=err,env=env,cwd=P)
  identity=process_identity(proc.pid);save(P/'writer-process.json',identity)
  if identity['exe_sha256']!=PINS[CLI]:raise ValueError('writer ELF differs')
  proc.stdin.write(b'{"op":"call","name":"payer_retard_dense","arguments":{}}\n');proc.stdin.flush();shutdown_sent=False
  while proc.poll() is None:
   require_time(call_end,0)
   if (Path('/proc')/str(proc.pid)/'stat').read_text().rsplit(') ',1)[1].split()[19]!=identity['start']:raise ValueError('writer start changed')
   maps=(Path('/proc')/str(proc.pid)/'maps').read_text()
   if str(LIB/'librag3db.so') in maps and str(LIB/'libvector.rag3db_extension') in maps and not captured:
    raw_env=(Path('/proc')/str(proc.pid)/'environ').read_bytes();observed_env={}
    for field in raw_env.split(b'\0'):
     if b'=' in field:
      key,val=field.split(b'=',1);key=os.fsdecode(key)
      if key in profile:observed_env[key]=os.fsdecode(val)
    save(P/'writer-profile.json',dict(pid=proc.pid,start=identity['start'],expected=profile,actual=observed_env,diff={k:dict(expected=v,actual=observed_env.get(k)) for k,v in profile.items() if observed_env.get(k)!=v}))
    if observed_env!=profile:raise ValueError('actual writer environment differs')
    save(P/'writer-native-maps.json',dict(pid=proc.pid,start=identity['start'],exe_sha256=sha(Path('/proc')/str(proc.pid)/'exe'),maps=maps));captured=True
   for line in (P/'writer.stdout').read_bytes().splitlines():
    try:response=json.loads(line)
    except json.JSONDecodeError:continue
    if response.get('ok') is False:raise ValueError('writer first refused response')
   complete=[]
   for line in (P/'writer.stdout').read_bytes().splitlines():
    try:complete.append(json.loads(line))
    except json.JSONDecodeError:pass
   if complete and not shutdown_sent:
    verify_meta(complete[0]);proc.stdin.write(b'{"op":"shutdown"}\n');proc.stdin.close();shutdown_sent=True
   time.sleep(.05)
  rc=proc.wait()
 if rc or not captured:raise ValueError('writer exit or real SDK maps unqualified')
 responses=[json.loads(line) for line in (P/'writer.stdout').read_bytes().splitlines()]
 if len(responses)!=2 or not all(x.get('ok') is True for x in responses) or responses[1].get('result')!={'closed':True}:raise ValueError('tool/shutdown acknowledgement differs')
 verify_meta(responses[0])
 save(P/'writer-closed.json',dict(pid=proc.pid,start=identity['start'],rc=rc,shutdown_acknowledged=True))
 grant_after=health(grant,'provider-after',end)
 if grant_after.get('cut_texts')!=health_before.get('cut_texts'):raise ValueError('provider truncated input during allocated work')
 after=audit.snapshot(reader,cohort);save(P/'graph-after.json',after)
 if before!=after:raise ValueError('graph/schema/config/generation/visibility changed outside allowed cohort vector delta')
 after_fts=fts(reader,fs,env,end,after['offsets'],'after');save(P/'fts-after.json',after_fts)
 if before_fts!=after_fts:raise ValueError('FTS IDs/generation/files changed')
 finish=audit.finished(reader,cohort);save(P/'DEBT_ZERO_OBSERVED.json',finish)
 if fs.file_inventory(SOURCE)!=source_before or directories(SOURCE)!=source_dirs:raise ValueError('COPY02 source mutated')
 save(P/'source-after.json',fs.file_inventory(SOURCE));save(P/'source-directories-after.json',directories(SOURCE));save(P/'copy-after.json',fs.file_inventory(BASE.parent));guard(cfg);require_time(end,5)
 save(P/'driver-result.json',dict(status='COPY_DEBT_ZERO_JUDGES_PASSED',deadline=end,elapsed=time.monotonic()-start,source_unchanged=True,copy_only=True,live_authorized=False,computed_count=None,reused_count=None),deadline=end)

def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=loadmod('confiner_validation');parent=Path('/proc')/str(os.getppid())
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','1798s',sys.executable,str(Q/'qualify.py'),'--compose']
 if os.readlink(parent/'exe')!=cfg['timeout'] or [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]!=expected or not common.poste_tenu(common.processus()):raise ValueError('real bounded outer/poste absent')
 P.mkdir(mode=0o700)
 def stop(sig,frame):first_red('outer deadline signal');raise RuntimeError('outer stopped COPY qualification')
 signal.signal(signal.SIGTERM,stop);status=dict(status='RED')
 try:
  guard(cfg);command=[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','1730','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver']
  rc=subprocess.run(command,cwd='/var/tmp').returncode
  authority_path=Path(json.loads((P/'authority-path.json').read_text())['path']);authority=json.loads(authority_path.read_text());scope=json.loads((authority_path.parent/'verdict.json').read_text())
  if rc or scope['verdict']!='VERT' or scope['code'] or scope['restes'] or not scope['nettoye'] or authority['etat']!='termine' or Path(scope['prive']).exists():raise ValueError('common scope not quiet green')
  owned=json.loads((authority_path.parent/'enfants.json').read_text())
  for row in owned:
   proc=Path('/proc')/str(row['pid'])
   try:
    fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[19]==row['debut'] and fields[0]!='Z':raise ValueError('owned process remains')
   except (FileNotFoundError,ProcessLookupError):pass
  save(P/'copy-final-quiet.json',copy_quiet())
  result=json.loads((P/'driver-result.json').read_text())
  if result['status']!='COPY_DEBT_ZERO_JUDGES_PASSED' or (P/'first-red.json').exists():raise ValueError('driver or first-red')
  guard(cfg)
  fs=loadmod('inspect-fts')
  if fs.file_inventory(SOURCE)!=json.loads((P/'source-before.json').read_text()) or directories(SOURCE)!=json.loads((P/'source-directories-before.json').read_text()):raise ValueError('source changed during finalization')
  if sha(P/'read-typed')!=json.loads((P/'typed-before-db.json').read_text())['elf_sha256']:raise ValueError('typed reader changed')
  require_time(authority['echeance'],0)
  status=dict(status='COPY_DEBT_ZERO_CHECKED',deadline=authority['echeance'],scope=scope,owned_checked=len(owned),live_authorized=False,independent_review_pending=True)
 except BaseException as error:first_red(error);status=dict(status='RED',error=str(error),live_authorized=False)
 save(P/'composed.json',status,deadline=status.get('deadline'));return 0 if status['status']=='COPY_DEBT_ZERO_CHECKED' else 1

def prepare(root,pilot):
 if os.uname().nodename.lower()!='luciepc' or git(root,'rev-parse','HEAD')!=pilot or git(root,'status','--porcelain'):raise ValueError('exact clean PC pilot required')
 if P.exists() or S.exists() or BASE.exists():raise ValueError('COPY/proof scope must be new')
 if any(p.resolve()!=p for p in [Q,P,S,SOURCE]):raise ValueError('private path alias')
 Q.mkdir(mode=0o700,exist_ok=False)
 files=['qualify.py','typed_rows.py','audit.py','read-typed.cpp','backend.json','payer-retard-dense.mmd','provider-go.schema.json']
 for n in files:shutil.copy2(HERE/n,Q/n)
 helpers=Path(root)/'docs/arbre-principal/16-11-2026/bascule'
 for n in ['inspect-fts.py']:shutil.copy2(helpers/n,Q/n);files.append(n)
 for n in ['confiner_validation.py','poste']:shutil.copy2(Path(root)/'extension/rag3weaver/scripts'/n,Q/n);files.append(n)
 compiler=str(Path(shutil.which('c++')).resolve());timeout=str(Path(shutil.which('timeout')).resolve())
 cfg=dict(source_root=str(root),pilot=pilot,files={n:sha(Q/n) for n in files},compiler=compiler,compiler_sha=sha(compiler),timeout=timeout,timeout_sha=sha(timeout))
 save(Q/'prepared.json',cfg);guard(cfg);return cfg

def launch():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg)
 if args.launch!=cfg['pilot']:raise ValueError('launch is not the exact prepared pilot')
 if not args.allocation or P.exists() or S.exists() or 'RAG3WEAVER_VALIDATION_RECU' in os.environ:raise ValueError('explicit new allocation and fresh COPY required')
 save(Q/'allocation.json',dict(reference=args.allocation,copy_only=True,live_authorized=False))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
 env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 # All runtime, cleanup, hashes and publication are inside this after-poste outer cap.
 command=[str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','1798s',sys.executable,str(Q/'qualify.py'),'--compose']
 with (Q/'outer.log').open('x') as out:result=subprocess.run(command,cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--prepare',metavar='EXACT_PILOT');ap.add_argument('--launch',metavar='EXACT_PILOT');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.prepare:
  cfg=prepare(Path(__file__).resolve().parents[4],args.prepare);print(json.dumps(dict(prepared=str(Q/'prepared.json'),sha256=sha(Q/'prepared.json'),writer=False,provider_contact=False)))
 elif args.launch:sys.exit(launch())
 elif args.compose:sys.exit(compose())
 elif args.driver:
  try:driver()
  except BaseException as error:first_red(error);raise
 else:ap.error('choose explicit prepare/launch')
