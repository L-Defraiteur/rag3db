#!/usr/bin/env python3
"""Proposed frozen-result RO observation. Never starts a writer or provider."""
from pathlib import Path
import argparse,hashlib,json,os,shutil,signal,stat,subprocess,sys,time
import qualify as q
import audit

R=q.R;OLD=R/'debt-acquittement-copy-v1'
P=R/'debt-audit-ro-result-v1';Q=R/'debt-audit-ro-control-v1';S=R/'debt-audit-ro-scopes-v1'
q.Q=Q
RESULT=OLD/'copy';ORIGIN=q.SOURCE;BASE=P/'copy/code-principal.rag3db'
BINARY=OLD/'read-typed';BINARY_SHA='add01058f3b0d6efd28d936c865b1ba9071c0e2eb85f559743bf0c52aeea6371'
RED_PROOFS=R/'debt-copy-a-red-01'
RED_MANIFEST='373dccfa2a9550621fed8c89e0922ea3db56a9ff0666f7a0ed1c7528d398e687'
OLD_REFERENCES=['DEBT_PENDING.json','graph-before.json','fts-before.json','source-before.json','source-directories-before.json','typed-before-db.json','first-red.json','composed.json','writer.stdout']
PREFIX='docs/arbre-principal/16-11-2026/debt-copy-a/'
FILES={n:PREFIX+n for n in ['audit_result.py','qualify.py','audit.py','typed_rows.py']}
FILES.update({'inspect-fts.py':'docs/arbre-principal/16-11-2026/bascule/inspect-fts.py',
 'confiner_validation.py':'extension/rag3weaver/scripts/confiner_validation.py','poste':'extension/rag3weaver/scripts/poste'})

def save(name,data):q.save(P/name,data)
def red(error):
 try:save('first-red.json',dict(reason=str(error),observed=time.monotonic()))
 except FileExistsError:pass
def guard(cfg):
 if q.git(cfg['source_root'],'rev-parse','HEAD')!=cfg['pilot'] or q.git(cfg['source_root'],'status','--porcelain'):raise ValueError('source pilot not exact and clean')
 if set(cfg['files'])!=set(FILES):raise ValueError('controller set differs')
 for name,path in FILES.items():
  blob=subprocess.check_output(['git','-C',cfg['source_root'],'show',cfg['pilot']+':'+path])
  if q.sha(Q/name)!=cfg['files'][name] or hashlib.sha256(blob).hexdigest()!=cfg['files'][name]:raise ValueError('controller pin differs')
 for path,h in q.PINS.items():
  if q.sha(path)!=h:raise ValueError('SDK/tool pin differs')
 if q.sha(BINARY)!=BINARY_SHA or q.sha(cfg['timeout'])!=cfg['timeout_sha']:raise ValueError('reader/timeout pin differs')
 if q.sha(RED_PROOFS/'proof-manifest.json')!=RED_MANIFEST:raise ValueError('red evidence manifest differs')
 if set(cfg['old_proofs'])!=set(OLD_REFERENCES):raise ValueError('red reference set differs')
 references={row['name']:row['sha256'] for row in json.loads((RED_PROOFS/'proof-manifest.json').read_text())}
 for name,h in cfg['old_proofs'].items():
  if references.get('client/'+name)!=h or q.sha(OLD/name)!=h:raise ValueError('frozen reference proof differs')
def bind():
 # Reuse the reviewed RO Reader, FTS and framing primitives only.
 q.P=P;q.Q=Q;q.BASE=BASE
def stable_readers(before,after):
 """Only an absent, zero-byte SDK reader file may appear on the NEW copy."""
 previous={x['path']:x for x in before};current={x['path']:x for x in after}
 added=set(current)-set(previous)
 if added:
  allowed=BASE.name+'.readers'
  if added!={allowed} or current[allowed]!=dict(path=allowed,size=0,mode=420,sha256=hashlib.sha256(b'').hexdigest()):raise ValueError('unexpected new file, including WAL')
  current.pop(allowed)
 if current!=previous:raise ValueError('read-only copy changed existing bytes or modes')
 return dict(readers_zero_added=bool(added),existing_files_unchanged=True,new_wal=False)
def invariant_sources(fs):
 return dict(result_files=fs.file_inventory(RESULT),result_dirs=q.directories(RESULT),origin_files=fs.file_inventory(ORIGIN),origin_dirs=q.directories(ORIGIN))
def quiet_source():
 old=q.BASE
 try:q.BASE=RESULT/BASE.name;return q.copy_quiet()
 finally:q.BASE=old

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);bind();common=q.loadmod('confiner_validation');authority=common.autorite(common.processus());end=authority['echeance']
 recipe=json.loads((Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).parent/'recette.json').read_text())
 if recipe['delai']!=830:raise ValueError('common budget differs')
 cg=Path('/sys/fs/cgroup')/authority['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 quota,period=limits['cpu.max'].split()
 if limits['memory.max']!='34359738368' or limits['memory.high']!='max' or limits['memory.swap.max']!='0' or quota=='max' or not 0<int(quota)<=8*int(period):raise ValueError('resource contract differs')
 save('authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,limits=limits))
 fs=q.loadmod('inspect-fts');save('result-before-quiet.json',quiet_source());initial=invariant_sources(fs)
 if initial['origin_files']!=json.loads((OLD/'source-before.json').read_text()) or initial['origin_dirs']!=json.loads((OLD/'source-directories-before.json').read_text()):raise ValueError('COPY02 source changed')
 save('sources-before.json',initial)
 save('ledger-before-copy.json',dict(pilot=cfg['pilot'],reader_sha256=q.sha(BINARY),ids_sha256=q.sha(q.IDS),prepared_sha256=q.sha(Q/'prepared.json'),old_proofs=cfg['old_proofs'],original_shutdown_observed=False))
 for name in ['tmp','config','data','cache','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 env.update(LD_LIBRARY_PATH=str(q.LIB),TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=q.LIB.name,RAG3DB_LIBS_DIR=str(q.LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',RAYON_NUM_THREADS='4')
 save('profile-before-db.json',{k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'})
 q.require_time(end);cap=min(120,end-time.monotonic()-15);argv=['cp','-a','--reflink=always','--',str(RESULT),str(P/'copy')]
 save('copy-command.json',dict(argv=argv,cap=cap));result=None;timeout=None
 try:result=subprocess.run(argv,env=env,capture_output=True,timeout=cap)
 except subprocess.TimeoutExpired as error:timeout=error
 (P/'copy.stdout').write_bytes((timeout.stdout or b'') if timeout else result.stdout);(P/'copy.stderr').write_bytes((timeout.stderr or b'') if timeout else result.stderr)
 save('copy-process.json',dict(rc=None if timeout else result.returncode,timeout=timeout is not None))
 if timeout or result.returncode:raise ValueError('copy refused; outputs preserved')
 if fs.file_inventory(BASE.parent)!=initial['result_files'] or q.directories(BASE.parent)!=initial['result_dirs']:raise ValueError('full result copy not faithful')
 if invariant_sources(fs)!=initial:raise ValueError('source changed during cloning')
 save('copy-before-db.json',initial['result_files']);save('copy-before-quiet.json',q.copy_quiet())
 # No writable opening, recovery, LOAD, SourceNode or model call exists here.
 cohort=json.loads((OLD/'DEBT_PENDING.json').read_text())['ids']
 if len(cohort)!=469 or len(set(cohort))!=469:raise ValueError('cohort differs')
 reader=q.Reader(BINARY,end,env,cfg['timeout'])
 graph=audit.snapshot(reader,cohort);save('graph-observed.json',graph)
 if graph!=json.loads((OLD/'graph-before.json').read_text()):raise ValueError('graph/config/gen/visibility changed outside permitted cohort fields')
 fts=q.fts(reader,fs,env,end,graph['offsets'],'observed');save('fts-observed.json',fts)
 if fts!=json.loads((OLD/'fts-before.json').read_text()):raise ValueError('FTS IDs/gens/files changed')
 vectors=audit.finished(reader,cohort);save('vectors-observed.json',vectors)
 after=fs.file_inventory(BASE.parent);exception=stable_readers(initial['result_files'],after)
 if q.directories(BASE.parent)!=initial['result_dirs']:raise ValueError('copy directories changed')
 save('copy-after.json',after);save('copy-reader-exception.json',exception)
 if invariant_sources(fs)!=initial:raise ValueError('sources changed after read-only observation')
 save('sources-after.json',initial);guard(cfg);q.require_time(end,5)
 save('driver-result.json',dict(status='FROZEN_RESULT_RO_JUDGES_PASSED',debt=vectors['debt'],original_shutdown_observed=False,durability_qualified=False,live_authorized=False))

def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=q.loadmod('confiner_validation');parent=Path('/proc')/str(os.getppid())
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','898s',sys.executable,str(Q/'audit_result.py'),'--compose']
 if os.readlink(parent/'exe')!=cfg['timeout'] or [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]!=expected or not common.poste_tenu(common.processus()):raise ValueError('real outer/poste absent')
 P.mkdir(mode=0o700);bind();status=dict(status='RED',live_authorized=False)
 def stop(sig,frame):red('outer deadline');raise RuntimeError('outer deadline')
 signal.signal(signal.SIGTERM,stop)
 try:
  guard(cfg);rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','830','--preuves',str(S),'--',sys.executable,str(Q/'audit_result.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);authority=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text())
  if rc or scope['verdict']!='VERT' or scope['code'] or scope['restes'] or not scope['nettoye'] or authority['etat']!='termine' or Path(scope['prive']).exists():raise ValueError('scope not green quiet')
  owned=json.loads((path.parent/'enfants.json').read_text())
  for row in owned:
   try:
    fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[19]==row['debut'] and fields[0]!='Z':raise ValueError('owned process remains')
   except (FileNotFoundError,ProcessLookupError):pass
  if (P/'first-red.json').exists() or json.loads((P/'driver-result.json').read_text())['status']!='FROZEN_RESULT_RO_JUDGES_PASSED':raise ValueError('driver refused')
  save('copy-final-quiet.json',q.copy_quiet());save('result-final-quiet.json',quiet_source());fs=q.loadmod('inspect-fts')
  if invariant_sources(fs)!=json.loads((P/'sources-before.json').read_text()):raise ValueError('source changed during finalization')
  stable_readers(json.loads((P/'copy-before-db.json').read_text()),fs.file_inventory(BASE.parent))
  if q.directories(BASE.parent)!=json.loads((P/'sources-before.json').read_text())['result_dirs']:raise ValueError('copy directories changed at finalization')
  guard(cfg);q.require_time(authority['echeance'],0)
  status=dict(status='FROZEN_RESULT_RO_OBSERVED',scope=scope,deadline=authority['echeance'],original_copya_verdict='RED',original_shutdown_observed=False,durability_qualified=False,live_authorized=False,independent_review_pending=True)
 except BaseException as error:red(error);status=dict(status='RED',error=str(error),live_authorized=False)
 q.save(P/'composed.json',status,deadline=status.get('deadline'));return 0 if status['status']=='FROZEN_RESULT_RO_OBSERVED' else 1

def prepare(root,pilot):
 if os.uname().nodename.lower()!='luciepc' or q.git(root,'rev-parse','HEAD')!=pilot or q.git(root,'status','--porcelain'):raise ValueError('PC exact clean source required')
 if any(x.exists() for x in [P,Q,S]) or any(x.resolve()!=x for x in [P,Q,S,RESULT,ORIGIN]):raise ValueError('new canonical destinations required')
 Q.mkdir(mode=0o700)
 for name,path in FILES.items():shutil.copy2(Path(root)/path,Q/name)
 timeout=str(Path(shutil.which('timeout')).resolve())
 cfg=dict(source_root=str(root),pilot=pilot,files={n:q.sha(Q/n) for n in FILES},timeout=timeout,timeout_sha=q.sha(timeout),old_proofs={n:q.sha(OLD/n) for n in OLD_REFERENCES})
 q.save(Q/'prepared.json',cfg);guard(cfg);return cfg
def launch(pilot,allocation):
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg)
 if pilot!=cfg['pilot'] or not allocation or P.exists() or S.exists() or 'RAG3WEAVER_VALIDATION_RECU' in os.environ:raise ValueError('exact GO and fresh allocation required')
 q.save(Q/'allocation.json',dict(reference=allocation,read_only=True,writer_authorized=False,live_authorized=False))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 with (Q/'outer.log').open('x') as stream:return subprocess.run([str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','898s',sys.executable,str(Q/'audit_result.py'),'--compose'],cwd='/var/tmp',env=env,stdout=stream,stderr=subprocess.STDOUT).returncode

if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--prepare');ap.add_argument('--launch');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.prepare:
  cfg=prepare(Path(__file__).resolve().parents[4],args.prepare);print(json.dumps(dict(prepared=str(Q/'prepared.json'),sha256=q.sha(Q/'prepared.json'),database_opened=False)))
 elif args.launch:sys.exit(launch(args.launch,args.allocation))
 elif args.compose:sys.exit(compose())
 elif args.driver:
  try:driver()
  except BaseException as error:red(error);raise
 else:ap.error('explicit mode required')
