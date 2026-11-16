#!/usr/bin/env python3
"""Proposed 600s read-only diagnostic on a new private copy; explicit C7 GO required."""
import argparse,hashlib,importlib.util,json,os,re,shutil,signal,stat,subprocess,sys,time
from pathlib import Path
from blob_adapter import decode_frames,materialize
from observations import native_queries,snapshot_ids,compare_ids
R=Path.home()/'.cache/rag3weaver-build'
Q=R/'cx-control-recherche-fts-blobs-ro-v2';P=R/'cx-proofs-recherche-fts-blobs-ro-v2';S=R/'cx-scope-recherche-fts-blobs-ro-v2'
ARCHIVE=R/'cx-archive-recherche-fts-existing-canon-fixture-v1'
ARCHIVE_SHA='69b1734910cace8d0b05ff4d2a5b128d2594c9e2834eb136167aa462dd433192'
NATIVE=Path.home()/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc'
NATIVE_SHA='48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616'
IDS=R/'bascule-live-20261116/id-sets-pc-01/snapshot-files'
IDS_SHA='d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7'
LIB=R.parent/'rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
SQL=[
 "MATCH (m:_catalog_meta) WHERE m._key = 'code_scope_form' RETURN m._value",
 "MATCH (m:_catalog_meta) WHERE m._key = 'entity_config:Scope' RETURN m._value",
 "MATCH (m:_catalog_meta) WHERE m._key = 'entity_config:ScopeContent' RETURN m._value",
 "MATCH (g:_generation) RETURN g",
 "MATCH (s:Scope) RETURN DISTINCT s.source, s._couche, s._depuis, s._jusqu_a, s._org, s._project",
 "MATCH (n:ScopeContent) RETURN DISTINCT n._org, n._project",
 "MATCH (n:ScopeContent) WHERE n._org = 'default' AND n._project = 'default' RETURN OFFSET(id(n)) ORDER BY OFFSET(id(n))",
 "MATCH (b:_index_blobs) WHERE b._deleted_gen IS NULL OR b._deleted_gen < 0 RETURN count(b)",
]
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value):
 with (root/name).open('x') as f:json.dump(value,f,indent=2)
def inventory(root):
 result={}
 for path in sorted(root.rglob('*')):
  m=path.lstat();assert not path.is_symlink() and m.st_uid==os.getuid()
  if path.is_dir():continue
  assert stat.S_ISREG(m.st_mode)
  result[str(path.relative_to(root))]=dict(bytes=m.st_size,sha256=sha(path))
 return result
def guard(cfg):
 root=Path(cfg['source_root']);assert git(root,'rev-parse','HEAD')==cfg['pilot'] and not git(root,'status','--porcelain')
 assert sha(ARCHIVE/'manifest.json')==ARCHIVE_SHA
 assert inventory(ARCHIVE/'copy')==cfg['files']
 assert not any(p.is_dir() for p in (ARCHIVE/'copy').rglob('*')),'unexpected archive directory'
 assert sha(cfg['compiler'])==cfg['compiler_sha256']
 assert sha(cfg['timeout'])==cfg['timeout_sha256']
 for name,value in cfg['copies'].items():assert sha(Q/name)==value
 assert sha(NATIVE)==NATIVE_SHA and sha(IDS)==IDS_SHA
 for name,value in SO.items():assert sha(LIB/name)==value

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());assert P.is_dir();guard(cfg)
 subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
 receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
 assert auth['etat']=='actif' and json.loads((receipt.parent/'recette.json').read_text())['delai']==550
 group=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={name:(group/name).read_text().strip() for name in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits['memory.max']=='34359738368' and limits['memory.high']=='max' and limits['memory.swap.max']=='0'
 quota,period=limits['cpu.max'].split();assert quota!='max' and 0<int(quota)<=8*int(period)
 save(P,'authority-path.json',dict(path=str(receipt),deadline=end,initial=end-550,limits=limits))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ};env['LD_LIBRARY_PATH']=str(LIB)
 def run(label,argv,cap):
  guard(cfg);duration=min(cap,end-time.monotonic()-15);assert duration>0
  save(P,label+'.command.json',dict(argv=argv,observed=time.monotonic(),cap=duration))
  with (P/(label+'.stdout')).open('xb') as out,(P/(label+'.stderr')).open('xb') as err:
   result=subprocess.run(argv,cwd=P,env=env,stdout=out,stderr=err,timeout=duration)
  save(P,label+'.result.json',dict(code=result.returncode,observed=time.monotonic()))
  if result.returncode:
   save(P,'first-red.json',dict(phase=label,code=result.returncode));raise RuntimeError('first red: '+label)
  assert time.monotonic()<end-15
  return (P/(label+'.stdout')).read_bytes()
 binary=P/'dump-blobs'
 run('compile',[cfg['compiler'],'-std=c++20','-O2','-I'+str(LIB),'-I'+str(LIB/'include'),str(Q/'dump-blobs.cpp'),'-L'+str(LIB),'-Wl,-rpath,'+str(LIB),'-lrag3db','-pthread','-o',str(binary)],90)
 golden=run('typed-selftest',[str(binary),'--self-test'],30)
 assert decode_frames(golden)=={'index/blob':b'\x00\xff\\\n"'}
 save(P,'adapter-before-db.json',dict(binary_sha256=sha(binary),selftest_sha256=hashlib.sha256(golden).hexdigest(),meaning='typed BLOB transport only, no Database in selftest'))
 cold=P/'cold-copy';shutil.copytree(ARCHIVE/'copy',cold);assert inventory(cold)==cfg['files'];base=cold/'base.rag3db'
 save(P,'copy-before.json',inventory(cold))
 def unchanged():
  current=inventory(cold)
  assert not any(p.is_dir() for p in cold.rglob('*')),'no new database directory is authorized'
  for name,value in cfg['files'].items():assert current.get(name)==value,'data/WAL/sidecar changed: '+name
  for name,value in current.items():
   if name not in cfg['files']:
    path=cold/name;meta=path.lstat();assert name=='base.rag3db.readers' and stat.S_ISREG(meta.st_mode) and meta.st_uid==os.getuid() and value['bytes']==0,'unapproved sidecar'
  assert inventory(ARCHIVE/'copy')==cfg['files']
 raw=run('native-inventory',[str(NATIVE),str(base),*SQL],90);unchanged()
 assert 'BUILD_ID' in (P/'native-inventory.stderr').read_text() and '59ac9c86bc' in (P/'native-inventory.stderr').read_text()
 rows=native_queries(raw.decode('utf-8'),len(SQL))
 assert rows[0]==['code-scope-v1/content/generations']
 assert len(rows[1])==len(rows[2])==1
 assert json.loads(rows[1][0])['searchIndex']=='ScopeContent' and json.loads(rows[2][0])['derived']['from']=='Scope'
 assert rows[5]==['default|default'],'unexpected physical cell; no alias fallback'
 framed=run('blob-dump',[str(binary),str(base)],90);unchanged()
 assert 'BUILD_ID' in (P/'blob-dump.stderr').read_text() and '59ac9c86bc' in (P/'blob-dump.stderr').read_text()
 blobs=decode_frames(framed)
 assert rows[7]==[str(len(blobs))]
 root=P/'immutable-fts';manifest=materialize(blobs,'Lucivy_ScopeContent',root);save(P,'materialized.json',manifest)
 materialized_dirs=sorted(str(p.relative_to(root)) for p in root.rglob('*') if p.is_dir())
 decoded=run('immutable-ids',[str(IDS),str(root),'--live-ids'],90)
 assert inventory(root)==manifest and materialized_dirs==sorted(str(p.relative_to(root)) for p in root.rglob('*') if p.is_dir())
 live=snapshot_ids(decoded.decode('utf-8'));comparison=compare_ids(rows[6],live)
 save(P,'id-sets.json',dict(graph_rows=rows[6],fts_live_ids=live,comparison=comparison,scope='physical ScopeContent/default/default'))
 unchanged();guard(cfg);save(P,'copy-after.json',inventory(cold));assert time.monotonic()<end-10
 save(P,'observation.json',dict(status='OBSERVATION_RO_VALIDATED_ONLY',comparison=comparison,source_database_opened=False,deadline=end,elapsed_since_common=time.monotonic()-(end-550),global_hardcap_seconds=600,branch_at_failed_runtime_traced=False,writer_maintenance_qualified=False))

def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 root=Path(__file__).resolve().parents[4];assert git(root,'rev-parse','HEAD')==args.pilot and not git(root,'status','--porcelain')
 assert not any(p.exists() or p.is_symlink() for p in [P,S])
 assert sha(ARCHIVE/'manifest.json')==ARCHIVE_SHA
 files=json.loads((ARCHIVE/'manifest.json').read_text())['files'];assert inventory(ARCHIVE/'copy')==files
 assert sha(NATIVE)==NATIVE_SHA and sha(IDS)==IDS_SHA
 for name,value in SO.items():assert sha(LIB/name)==value
 compiler=str(Path(shutil.which('c++')).resolve());assert Path(compiler).is_file()
 Q.mkdir(mode=0o700,exist_ok=True);assert Q.resolve()==Q and Q.stat().st_uid==os.getuid()
 copies={};folder=Path(__file__).parent
 for name,source in [(name,folder/name) for name in ['qualify.py','blob_adapter.py','observations.py','dump-blobs.cpp']]+[('confiner_validation.py',root/'extension/rag3weaver/scripts/confiner_validation.py'),('poste',root/'extension/rag3weaver/scripts/poste')]:
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(source)
  else:shutil.copy2(source,dest)
  copies[name]=sha(dest)
 cfg=dict(source_root=str(root),pilot=args.pilot,files=files,copies=copies,compiler=compiler,compiler_sha256=sha(compiler),timeout=str(Path(shutil.which('timeout')).resolve()),timeout_sha256=sha(Path(shutil.which('timeout')).resolve()))
 prepared=Q/'prepared.json'
 if prepared.exists():assert json.loads(prepared.read_text())==cfg
 else:save(Q,'prepared.json',cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=str(prepared),cargo=False,native_execution=False,authority_created=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 save(Q,'allocation.json',dict(reference=args.allocation))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',CARGO_BUILD_JOBS='8')
 return run_outer(cfg,env)

def outer_command(cfg):
 return [str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','598s',sys.executable,str(Q/'qualify.py'),'--compose']

def run_outer(cfg,env):
 # No hash, scan, composition or publication after this bounded child returns.
 with (Q/'launch.log').open('x') as out:
  result=subprocess.run(outer_command(cfg),cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode

def verify_outer_signature(observed,cfg):
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','598s',sys.executable,str(Q/'qualify.py'),'--compose']
 assert observed['exe']==cfg['timeout'] and observed['argv']==expected and observed['poste'],'real outer timeout/poste required'

def owned_quiet(scope):
 rows=json.loads((scope/'enfants.json').read_text());assert isinstance(rows,list)
 alive=[];unknown=[]
 for row in rows:
  try:
   fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
   if fields[19]==row['debut'] and fields[0]!='Z':alive.append(row)
  except (FileNotFoundError,ProcessLookupError):pass
  except OSError as error:unknown.append(dict(pid=row['pid'],error=repr(error)))
 return dict(checked=len(rows),alive=alive,unknown=unknown)

def compose():
 # This entire function, including imports/reads/cleanup and final guard, is under
 # the 598s+2s outer timeout which poste has already started.
 cfg=json.loads((Q/'prepared.json').read_text())
 spec=importlib.util.spec_from_file_location('common_validation',Q/'confiner_validation.py')
 common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 parent=Path('/proc')/str(os.getppid())
 observed=dict(exe=str(Path(os.readlink(parent/'exe')).resolve()),argv=[os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v],poste=common.poste_tenu(common.processus()))
 verify_outer_signature(observed,cfg)
 P.mkdir(mode=0o700);save(P,'outer-supervision.json',dict(observed=time.monotonic(),parent=observed,soft_timeout=598,kill_after=2,hardcap=600,common_budget=550))
 def stop(sig,frame):
  if not (P/'first-red.json').exists():save(P,'first-red.json',dict(reason='outer supervision signal',signal=sig,observed=time.monotonic()))
  raise RuntimeError('outer supervision stopped the whole diagnostic')
 signal.signal(signal.SIGTERM,stop)
 record=dict(status='RED');rc=1
 try:
  guard(cfg)
  command=[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','550','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver']
  rc=subprocess.run(command,cwd='/var/tmp',env=os.environ.copy()).returncode
  authority=json.loads((P/'authority-path.json').read_text());path=Path(authority['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text())
  quiet=owned_quiet(path.parent);save(P,'owned-after-scope.json',quiet)
  record.update(code=rc,scope=scope,owned=quiet,authority_state=auth['etat'])
  assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and auth['etat']=='termine' and not Path(scope['prive']).exists()
  assert not quiet['alive'] and not quiet['unknown']
  assert json.loads((P/'observation.json').read_text())['status']=='OBSERVATION_RO_VALIDATED_ONLY' and not (P/'first-red.json').exists()
  guard(cfg)
  record.update(status='OBSERVATION_RO_VALIDATED_ONLY',deadline=auth['echeance'])
 except BaseException as error:
  record.update(status='RED',code=rc,error=repr(error))
  if not (P/'first-red.json').exists():save(P,'first-red.json',dict(reason=repr(error),observed=time.monotonic()))
 with (P/'composed.json').open('x') as f:
  if record['status']=='OBSERVATION_RO_VALIDATED_ONLY' and time.monotonic()>=record['deadline']:record=dict(status='RED',error='deadline before publication')
  json.dump(record,f,indent=2)
 print(json.dumps(record));return 0 if record['status']=='OBSERVATION_RO_VALIDATED_ONLY' else 1
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--pilot');parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--allocation');parser.add_argument('--driver',action='store_true');parser.add_argument('--compose',action='store_true');args=parser.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:
   if P.exists() and not (P/'first-red.json').exists():save(P,'first-red.json',dict(error=repr(error),observed=time.monotonic()))
   raise
 else:sys.exit(launch(args))
