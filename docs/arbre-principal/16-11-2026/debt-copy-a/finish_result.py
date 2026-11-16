#!/usr/bin/env python3
"""Proposed FIN-RO600, a distinct allocation; never calls a writer/provider."""
from pathlib import Path
import argparse,hashlib,json,os,shutil,signal,subprocess,sys,time
import continue_result as carried
a=carried.a;q=a.q;R=a.R
P=R/'debt-fin-ro-result-v1';Q=R/'debt-fin-ro-control-v1';S=R/'debt-fin-ro-scopes-v1'
FROZEN=carried.PRIOR/'copy'
BASE=P/'copy/code-principal.rag3db'
OLD_Q=R/'debt-audit-ro-continuation-control-v1'
OLD_PREPARED='3ba4c7d1df85c0e7d15d351738f8d5a20314f41cae80a6823465e5494bb72ee9'
OLD_PILOT='cc18bc7d1dd513c71407f07e887134871af0576e'
HISTORY=dict(audit_deadline=1190939.194661205,continuation_status='EXPIRED_WITHOUT_LAUNCH',old_prepared_sha256=OLD_PREPARED,old_pilot=OLD_PILOT)
# Reuse admission, typed Reader, immutable FTS and vector judges, not old drivers.
a.FILES=dict(a.FILES,**{'finish_result.py':a.PREFIX+'finish_result.py'})
carried.Q=Q;a.Q=Q;a.P=P;a.S=S;a.BASE=BASE;q.Q=Q
def save(name,value):q.save(P/name,value)
def carry(cfg):
 if q.sha(OLD_Q/'prepared.json')!=OLD_PREPARED:raise ValueError('expired preparation differs')
 old=json.loads((OLD_Q/'prepared.json').read_text())
 if old['pilot']!=OLD_PILOT or len(old['files'])!=8:raise ValueError('expired pilot/source set differs')
 for name,h in old['files'].items():
  blob=subprocess.check_output(['git','-C',cfg['source_root'],'show',OLD_PILOT+':'+a.FILES[name]])
  if q.sha(OLD_Q/name)!=h or hashlib.sha256(blob).hexdigest()!=h:raise ValueError('expired source copy/Git differs')
 expired=json.loads((OLD_Q/'EXPIRED_WITHOUT_LAUNCH.json').read_text())
 if expired['status']!='EXPIRED_WITHOUT_LAUNCH' or not expired['expired'] or expired['original_deadline']!=HISTORY['audit_deadline'] or expired['p_exists'] or expired['s_exists'] or expired['allocation_exists']:raise ValueError('old authority history differs')
 if (OLD_Q/'allocation.json').exists() or carried.P.exists() or carried.S.exists():raise ValueError('expired continuation was launched')
 return dict(history=HISTORY,copies=old['files'],expired=expired)
def guard(cfg):
 carried.guard(cfg)
 if carry(cfg)!=cfg['carry']:raise ValueError('expired carry changed')
def sources(fs):
 return dict(originals=a.invariant_sources(fs),frozen_files=fs.file_inventory(FROZEN),frozen_dirs=q.directories(FROZEN))
def quiet_frozen():
 previous=q.BASE
 try:q.BASE=FROZEN/BASE.name;return q.copy_quiet()
 finally:q.BASE=previous

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);a.bind();common=q.loadmod('confiner_validation');authority=common.autorite(common.processus());end=authority['echeance']
 if json.loads((Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).parent/'recette.json').read_text())['delai']!=530:raise ValueError('FIN common budget differs')
 cg=Path('/sys/fs/cgroup')/authority['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']};quota,period=limits['cpu.max'].split()
 if limits['memory.max']!='34359738368' or limits['memory.high']!='max' or limits['memory.swap.max']!='0' or quota=='max' or not 0<int(quota)<=8*int(period):raise ValueError('FIN resource contract differs')
 save('authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,limits=limits,history=HISTORY))
 fs=q.loadmod('inspect-fts');save('frozen-before-quiet.json',quiet_frozen());save('result-before-quiet.json',a.quiet_source());initial=sources(fs)
 if initial['originals']!=json.loads((carried.PRIOR/'sources-before.json').read_text()):raise ValueError('original COPYA/COPY02 bundles changed')
 a.stable_readers(json.loads((carried.PRIOR/'copy-before-db.json').read_text()),initial['frozen_files'])
 if initial['frozen_dirs']!=initial['originals']['result_dirs']:raise ValueError('frozen audit directories changed')
 save('sources-before.json',initial);save('history.json',cfg['carry'])
 for name in ['tmp','config','data','cache','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 env.update(LD_LIBRARY_PATH=str(q.LIB),TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=q.LIB.name,RAG3DB_LIBS_DIR=str(q.LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',RAYON_NUM_THREADS='4')
 save('ledger-before-copy.json',dict(pilot=cfg['pilot'],prepared_sha256=q.sha(Q/'prepared.json'),reader_sha256=q.sha(a.BINARY),ids_sha256=q.sha(q.IDS),profile={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'},history=HISTORY))
 q.require_time(end);cap=min(120,end-time.monotonic()-15);argv=['cp','-a','--reflink=always','--',str(FROZEN),str(P/'copy')]
 save('copy-command.json',dict(argv=argv,cap=cap));result=None;timeout=None
 try:result=subprocess.run(argv,env=env,capture_output=True,timeout=cap)
 except subprocess.TimeoutExpired as error:timeout=error
 (P/'copy.stdout').write_bytes((timeout.stdout or b'') if timeout else result.stdout);(P/'copy.stderr').write_bytes((timeout.stderr or b'') if timeout else result.stderr)
 save('copy-process.json',dict(rc=None if timeout else result.returncode,timeout=timeout is not None))
 if timeout or result.returncode:raise ValueError('FIN clone refused; outputs preserved')
 if fs.file_inventory(BASE.parent)!=initial['frozen_files'] or q.directories(BASE.parent)!=initial['frozen_dirs'] or sources(fs)!=initial:raise ValueError('FIN clone not faithful or sources changed')
 save('copy-before-db.json',initial['frozen_files']);save('copy-before-quiet.json',q.copy_quiet())
 graph=json.loads((carried.PRIOR/'graph-observed.json').read_text());reference=json.loads((a.OLD/'graph-before.json').read_text())
 save('graph-publications.json',dict(observed=a.published_graph(graph),reference=a.published_graph(reference)))
 if not a.same_graph_publication(graph,reference):raise ValueError('saved graph publication differs')
 save('saved-graph-qualified.json',dict(equal=True,no_snapshot_query_replayed=True,original_audit_verdict='RED'))
 reader=q.Reader(a.BINARY,end,env,cfg['timeout']);fts=q.fts(reader,fs,env,end,graph['offsets'],'observed');save('fts-observed.json',fts)
 if fts!=json.loads((a.OLD/'fts-before.json').read_text()):raise ValueError('FTS IDs/gens/files changed')
 cohort=json.loads((a.OLD/'DEBT_PENDING.json').read_text())['ids']
 if len(cohort)!=469 or len(set(cohort))!=469:raise ValueError('cohort changed')
 vectors=a.audit.finished(reader,cohort);save('vectors-observed.json',vectors)
 after=fs.file_inventory(BASE.parent);exception=a.stable_readers(initial['frozen_files'],after)
 if q.directories(BASE.parent)!=initial['frozen_dirs'] or sources(fs)!=initial:raise ValueError('RO copy directories or sources changed')
 save('copy-after.json',after);save('reader-exception.json',exception);save('sources-after.json',initial);guard(cfg);q.require_time(end,5)
 save('driver-result.json',dict(status='FIN_RO_JUDGES_PASSED',history=HISTORY,original_shutdown_observed=False,durability_qualified=False,live_authorized=False))

def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=q.loadmod('confiner_validation');parent=Path('/proc')/str(os.getppid());expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','598s',sys.executable,str(Q/'finish_result.py'),'--compose']
 if os.readlink(parent/'exe')!=cfg['timeout'] or [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]!=expected or not common.poste_tenu(common.processus()):raise ValueError('real FIN outer/poste absent')
 P.mkdir(mode=0o700);a.bind();status=dict(status='RED')
 def stop(sig,frame):a.red('FIN outer deadline');raise RuntimeError('FIN outer stopped')
 signal.signal(signal.SIGTERM,stop)
 try:
  guard(cfg);rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','530','--preuves',str(S),'--',sys.executable,str(Q/'finish_result.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text())
  if rc or scope['verdict']!='VERT' or scope['code'] or scope['restes'] or not scope['nettoye'] or auth['etat']!='termine' or Path(scope['prive']).exists():raise ValueError('FIN scope not green quiet')
  for row in json.loads((path.parent/'enfants.json').read_text()):
   try:
    fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[19]==row['debut'] and fields[0]!='Z':raise ValueError('FIN owned process remains')
   except (FileNotFoundError,ProcessLookupError):pass
  if (P/'first-red.json').exists() or json.loads((P/'driver-result.json').read_text())['status']!='FIN_RO_JUDGES_PASSED':raise ValueError('FIN driver refused')
  fs=q.loadmod('inspect-fts');before=json.loads((P/'sources-before.json').read_text())
  if sources(fs)!=before or q.directories(BASE.parent)!=before['frozen_dirs']:raise ValueError('FIN sources or clone directories changed')
  a.stable_readers(json.loads((P/'copy-before-db.json').read_text()),fs.file_inventory(BASE.parent))
  save('copy-final-quiet.json',q.copy_quiet());save('frozen-final-quiet.json',quiet_frozen());save('result-final-quiet.json',a.quiet_source());guard(cfg);q.require_time(auth['echeance'],0)
  status=dict(status='FIN_RO_OBSERVED',deadline=auth['echeance'],scope=scope,history=HISTORY,original_copya_verdict='RED',original_audit_verdict='RED',original_shutdown_observed=False,durability_qualified=False,live_authorized=False,independent_review_pending=True)
 except BaseException as error:a.red(error);status=dict(status='RED',error=str(error),history=HISTORY,live_authorized=False)
 q.save(P/'composed.json',status,deadline=status.get('deadline'));return 0 if status['status']=='FIN_RO_OBSERVED' else 1
def prepare(root,pilot):
 if os.uname().nodename.lower()!='luciepc' or q.git(root,'rev-parse','HEAD')!=pilot or q.git(root,'status','--porcelain') or any(x.exists() for x in [P,Q,S]):raise ValueError('exact clean FIN source/new P-Q-S required')
 if any(x.resolve()!=x for x in [P,Q,S,FROZEN]):raise ValueError('canonical paths required')
 Q.mkdir(mode=0o700)
 for name,path in a.FILES.items():shutil.copy2(root/path,Q/name)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(source_root=str(root),pilot=pilot,files={n:q.sha(Q/n) for n in a.FILES},timeout=timeout,timeout_sha=q.sha(timeout),old_proofs={n:q.sha(a.OLD/n) for n in a.OLD_REFERENCES},audit_refs={n:q.sha(carried.PRIOR/n) for n in carried.REFERENCES})
 cfg['carry']=carry(cfg);q.save(Q/'prepared.json',cfg);guard(cfg);print(json.dumps(dict(prepared_sha256=q.sha(Q/'prepared.json'),database_opened=False)))
def launch(pilot,allocation):
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg)
 if pilot!=cfg['pilot'] or not allocation or P.exists() or S.exists() or 'RAG3WEAVER_VALIDATION_RECU' in os.environ:raise ValueError('exact fresh FIN allocation required')
 q.save(Q/'allocation.json',dict(reference=allocation,distinct_budget=600,history=HISTORY,read_only=True))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 with (Q/'outer.log').open('x') as out:return subprocess.run([str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','598s',sys.executable,str(Q/'finish_result.py'),'--compose'],env=env,cwd='/var/tmp',stdout=out,stderr=subprocess.STDOUT).returncode
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--prepare');ap.add_argument('--launch');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.prepare:prepare(Path(__file__).resolve().parents[4],args.prepare)
 elif args.launch:sys.exit(launch(args.launch,args.allocation))
 elif args.compose:sys.exit(compose())
 elif args.driver:
  try:driver()
  except BaseException as error:a.red(error);raise
 else:ap.error('explicit mode required')
