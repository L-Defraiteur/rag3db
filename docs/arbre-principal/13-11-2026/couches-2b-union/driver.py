#!/usr/bin/env python3
"""One bounded F8/large/NAV qualification through the guarded entry points."""
from pathlib import Path
import hashlib,json,mmap,os,re,shutil,signal,subprocess,sys,time,tempfile,importlib.util,stat,traceback
R=Path('/home/lucied/.cache/rag3weaver-build');Q=Path(__file__).parent;W=R/'cx-wt-couches-2b-union';C=W/'extension/rag3weaver';T=R/'cx-target-contenu-derive';P=R/'cx-validation-couches-2b-union-v1';S=R/'cx-confinement-couches-2b-union-v1'
PROVENANCE=json.loads((Q/'provenance.json').read_text());APP=PROVENANCE['app'];PILOT=PROVENANCE['pilot'];CONTROL=Path(PROVENANCE['control'])
assert APP=='193d77cd71fa4cc1608e961ea4c8a8d3d445412e'
AUTH_PATH=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);AUTH=json.loads(AUTH_PATH.read_text());END=AUTH['echeance'];START=END-4800
assert START<=time.monotonic()<END
assert json.loads((AUTH_PATH.parent/'recette.json').read_text())['delai']==4800
assert AUTH_PATH.parent.parent==S
P.mkdir(exist_ok=False);assert T.is_dir() and T.resolve()==T and T.stat().st_uid==os.getuid()
assert not json.loads((Q/'cache-before.json').read_text())['quiescence']
for name in ['tmp','config','data','cache','xdg-state','state-fixture']:(P/name).mkdir()
CG=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1];G=Path('/sys/fs/cgroup'+CG)
assert (G/'memory.max').read_text().strip()=='34359738368' and (G/'memory.high').read_text().strip()=='max'
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
LIB=R.parent/'rag3db-libs'/NATIVE/'librag3db.so'
E={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','SHELL','LANG','LC_ALL','TZ','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
E.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',LUCIVY_SCHEDULER_THREADS='8',RUST_TEST_THREADS='8',RUST_BACKTRACE='1',TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_CACHE_DIR=str(P/'cache'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',RAG3WEAVER_TARGET_MAX_GO='32',RAG3WEAVER_TARGET_REPRENDRE='1')
assert not Path(E['RAG3WEAVER_STATE_DIR']).is_relative_to(Path(E['XDG_STATE_HOME'])) and not Path(E['XDG_STATE_HOME']).is_relative_to(Path(E['RAG3WEAVER_STATE_DIR']))
for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
EXPORT=['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','native-choice',str(C/'scripts/lib_du_magasin.sh')]
F='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
E['CHECK_FEATURES_LOGS']=str(P/'features');E['CHECK_FEATURES_TIMEOUT']='1500'
E.update(RAG3WEAVER_PAQUETS_REACT=str(R/'I-paquets-react'),RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_NAVIGATEUR='/usr/lib/chromium/chromium')
SDK_PROFILE=dict(RAG3DB_LIB=NATIVE,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(LIB.parent),RAG3DB_INCLUDE_DIR=str(LIB.parent),LD_LIBRARY_PATH=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
CARGO_EXE='/home/lucied/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo'
CARGO_SHA='ff3022fcbd13b08434ea7afde9a0ef9d5b3f5c17b5fc7a40031be3ee000a6a24'
RUST_LIB='/home/lucied/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib'
CARGO_PROFILE=dict(SDK_PROFILE,LD_LIBRARY_PATH=RUST_LIB+':'+SDK_PROFILE['LD_LIBRARY_PATH'])
METADATA_BEFORE_EXPORT={key:None for key in SDK_PROFILE};METADATA_BEFORE_EXPORT['LD_LIBRARY_PATH']=RUST_LIB
UI_CASE='daemon_host::tests::served_ide_assets_match_the_current_worktree'
PREFIX='dataflow::migrations::publication::';LAYER=PREFIX+'layers::tests::'
ED_REQUIRED={'daemon_host::atelier_fichiers::tests::'+name for name in ['read_scope_preserves_a_backend_error_instead_of_claiming_absence','read_scope_preserves_a_usages_error_instead_of_claiming_absence','read_scope_refuses_unproven_uniqueness_after_a_real_definition','read_scope_keeps_real_absence_and_ambiguity_after_complete_queries']}

LAYER_REQUIRED={LAYER+n for n in ['inherited_values_keep_occurrence_coordinates_and_exact_publication_provenance','tombstones_hide_all_ancestors_and_a_descendant_can_reintroduce_a_file','overrides_do_not_resurrect_an_old_file_and_anchor_advancement_is_not_followed','malformed_piles_and_ambiguous_local_changes_are_refused_without_partial_resolution']}
EXPECTED=set(json.loads((Q/'expected.json').read_text())['exacts']);assert len(EXPECTED)==15
owned={};seen={};maps={};protected={};devices={};phases=[];phase='preparation';child=None;binary=None;last_size=0;binary_sha=None;artifacts={};large=None;nav=None;main_large=None;attempts={}

def save(name,value):(P/name).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
def digest(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],env=dict(os.environ,GIT_OPTIONAL_LOCKS='0'),text=True).strip()
def guard():
 for root,sha in [(W,APP),(CONTROL,PILOT)]:
  assert git(root,'rev-parse','HEAD')==sha and not git(root,'status','--porcelain')
  for name in ['rebase-merge','rebase-apply']:assert not Path(git(root,'rev-parse','--path-format=absolute','--git-path',name)).exists()
 for name,sha in PROVENANCE['files'].items():assert digest(Q/name)==sha
 assert not any(k.endswith('_API_KEY') or k.startswith('RAG3WEAVER_SERVICE_') for k in E)
def memory():
 sample=dict(time=time.monotonic(),current=int((G/'memory.current').read_text()),peak=int((G/'memory.peak').read_text()),events=dict((k,int(v)) for k,v in (line.split() for line in (G/'memory.events').read_text().splitlines())))
 with (P/'memory-samples.jsonl').open('a') as f:f.write(json.dumps(sample)+'\n')
 return sample

def cargo_expected_for(phase_name):
 return METADATA_BEFORE_EXPORT if phase_name in ['cache-reprise','cpu'] else CARGO_PROFILE
def record_attempt(kind,payload):
 fingerprint=(kind,hashlib.sha256(json.dumps(payload,sort_keys=True).encode()).hexdigest())
 if fingerprint not in attempts:
  name=f'attempt-{len(attempts):05d}-{kind}.json'
  with (P/name).open('x') as f:json.dump(dict(payload,observed=time.monotonic()),f,indent=2)
  attempts[fingerprint]=name
 return attempts[fingerprint]
def latch_first_red(reason,payload=None):
 path=P/'first-red.json'
 try:
  with path.open('x') as f:json.dump(dict(phase=phase,reason=reason,payload=payload,observed=time.monotonic()),f,indent=2)
 except FileExistsError:pass
def require(ok,reason,payload=None):
 if not ok:
  latch_first_red(reason,payload)
  raise AssertionError(reason)
def in_cache_context(value,roots):
 return any(value==str(root) or value.startswith(str(root)+'/') for root in roots)
def controller_role(info,ancestors,roles):
 ancestor=ancestors.get(info['pid'])
 if not ancestor or any(info[k]!=ancestor[k] for k in ['starttime','ppid','cgroup']):return None
 matches=[]
 for name,role in roles.items():
  if (info['uid']==role['uid'] and info['exe']==role['exe'] and info['exe_sha256']==role['exe_sha256']
      and info['argv'][1:]==role['argv_tail'] and info['cwd']==role['cwd']):matches.append(name)
 return matches[0] if len(matches)==1 else None
def controller_fd_allowed(role,value,roles,protected):
 return role=='poste' and not in_cache_context(value,protected) and value==roles[role]['script']

def build_controller_policy(launcher,poste,pyexe,shellexe,uid,pilot,scope,proofs,sha):
 return {
  'launcher':dict(uid=uid,exe=pyexe,exe_sha256=sha(pyexe),cwd='/var/tmp',script=str(launcher),script_sha256=sha(launcher),argv_tail=[str(launcher),pilot]),
  'poste':dict(uid=uid,exe=shellexe,exe_sha256=sha(shellexe),cwd='/var/tmp',script=str(poste),script_sha256=sha(poste),argv_tail=[str(poste),'lourd','python3',str(proofs/'admettre.py')]),
 }

def fresh_cache_guard():
 # Called after actual poste admission, immediately before the real common guard.
 subprocess.run(['python3',str(Q/'confiner_validation.py'),'verifier'],env=os.environ.copy(),check=True,stdout=subprocess.DEVNULL)
 old=R/'cx-validation-couches-2b-a-v1';initial=json.loads((Q/'cache-before.json').read_text())
 previous=json.loads((old/'source.json').read_text());oldc=Path(initial['old_crate']);oldw=oldc.parent.parent
 require(previous['app']=='affa6d03b5a57dd99810b6853783dc1a267f76b9','old application changed')
 require(git(oldw,'rev-parse','HEAD')==previous['app'] and not git(oldw,'status','--porcelain'),'old worktree changed')
 require(json.loads(Path(previous['receipt']).read_text())['etat']=='termine','old authority active')
 require(json.loads((old/'verdict-compose.json').read_text())['verdict']=='VERT','old composite changed')
 ledger_path=old/'inventaire-avant-runtime.json';require(digest(ledger_path)==initial['previous_ledger_sha256']=='22e3ccb9af4c9711db3491d545486a41b231bff0cbb96ae23b78dfda909b1be2','old ledger changed')
 ledger=json.loads(ledger_path.read_text())
 for file in [Path(ledger['binary']),old/'binaire-cpu']:require(digest(file)==ledger['sha256'],'old ELF changed',dict(file=str(file)))
 target_stat=T.lstat();marker=T/'.arbre-du-target'
 require(T.resolve()==T and stat.S_ISDIR(target_stat.st_mode) and target_stat.st_uid==os.getuid(),'target identity unsafe')
 require((target_stat.st_dev,target_stat.st_ino)==(initial['device'],initial['inode']),'target inode changed')
 require(target_stat.st_mode==initial['mode'] and target_stat.st_uid==initial['uid'],'target mode or owner changed')
 require(marker.is_file() and not marker.is_symlink() and marker.read_text().strip()==str(oldc),'target marker changed')
 roots=[T,oldw,W,CONTROL];found=[];unknown=[];controllers=[]
 old_groups={previous['cgroup']}
 ancestors={};cursor=os.getppid()
 while cursor>1 and cursor not in ancestors:
  proc=Path('/proc')/str(cursor)
  try:
   st=(proc/'stat').read_text().rsplit(') ',1)[1].split();cg=(proc/'cgroup').read_text().strip().split('::',1)[1]
   require((proc/'stat').read_text().rsplit(') ',1)[1].split()[19]==st[19],'controller ancestry changed')
   ancestors[cursor]=dict(starttime=st[19],ppid=int(st[1]),cgroup=cg);cursor=int(st[1])
  except (OSError,ProcessLookupError):break
 launcher=CONTROL/'docs/arbre-principal/13-11-2026/couches-2b-union/lancer.py';poste=C/'scripts/poste'
 launcher_blob=subprocess.check_output(['git','-C',str(CONTROL),'show',PILOT+':docs/arbre-principal/13-11-2026/couches-2b-union/lancer.py'])
 poste_blob=subprocess.check_output(['git','-C',str(W),'show',APP+':extension/rag3weaver/scripts/poste'])
 require(digest(launcher)==hashlib.sha256(launcher_blob).hexdigest() and digest(poste)==hashlib.sha256(poste_blob).hexdigest(),'controller script source changed')
 pyexe=str(Path(sys.executable).resolve());shellexe=str(Path('/bin/sh').resolve())
 roles=build_controller_policy(launcher,poste,pyexe,shellexe,os.getuid(),PILOT,S,Q,digest)
 for proc in Path('/proc').iterdir():
  if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
  stamp=None;attributed=False;role=None;field='stat'
  try:
   st=(proc/'stat').read_text().rsplit(') ',1)[1].split();stamp=st[19]
   if st[0]=='Z':continue
   field='cgroup';cg=(proc/'cgroup').read_text().strip().split('::',1)[1]
   attributed=cg in old_groups or owned.get(int(proc.name))==stamp
   if proc.stat().st_uid!=os.getuid() and not attributed:continue
   field='cmdline';argv=[os.fsdecode(v) for v in (proc/'cmdline').read_bytes().split(b'\0') if v]
   base_attributed=attributed;attributed=base_attributed or any(in_cache_context(value,roots) for value in argv)
   field='exe';exe=os.readlink(proc/'exe');field='cwd';cwd=os.readlink(proc/'cwd')
   if int(proc.name) in ancestors:
    field='controller-exe-sha';exe_sha=digest(proc/'exe')
    info=dict(pid=int(proc.name),starttime=stamp,ppid=int(st[1]),cgroup=cg,uid=proc.stat().st_uid,exe=exe,exe_sha256=exe_sha,argv=argv,cwd=cwd)
    role=controller_role(info,ancestors,roles)
    if role:
     require((proc/'stat').read_text().rsplit(') ',1)[1].split()[19]==stamp and (proc/'cgroup').read_text().strip().split('::',1)[1]==cg,'controller identity changed',info)
     controllers.append(dict(info,role=role,script_sha256=roles[role]['script_sha256'],allowed_script_fds=[]))
   # A verified role permits only its exact control argv. Context cwd/exe/FD
   # are still inspected, particularly target and old worktree descriptors.
   if role:attributed=base_attributed
   for name,value in [('exe',exe),('cwd',cwd)]:
    if in_cache_context(value,roots):attributed=True;found.append(dict(pid=proc.name,starttime=stamp,field=name,path=value))
   field='fd'
   for fd in (proc/'fd').iterdir():
    try:value=os.readlink(fd)
    except (FileNotFoundError,ProcessLookupError):continue
    if in_cache_context(value,roots):
     if controller_fd_allowed(role,value,roles,[T,oldw]):
      require(digest(value)==roles[role]['script_sha256'],'controller source FD changed',dict(pid=proc.name,fd=str(fd)))
      controllers[-1]['allowed_script_fds'].append(dict(fd=str(fd),path=value));continue
     attributed=True;found.append(dict(pid=proc.name,starttime=stamp,field=str(fd),path=value))
   if attributed and not any(v['pid']==proc.name for v in found):found.append(dict(pid=proc.name,starttime=stamp,observation='attributed-live'))
  except PermissionError:
   attributed=attributed or role is not None
   row=dict(pid=proc.name,starttime=stamp,field=field,observation='UNKNOWN',attributed=attributed)
   unknown.append(row)
   if attributed:found.append(row)
  except (FileNotFoundError,ProcessLookupError):continue
 record=dict(observed=time.monotonic(),before_observed=initial['observed'],device=target_stat.st_dev,inode=target_stat.st_ino,mode=target_stat.st_mode,uid=target_stat.st_uid,marker=str(oldc),old_app=previous['app'],roots=[str(r) for r in roots],found=found,unknown=unknown,elf_sha256=ledger['sha256'],controllers=controllers,controller_policy=roles,ancestor_snapshot=ancestors)
 save('cache-after-poste.json',record)
 require({v['role'] for v in controllers}=={'launcher','poste'} and len(controllers)==2,'exact launcher/poste ancestry not proved',record)
 require(not found,'cache activity or attributed UNKNOWN after poste',record)

def observe(cleanup=False):
 table={}
 for q in Path('/proc').iterdir():
  if not q.name.isdigit():continue
  try:
   s=(q/'stat').read_text().rsplit(') ',1)[1].split();table[int(q.name)]=(int(s[1]),s[19],s[0])
  except (OSError,ProcessLookupError):continue
 changed=True
 while changed:
  changed=False
  for pid,(parent,stamp,_) in table.items():
   if (pid not in owned or owned[pid]!=stamp) and (parent==os.getpid() or (parent in table and owned.get(parent)==table[parent][1])):
    owned[pid]=stamp;changed=True
 live=[]
 for pid,stamp in list(owned.items()):
  if pid not in table or table[pid][1]!=stamp or table[pid][2]=='Z':continue
  live.append(pid);q=Path('/proc')/str(pid);verified=False;cg=None;exe=None;field='cgroup'
  if cleanup:
   # Attribution/confinement only. Never create or replace a profile on stop.
   try:
    cleanup_cg=(q/'cgroup').read_text().strip().split('::',1)[1]
    with (P/'cleanup-attribution.jsonl').open('a') as f:f.write(json.dumps(dict(pid=pid,starttime=stamp,cgroup=cleanup_cg,in_scope=cleanup_cg==CG))+'\n')
   except (OSError,ProcessLookupError):pass
   continue
  try:
   cg=(q/'cgroup').read_text().strip().split('::',1)[1];maximum=(Path('/sys/fs/cgroup'+cg)/'memory.max').read_text().strip()
   assert cg==CG and maximum=='34359738368',('confinement',pid,cg,maximum)
   verified=True;field='exe';exe=os.readlink(q/'exe');seen[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,memory_max=maximum)
   field='fd'
   for fd in (q/'fd').iterdir():
    try:dest=os.readlink(fd)
    except (FileNotFoundError,ProcessLookupError):continue
    if dest.startswith('/dev/dri/') or dest=='/dev/kfd':devices[(pid,stamp,dest)]=dict(pid=pid,starttime=stamp,device=dest)
   if phase in ['cache-reprise','features','large','navigateur','cpu','nav40'] and Path(exe).name=='cargo':
    field='cargo-profile';raw=dict(v.split(b'=',1) for v in (q/'environ').read_bytes().split(b'\0') if b'=' in v)
    actual={k:(os.fsdecode(raw[os.fsencode(k)]) if os.fsencode(k) in raw else None) for k in SDK_PROFILE}
    argv=[os.fsdecode(v) for v in (q/'cmdline').read_bytes().split(b'\0') if v]
    expected=cargo_expected_for(phase)
    diff={k:dict(expected=expected[k],actual=actual[k]) for k in expected if expected[k]!=actual[k]}
    actual_sha=digest(q/'exe')
    payload=dict(pid=pid,starttime=stamp,cgroup=cg,exe=exe,exe_sha256=actual_sha,argv=argv,actual=actual,expected=expected,diff=diff,phase=phase)
    attempt=record_attempt('cargo',payload)
    require(exe==CARGO_EXE and actual_sha==CARGO_SHA,'Cargo executable identity differs',dict(payload,attempt=attempt))
    require((q/'stat').read_text().rsplit(') ',1)[1].split()[19]==stamp,'Cargo PID reused',dict(payload,attempt=attempt))
    if phase=='cache-reprise':require(argv[1:]==['metadata','--format-version','1','--all-features'],'unexpected command before SDK export',dict(payload,attempt=attempt))
    require(not diff,'Cargo profile differs in '+phase,dict(payload,attempt=attempt))
    summary='cache-metadata-profile.json' if phase=='cache-reprise' else 'features-real-sdk-env.json'
    if (phase=='cache-reprise' or phase=='features') and not (P/summary).exists():
     with (P/summary).open('x') as f:json.dump(dict(payload,attempt=attempt),f,indent=2)
   if exe.startswith(str(T)+'/'):
    field='maps';data=(q/'maps').read_text();native=[line for line in data.splitlines() if 'librag3db.so' in line]
    assert all('/'+NATIVE+'/librag3db.so' in line for line in native),('wrong native maps',pid)
    if (pid,stamp) not in maps:
     first=f'maps-first-{pid}-{stamp}.txt';(P/first).write_text(data)
     maps[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,first_file=first,first_native=native,file=first,native=native,phase=phase)
    if native:
     loaded=f'maps-sdk-{pid}-{stamp}.txt';(P/loaded).write_text(data)
     maps[(pid,stamp)].update(file=loaded,native=native,loaded_phase=phase)
    if nav and exe==str(nav) and phase=='nav40' and native and not (P/'nav-real-profile.json').exists():
     field='nav-profile';raw=dict(v.split(b'=',1) for v in (q/'environ').read_bytes().split(b'\0') if b'=' in v)
     actual={k:(os.fsdecode(raw[os.fsencode(k)]) if os.fsencode(k) in raw else None) for k in SDK_PROFILE}
     roles={x:os.fsdecode(raw[os.fsencode('RAG3WEAVER_BURN_DEVICE'+x)]) if os.fsencode('RAG3WEAVER_BURN_DEVICE'+x) in raw else None for x in ['','_EMBEDDER','_RERANKER','_OCR']}
     payload=dict(pid=pid,starttime=stamp,exe=exe,exe_sha256=digest(q/'exe'),cgroup=cg,env=actual,roles=roles,native=native,meaning='LD constructed by Cargo test; maps qualify SDK29')
     attempt=record_attempt('nav',payload)
     require(all(actual[k]==v for k,v in SDK_PROFILE.items() if k!='LD_LIBRARY_PATH'),'NAV SDK profile differs',dict(payload,attempt=attempt))
     require(all(v=='cpu' for v in roles.values()),'NAV CPU role differs',dict(payload,attempt=attempt))
     with (P/'nav-real-profile.json').open('x') as f:json.dump(dict(payload,attempt=attempt),f,indent=2)
  except PermissionError as error:
   protected[(pid,stamp,field)]=dict(pid=pid,starttime=stamp,field=field,exe=exe,cgroup=cg,cgroup_verified=verified,observation='UNKNOWN',error=str(error))
   assert verified,('owned cgroup unreadable',pid,field)
  except (FileNotFoundError,ProcessLookupError):continue
 return live

def evidence():
 save('descendants.json',list(seen.values()));save('owned.json',owned);save('maps.json',list(maps.values()));save('devices.json',list(devices.values()));save('unknown.json',list(protected.values()))
def quiet():
 found=[];unknown=[]
 for q in Path('/proc').iterdir():
  if not q.name.isdigit() or int(q.name)==os.getpid():continue
  stamp=None;pid=int(q.name)
  try:
   s=(q/'stat').read_text().rsplit(') ',1)[1].split();stamp=s[19]
   if s[0]=='Z' or (q.stat().st_uid!=os.getuid() and owned.get(pid)!=s[19]):continue
   exe=os.readlink(q/'exe');cwd=os.readlink(q/'cwd')
   if exe.startswith(str(T)+'/') or any(cwd==str(root) or cwd.startswith(str(root)+'/') for root in [W,CONTROL]):found.append(dict(pid=pid,starttime=s[19],exe=exe,cwd=cwd))
  except PermissionError:
   unknown.append(q.name)
   if pid in owned and (stamp is None or owned[pid]==stamp):found.append(dict(pid=q.name,observation='UNKNOWN-owned-live'))
  except (FileNotFoundError,ProcessLookupError):continue
 return dict(found=found,unknown_baseline=unknown)
def stop():
 try:observe(cleanup=True)
 except BaseException as error:save('stop-observe-error.json',repr(error))
 for sig in [signal.SIGTERM,signal.SIGKILL]:
  for pid,stamp in list(owned.items()):
   try:
    fd=os.pidfd_open(pid)
    try:
     if (Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()[19]==stamp:signal.pidfd_send_signal(fd,sig)
    finally:os.close(fd)
   except (OSError,ProcessLookupError):pass
  time.sleep(.3)
 if child is not None:
  try:child.wait(timeout=3)
  except subprocess.TimeoutExpired:pass

def state(verdict,error=None):save('etat.json',dict(verdict=verdict,phase=phase,error=error,phases=phases,remaining=END-time.monotonic()))
def first_red(text):
 # nocapture can split the test prefix, panic and final FAILED across lines.
 return re.search(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|\bpanicked at\b',text,re.M) is not None

def first_lib_red(text):
 # --show-output keeps ordinary status lines intact; expected panics aren't reds.
 return re.search(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$',text,re.M) is not None

def run(name,args,cap,sdk=False,absolute=None):
 global child,phase,last_size,main_large
 guard();phase=name;limit=min(time.monotonic()+cap,END-15,absolute if absolute is not None else END);assert time.monotonic()<limit
 state('EN_COURS');path=P/(name+'.log');start=time.monotonic();print('DEBUT',name,flush=True)
 env=E.copy();command=args
 if sdk:
  env.update(RAG3DB_LIB=NATIVE,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
  if sdk=='export':command=EXPORT+['python3',str(Q/'sdk-exec.py'),str(P),name,*args]
  elif sdk=='gdb':command=EXPORT+args
  else:assert sdk=='script' # Canonical F8 already exports the SDK once.

 if name=='cache-reprise':fresh_cache_guard()
 require(time.monotonic()<limit,'preparation exceeded command budget before launch')
 with path.open('w') as f:
  child=subprocess.Popen(command,cwd=C,env=env,stdout=f,stderr=subprocess.STDOUT)
  while True:
   observe();memory()
   assert time.monotonic()<limit,('deadline',name)
   current=path.read_text(errors='replace')
   if name.startswith('exact-') or name=='publication':assert not first_red(current),('first red',name)
   elif name=='lib-large':assert not first_lib_red(current),('first lib FAILED',name)
   elif name=='nav40':assert not first_lib_red(current),('first NAV FAILED',name)
   elif name=='features':assert not re.search(r'^(?:CHECK_FEATURES )?ROUGE\b',current,re.M),('first F8 red',name)

   if time.monotonic()-last_size>5:
    size=int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0]);save('target-size.json',dict(bytes=size,time=time.monotonic()));assert size<=32*1024**3;last_size=time.monotonic()
   if child.poll() is not None:
    if name=='lib-large' and child.returncode==0 and not (P/'elf-exit-real.json').exists():time.sleep(.01);continue
    break
   time.sleep(.025)
  rc=child.wait();child=None
 size=int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0]);save('target-size.json',dict(bytes=size,time=time.monotonic()));assert size<=32*1024**3
 phases.append(dict(name=name,rc=rc,elapsed=time.monotonic()-start,cap=cap,effective_deadline=limit));evidence();state('EN_COURS');assert rc==0,(name,rc)
 guard();print('VERT',name,flush=True);return path.read_text(errors='replace')
def inventory(label,prefix,elf=None,sdk=False):
 elf=elf or binary
 filters=[prefix] if prefix else []
 text=run(label+'-list',[str(elf),*filters,'--list','--format','terse'],30,sdk=sdk)
 ignored=run(label+'-ignored',[str(elf),*filters,'--list','--ignored','--format','terse'],30,sdk=sdk)
 names=sorted(line[:-6] for line in text.splitlines() if line.endswith(': test'));skips=sorted(line[:-6] for line in ignored.splitlines() if line.endswith(': test'))
 assert names and set(skips)<=set(names);save(label+'-inventory.json',dict(names=names,ignored=skips));return names,skips
def judged(name,args,expected,ignored,cap,attest_main=False):
 command=[str(binary),*args,'--nocapture','--test-threads=8']
 if attest_main:
  # Stop the real first exact invocation at main, capture, then continue it.
  # No extra test invocation, fake ELF, injected delay or storage operation.
  gdb=P/'elf-main.gdb'
  wanted=['CARGO_MANIFEST_DIR','CARGO_TARGET_DIR','CARGO_BUILD_JOBS','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_CACHE_HOME','XDG_STATE_HOME','RAG3WEAVER_STATE_DIR','TMPDIR','HOME',*['RAG3WEAVER_BURN_DEVICE'+x for x in ['','_EMBEDDER','_RERANKER','_OCR']]]
  body='''set confirm off
set pagination off
set debuginfod enabled off
set startup-with-shell off
start
python
import gdb,json,os,hashlib
from pathlib import Path
pid=gdb.selected_inferior().pid
assert pid>0
proc=Path('/proc')/str(pid)
proof=Path(PROOF)
raw=dict(x.split(b'=',1) for x in (proc/'environ').read_bytes().split(b'\\0') if b'=' in x)
wanted=WANTED
meta=dict(pid=pid,starttime=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19],exe=os.readlink(proc/'exe'),cgroup=(proc/'cgroup').read_text().strip().split('::',1)[1],env={k:os.fsdecode(raw[os.fsencode(k)]) for k in wanted},capture='gdb-main-first-exact-then-continue')
with (proof/'elf-main-maps.txt').open('x') as f:f.write((proc/'maps').read_text())
with (proof/'elf-main.json').open('x') as f:json.dump(meta,f,indent=2)
end
continue
'''.replace('PROOF',repr(str(P))).replace('WANTED',repr(wanted))
  gdb.write_text(body);command=['gdb','--nx','--batch','--return-child-result','-x',str(gdb),'--args',*command]
 text=run(name,command,cap)
 if attest_main:
  meta=json.loads((P/'elf-main.json').read_text());assert meta['exe']==str(binary) and meta['cgroup']==CG
  assert all(meta['env'][k]==E[k] for k in meta['env'])
  native=[line for line in (P/'elf-main-maps.txt').read_text().splitlines() if 'librag3db.so' in line]
  assert not native,('unexpected native SDK mapping at CPU main',native)
  save('elf-main-native.json',dict(rows=native,native_runtime_qualified=False))

 counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 assert counts and tuple(map(int,counts[-1]))==(expected,0,ignored),(name,counts)
 assert not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',text)
 return text

def interrupted(sig,_):raise RuntimeError('signal '+str(sig))
for sig in [signal.SIGTERM,signal.SIGINT,signal.SIGHUP]:signal.signal(sig,interrupted)
verdict='ROUGE';qualification=dict(features='NON_QUALIFIE',large='NON_QUALIFIE',nav='NON_QUALIFIE',assets='NON_QUALIFIE',cpu='NON_QUALIFIE',publication='NON_QUALIFIE');before=memory()
def feature_closure(features,defaults=True):
 import tomllib
 definitions=tomllib.loads((C/'Cargo.toml').read_text())['features']
 result=set(features.split(',') if features else []);result.update(['default'] if defaults else []);pending=list(result)
 while pending:
  for value in definitions[pending.pop()]:
   if value in definitions and value not in result:result.add(value);pending.append(value)
 return result

def compile_elf(label,features,cap,sdk=False):
 args=(['cargo','test','--lib','--no-default-features'] if label=='cpu' else ['bash','scripts/cargo_magasin.sh','test','--lib'])
 if features:args+=['--features',features]
 text=run(label,args+['--no-run','--message-format=json-render-diagnostics'],cap,sdk=sdk);paths={};builds={}
 for line in text.splitlines():
  try:item=json.loads(line)
  except ValueError:continue
  if item.get('reason')=='build-script-executed':builds[item['package_id']]=item['out_dir']
  if item.get('reason')=='compiler-artifact' and item.get('target',{}).get('name')=='rag3weaver' and item.get('profile',{}).get('test') and item.get('executable'):paths[item['executable']]=item['features']
 assert len(paths)==1
 path=Path(next(iter(paths)));assert path.is_relative_to(T)
 assert set(paths[str(path)])==feature_closure(features,defaults=label!='cpu')
 with path.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:assert data.find(APP[:9].encode())>=0
 sha=digest(path);copy=P/('binaire-'+label);shutil.copy2(path,copy);assert digest(copy)==sha
 artifacts[label]=dict(binary=str(path),copy=str(copy),sha256=sha,size=path.stat().st_size,features=sorted(paths[str(path)]),requested_features=features.split(',') if features else [],app=APP,builds=builds,artifact=[json.loads(l) for l in text.splitlines() if l.startswith('{') and json.loads(l).get('executable')==str(path)][-1])
 return path
try:
 guard();assert digest(LIB)=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
 save('source.json',dict(app=APP,pilot=PILOT,worktree=str(W),control=str(CONTROL),target=str(T),target_reused_from='affa6d03b5a57dd99810b6853783dc1a267f76b9',cpu_features=[],state_roots=dict(xdg=str(P/'xdg-state'),rag=str(P/'state-fixture')),initial=START,deadline=END,budget=4800,receipt=str(AUTH_PATH),large_features=F.split(','),nav_features=(F+',navigateur').split(','),native=NATIVE,driver_sha256=digest(__file__),cgroup=CG,memory_max=(G/'memory.max').read_text().strip(),memory_high=(G/'memory.high').read_text().strip(),caps=dict(cache_reprise=120,features=2100,compile_large=900,compile_nav=600,compile_cpu=480,each_exact=30,publication=120,large_runtime=600,nav40=360,ui=60,attestation=120),environment_keys=sorted(E),cpu_roles=4))
 run('verifier',['python3',str(Q/'confiner_validation.py'),'verifier'],30)
 save('ledger-preparation.json',dict(app=APP,pilot=PILOT,check_features_sha256=digest(C/'scripts/check_features.sh'),target_provenance=json.loads((Q/'cache-before.json').read_text()),sdk=NATIVE,sdk_sha256=digest(LIB)))
 assert Path(E['RAG3WEAVER_PAQUETS_REACT']).is_dir()
 fp_before=sorted(str(c.relative_to(T)) for folder in T.rglob('.fingerprint') if folder.is_dir() for c in folder.iterdir() if c.is_dir());assert fp_before;save('fingerprints-before.json',fp_before)
 f8_deadline=min(END-15,time.monotonic()+2100)
 cachelog=run('cache-reprise',['bash','-c','. "$1"; garder_le_target "$2" "$3"','common-cache-guard',str(C/'scripts/garde_target.sh'),str(T),str(C)],120,absolute=f8_deadline)
 expected_resume='target repris de '+json.loads((Q/'cache-before.json').read_text())['old_crate']+' : empreintes effacées pour '
 require(expected_resume in cachelog,'common target invalidation branch not observed')
 require((P/'cache-metadata-profile.json').exists(),'real initial guard Cargo not observed')
 packages=next(line for line in cachelog.splitlines() if expected_resume in line).split(expected_resume,1)[1].split();assert packages
 fp_after=sorted(str(c.relative_to(T)) for folder in T.rglob('.fingerprint') if folder.is_dir() for c in folder.iterdir() if c.is_dir())
 assert not any(re.fullmatch(re.escape(pkg)+r'-[0-9a-f]{16}',Path(x).name) for pkg in packages for x in fp_after),'local fingerprints survived common guard'
 save('fingerprints-after-guard.json',dict(packages=packages,paths=fp_after))
 f8=run('features',['bash','scripts/check_features.sh'],2100,sdk='script',absolute=f8_deadline);require('CHECK_FEATURES VERT 8/8' in f8,'F8 not fully judged')
 require((P/'features-real-sdk-env.json').exists(),'real F8 Cargo SDK profile missing')
 assert (T/'.arbre-du-target').read_text().strip()==str(C)
 save('cache-reprise-reelle.json',dict(before=json.loads((Q/'cache-before.json').read_text()),message=next(line for line in cachelog.splitlines() if expected_resume in line),new_crate=str(C),manual_purge=False))
 packages=next(line for line in cachelog.splitlines() if expected_resume in line).split(expected_resume,1)[1].split();assert packages
 external=[x for x in fp_before if not any(re.fullmatch(re.escape(pkg)+r'-[0-9a-f]{16}',Path(x).name) for pkg in packages)]
 assert all((T/x).is_dir() for x in external),'external fingerprints removed'
 save('fingerprints-external-retained.json',external)
 qualification['features']='VERT_8_SUR_8'
 large=compile_elf('large',F,900,sdk='script');nav=compile_elf('navigateur',F+',navigateur',600,sdk='script');binary=compile_elf('cpu','',480);binary_sha=artifacts['cpu']['sha256']
 spec=importlib.util.spec_from_file_location('git_attestation',Q/'attester.py');att=importlib.util.module_from_spec(spec);spec.loader.exec_module(att)
 attest_limit=min(END-15,time.monotonic()+120)
 for label,item in artifacts.items():att.attest(C,APP,Path(item['binary']),item['artifact'],item['builds'],P,label,attest_limit)
 largenames,largeignored=inventory('large','',large,sdk='export');assert EXPECTED|LAYER_REQUIRED|ED_REQUIRED|{UI_CASE}<=set(largenames)-set(largeignored)
 short=set()
 for src in (C/'src').rglob('*.rs'):
  lines=src.read_text().splitlines()
  for i,line in enumerate(lines):
   if 'feature = "navigateur"), ignore' in line:short.update(re.findall(r'fn ([a-z0-9_]+)\(', '\n'.join(lines[i:i+5])))
 filters=sorted(short);assert filters
 listed=run('nav-list',[str(nav),*filters,'--list','--format','terse'],30,sdk='export');ignored=run('nav-ignored',[str(nav),*filters,'--list','--ignored','--format','terse'],30,sdk='export')
 navnames=[line[:-6] for line in listed.splitlines() if line.endswith(': test')];assert len(navnames)==40 and ': test' not in ignored
 cpunames,cpuignored=inventory('cpu',PREFIX+'admission::tests::',binary);assert set(cpunames)==EXPECTED and not cpuignored
 pubnames,pubignored=inventory('publication',PREFIX,binary);assert len(pubnames)==25 and EXPECTED<=set(pubnames) and not pubignored
 save('artefacts.json',artifacts)
 save('inventaire-avant-runtime.json',dict(app=APP,pilot=PILOT,artifacts=artifacts,cpu=cpunames,cpu_ignored=cpuignored,publication=pubnames,publication_ignored=pubignored,nav=navnames,large=largenames,large_ignored=largeignored,ed_required=sorted(ED_REQUIRED),layer_required=sorted(LAYER_REQUIRED),deadline=END))
 ledger=digest(P/'inventaire-avant-runtime.json');phase='attente-inventaire-publie';state('EN_COURS');print('INVENTAIRE_A_PUBLIER',ledger,flush=True)
 while not (P/'inventaire-publie').exists():assert time.monotonic()<END-15;memory();time.sleep(.1)
 assert (P/'inventaire-publie').read_text().strip()==ledger
 runtime_expected={key:E[key] for key in ['CARGO_MANIFEST_DIR','CARGO_TARGET_DIR','CARGO_BUILD_JOBS','HOME','TMPDIR','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_CACHE_HOME','XDG_STATE_HOME','RAG3WEAVER_STATE_DIR',*['RAG3WEAVER_BURN_DEVICE'+x for x in ['','_EMBEDDER','_RERANKER','_OCR']]]}
 save('stable-config.json',dict(runtime_expected=runtime_expected,expected=SDK_PROFILE,exe=str(large),sha256=artifacts['large']['sha256'],cgroup=CG,proof=str(P),deadline=min(END-15,time.monotonic()+600)))
 script=P/'large-stable.gdb'
 script.write_text("set confirm off\nset pagination off\nset debuginfod enabled off\nset startup-with-shell off\nset follow-exec-mode same\nset follow-fork-mode child\nset detach-on-fork on\nset print thread-events off\ncatch exec\nrun\npython\nexec(compile(PathHelper.read_text(),str(PathHelper),'exec'))\ncapture('exec',False)\nend\ntbreak main\ncontinue\npython\ncapture('main',True)\nend\ndetach\npython\nwait_for_real_exit()\nend\n".replace("PathHelper", "__import__('pathlib').Path("+repr(str(Q/'capture-stable.py'))+")"))
 command=['gdb','--nx','--batch','--return-child-result','-x',str(script),'--args',shutil.which('python3'),str(Q/'superviser-elf.py'),str(P),str(Q/'sdk-exec.py'),str(large)]
 text=run('lib-large',command,600,sdk='gdb')
 main_large=json.loads((P/'large-main-process.json').read_text())
 real=json.loads((P/'elf-exit-real.json').read_text());detached=json.loads((P/'elf-detached.json').read_text())
 assert real['pid']==detached['pid']==main_large['pid'] and real['starttime']==detached['starttime']==main_large['starttime']
 assert real['exit_code']==0 and real['wait_status']==0 and detached['tracer_pid']==0
 counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 assert counts and tuple(map(int,counts[-1]))==(len(largenames)-len(largeignored),0,len(largeignored))
 assert not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',text)
 successes=set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',text,re.M));assert set(largenames)-set(largeignored)<=successes
 attestation=json.loads((P/'large-main-attestation.json').read_text())
 assert attestation['pid']==main_large['pid'] and attestation['starttime']==main_large['starttime']
 assert attestation['exe']==str(large) and attestation['exe_sha256']==artifacts['large']['sha256'] and attestation['cgroup']==CG
 assert attestation['native'] and attestation['env']==SDK_PROFILE and attestation['phase']=='lib-large'
 sdk_launch=json.loads((P/'sdk-env-lib-large.json').read_text())
 assert sdk_launch['pid']==main_large['pid'] and sdk_launch['starttime']==main_large['starttime']
 assert sdk_launch['cgroup'].strip().split('::',1)[1]==CG and sdk_launch['env']==attestation['env']
 assert [line for line in (P/attestation['maps']).read_text().splitlines() if 'librag3db.so' in line]==attestation['native']
 assert (P/'large-real-sdk-env.json').exists();qualification['large']='VERT'
 ui_limit=min(END-15,time.monotonic()+60)
 witnesses=[json.loads(line.split('UI_ASSET_WITNESS ',1)[1]) for line in text.splitlines() if 'UI_ASSET_WITNESS ' in line]
 assert {v['asset'] for v in witnesses}=={'app.js','app.css'} and len(witnesses)==2
 for v in witnesses:
  rel='templates/ide/app/'+v['asset'];data=subprocess.check_output(['git','-C',str(W),'show',APP+':extension/rag3weaver/'+rel])
  assert v['status']==200 and v['route']=='/ide/'+v['asset'] and v['source']==str(C/rel) and v['bytes']==len(data) and v['sha256']==hashlib.sha256(data).hexdigest()
 assert time.monotonic()<ui_limit
 save('assets-http-contre-git.json',witnesses);qualification['assets']='DEUX_CORPS_HTTP_IDENTIQUES_AUX_BLOBS_GIT'
 for index,name in enumerate(cpunames):judged('exact-'+str(index),['--exact',name],1,0,30,attest_main=index==0)
 qualification['cpu']='VERT_15_EXACTS'
 judged('publication',[PREFIX],25,0,120);qualification['publication']='VERT_25_SUR_25'
 navtmp=Path(tempfile.mkdtemp(prefix='cx-garde-nav.',dir='/var/tmp'));(navtmp/'.possede-garde-target').write_text(APP+'\n');save('nav-tmp.json',dict(path=str(navtmp),app=APP,preserved=True))
 original_tmp=E['TMPDIR'];E['TMPDIR']=str(navtmp)
 text=run('nav40',['bash','scripts/temoins_navigateur.sh',F],360,sdk='script');E['TMPDIR']=original_tmp
 assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)[-1]==('40','0','0')
 successes=set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',text,re.M));assert set(navnames)<=successes
 profile=json.loads((P/'nav-real-profile.json').read_text());assert profile['exe_sha256']==artifacts['navigateur']['sha256'] and profile['cgroup']==CG
 assert any(v['exe']=='/usr/lib/chromium/chromium' for v in seen.values());assert not observe();scan=quiet();save('nav-quiescence.json',scan);assert not scan['found'];qualification['nav']='VERT_40_SUR_40'
 attest_limit=min(END-15,time.monotonic()+120)
 for label,item in artifacts.items():att.attest(C,APP,Path(item['binary']),item['artifact'],item['builds'],P,label,attest_limit)
 for item in artifacts.values():assert digest(item['binary'])==digest(item['copy'])==item['sha256']
 assert digest(LIB)=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
 after=memory();save('memory-final.json',dict(before=before,after=after));assert all(after['events'][k]==before['events'][k] for k in ['high','max','oom','oom_kill'])
 assert not observe();q=quiet();save('quiescence.json',q);assert not q['found'];guard();assert time.monotonic()<END-10;verdict='VERT';state(verdict)
except BaseException as error:
 latch_first_red(repr(error),dict(traceback=traceback.format_exc()))
 state('ROUGE',repr(error));save('error.json',dict(phase=phase,error=repr(error)));print('ROUGE',phase,repr(error),flush=True);stop()
finally:
 evidence();save('qualification.json',qualification);save('verdict.json',dict(verdict=verdict,app=APP,pilot=PILOT,phase=phase,initial=START,deadline=END,elapsed=time.monotonic()-START,phases=phases));(P/'VERDICT').write_text(verdict+'\n')
sys.exit(0 if verdict=='VERT' else 1)
