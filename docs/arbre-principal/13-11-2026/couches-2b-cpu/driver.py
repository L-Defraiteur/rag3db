#!/usr/bin/env python3
"""A single bounded CPU qualification; compilation and tests are separate claims."""
from pathlib import Path
import hashlib,json,mmap,os,re,shutil,signal,subprocess,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');Q=Path(__file__).parent;W=R/'cx-wt-couches-2b-admission';C=W/'extension/rag3weaver';T=R/'cx-target-contenu-derive';P=R/'cx-validation-couches-2b-a-v1'
PROVENANCE=json.loads((Q/'provenance.json').read_text());APP=PROVENANCE['app'];PILOT=PROVENANCE['pilot'];CONTROL=Path(PROVENANCE['control'])
assert APP=='affa6d03b5a57dd99810b6853783dc1a267f76b9'
AUTH_PATH=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);AUTH=json.loads(AUTH_PATH.read_text());END=AUTH['echeance'];START=END-1800
assert START<=time.monotonic()<END
assert json.loads((AUTH_PATH.parent/'recette.json').read_text())['delai']==1800
P.mkdir(exist_ok=False);assert not T.exists(),'target must start absent';T.mkdir()
for name in ['tmp','config','data','cache']:(P/name).mkdir()
CG=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1];G=Path('/sys/fs/cgroup'+CG)
assert (G/'memory.max').read_text().strip()=='34359738368' and (G/'memory.high').read_text().strip()=='max'
E={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','SHELL','LANG','LC_ALL','TZ','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
E.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',LUCIVY_SCHEDULER_THREADS='8',RUST_TEST_THREADS='8',RUST_BACKTRACE='1',TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),RAG3WEAVER_CACHE_DIR=str(P/'cache'),RAG3WEAVER_STATE_DIR=str(P/'state'),POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',RAG3WEAVER_TARGET_MAX_GO='32')
for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:E['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
PREFIX='dataflow::migrations::publication::';ADMISSION=PREFIX+'admission::tests::'
EXPECTED=set(json.loads((Q/'expected.json').read_text())['exacts']);assert len(EXPECTED)==15 and all(n.startswith(ADMISSION) for n in EXPECTED)
owned={};seen={};maps={};protected={};devices={};phases=[];phase='preparation';child=None;binary=None;last_size=0;binary_sha=None

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
  try:
   cg=(q/'cgroup').read_text().strip().split('::',1)[1];maximum=(Path('/sys/fs/cgroup'+cg)/'memory.max').read_text().strip()
   assert cg==CG and maximum=='34359738368',('confinement',pid,cg,maximum)
   verified=True
   if cleanup:continue
   field='exe';exe=os.readlink(q/'exe');seen[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,memory_max=maximum)
   field='fd'
   for fd in (q/'fd').iterdir():
    try:dest=os.readlink(fd)
    except (FileNotFoundError,ProcessLookupError):continue
    if dest.startswith('/dev/dri/') or dest=='/dev/kfd':devices[(pid,stamp,dest)]=dict(pid=pid,starttime=stamp,device=dest)
   if exe.startswith(str(T)+'/'):
    field='maps';data=(q/'maps').read_text();native=[line for line in data.splitlines() if 'librag3db.so' in line]
    if (pid,stamp) not in maps:
     filename=f'maps-{pid}-{stamp}.txt';(P/filename).write_text(data);maps[(pid,stamp)]=dict(pid=pid,starttime=stamp,exe=exe,cgroup=cg,file=filename,native=native)
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

def run(name,args,cap):
 global child,phase,last_size
 guard();phase=name;limit=min(time.monotonic()+cap,END-15);assert time.monotonic()<limit
 state('EN_COURS');path=P/(name+'.log');start=time.monotonic();print('DEBUT',name,flush=True)
 env=E.copy();command=args
 with path.open('w') as f:
  child=subprocess.Popen(command,cwd=C,env=env,stdout=f,stderr=subprocess.STDOUT)
  while True:
   observe();memory()
   assert time.monotonic()<limit,('deadline',name)
   current=path.read_text(errors='replace')
   if name.startswith('exact-'):assert not first_red(current),('first red',name)
   elif name=='lib-cpu':assert not re.search(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$',current,re.M),('first lib FAILED',name)
   if time.monotonic()-last_size>5:
    size=int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0]);save('target-size.json',dict(bytes=size,time=time.monotonic()));assert size<=32*1024**3;last_size=time.monotonic()
   if child.poll() is not None:break
   time.sleep(.025)
  rc=child.wait();child=None
 size=int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0]);save('target-size.json',dict(bytes=size,time=time.monotonic()));assert size<=32*1024**3
 phases.append(dict(name=name,rc=rc,elapsed=time.monotonic()-start,cap=cap,effective_deadline=limit));evidence();state('EN_COURS');assert rc==0,(name,rc)
 guard();print('VERT',name,flush=True);return path.read_text(errors='replace')
def inventory(label,prefix):
 text=run(label+'-list',[str(binary),prefix,'--list','--format','terse'],30)
 ignored=run(label+'-ignored',[str(binary),prefix,'--list','--ignored','--format','terse'],30)
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
verdict='ROUGE';qualification=dict(cpu_exacts='NON_QUALIFIE',cpu_lib='NON_QUALIFIE');before=memory()
try:
 guard()
 save('source.json',dict(app=APP,pilot=PILOT,worktree=str(W),control=str(CONTROL),target=str(T),target_started_absent=True,initial=START,deadline=END,budget=1800,receipt=str(AUTH_PATH),cpu_features=[],native_runtime_qualified=False,driver_sha256=digest(__file__),cgroup=CG,memory_max=(G/'memory.max').read_text().strip(),memory_high=(G/'memory.high').read_text().strip(),caps=dict(target_guard=30,compile=900,each_exact=30,cpu_lib=300),environment_keys=sorted(E),cpu_roles=4))
 run('verifier',['python3',str(Q/'confiner_validation.py'),'verifier'],30)
 run('target-guard',['bash','-c','. "$1"; garder_le_target "$2" "$3"','initial-empty-target',str(Q/'garde_target.sh'),str(T),str(C)],30)
 assert (T/'.arbre-du-target').read_text().strip()==str(C) and not list(T.glob('**/.fingerprint/*'))
 text=run('compile-cpu',['cargo','test','--lib','--no-default-features','--no-run','--message-format=json-render-diagnostics'],900)
 paths={}
 for line in text.splitlines():
  try:item=json.loads(line)
  except ValueError:continue
  if item.get('reason')=='compiler-artifact' and item.get('target',{}).get('name')=='rag3weaver' and item.get('profile',{}).get('test') and item.get('executable'):paths[item['executable']]=item['features']
 assert len(paths)==1,('canonical lib test ELF',paths);binary=Path(next(iter(paths)));assert binary.is_relative_to(T) and paths[str(binary)]==[]
 with binary.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:assert data.find(APP[:9].encode())>=0
 binary_sha=digest(binary);shutil.copy2(binary,P/'binaire-cpu');assert digest(P/'binaire-cpu')==binary_sha
 names,ignored=inventory('admission',ADMISSION);assert set(names)==EXPECTED and not ignored
 pubnames,pubignored=inventory('publication',PREFIX);assert len(pubnames)==25 and EXPECTED<=set(pubnames) and not pubignored
 allnames,allignored=inventory('all','');assert set(pubnames)<=set(allnames)-set(allignored)
 save('inventaire-avant-runtime.json',dict(app=APP,pilot=PILOT,binary=str(binary),sha256=binary_sha,size=binary.stat().st_size,features=[],admission=names,ignored=ignored,publication=pubnames,publication_ignored=pubignored,all=allnames,all_ignored=allignored,deadline=END))
 ledger=digest(P/'inventaire-avant-runtime.json');phase='attente-inventaire-publie';state('EN_COURS');print('INVENTAIRE_A_PUBLIER',ledger,flush=True)
 while not (P/'inventaire-publie').exists():assert time.monotonic()<END-15;memory();time.sleep(.1)
 assert (P/'inventaire-publie').read_text().strip()==ledger
 for index,name in enumerate(names):judged('exact-'+str(index),['--exact',name],1,0,30,attest_main=index==0)
 qualification['cpu_exacts']='VERT_15_EXACTS'
 full=run('lib-cpu',[str(binary),'--show-output','--test-threads=8'],300)
 counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',full)
 assert counts and tuple(map(int,counts[-1]))==(len(allnames)-len(allignored),0,len(allignored))
 successes=set(re.findall(r'^test (.*?)(?: - should panic)? \.\.\. ok$',full,re.M));assert set(allnames)-set(allignored)<=successes
 assert not re.search(r'\b(?:SAUTÉ|SKIPPED)\b',full)
 qualification['cpu_lib']='VERT_CPU_LIB'
 assert digest(binary)==digest(P/'binaire-cpu')==binary_sha
 assert (T/'.arbre-du-target').read_text().strip()==str(C)
 assert (P/'elf-main-maps.txt').is_file() and (P/'elf-main.json').is_file(),'actual CPU main maps/profile missing'
 after=memory();save('memory-final.json',dict(before=before,after=after));assert all(after['events'][k]==before['events'][k] for k in ['high','max','oom','oom_kill'])
 assert not observe();q=quiet();save('quiescence.json',q);assert not q['found'];guard();assert time.monotonic()<END-10;verdict='VERT';state(verdict)
except BaseException as error:
 with (P/'first-red.json').open('x') as f:json.dump(dict(phase=phase,error=repr(error),observed=time.monotonic()),f,indent=2)
 state('ROUGE',repr(error));save('error.json',dict(phase=phase,error=repr(error)));print('ROUGE',phase,repr(error),flush=True);stop()
finally:
 evidence();save('qualification.json',qualification);save('verdict.json',dict(verdict=verdict,app=APP,pilot=PILOT,phase=phase,initial=START,deadline=END,elapsed=time.monotonic()-START,phases=phases));(P/'VERDICT').write_text(verdict+'\n')
sys.exit(0 if verdict=='VERT' else 1)
