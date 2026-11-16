#!/usr/bin/env python3
"""Proposed default canonical pass. Exact C7 GO plus a fresh Embar receipt required."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,math,os,re,shutil,signal,socket,stat,subprocess,sys,time,urllib.request
R=Path.home()/'.cache/rag3weaver-build';Q=R/'cx-control-recherche-fts-generation-global-v1';P=R/'cx-proofs-recherche-fts-generation-global-v1';S=R/'cx-scope-recherche-fts-generation-global-v1'
APP='5f59dd88a2888199c5ad934f72f0debed4ba2a49';BASE='b0a845bb4877328eada37951c174e3d208986630'
W=Path.home()/'git_workspaces/rag3db-cx-recherche-fts-generation-union-r1';OLD=R/'cx-wt-recherche-fts-generation-guard-source-v1';OLD_SHA='bc9526408ea3a48320b4a27e7644075dcf7307b0';T=R/'cx-target-recherche-fts-counts-freshness'
ARCHIVE=R/'cx-archive-recherche-fts-generation-guard-v1';MANIFEST='57fbd0280d3c46452366eb546ad3d7dedac484051e60ea859b59b2e60e9deb3c'
PIN='2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
LIB=R.parent/'rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
WITNESS='code_sync::fts_guard_double_tests::generations_sync_removes_a_committed_fts_ghost'
REQUIRED=['e2e_code_sync','e2e_lectures','e2e_code','e2e_contrat_transactions','e2e_tx_ligne_a_ligne','e2e_tx_merge_puis_copy','e2e_tx_par_paquet_arret','e2e_tx_vecteurs_apres_rollback']
FAILED=re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.',re.M)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(root,name,obj):
 with (root/name).open('x') as f:json.dump(obj,f,indent=2)
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def module(name):
 spec=importlib.util.spec_from_file_location(name,Q/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
def first_red(error):
 try:save(P,'first-red.json',dict(reason=repr(error),observed=time.monotonic()))
 except FileExistsError:pass
def canonical_overrides():
 return dict(CIBLE=T.name,SUITE_DU_TEMOIN='lib',SUITES_EN_PLUS=' '.join(REQUIRED[3:]),RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_TARGET_MAX_GO='32',RAG3DB_LIB=LIB.name,RAG3DB_LIBS_DIR=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',CARGO_BUILD_JOBS='8',CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',RUST_TEST_THREADS='8',RUST_BACKTRACE='1')
def validate_services(grant,now,end):
 assert grant.get('c7_reference') and grant.get('embar_reference') and grant.get('restart_authorized') is False
 for key in ['observed_monotonic','valid_until_monotonic']:assert type(grant.get(key)) in [int,float] and math.isfinite(grant[key])
 assert 0<=now-grant['observed_monotonic']<=120 and grant['valid_until_monotonic']>=end and now<end
 providers=grant['providers'];assert len(providers)==2
 expected={'127.0.0.1:7878':('granite-278m',768),'127.0.0.1:7879':('bge-m3',1024)}
 assert {row['address'] for row in providers}==set(expected)
 for row in providers:
  identity=row['identity'];assert set(identity)=={'service','modele','dim','factice','dual','sparse','precision','executable','max_chars_per_text'}
  assert (identity['modele'],identity['dim'])==expected[row['address']] and identity['service']=='rag3weaver-embeddings' and identity['factice'] is False
  assert type(identity['dual']) is bool and type(identity['sparse']) is bool and type(identity['max_chars_per_text']) is int and identity['max_chars_per_text']>0 and identity['precision'] and identity['executable']
  assert type(row['pid']) is int and row['pid']>0 and re.fullmatch('[0-9]+',row['start']) and re.fullmatch('[0-9a-f]{64}',row['exe_sha256'])
 pg=grant['postgres'];assert pg['host']=='127.0.0.1' and pg['port']==5433 and pg['database']=='rag3weaver_test' and pg['ready'] is True
 return grant

def cache_contract():
 assert T.resolve()==T and T.is_dir() and T.stat().st_uid==os.getuid();marker=T/'.arbre-du-target';m=marker.lstat()
 assert stat.S_ISREG(m.st_mode) and m.st_uid==os.getuid() and marker.read_text().strip()==str(OLD/'extension/rag3weaver')
 assert git(OLD,'rev-parse','HEAD')==OLD_SHA and not git(OLD,'status','--porcelain') and sha(ARCHIVE/'manifest.json')==MANIFEST
 manifest=json.loads((ARCHIVE/'manifest.json').read_text());selected={}
 for name in ['proofs/ledger.json','proofs/unit-tests','proofs/composed.json','scope/validation-i6edjos5/autorite.json','scope/validation-i6edjos5/verdict.json']:
  h=sha(ARCHIVE/name);assert h==manifest[name]['sha256'];selected[name]=h
 live=R/'cx-scope-recherche-fts-generation-guard-v1/validation-i6edjos5'
 for name in ['autorite.json','verdict.json']:assert sha(live/name)==selected['scope/validation-i6edjos5/'+name]
 auth=json.loads((live/'autorite.json').read_text());scope=json.loads((live/'verdict.json').read_text());assert auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 ledger=json.loads((ARCHIVE/'proofs/ledger.json').read_text());assert ledger['app']==OLD_SHA and ledger['elf_sha256']=='b23414d3742f7d8f1b2a230d72b2d91df187926375c1389f90b05ad277458c19'
 for key in ['source_executable','copied_executable']:assert sha(ledger[key])==ledger['elf_sha256']
 return dict(marker=dict(dev=m.st_dev,ino=m.st_ino,mode=m.st_mode,uid=m.st_uid,value=marker.read_text()),selected=selected,old=OLD_SHA,authority=auth,scope=scope)
def guard(cfg):
 for root,value in [(W,APP),(Path(cfg['control']),cfg['pilot'])]:assert git(root,'rev-parse','HEAD')==value and not git(root,'status','--porcelain')
 assert git(W,'merge-base','HEAD','origin/master')==BASE,'master changed; review before any new pass'
 assert (T/'.arbre-du-target').read_text().strip() in [str(OLD/'extension/rag3weaver'),str(W/'extension/rag3weaver')]
 for n,h in cfg['copies'].items():assert sha(Q/n)==h
 assert sha(cfg['timeout'])==cfg['timeout_sha256'] and sha(R/'passe_epinglee.sh')==PIN and sha(Q/'passe_epinglee.sh')==PIN
 for n,h in SO.items():assert sha(LIB/n)==h
 assert sha(W/'tools/poste/required_suites.txt')==cfg['required_table_sha256']

def provider_stable(row,hash_exe=False):
 p=Path('/proc')/str(row['pid']);fields=(p/'stat').read_text().rsplit(') ',1)[1].split()
 assert fields[19]==row['start'] and fields[0] not in ['Z','T']
 if hash_exe:assert sha(p/'exe')==row['exe_sha256']
def health(grant,label,end):
 assert time.monotonic()<end-15;result=[]
 for row in grant['providers']:
  provider_stable(row,True)
  with urllib.request.urlopen('http://'+row['address']+'/sante',timeout=min(5,end-time.monotonic()-15)) as response:
   assert response.status==200;raw=response.read(1048577)
  assert len(raw)<=1048576;data=json.loads(raw);save(P,label+'-'+row['address'].rsplit(':',1)[1]+'.json',data)
  for key,value in row['identity'].items():assert data.get(key)==value
  assert data.get('usage')=={'baux':0,'en_cours':0};provider_stable(row,True);result.append(data)
 return result

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);common=module('confiner_validation');auth=common.autorite(common.processus());end=auth['echeance'];receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
 assert json.loads((receipt.parent/'recette.json').read_text())['delai']==5900
 cg=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits=={'memory.max':'34359738368','memory.high':'max','memory.swap.max':'0','cpu.max':'500000 100000'}
 save(P,'authority-path.json',dict(path=str(receipt),deadline=end,initial=end-5900,limits=limits))
 base=module('base_controller');base.OLD=OLD
 contract=cache_contract();save(P,'cache-after-poste.json',contract);assert contract==cfg['cache_contract'];save(P,'cache-quiet-after-poste.json',base.cache_quiet({'source_root':str(W)}))
 C=W/'extension/rag3weaver';env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for n in ['tmp','config','data','cache','xdg-state','state-fixture']:(P/n).mkdir(mode=0o700)
 env.update(canonical_overrides());env.update(TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'))
 def run_guard(label,owner,resume):
  deadline=min(end-15,time.monotonic()+120);assert deadline>time.monotonic();phase_env=dict(env,RAG3WEAVER_TARGET_REPRENDRE=resume)
  argv=['bash','-c','. "$1"; garder_le_target "$2" "$3"','target-guard',str(Q/'garde_target.sh'),str(T),str(owner)]
  save(P,label+'.command.json',dict(argv=argv,deadline=deadline))
  with (P/(label+'.log')).open('x') as out:result=subprocess.run(argv,cwd=owner,env=phase_env,stdout=out,stderr=subprocess.STDOUT,timeout=deadline-time.monotonic())
  save(P,label+'.result.json',dict(rc=result.returncode,finished=time.monotonic()));assert result.returncode==0
 run_guard('old-owner',OLD/'extension/rag3weaver','0');run_guard('adopt-union',C,'1');assert (T/'.arbre-du-target').read_text().strip()==str(C)
 required=subprocess.check_output(['bash',str(W/'tools/poste/required_suites.sh'),BASE,APP],cwd=W,text=True);assert required=='e2e_code_sync extension/rag3weaver/src/catalog.rs\n'
 save(P,'required-suites.json',dict(base=BASE,app=APP,result=required,table_sha256=cfg['required_table_sha256']))
 save(P,'READY_FOR_SERVICES.json',dict(observed=time.monotonic(),deadline=end,app=APP,pilot=cfg['pilot'],scope=str(receipt.parent),requested=['127.0.0.1:7878','127.0.0.1:7879','127.0.0.1:5433/rag3weaver_test'],receipt=str(Q/'SERVICES_GO.json'),service_operations_authorized=False))
 wait_end=min(end-60,time.monotonic()+120)
 while not (Q/'SERVICES_GO.json').exists():
  assert time.monotonic()<wait_end,'no fresh actual Embar receipt received';time.sleep(.2)
 grant_path=Q/'SERVICES_GO.json';m=grant_path.lstat();assert not grant_path.is_symlink() and stat.S_ISREG(m.st_mode) and m.st_uid==os.getuid() and m.st_size<=65536
 grant=validate_services(json.loads(grant_path.read_text()),time.monotonic(),end);save(P,'services-receipt.json',grant);health(grant,'services-before',end)
 # Only protocol availability is observed here; canonical SQL tests own their assertions.
 with socket.create_connection(('127.0.0.1',5433),timeout=min(5,end-time.monotonic()-15)) as pg:pg.sendall(bytes.fromhex('0000000804d2162f'));answer=pg.recv(1)
 assert answer in [b'N',b'S'];save(P,'postgres-protocol.json',dict(response=answer.decode(),sql_executed=False,identity_claim='fresh Embar receipt, not independently inferred from protocol'))
 guard(cfg);official=P/'official';official.mkdir();argv=['bash',str(Q/'passe_epinglee.sh'),W.name,str(official.relative_to(R)),WITNESS]
 save(P,'canonical-ledger.json',dict(app=APP,pilot=cfg['pilot'],base=BASE,argv=argv,script_sha256=PIN,cache_owner=str(C),services_receipt_sha256=sha(P/'services-receipt.json'),expected_before_script_environment={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'},required_suites=required,sdk=SO,compiled_elf_attested_before_tests=False))
 save(P,'runtime-marker.json',dict(observed=time.monotonic(),ledger_sha256=sha(P/'canonical-ledger.json')))
 with (P/'canonical.log').open('x') as out:
  child=subprocess.Popen(argv,cwd=C,env=env,stdout=out,stderr=subprocess.STDOUT);last_pid_scan=0
  try:
   while True:
    assert time.monotonic()<end-30 and time.monotonic()<grant['valid_until_monotonic']
    for name in ['temoins',*REQUIRED,'check_features','preuve','lib','navigateur','resume']:
     log=official/(name+'.log')
     if not log.exists():continue
     with log.open('rb') as stream:before=os.fstat(stream.fileno());raw=stream.read();after=os.fstat(stream.fileno())
     text=raw.decode(errors='replace');match=FAILED.search(text)
     if match is None and name in ['check_features','resume']:match=re.search(r'^(?:CHECK_FEATURES )?ROUGE\b.*$',text,re.M)
     if match:
      save(P,'watcher-first-match.json',dict(path=str(log),observed=time.monotonic(),dev=before.st_dev,inode=before.st_ino,bytes_read=len(raw),size_after=after.st_size,sha256=hashlib.sha256(raw).hexdigest(),matched=match.group()));raise RuntimeError('first failure in active log '+name)
    if time.monotonic()-last_pid_scan>=1:
     for row in grant['providers']:provider_stable(row)
     last_pid_scan=time.monotonic()
    if child.poll() is not None:break
    time.sleep(.1)
   rc=child.wait();save(P,'canonical-process.json',dict(rc=rc,finished=time.monotonic()));assert rc==0
  except BaseException as error:
   first_red(error)
   if child.poll() is None:child.terminate()
   raise
 summary=(official/'resume.log').read_text();assert summary.splitlines()[-1]=='VERT' and (official/'base').read_text().strip()==BASE
 assert 'suites réduites' not in summary and all('== '+name in summary for name in REQUIRED)
 for name in ['temoins',*REQUIRED,'lib','navigateur']:assert re.search(r'test result: ok\.',(official/(name+'.log')).read_text(errors='replace'))
 assert (official/'check_features.log').read_text().splitlines()[-1]=='CHECK_FEATURES VERT'
 assert 'preuve : rag3weaver revérifié' in summary and '== témoins navigateur' in summary
 health(grant,'services-after',end);guard(cfg);assert time.monotonic()<end-10
 save(P,'driver-result.json',dict(status='CANONICAL_PASSED',app=APP,deadline=end,services_unchanged=True,canonical_summary=str(official/'resume.log'),baseline_old_master_qualified=False))
def outer_command(cfg):return [str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','5998s',sys.executable,str(Q/'qualify.py'),'--compose']
def run_outer(cfg,env):
 with (Q/'launch.log').open('x') as out:result=subprocess.run(outer_command(cfg),cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode

def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=module('confiner_validation');parent=Path('/proc')/str(os.getppid())
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','5998s',sys.executable,str(Q/'qualify.py'),'--compose']
 assert os.readlink(parent/'exe')==cfg['timeout'] and [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]==expected and common.poste_tenu(common.processus())
 P.mkdir(mode=0o700);save(P,'outer-supervision.json',dict(observed=time.monotonic(),soft_timeout=5998,kill_after=2,hardcap=6000,common_budget=5900))
 def stop(sig,frame):first_red('outer supervision signal');raise RuntimeError('outer stopped canonical qualification')
 signal.signal(signal.SIGTERM,stop);record=dict(status='RED');rc=1
 try:
  guard(cfg);rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','5900','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text());base=module('base_controller');base.OLD=OLD;quiet=base.owned_quiet(path.parent)
  save(P,'owned-after-scope.json',quiet);save(P,'cache-quiet-after-scope.json',base.cache_quiet({'source_root':str(W)}));record.update(code=rc,scope=scope,owned=quiet,authority_state=auth['etat'])
  assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and auth['etat']=='termine' and not Path(scope['prive']).exists()
  assert not quiet['alive'] and not quiet['unknown'] and not (P/'first-red.json').exists()
  assert json.loads((P/'driver-result.json').read_text())['status']=='CANONICAL_PASSED';guard(cfg)
  record.update(status='CANONICAL_PASSED',deadline=auth['echeance'],app=APP,master_integration_authorized=False)
 except BaseException as error:first_red(error);record.update(status='RED',code=rc,error=repr(error))
 with (P/'composed.json').open('x') as f:
  if record['status']=='CANONICAL_PASSED' and time.monotonic()>=record['deadline']:record=dict(status='RED',error='deadline immediately before publication')
  json.dump(record,f,indent=2)
 print(json.dumps(record));return 0 if record['status']=='CANONICAL_PASSED' else 1

def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 control=Path(__file__).resolve().parents[4];assert git(control,'rev-parse','HEAD')==args.pilot and not git(control,'status','--porcelain')
 assert git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain')
 cp=W/'extension/rag3weaver/codeparsers';expected=git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2];assert (cp/'Cargo.toml').is_file() and git(cp,'rev-parse','HEAD')==expected and not git(cp,'status','--porcelain')
 assert not any(p.exists() or p.is_symlink() for p in [P,S])
 contract=cache_contract();Q.mkdir(mode=0o700,exist_ok=True);assert Q.resolve()==Q and Q.stat().st_uid==os.getuid()
 sources={'qualify.py':Path(__file__),'base_controller.py':control/'docs/recherche/16-11-2026/fts-generation-guard/qualify.py','passe_epinglee.sh':W/'tools/poste/passe_epinglee.sh'}
 for name in ['confiner_validation.py','poste','garde_target.sh']:sources[name]=W/'extension/rag3weaver/scripts'/name
 for name,path in sources.items():
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(path)
  else:shutil.copy2(path,dest)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(control=str(control),pilot=args.pilot,copies={n:sha(Q/n) for n in sources},timeout=timeout,timeout_sha256=sha(timeout),cache_contract=contract,required_table_sha256=sha(W/'tools/poste/required_suites.txt'))
 if (Q/'prepared.json').exists():assert json.loads((Q/'prepared.json').read_text())==cfg
 else:save(Q,'prepared.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=str(Q/'prepared.json'),sha256=sha(Q/'prepared.json'),cargo=False,native_execution=False,authority_created=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ and not (Q/'SERVICES_GO.json').exists()
 runtime=Path('/run/user')/str(os.getuid());bus=runtime/'bus';before=[p.lstat() for p in [runtime,bus]]
 assert runtime.resolve()==runtime and bus.resolve()==bus and stat.S_ISDIR(before[0].st_mode) and stat.S_ISSOCK(before[1].st_mode) and all(m.st_uid==os.getuid() for m in before)
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
 env.update(XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus),POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 subprocess.run(['busctl','--user','--timeout=5','call','org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus.Peer','Ping'],env=env,check=True)
 for path,old in zip([runtime,bus],before):
  new=path.lstat();assert (old.st_dev,old.st_ino,old.st_mode,old.st_uid)==(new.st_dev,new.st_ino,new.st_mode,new.st_uid)
 save(Q,'allocation.json',dict(reference=args.allocation,services_grant_still_required=True));return run_outer(cfg,env)
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--pilot');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:first_red(error);raise
 else:sys.exit(launch(args))
