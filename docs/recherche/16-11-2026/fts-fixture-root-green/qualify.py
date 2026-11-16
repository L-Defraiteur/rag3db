#!/usr/bin/env python3
"""Fixture root preparation/validation, exact C7 allocation required; no services."""
import argparse,hashlib,importlib.util,json,os,re,shutil,signal,stat,subprocess,sys,time
from pathlib import Path
R=Path.home()/'.cache/rag3weaver-build'
Q=R/'cx-control-recherche-fts-fixture-root-green-v1';P=R/'cx-proofs-recherche-fts-fixture-root-green-v1';S=R/'cx-scope-recherche-fts-fixture-root-green-v1'
T=R/'cx-target-recherche-fts-counts-freshness'
APP='84ddeb88b7452278b0d697fad817c7e44f5fd3d3'
OLD=R/'cx-wt-recherche-fts-generation-guard-source-v1'
OLD_SHA='bc9526408ea3a48320b4a27e7644075dcf7307b0'
LIB=R.parent/'rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
NAMES=['code_sync::fts_guard_double_tests::'+name for name in [
'classic_sync_removes_a_committed_fts_ghost',
'generations_sync_removes_a_committed_fts_ghost',
'generations_sync_rebuilds_pending_fts_even_when_counts_match',
'generations_maintenance_precedes_an_early_source_error',
'generations_maintenance_error_prevents_source_listing_and_preserves_pending',
'explicit_fixture_root_keeps_unique_cases_and_prior_bytes',
'absent_fixture_root_uses_only_the_private_disk_target',
'invalid_explicit_fixture_roots_never_fall_back_or_follow_symlinks']]
FAILED=re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.|panicked at',re.M)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value):
 with (root/name).open('x') as f:json.dump(value,f,indent=2)
def first_red(reason):
 try:save(P,'first-red.json',dict(reason=repr(reason),observed=time.monotonic()))
 except FileExistsError:pass
def inventory_names(raw):
 names=sorted(line[:-6] for line in raw.splitlines() if line.endswith(': test'))
 assert names==sorted(NAMES),names
 return NAMES
def choose_artifact(rows,source):
 matches=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('target',{}).get('name')=='rag3weaver' and r['target'].get('kind')==['lib'] and r['target'].get('src_path')==source and r.get('profile',{}).get('test') and r.get('executable')]
 assert len(matches)==1,len(matches)
 assert set(matches[0]['features'])=={'code','rag3db-native'}
 return matches[0]
def cache_contract():
 assert T.resolve()==T and T.is_dir() and T.stat().st_uid==os.getuid()
 marker=T/'.arbre-du-target';m=marker.lstat();assert stat.S_ISREG(m.st_mode) and m.st_uid==os.getuid()
 assert marker.read_text().strip()==str(OLD/'extension/rag3weaver')
 assert git(OLD,'rev-parse','HEAD')==OLD_SHA and not git(OLD,'status','--porcelain')
 prior=R/'cx-archive-recherche-fts-generation-guard-v1'
 assert sha(prior/'manifest.json')=='57fbd0280d3c46452366eb546ad3d7dedac484051e60ea859b59b2e60e9deb3c'
 manifest=json.loads((prior/'manifest.json').read_text());selected={}
 for name in ['proofs/ledger.json','proofs/composed.json','proofs/unit-tests','scope/validation-i6edjos5/autorite.json','scope/validation-i6edjos5/verdict.json']:
  h=sha(prior/name);assert h==manifest[name]['sha256'];selected[name]=h
 live=R/'cx-scope-recherche-fts-generation-guard-v1/validation-i6edjos5'
 for name in ['autorite.json','verdict.json']:assert sha(live/name)==sha(prior/'scope/validation-i6edjos5'/name)
 auth=json.loads((live/'autorite.json').read_text());scope=json.loads((live/'verdict.json').read_text())
 assert auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 ledger=json.loads((prior/'proofs/ledger.json').read_text());assert ledger['app']==OLD_SHA and ledger['elf_sha256']=='b23414d3742f7d8f1b2a230d72b2d91df187926375c1389f90b05ad277458c19'
 for name in ['source_executable','copied_executable']:assert sha(ledger[name])==ledger['elf_sha256']
 return dict(marker=dict(dev=m.st_dev,ino=m.st_ino,mode=m.st_mode,uid=m.st_uid,value=marker.read_text()),archive=selected,old=OLD_SHA,previous_elf_sha256=ledger['elf_sha256'])
def guard(cfg):
 for key,value in [('source_root',APP),('control_root',cfg['pilot'])]:
  root=Path(cfg[key]);assert git(root,'rev-parse','HEAD')==value and not git(root,'status','--porcelain')
 for name,h in cfg['copies'].items():assert sha(Q/name)==h
 for name,h in SO.items():assert sha(LIB/name)==h
 for name in ['timeout','gdb']:assert sha(cfg[name])==cfg[name+'_sha256']
 assert (T/'.arbre-du-target').read_text().strip() in [str(OLD/'extension/rag3weaver'),str(Path(cfg['source_root'])/'extension/rag3weaver')]
def outer_command(cfg):
 return [str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','1198s',sys.executable,str(Q/'qualify.py'),'--compose']

def run_outer(cfg,env):
 # No hash, scan, composition or publication after this bounded child returns.
 with (Q/'launch.log').open('x') as out:
  result=subprocess.run(outer_command(cfg),cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode

def verify_outer_signature(observed,cfg):
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','1198s',sys.executable,str(Q/'qualify.py'),'--compose']
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

def collect_process_paths(proc,observe):
 for role in ['exe','cwd']:observe(role,os.readlink(proc/role))
 for fd in (proc/'fd').iterdir():
  try:path=os.readlink(fd)
  except (FileNotFoundError,ProcessLookupError):continue
  observe('fd',path)
def cache_quiet(cfg):
 roots=[Path(cfg['source_root']),OLD,T];found=[];unknown=[];ancestors=set();pid=os.getpid()
 while pid>1 and pid not in ancestors:
  ancestors.add(pid);pid=int((Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()[1])
 for proc in Path('/proc').iterdir():
  if not proc.name.isdigit():continue
  attributed=False;start=None
  try:
   fields=(proc/'stat').read_text().rsplit(') ',1)[1].split();start=fields[19]
   if fields[0]=='Z':continue
   def observe(role,path):
    nonlocal attributed
    if path==str(Q/'poste') and int(proc.name) in ancestors and proc.stat().st_uid==os.getuid():return
    if any(path==str(root) or path.startswith(str(root)+'/') for root in roots):
     attributed=True;found.append(dict(pid=proc.name,start=start,role=role,path=path))
   collect_process_paths(proc,observe)
  except (FileNotFoundError,ProcessLookupError):continue
  except OSError as error:unknown.append(dict(pid=proc.name,start=start,attributed=attributed,error=repr(error)))
 assert not found and not any(row['attributed'] for row in unknown)
 return dict(found=found,unknown=unknown,observed=time.monotonic())
def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg)
 subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
 receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
 assert auth['etat']=='actif' and json.loads((receipt.parent/'recette.json').read_text())['delai']==1150
 cg=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits['memory.max']=='34359738368' and limits['memory.high']=='max' and limits['memory.swap.max']=='0'
 quota,period=limits['cpu.max'].split();assert quota!='max' and int(quota)==5*int(period)
 save(P,'authority-path.json',dict(path=str(receipt),deadline=end,limits=limits))
 C=Path(cfg['source_root'])/'extension/rag3weaver'
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for name in ['tmp','config','data','cache','xdg-state','state-fixture','databases','home']:(P/name).mkdir(mode=0o700)
 env.update(CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',RUST_TEST_THREADS='1',RUST_BACKTRACE='1',TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3WEAVER_FTS_DOUBLE_ROOT=str(P/'databases'),RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(LIB),RAG3DB_INCLUDE_DIR=str(LIB),RAG3DB_ROOT=str(Path(cfg['source_root'])),LD_LIBRARY_PATH=str(LIB),RAG3DB_LIB=LIB.name,RAG3DB_LIBS_DIR=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',RAG3WEAVER_TARGET_MAX_GO='32')
 save(P,'profile.json',dict(expected={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'},services_required=[],embedder='HashEmbedder16',native_runtime_qualified=False))
 def run(label,argv,cap,cwd=C,phase_env=None):
  guard(cfg);deadline=min(end-15,time.monotonic()+cap);assert deadline>time.monotonic()
  save(P,label+'.command.json',dict(argv=argv,cwd=str(cwd),cap=cap,deadline=deadline,started=time.monotonic()))
  with (P/(label+'.stdout')).open('xb') as out,(P/(label+'.stderr')).open('xb') as err:
   child=subprocess.Popen(argv,cwd=cwd,env=env if phase_env is None else phase_env,stdout=out,stderr=err)
   try:
    while True:
     raw=(P/(label+'.stdout')).read_bytes()+(P/(label+'.stderr')).read_bytes()
     if FAILED.search(raw.decode(errors='replace')):raise RuntimeError('first failure/panic in '+label)
     if time.monotonic()>=deadline:raise TimeoutError(label)
     if child.poll() is not None:break
     time.sleep(.05)
    rc=child.wait();save(P,label+'.result.json',dict(rc=rc,finished=time.monotonic()))
    assert rc==0,(label,rc)
   except BaseException as error:
    first_red(error)
    if child.poll() is None:child.terminate()
    raise
  return (P/(label+'.stdout')).read_text(errors='replace')
 before=cache_contract();save(P,'cache-after-poste.json',before);assert before==cfg['cache_contract']
 save(P,'cache-quiet-after-poste.json',cache_quiet(cfg))
 oldenv=dict(env,RAG3WEAVER_TARGET_REPRENDRE='0');newenv=dict(env,RAG3WEAVER_TARGET_REPRENDRE='1')
 guard_argv=['bash','-c','. "$1"; garder_le_target "$2" "$3"','cache-guard',str(Q/'garde_target.sh'),str(T)]
 run('cache-old-owner',guard_argv+[str(OLD/'extension/rag3weaver')],120,cwd=OLD/'extension/rag3weaver',phase_env=oldenv)
 assert (T/'.arbre-du-target').read_text().strip()==str(OLD/'extension/rag3weaver')
 run('cache-adopt',guard_argv+[str(C)],120,phase_env=newenv)
 assert (T/'.arbre-du-target').read_text().strip()==str(C)
 save(P,'cache-adopted.json',dict(marker=(T/'.arbre-du-target').read_text(),metadata='real shared guard called twice; no Python invalidation'))
 raw=run('compile',[sys.executable,str(C/'scripts/cargo_garde.py'),'test','--lib','--no-default-features','--features','rag3db-native,code','--no-run','-j8','--message-format=json-render-diagnostics'],900)
 rows=[json.loads(line) for line in raw.splitlines() if line.startswith('{')]
 artifact=choose_artifact(rows,str(C/'src/lib.rs'));elf=Path(artifact['executable']);assert elf.is_file() and not elf.is_symlink() and T in elf.parents
 executable=P/'unit-tests';shutil.copy2(elf,executable);assert sha(elf)==sha(executable)
 inventory=run('inventory',[str(executable),'fts_guard_double_tests','--list'],15);inventory_names(inventory)
 ignored=run('ignored',[str(executable),'fts_guard_double_tests','--list','--ignored'],15)
 assert not any(line.endswith(': test') for line in ignored.splitlines())
 all_names=run('inventory-all',[str(executable),'--list'],15)
 assert all((name+': test') in all_names.splitlines() for name in NAMES)
 ledger=dict(app=APP,pilot=cfg['pilot'],artifact=artifact,source_executable=str(elf),copied_executable=str(executable),elf_sha256=sha(executable),names=NAMES,ignored=0,published_before_runtime=time.monotonic(),sdk=SO)
 save(P,'ledger.json',ledger);assert json.loads((P/'ledger.json').read_text())==ledger
 # Exact same invocation reaches main, captures real profile/SDK, then continues.
 for index,name in enumerate(NAMES):
  assert sha(executable)==ledger['elf_sha256'];save(P,'runtime-'+str(index)+'-marker.json',dict(ledger_sha256=sha(P/'ledger.json'),name=name,observed=time.monotonic()))
  if index==0:
   argv=[cfg['gdb'],'--batch','--return-child-result','-ex','set pagination off','-ex','set confirm off','-ex','set print thread-events off','-ex','start','-ex','source '+str(Q/'main-profile.py'),'-ex','continue','--args',str(executable),name,'--exact','--nocapture']
  else:argv=[str(executable),name,'--exact','--nocapture']
  output=run('exact-'+str(index),argv,60)
  assert name+' ... ok' in output or re.search(r'test '+re.escape(name)+r'\s+\.\.\.\s+ok',output)
  assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',output)
  if index==0:
   profile=json.loads((P/'main-profile.json').read_text());assert profile['expected']==profile['actual'] and not profile['diff'] and str(LIB/'librag3db.so') in profile['maps']
   assert profile['exe_sha256']==ledger['elf_sha256']
 def files(root):
  result={}
  for path in sorted(root.rglob('*')):
   assert not path.is_symlink()
   if path.is_file():result[str(path.relative_to(root))]=dict(kind='file',mode=path.stat().st_mode,size=path.stat().st_size,sha256=sha(path))
   elif path.is_dir():result[str(path.relative_to(root))]=dict(kind='dir',mode=path.stat().st_mode)
   else:raise ValueError('unexpected fixture object')
  return result
 def extra_exact(label,phase_env,parent):
  assert sha(executable)==ledger['elf_sha256'];before_cases={str(p) for p in parent.iterdir()} if parent.exists() else set()
  save(P,label+'-marker.json',dict(ledger_sha256=sha(P/'ledger.json'),name=NAMES[1],profile={k:v for k,v in phase_env.items() if k!='RAG3WEAVER_VALIDATION_JETON'}))
  output=run(label,[str(executable),NAMES[1],'--exact','--nocapture'],60,phase_env=phase_env)
  assert re.search(r'test '+re.escape(NAMES[1])+r'\s+\.\.\.\s+ok',output) and 'test result: ok. 1 passed; 0 failed; 0 ignored;' in output
  paths=re.findall(r'^FTS_DOUBLE_CASE (.+)$',(P/(label+'.stderr')).read_text(),re.M);assert len(paths)==1
  case=Path(paths[0]);assert case.parent==parent and case.is_dir() and not case.is_symlink() and str(case) not in before_cases
  return str(case)
 absent=dict(env,HOME=str(P/'home'),CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(P/'fallback-target'))
 absent.pop('RAG3WEAVER_FTS_DOUBLE_ROOT',None);assert 'RAG3WEAVER_FTS_DOUBLE_ROOT' not in absent
 home_before=files(P/'home');fallback=extra_exact('root-absent',absent,P/'fallback-target/fts-double-fixtures')
 assert files(P/'home')==home_before
 repeated=dict(env,HOME=str(P/'home'),CARGO_MANIFEST_DIR=str(C));prior_files=files(P/'databases')
 first=extra_exact('same-root-first',repeated,P/'databases')
 after_first=files(P/'databases');assert all(after_first.get(n)==value for n,value in prior_files.items())
 second=extra_exact('same-root-second',repeated,P/'databases')
 after_second=files(P/'databases');assert first!=second and all(after_second.get(n)==value for n,value in after_first.items())
 assert files(P/'home')==home_before
 save(P,'fixture-path-evidence.json',dict(absent=fallback,same_root=[first,second],home_unchanged=True,prior_cases_preserved=True,distinct_judges=8,invocations=11))
 assert sha(elf)==sha(executable)==ledger['elf_sha256'];guard(cfg);assert time.monotonic()<end-10
 save(P,'driver-result.json',dict(status='FIXTURE_ROOT_PASSED',names=NAMES,deadline=end,baseline_old_master_qualified=False,canonical_rerun=False))
def compose():
 cfg=json.loads((Q/'prepared.json').read_text())
 spec=importlib.util.spec_from_file_location('common_validation',Q/'confiner_validation.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 parent=Path('/proc')/str(os.getppid())
 observed=dict(exe=str(Path(os.readlink(parent/'exe')).resolve()),argv=[os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v],poste=common.poste_tenu(common.processus()))
 verify_outer_signature(observed,cfg)
 P.mkdir(mode=0o700);save(P,'outer-supervision.json',dict(observed=time.monotonic(),parent=observed,soft_timeout=1198,kill_after=2,hardcap=1200,common_budget=1150))
 def stop(sig,frame):first_red('outer supervision signal');raise RuntimeError('outer stopped generation guard')
 signal.signal(signal.SIGTERM,stop);record=dict(status='RED');rc=1
 try:
  guard(cfg)
  rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','1150','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text());quiet=owned_quiet(path.parent)
  save(P,'owned-after-scope.json',quiet);save(P,'cache-quiet-after-scope.json',cache_quiet(cfg));record.update(code=rc,scope=scope,owned=quiet,authority_state=auth['etat'])
  assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and auth['etat']=='termine' and not Path(scope['prive']).exists()
  assert not quiet['alive'] and not quiet['unknown'] and not (P/'first-red.json').exists()
  assert json.loads((P/'driver-result.json').read_text())['status']=='FIXTURE_ROOT_PASSED'
  ledger=json.loads((P/'ledger.json').read_text());assert sha(P/'unit-tests')==sha(ledger['source_executable'])==ledger['elf_sha256']
  guard(cfg);record.update(status='FIXTURE_ROOT_PASSED',deadline=auth['echeance'],baseline_old_master_qualified=False)
 except BaseException as error:first_red(error);record.update(status='RED',code=rc,error=repr(error))
 with (P/'composed.json').open('x') as f:
  if record['status']=='FIXTURE_ROOT_PASSED' and time.monotonic()>=record['deadline']:record=dict(status='RED',error='deadline immediately before publication')
  json.dump(record,f,indent=2)
 print(json.dumps(record));return 0 if record['status']=='FIXTURE_ROOT_PASSED' else 1

def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 control=Path(__file__).resolve().parents[4];source=R/'cx-wt-recherche-fts-fixture-root-source-v1'
 assert git(control,'rev-parse','HEAD')==args.pilot and not git(control,'status','--porcelain')
 assert git(source,'rev-parse','HEAD')==APP and not git(source,'status','--porcelain')
 cp=source/'extension/rag3weaver/codeparsers';expected=git(source,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2]
 assert (cp/'Cargo.toml').is_file() and git(cp,'rev-parse','HEAD')==expected and not git(cp,'status','--porcelain')
 assert not any(p.exists() or p.is_symlink() for p in [P,S])
 contract=cache_contract()
 Q.mkdir(mode=0o700,exist_ok=True);assert Q.resolve()==Q and Q.stat().st_uid==os.getuid()
 copies={};folder=Path(__file__).parent
 for name,src in [(n,folder/n) for n in ['qualify.py','main-profile.py','sdk_contract.py']]+[(n,source/'extension/rag3weaver/scripts'/n) for n in ['confiner_validation.py','poste','garde_target.sh']]:
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(src)
  else:shutil.copy2(src,dest)
  copies[name]=sha(dest)
 cfg=dict(control_root=str(control),source_root=str(source),pilot=args.pilot,copies=copies,cache_contract=contract)
 for name in ['timeout','gdb']:
  cfg[name]=str(Path(shutil.which(name)).resolve());cfg[name+'_sha256']=sha(cfg[name])
 if (Q/'prepared.json').exists():assert json.loads((Q/'prepared.json').read_text())==cfg
 else:save(Q,'prepared.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=str(Q/'prepared.json'),native_execution=False,cargo=False,authority_created=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 runtime=Path('/run/user')/str(os.getuid());bus=runtime/'bus';before=[p.lstat() for p in [runtime,bus]]
 assert runtime.resolve()==runtime and bus.resolve()==bus and stat.S_ISDIR(before[0].st_mode) and stat.S_ISSOCK(before[1].st_mode) and all(m.st_uid==os.getuid() for m in before)
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
 env.update(XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus),POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 subprocess.run(['busctl','--user','--timeout=5','call','org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus.Peer','Ping'],env=env,check=True)
 for path,old in zip([runtime,bus],before):
  new=path.lstat();assert (old.st_dev,old.st_ino,old.st_mode,old.st_uid)==(new.st_dev,new.st_ino,new.st_mode,new.st_uid)
 save(Q,'allocation.json',dict(reference=args.allocation));return run_outer(cfg,env)
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--pilot');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:first_red(error);raise
 else:sys.exit(launch(args))
