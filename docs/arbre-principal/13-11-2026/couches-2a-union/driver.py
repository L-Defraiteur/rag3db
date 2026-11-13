#!/usr/bin/env python3
"""A single bounded CPU qualification; compilation and tests are separate claims."""
from pathlib import Path
import hashlib,json,mmap,os,re,shutil,signal,subprocess,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');Q=Path(__file__).parent;W=R/'cx-wt-couches-union-r2';C=W/'extension/rag3weaver';T=R/'cx-target-contenu-derive';P=R/'cx-validation-couches-2a-union-v1'
PROVENANCE=json.loads((Q/'provenance.json').read_text());APP=PROVENANCE['app'];PILOT=PROVENANCE['pilot'];CONTROL=Path(PROVENANCE['control'])
assert APP=='236e1e9f46f7e73eda21ac8cfa765f45a29f4595'
AUTH_PATH=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);AUTH=json.loads(AUTH_PATH.read_text());END=AUTH['echeance'];START=END-4800
assert START<=time.monotonic()<END
assert json.loads((AUTH_PATH.parent/'recette.json').read_text())['delai']==4800
P.mkdir(exist_ok=False);assert T.is_dir() and T.resolve()==T and T.stat().st_uid==os.getuid()
assert not json.loads((Q/'cache-before.json').read_text())['quiescence']
for name in ['tmp','config','data','cache']:(P/name).mkdir()
CG=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1];G=Path('/sys/fs/cgroup'+CG)
assert (G/'memory.max').read_text().strip()=='34359738368' and (G/'memory.high').read_text().strip()=='max'
NATIVE='rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
LIB=R.parent/'rag3db-libs'/NATIVE/'librag3db.so'
E={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','SHELL','LANG','LC_ALL','TZ','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
E.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',LUCIVY_SCHEDULER_THREADS='8',RUST_TEST_THREADS='8',RUST_BACKTRACE='1',TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),RAG3WEAVER_CACHE_DIR=str(P/'cache'),RAG3WEAVER_STATE_DIR=str(P/'state'),POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',RAG3WEAVER_TARGET_MAX_GO='32',RAG3WEAVER_TARGET_REPRENDRE='1')
for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
EXPORT=['bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','native-choice',str(C/'scripts/lib_du_magasin.sh')]
F='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
E['CHECK_FEATURES_LOGS']=str(P/'features');E['CHECK_FEATURES_TIMEOUT']='900'
SDK_PROFILE=dict(RAG3DB_LIB=NATIVE,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=str(LIB.parent),RAG3DB_INCLUDE_DIR=str(LIB.parent),LD_LIBRARY_PATH=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
PREFIX='dataflow::migrations::publication::';LAYER=PREFIX+'layers::tests::'
ED_REQUIRED={'daemon_host::atelier_fichiers::tests::'+name for name in ['read_scope_preserves_a_backend_error_instead_of_claiming_absence','read_scope_preserves_a_usages_error_instead_of_claiming_absence','read_scope_refuses_unproven_uniqueness_after_a_real_definition','read_scope_keeps_real_absence_and_ambiguity_after_complete_queries']}

EXPECTED={LAYER+name for name in ['inherited_values_keep_occurrence_coordinates_and_exact_publication_provenance','tombstones_hide_all_ancestors_and_a_descendant_can_reintroduce_a_file','overrides_do_not_resurrect_an_old_file_and_anchor_advancement_is_not_followed','malformed_piles_and_ambiguous_local_changes_are_refused_without_partial_resolution']}
owned={};seen={};maps={};protected={};devices={};phases=[];phase='preparation';child=None;binary=None;last_size=0;binary_sha=None;artifacts={};large=None;run_binary=None

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

def observe():
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
  try:
   cg=(q/'cgroup').read_text().strip().split('::',1)[1];maximum=(Path('/sys/fs/cgroup'+cg)/'memory.max').read_text().strip()
   assert cg==CG and maximum=='34359738368',('confinement',pid,cg,maximum)
   verified=True;field='exe';exe=os.readlink(q/'exe');seen[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,memory_max=maximum)
   field='fd'
   for fd in (q/'fd').iterdir():
    try:dest=os.readlink(fd)
    except (FileNotFoundError,ProcessLookupError):continue
    if dest.startswith('/dev/dri/') or dest=='/dev/kfd':devices[(pid,stamp,dest)]=dict(pid=pid,starttime=stamp,device=dest)
   if phase=='features' and Path(exe).name=='cargo':
    field='environ';raw=dict(x.split(b'=',1) for x in (q/'environ').read_bytes().split(b'\0') if b'=' in x)
    actual={k:os.fsdecode(raw[os.fsencode(k)]) for k in SDK_PROFILE};assert actual==SDK_PROFILE
    save('features-real-sdk-env.json',dict(pid=pid,starttime=stamp,cgroup=cg,env=actual))
   if exe.startswith(str(T)+'/'):
    field='maps';data=(q/'maps').read_text();native=[line for line in data.splitlines() if 'librag3db.so' in line]
    assert all('/'+NATIVE+'/librag3db.so' in line for line in native),('wrong native maps',pid)
    if (pid,stamp) not in maps:
     filename=f'maps-{pid}-{stamp}.txt';(P/filename).write_text(data);maps[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,file=filename,native=native,phase=phase)
    if large and exe==str(large) and phase=='lib-large' and child is not None and pid==child.pid:
     field='environ';raw=dict(x.split(b'=',1) for x in (q/'environ').read_bytes().split(b'\0') if b'=' in x)
     actual={k:os.fsdecode(raw[os.fsencode(k)]) for k in SDK_PROFILE};assert actual==SDK_PROFILE
     save('large-real-sdk-env.json',dict(pid=pid,starttime=stamp,cgroup=cg,exe=exe,env=actual,phase=phase))
    if binary and exe==str(binary):
     field='environ';raw=dict(x.split(b'=',1) for x in (q/'environ').read_bytes().split(b'\0') if b'=' in x)
     wanted=['CARGO_MANIFEST_DIR','CARGO_TARGET_DIR','CARGO_BUILD_JOBS',*['RAG3WEAVER_BURN_DEVICE'+s for s in ['','_EMBEDDER','_RERANKER','_OCR']]]
     profile={k:os.fsdecode(raw[os.fsencode(k)]) for k in wanted};assert all(profile[k]==E[k] for k in wanted)
     save('profil-elf-reel.json',dict(pid=pid,starttime=stamp,env=profile))
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
 try:observe()
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

def run(name,args,cap,sdk=False):
 global child,phase,last_size
 guard();phase=name;limit=min(time.monotonic()+cap,END-15);assert time.monotonic()<limit
 state('EN_COURS');path=P/(name+'.log');start=time.monotonic();print('DEBUT',name,flush=True)
 env=E.copy();command=args
 if sdk:
  env.update(RAG3DB_LIB=NATIVE,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_MOTEUR_ANCIEN='1')
  if sdk=='export':command=EXPORT+['python3',str(Q/'sdk-exec.py'),str(P),name,*args]
  else:assert sdk=='script' # Canonical F8 already exports the SDK once.

 with path.open('w') as f:
  child=subprocess.Popen(command,cwd=C,env=env,stdout=f,stderr=subprocess.STDOUT)
  while True:
   observe();memory()
   assert time.monotonic()<limit,('deadline',name)
   current=path.read_text(errors='replace')
   if name.startswith('exact-') or name=='publication':assert not first_red(current),('first red',name)
   elif name=='lib-large':assert not first_lib_red(current),('first lib FAILED',name)
   elif name=='features':assert not re.search(r'^(?:CHECK_FEATURES )?ROUGE\b',current,re.M),('first F8 red',name)

   if time.monotonic()-last_size>5:
    size=int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0]);save('target-size.json',dict(bytes=size,time=time.monotonic()));assert size<=32*1024**3;last_size=time.monotonic()
   if child.poll() is not None:break
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
  wanted=['CARGO_MANIFEST_DIR','CARGO_TARGET_DIR','CARGO_BUILD_JOBS',*['RAG3WEAVER_BURN_DEVICE'+x for x in ['','_EMBEDDER','_RERANKER','_OCR']]]
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
(proof/'elf-main-maps.txt').write_text((proc/'maps').read_text())
(proof/'elf-main.json').write_text(json.dumps(meta,indent=2))
end
continue
'''.replace('PROOF',repr(str(P))).replace('WANTED',repr(wanted))
  gdb.write_text(body);command=['gdb','--nx','--batch','--return-child-result','-x',str(gdb),'--args',*command]
 text=run(name,command,cap)
 if attest_main:
  meta=json.loads((P/'elf-main.json').read_text());assert meta['exe']==str(binary) and meta['cgroup']==CG
  assert all(meta['env'][k]==E[k] for k in meta['env'])
  native=[line for line in (P/'elf-main-maps.txt').read_text().splitlines() if 'librag3db.so' in line]
  assert all('/'+NATIVE+'/librag3db.so' in line for line in native)
  save('elf-main-native.json',dict(rows=native,native_runtime_qualified=False))

 counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 assert counts and tuple(map(int,counts[-1]))==(expected,0,ignored),(name,counts)
 assert not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',text)
 return text

def interrupted(sig,_):raise RuntimeError('signal '+str(sig))
for sig in [signal.SIGTERM,signal.SIGINT,signal.SIGHUP]:signal.signal(sig,interrupted)
verdict='ROUGE';qualification=dict(features='NON_QUALIFIE',large='NON_QUALIFIE',cpu='NON_QUALIFIE',publication='NON_QUALIFIE',minimal_native_check='NON_QUALIFIE');before=memory()
def feature_closure(features):
 import tomllib
 definitions=tomllib.loads((C/'Cargo.toml').read_text())['features']
 result=set(features.split(',') if features else []);pending=list(result)
 while pending:
  for value in definitions[pending.pop()]:
   if value in definitions and value not in result:result.add(value);pending.append(value)
 return result

def compile_elf(label,features,cap,sdk=False):
 args=['cargo','test','--lib','--no-default-features']
 if features:args+=['--features',features]
 text=run(label,args+['--no-run','--message-format=json-render-diagnostics'],cap,sdk=sdk);paths={}
 for line in text.splitlines():
  try:item=json.loads(line)
  except ValueError:continue
  if item.get('reason')=='compiler-artifact' and item.get('target',{}).get('name')=='rag3weaver' and item.get('profile',{}).get('test') and item.get('executable'):paths[item['executable']]=item['features']
 assert len(paths)==1
 path=Path(next(iter(paths)));assert path.is_relative_to(T)
 assert set(paths[str(path)])==feature_closure(features)
 with path.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:assert data.find(APP[:9].encode())>=0
 sha=digest(path);copy=P/('binaire-'+label);shutil.copy2(path,copy);assert digest(copy)==sha
 artifacts[label]=dict(binary=str(path),copy=str(copy),sha256=sha,size=path.stat().st_size,features=sorted(paths[str(path)]),requested_features=features.split(',') if features else [],app=APP)
 return path
try:
 guard();assert digest(LIB)=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
 save('source.json',dict(app=APP,pilot=PILOT,worktree=str(W),control=str(CONTROL),target=str(T),target_reused_from='69dc3fc25910f3e834652264edf6c71cca862ce0',initial=START,deadline=END,budget=4800,receipt=str(AUTH_PATH),cpu_features=[],large_features=F.split(','),native=NATIVE,driver_sha256=digest(__file__),cgroup=CG,memory_max=(G/'memory.max').read_text().strip(),memory_high=(G/'memory.high').read_text().strip(),caps=dict(features=2100,compile_large=900,compile_cpu=480,large_runtime=600,each_exact=60,publication=120),environment_keys=sorted(E),cpu_roles=4))
 run('verifier',['python3',str(Q/'confiner_validation.py'),'verifier'],30)
 save('ledger-preparation.json',dict(app=APP,pilot=PILOT,check_features_sha256=digest(C/'scripts/check_features.sh'),target_provenance=json.loads((Q/'cache-before.json').read_text()),sdk=NATIVE,sdk_sha256=digest(LIB)))
 f8=run('features',['bash','scripts/check_features.sh'],2100,sdk='script');assert 'CHECK_FEATURES VERT 8/8' in f8
 assert (P/'features-real-sdk-env.json').exists()
 qualification['features']='VERT_8_SUR_8';qualification['minimal_native_check']='COMPILATION_F8_JEU8_CHECK_TESTS_SANS_RUNTIME'
 large=compile_elf('large',F,900,sdk='export');binary=compile_elf('cpu','',480);binary_sha=artifacts['cpu']['sha256']
 largenames,largeignored=inventory('large','',large,sdk='export');assert EXPECTED|ED_REQUIRED<=set(largenames)-set(largeignored)
 names,ignored=inventory('layers',LAYER);assert set(names)==EXPECTED and not ignored
 pubnames,pubignored=inventory('publication',PREFIX);assert len(pubnames)==10 and EXPECTED<=set(pubnames) and not pubignored
 save('artefacts.json',artifacts)
 save('inventaire-avant-runtime.json',dict(app=APP,pilot=PILOT,artifacts=artifacts,layers=names,ignored=ignored,publication=pubnames,publication_ignored=pubignored,large=largenames,large_ignored=largeignored,ed_required=sorted(ED_REQUIRED),deadline=END))
 ledger=digest(P/'inventaire-avant-runtime.json');phase='attente-inventaire-publie';state('EN_COURS');print('INVENTAIRE_A_PUBLIER',ledger,flush=True)
 while not (P/'inventaire-publie').exists():assert time.monotonic()<END-15;memory();time.sleep(.1)
 assert (P/'inventaire-publie').read_text().strip()==ledger
 text=run('lib-large',[str(large),'--show-output','--test-threads=8'],600,sdk='export')
 counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 assert counts and tuple(map(int,counts[-1]))==(len(largenames)-len(largeignored),0,len(largeignored))
 assert not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',text)
 successes=set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',text,re.M));assert set(largenames)-set(largeignored)<=successes
 assert any(m['exe']==str(large) and m['native'] and m['phase']=='lib-large' for m in maps.values()),'actual large SDK maps missing'
 assert (P/'large-real-sdk-env.json').exists();qualification['large']='VERT'
 for index,name in enumerate(names):judged('exact-'+str(index),['--exact',name],1,0,60,attest_main=index==0)
 qualification['cpu']='VERT'
 judged('publication',[PREFIX],10,0,120);qualification['publication']='VERT'
 for item in artifacts.values():assert digest(item['binary'])==digest(item['copy'])==item['sha256']
 assert digest(LIB)=='b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929'
 assert (P/'elf-main-maps.txt').is_file() and (P/'elf-main.json').is_file()
 after=memory();save('memory-final.json',dict(before=before,after=after));assert all(after['events'][k]==before['events'][k] for k in ['high','max','oom','oom_kill'])
 assert not observe();q=quiet();save('quiescence.json',q);assert not q['found'];guard();assert time.monotonic()<END-10;verdict='VERT';state(verdict)
except BaseException as error:
 state('ROUGE',repr(error));save('error.json',dict(phase=phase,error=repr(error)));print('ROUGE',phase,repr(error),flush=True);stop()
finally:
 evidence();save('qualification.json',qualification);save('verdict.json',dict(verdict=verdict,app=APP,pilot=PILOT,phase=phase,initial=START,deadline=END,elapsed=time.monotonic()-START,phases=phases));(P/'VERDICT').write_text(verdict+'\n')
sys.exit(0 if verdict=='VERT' else 1)
