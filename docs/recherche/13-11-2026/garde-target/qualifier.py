#!/usr/bin/env python3
"""Reviewed preparation only; launch requires the exact app/pilot coordination."""
from pathlib import Path
import hashlib,json,mmap,os,re,shutil,signal,subprocess,sys,time
R=Path('/home/lucied/.cache/rag3weaver-build');W=R/'cx-wt-recherche-garde-target';C=W/'extension/rag3weaver'
APP='698b952ab89fd48ac112645650000ebae34e6742'
P=R/'cx-validation-garde-target-v1';Q=R/'cx-pilote-garde-target-v1';S=R/'cx-confinement-garde-target-v1';T=R/'cx-target-garde-target-qualification-v1'
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(n,v):(P/n).write_text(json.dumps(v,indent=2)+'\n')
def git(root,*a):return subprocess.check_output(['git','-C',str(root),*a],env=dict(os.environ,GIT_OPTIONAL_LOCKS='0'))
def guard(control,pilot):
 if (Q/'provenance.json').exists():
  for n,h in json.loads((Q/'provenance.json').read_text())['files'].items():assert sha(Q/n)==h
 for root,expected in [(W,APP),(control,pilot)]:
  assert git(root,'rev-parse','HEAD').decode().strip()==expected
  assert not git(root,'status','--porcelain').strip()
  for name in ['rebase-merge','rebase-apply']:assert not Path(git(root,'rev-parse','--path-format=absolute','--git-path',name).decode().strip()).exists()
def quiet(control):
 found=[];unknown=[]
 for p in Path('/proc').iterdir():
  if not p.name.isdigit() or int(p.name)==os.getpid():continue
  try:
   if p.stat().st_uid!=os.getuid():continue
   st=(p/'stat').read_text().rsplit(') ',1)[1].split()
   if st[0]=='Z':continue
   for item in [p/'exe',p/'cwd',*list((p/'fd').iterdir())]:
    try:
     v=os.readlink(item)
     if any(v==str(root) or v.startswith(str(root)+'/') for root in [T,W,control]):found.append(dict(pid=p.name,start=st[19],field=str(item),value=v))
    except FileNotFoundError:pass
  except PermissionError:unknown.append(p.name)
  except (FileNotFoundError,ProcessLookupError):pass
 return dict(found=found,unknown_baseline=unknown)
def attester(binary,artifact,builds):
 paths=git(W,'ls-tree','-rz',APP,'--','extension/rag3weaver/templates').split(b'\0');expected={}
 for row in paths:
  if not row:continue
  meta,path=row.split(b'\t');mode,kind,oid=meta.split();assert kind==b'blob' and mode in (b'100644',b'100755')
  rel=path.decode().removeprefix('extension/rag3weaver/')
  data=git(W,'cat-file','blob',oid.decode());assert (C/rel).read_bytes()==data;expected[rel]=data
 assert len(expected)==223
 out=Path(builds[artifact['package_id']])/'builtin_templates.rs';generated=out.read_text()
 # Parse only the two JSON-compatible quoted strings from Rust debug output.
 references={}
 for line in generated.splitlines():
  m=re.match(r'^    \(("(?:[^"\\]|\\.)*"), include_bytes!\(("(?:[^"\\]|\\.)*")\)\),$',line)
  if m:references['templates/'+json.loads(m[1])]=json.loads(m[2])
 assert set(references)==set(expected)
 assert all(references[k]==str(C/k) for k in expected)
 with binary.open('rb') as f,mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ) as data:
  assert data.find(APP[:9].encode())>=0
  assert all(data.find(v)>=0 for v in expected.values())
 save('templates-contre-git.json',dict(app=APP,count=len(expected),files={k:hashlib.sha256(v).hexdigest() for k,v in expected.items()},generated=str(out),generated_sha256=sha(out),include_paths=references,elf_sha256=sha(binary)))
def driver():
 provenance=json.loads((Q/'provenance.json').read_text());control=Path(provenance['control']);pilot=provenance['pilot'];guard(control,pilot)
 auth=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);end=json.loads(auth.read_text())['echeance'];start=end-1800
 assert json.loads((auth.parent/'recette.json').read_text())['delai']==1800
 assert not T.exists();P.mkdir();env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','TZ','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for n in ['tmp','cache','data','config','state']:(P/n).mkdir()
 env.update(CARGO_TARGET_DIR=str(T),CARGO_MANIFEST_DIR=str(C),CARGO_BUILD_JOBS='8',CARGO_INCREMENTAL='0',RUST_TEST_THREADS='8',TMPDIR=str(P/'tmp'),XDG_CACHE_HOME=str(P/'cache'),XDG_DATA_HOME=str(P/'data'),XDG_CONFIG_HOME=str(P/'config'),XDG_STATE_HOME=str(P/'state'),RAG3WEAVER_STATE_DIR=str(P/'state'),RAG3WEAVER_CACHE_DIR=str(P/'cache'))
 for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
 cg=Path('/proc/self/cgroup').read_text().strip().split('::',1)[1];group=Path('/sys/fs/cgroup'+cg)
 assert (group/'memory.max').read_text().strip()=='34359738368' and (group/'memory.high').read_text().strip()=='max'
 save('source.json',dict(app=APP,pilot=pilot,control=str(control),receipt=str(auth),start=start,end=end,cgroup=cg))
 events=lambda:dict(line.split() for line in (group/'memory.events').read_text().splitlines())
 before=events();save('memory-before.json',before)
 def run(name,args,cap):
  guard(control,pilot);limit=min(end-15,time.monotonic()+cap);assert time.monotonic()<limit
  save('phase.json',dict(phase=name,deadline=limit,args=args));path=P/(name+'.log')
  with path.open('w') as f:
   child=subprocess.Popen(args,cwd=C,env=env,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
   try:
    while True:
     text=path.read_text(errors='replace')
     assert not re.search(r'^test .* \.\.\. FAILED$|^FAILED\b|^FAIL:|^ERROR:|^ÉCHEC',text,re.M),('first red',name)
     assert time.monotonic()<limit,('timeout',name)
     if T.exists():assert int(subprocess.check_output(['du','-sb',str(T)]).split()[0])<=32*1024**3
     if child.poll() is not None:break
     time.sleep(.05)
    assert child.wait()==0,(name,child.returncode)
   except BaseException:
    if child.poll() is None:
     os.killpg(child.pid,signal.SIGTERM)
     try:child.wait(timeout=2)
     except subprocess.TimeoutExpired:os.killpg(child.pid,signal.SIGKILL);child.wait()
    raise
  guard(control,pilot);return path.read_text(errors='replace')
 try:
  run('verifier',['python3',str(Q/'confiner_validation.py'),'verifier'],30)
  cpu=run('cpu',['python3','-B','scripts/test_garde_target_cpu.py','-v'],60);assert re.search(r'Ran 10 tests',cpu) and cpu.rstrip().endswith('OK')
  # Toy fixture owns its own target; the product target remains untouched here.
  toyenv=env.copy();env.pop('CARGO_TARGET_DIR');toy=run('toy',['bash','scripts/test_garde_target.sh'],180);assert len(re.findall(r'^OK ',toy,re.M))==8;env.update(toyenv)
  journals=run('journaux',['python3','-B','scripts/test_journaux_demons_e2e.py','-v'],240);assert re.search(r'Ran 7 tests',journals) and re.search(r'^OK$',journals,re.M)
  text=run('compile',['python3','-B','scripts/cargo_garde.py','test','--lib','--no-default-features','--no-run','--message-format=json-render-diagnostics'],900)
  artifacts=[];builds={}
  for line in text.splitlines():
   try:v=json.loads(line)
   except ValueError:continue
   if v.get('reason')=='build-script-executed':builds[v['package_id']]=v['out_dir']
   if v.get('reason')=='compiler-artifact' and v.get('target',{}).get('name')=='rag3weaver' and v['profile']['test'] and v.get('executable'):artifacts.append(v)
  assert len(artifacts)==1 and artifacts[0]['features']==[];artifact=artifacts[0];binary=Path(artifact['executable']);assert binary.is_relative_to(T)
  attester(binary,artifact,builds);shutil.copy2(binary,P/'binaire-cpu');digest=sha(binary);assert sha(P/'binaire-cpu')==digest
  names=run('inventory',[str(binary),'builtin_templates::tests::','--list','--format','terse'],30)
  ignored=run('ignored',[str(binary),'builtin_templates::tests::','--list','--ignored','--format','terse'],30)
  names=[l[:-6] for l in names.splitlines() if l.endswith(': test')];assert names and ': test' not in ignored
  save('ledger.json',dict(app=APP,pilot=pilot,binary=str(binary),sha256=digest,names=names,features=[]));ledger=sha(P/'ledger.json');print('INVENTAIRE_A_PUBLIER',ledger,flush=True)
  while not (P/'inventaire-publie').exists():assert time.monotonic()<end-15;time.sleep(.1)
  assert (P/'inventaire-publie').read_text().strip()==ledger
  text=run('templates',[str(binary),'builtin_templates::tests::','--show-output','--test-threads=8'],120)
  assert re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)[-1]==(str(len(names)),'0','0')
  attester(binary,artifact,builds);assert sha(binary)==sha(P/'binaire-cpu')==digest
  after=events();save('memory-after.json',dict(events=after,peak=int((group/'memory.peak').read_text())));assert all(before[k]==after[k] for k in ['high','max','oom','oom_kill'])
  guard(control,pilot);assert time.monotonic()<end-10;save('driver-verdict.json',dict(verdict='VERT'))
 except BaseException as e:save('driver-verdict.json',dict(verdict='ROUGE',error=repr(e)));raise

def main():
 if sys.argv[1:] == ['--driver']:driver();return
 pilot=sys.argv[1];assert re.fullmatch('[0-9a-f]{40}',pilot);control=Path(__file__).resolve().parents[4]
 assert os.uname().nodename.lower()=='luciepc';guard(control,pilot)
 assert (C/'codeparsers/Cargo.toml').is_file()
 assert git(C/'codeparsers','rev-parse','HEAD').decode().strip()==git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').decode().split()[2]
 assert not any(p.exists() for p in [P,Q,S,T]);Q.mkdir()
 shutil.copy2(__file__,Q/'qualifier.py');shutil.copy2(C/'scripts/confiner_validation.py',Q/'confiner_validation.py')
 (Q/'provenance.json').write_text(json.dumps(dict(app=APP,pilot=pilot,control=str(control),files={n:sha(Q/n) for n in ['qualifier.py','confiner_validation.py']})))
 os.chdir('/var/tmp');env=dict(os.environ,POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0')
 rc=subprocess.run([str(C/'scripts/poste'),'lourd','python3',str(Q/'confiner_validation.py'),'lancer','--delai','1800','--preuves',str(S),'--','python3',str(Q/'qualifier.py'),'--driver'],env=env).returncode
 record=dict(verdict='ROUGE',code=rc)
 try:
  source=json.loads((P/'source.json').read_text());scope=json.loads((Path(source['receipt']).parent/'verdict.json').read_text());assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and scope['restes']==[] and scope['nettoye'] and not Path(scope['prive']).exists()
  assert json.loads((P/'driver-verdict.json').read_text())['verdict']=='VERT'
  scan=quiet(control);save('final-scan.json',scan);assert not scan['found']
  ledger=json.loads((P/'ledger.json').read_text());assert sha(ledger['binary'])==sha(P/'binaire-cpu')==ledger['sha256']
  guard(control,pilot);assert time.monotonic()<source['end'] and time.monotonic()-source['start']<1800
  record.update(verdict='VERT',elapsed=time.monotonic()-source['start'],end=source['end'],scope=scope)
 except BaseException as e:record['error']=repr(e)
 if record['verdict']=='VERT' and time.monotonic()>=record['end']:record.update(verdict='ROUGE',error='deadline before publication')
 P.mkdir(exist_ok=True);save('verdict-compose.json',record);print(json.dumps(record));sys.exit(0 if record['verdict']=='VERT' else 1)
if __name__=='__main__':main()
