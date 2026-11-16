#!/usr/bin/env python3
"""Proposed continuation: classify saved graph, then RO FTS/vectors; original END."""
from pathlib import Path
import argparse,hashlib,json,math,os,shutil,signal,subprocess,sys,time
import audit_result as a
q=a.q;R=a.R;PRIOR=R/'debt-audit-ro-result-v1'
P=R/'debt-audit-ro-continuation-v1';Q=R/'debt-audit-ro-continuation-control-v1';S=R/'debt-audit-ro-continuation-scopes-v1'
END=1190939.194661205
REFERENCES=['graph-observed.json','copy-before-db.json','sources-before.json','first-red.json','post-red-invariants.json']
MANIFEST=R/'debt-audit-ro-red-01/proof-manifest.json';MANIFEST_SHA='e22f56a13b0eeef837d68f3a988c1dc2d98335ee49f3cae81a4373e85940e5eb'
a.FILES=dict(a.FILES,**{'continue_result.py':a.PREFIX+'continue_result.py'})
a.P=P;a.Q=Q;a.S=S;a.BASE=PRIOR/'copy/code-principal.rag3db';q.Q=Q
def save(name,value):q.save(P/name,value)
def guard(cfg):
 a.guard(cfg)
 if q.sha(MANIFEST)!=MANIFEST_SHA or set(cfg['audit_refs'])!=set(REFERENCES):raise ValueError('prior audit reference set/manifest differs')
 entries={x['name']:x['sha256'] for x in json.loads(MANIFEST.read_text())}
 for name,h in cfg['audit_refs'].items():
  if entries.get('client/'+name)!=h or q.sha(PRIOR/name)!=h:raise ValueError('prior audit evidence changed')
def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);a.bind();common=q.loadmod('confiner_validation');authority=common.autorite(common.processus());end=authority['echeance'];receipt=json.loads((Q/'after-poste.json').read_text())
 if end>END or json.loads((Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).parent/'recette.json').read_text())['delai']!=receipt['common']:raise ValueError('receipt exceeds original deadline')
 cg=Path('/sys/fs/cgroup')/authority['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']};quota,period=limits['cpu.max'].split()
 if limits['memory.max']!='34359738368' or limits['memory.high']!='max' or limits['memory.swap.max']!='0' or quota=='max' or not 0<int(quota)<=8*int(period):raise ValueError('resource contract differs')
 save('authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,original_deadline=END,limits=limits))
 fs=q.loadmod('inspect-fts');initial=a.invariant_sources(fs);expected=json.loads((PRIOR/'sources-before.json').read_text())
 if initial!=expected:raise ValueError('source bundles changed since prior audit')
 baseline=fs.file_inventory(a.BASE.parent);a.stable_readers(json.loads((PRIOR/'copy-before-db.json').read_text()),baseline);dirs=q.directories(a.BASE.parent)
 if dirs!=expected['result_dirs']:raise ValueError('frozen audit copy directories changed')
 save('copy-before-quiet.json',q.copy_quiet());save('result-before-quiet.json',a.quiet_source());save('sources-before.json',initial);save('copy-before.json',baseline)
 graph=json.loads((PRIOR/'graph-observed.json').read_text());reference=json.loads((a.OLD/'graph-before.json').read_text())
 save('graph-publications.json',dict(observed=a.published_graph(graph),reference=a.published_graph(reference)))
 if not a.same_graph_publication(graph,reference):raise ValueError('saved graph publication differs')
 save('saved-graph-qualified.json',dict(equal=True,scope='prior saved graph only',original_audit_verdict='RED',no_snapshot_query_replayed=True))
 for name in ['tmp','config','data','cache','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 env.update(LD_LIBRARY_PATH=str(q.LIB),TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=q.LIB.name,RAG3DB_LIBS_DIR=str(q.LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',RAYON_NUM_THREADS='4')
 save('ledger-before-db.json',dict(prepared_sha256=q.sha(Q/'prepared.json'),reader_sha256=q.sha(a.BINARY),ids_sha256=q.sha(q.IDS),prior_manifest=MANIFEST_SHA,profile={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'}))
 reader=q.Reader(a.BINARY,end,env,cfg['timeout']);fts=q.fts(reader,fs,env,end,graph['offsets'],'observed');save('fts-observed.json',fts)
 if fts!=json.loads((a.OLD/'fts-before.json').read_text()):raise ValueError('FTS IDs/gens/files changed')
 cohort=json.loads((a.OLD/'DEBT_PENDING.json').read_text())['ids']
 if len(cohort)!=469 or len(set(cohort))!=469:raise ValueError('cohort changed')
 vectors=a.audit.finished(reader,cohort);save('vectors-observed.json',vectors)
 if fs.file_inventory(a.BASE.parent)!=baseline or q.directories(a.BASE.parent)!=dirs or a.invariant_sources(fs)!=initial:raise ValueError('RO source/copy changed')
 save('sources-after.json',initial);guard(cfg);q.require_time(end,5)
 save('driver-result.json',dict(status='REMAINING_RO_JUDGES_PASSED',original_shutdown_observed=False,durability_qualified=False,live_authorized=False))
def compose():
 cfg=json.loads((Q/'prepared.json').read_text());receipt=json.loads((Q/'after-poste.json').read_text());common=q.loadmod('confiner_validation');parent=Path('/proc')/str(os.getppid())
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s',str(receipt['soft'])+'s',sys.executable,str(Q/'continue_result.py'),'--compose']
 if os.readlink(parent/'exe')!=cfg['timeout'] or [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]!=expected or not common.poste_tenu(common.processus()):raise ValueError('outer/poste absent')
 P.mkdir(mode=0o700);a.bind();status=dict(status='RED')
 def stop(sig,frame):a.red('outer deadline');raise RuntimeError('outer stopped continuation')
 signal.signal(signal.SIGTERM,stop)
 try:
  guard(cfg);q.require_time(END,0);rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai',str(receipt['common']),'--preuves',str(S),'--',sys.executable,str(Q/'continue_result.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text())
  if rc or scope['verdict']!='VERT' or scope['code'] or scope['restes'] or not scope['nettoye'] or auth['etat']!='termine' or Path(scope['prive']).exists():raise ValueError('scope not green quiet')
  for row in json.loads((path.parent/'enfants.json').read_text()):
   try:
    fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[19]==row['debut'] and fields[0]!='Z':raise ValueError('owned process remains')
   except (FileNotFoundError,ProcessLookupError):pass
  if (P/'first-red.json').exists() or json.loads((P/'driver-result.json').read_text())['status']!='REMAINING_RO_JUDGES_PASSED':raise ValueError('driver refused')
  fs=q.loadmod('inspect-fts');before=json.loads((P/'sources-before.json').read_text())
  if a.invariant_sources(fs)!=before or fs.file_inventory(a.BASE.parent)!=json.loads((P/'copy-before.json').read_text()) or q.directories(a.BASE.parent)!=before['result_dirs']:raise ValueError('sources changed during finalization')
  save('copy-final-quiet.json',q.copy_quiet());save('result-final-quiet.json',a.quiet_source());guard(cfg)
  deadline=min(END,auth['echeance']);q.require_time(deadline,0)
  status=dict(status='FROZEN_RESULT_RO_OBSERVED',deadline=deadline,original_deadline=END,scope=scope,original_copya_verdict='RED',original_audit_verdict='RED',original_shutdown_observed=False,durability_qualified=False,live_authorized=False,independent_review_pending=True)
 except BaseException as error:a.red(error);status=dict(status='RED',error=str(error),live_authorized=False)
 q.save(P/'composed.json',status,deadline=status.get('deadline'));return 0 if status['status']=='FROZEN_RESULT_RO_OBSERVED' else 1
def prepare(root,pilot):
 if os.uname().nodename.lower()!='luciepc' or q.git(root,'rev-parse','HEAD')!=pilot or q.git(root,'status','--porcelain') or any(x.exists() for x in [P,Q,S]):raise ValueError('exact clean PC/new P-Q-S required')
 if any(x.resolve()!=x for x in [P,Q,S,a.BASE.parent]):raise ValueError('canonical paths required')
 Q.mkdir(mode=0o700)
 for name,path in a.FILES.items():shutil.copy2(root/path,Q/name)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(source_root=str(root),pilot=pilot,files={n:q.sha(Q/n) for n in a.FILES},timeout=timeout,timeout_sha=q.sha(timeout),old_proofs={n:q.sha(a.OLD/n) for n in a.OLD_REFERENCES},audit_refs={n:q.sha(PRIOR/n) for n in REFERENCES})
 q.save(Q/'prepared.json',cfg);guard(cfg);print(json.dumps(dict(prepared_sha256=q.sha(Q/'prepared.json'),database_opened=False)))
def post():
 cfg=json.loads((Q/'prepared.json').read_text());common=q.loadmod('confiner_validation')
 if not common.poste_tenu(common.processus()):raise ValueError('real poste absent')
 remaining=math.floor(END-time.monotonic())-3
 if remaining<=60:raise ValueError('insufficient original allocation remaining')
 q.save(Q/'after-poste.json',dict(observed=time.monotonic(),original_deadline=END,remaining=remaining,soft=remaining-2,common=remaining-20))
 os.execv(cfg['timeout'],[cfg['timeout'],'--signal=TERM','--kill-after=2s',str(remaining-2)+'s',sys.executable,str(Q/'continue_result.py'),'--compose'])
def launch(pilot,allocation):
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg)
 if pilot!=cfg['pilot'] or not allocation or P.exists() or S.exists() or 'RAG3WEAVER_VALIDATION_RECU' in os.environ:raise ValueError('exact new allocation required')
 q.require_time(END,60);q.save(Q/'allocation.json',dict(reference=allocation,original_deadline=END,read_only=True))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 with (Q/'outer.log').open('x') as out:return subprocess.run([str(Q/'poste'),'lourd',sys.executable,str(Q/'continue_result.py'),'--post'],env=env,cwd='/var/tmp',stdout=out,stderr=subprocess.STDOUT).returncode
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--prepare');ap.add_argument('--launch');ap.add_argument('--allocation');ap.add_argument('--post',action='store_true');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.prepare:prepare(Path(__file__).resolve().parents[4],args.prepare)
 elif args.launch:sys.exit(launch(args.launch,args.allocation))
 elif args.post:post()
 elif args.compose:sys.exit(compose())
 elif args.driver:
  try:driver()
  except BaseException as error:a.red(error);raise
 else:ap.error('explicit mode required')
