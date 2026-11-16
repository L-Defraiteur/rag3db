#!/usr/bin/env python3
"""Proposed fresh CPU qualification; no real credentials or provider services."""
import argparse,hashlib,json,os,re,shutil,stat,subprocess,sys,time
from pathlib import Path
APP='c38fc3e4ddd8c806de904c7d08eb43b4988fec99'
BASE='df2a5789888d16247f0dad42dcdaa3c4ceea2094'
R=Path.home()/'.cache/rag3weaver-build'
W=Path.home()/'git_workspaces/rag3db-cx-ed-vertex-token-fix-r1'
C=W/'extension/rag3weaver'
P=R/'cx-ed-vertex-token-green-v1';Q=R/'cx-ed-vertex-token-green-controller-v1';S=R/'cx-ed-vertex-token-green-scope-v1'
T=R/'target-cx-ed-vertex-token-green-v1' # New absent private target; no reuse of red/TIES target.
PREFIX='openai_llm::vertex_refresh_tests::'
POSITIVE=['fixture_token_source_changes_its_cached_token_only_after_invalidation','a_fixed_api_bearer_is_not_refreshed_after_unauthorized','vertex_never_refreshes_or_replays_after_a_stream_fragment']
RED='vertex_rejected_token_is_refreshed_once_before_opening_a_stream'
PERSISTENT='vertex_persistent_unauthorized_is_named_after_one_refresh_only'
EXACTS=[*POSITIVE,RED,PERSISTENT,'declared_vertex_connector_retains_injected_source_and_refreshes','dynamic_vertex_auth_debug_never_discloses_a_token']
FEATURES=['daemon','openai-llm'];BUDGET=1130;OUTER=1198
LIB='rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value,end=None):
 with (root/name).open('x') as stream:
  if end is not None and time.monotonic()>=end:raise RuntimeError('deadline after open before publication')
  json.dump(value,stream,indent=2);stream.flush();os.fsync(stream.fileno())
def first_red(reason):
 try:save(P,'first-red.json',dict(reason=reason,observed=time.monotonic()))
 except FileExistsError:pass
def guard(cfg,end=None):
 for root,head in [(W,APP),(Path(cfg['control']),cfg['pilot'])]:
  assert git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain')
  for name in ['rebase-merge','rebase-apply']:assert not Path(git(root,'rev-parse','--path-format=absolute','--git-path',name)).exists()
 assert git(C/'codeparsers','rev-parse','HEAD')==git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2]
 assert not git(C/'codeparsers','status','--porcelain')
 for name,value in cfg['copies'].items():assert sha(Q/name)==value
 assert sha(cfg['timeout'])==cfg['timeout_sha256']
 for name,value in SO.items():assert sha(R.parent/'rag3db-libs'/LIB/name)==value
 if end is not None:assert time.monotonic()<end-15

def run(label,args,env,end,cap):
 limit=min(end-15,time.monotonic()+cap);assert time.monotonic()<limit
 save(P,label+'.command.json',dict(command=args,deadline=limit))
 with (P/(label+'.log')).open('x') as output:
  child=subprocess.Popen(args,cwd=C,env=env,stdout=output,stderr=subprocess.STDOUT)
  while True:
   assert time.monotonic()<limit,label+' timeout'
   if T.exists():assert int(subprocess.check_output(['du','-sb',str(T)],text=True).split()[0])<=32*1024**3
   current=(P/(label+'.log')).read_text(errors='replace')
   if re.search(r'^test .* \.\.\. FAILED\b|^test result: FAILED\.|panicked at',current,re.M):
    first_red('first failure in '+label);raise RuntimeError('first red STOP: '+label)
   if child.poll() is not None:break
   time.sleep(.1)
  rc=child.wait()
 text=(P/(label+'.log')).read_text(errors='replace');save(P,label+'.result.json',dict(rc=rc,observed=time.monotonic()))
 return rc,text

def short_tmp():
 root=R/('v-'+os.urandom(5).hex());os.mkdir(root,0o700);st=root.lstat()
 assert root.resolve()==root and stat.S_IMODE(st.st_mode)==0o700 and st.st_uid==os.getuid()
 assert subprocess.check_output(['findmnt','-n','-o','FSTYPE','-T',str(root)],text=True).strip() not in ['tmpfs','ramfs','']
 assert len(os.fsencode(root/'pont-9999999999-999999/ecoute/pont.sock'))<=107
 return root

def driver():
 cfg=json.loads((Q/'configuration.json').read_text());guard(cfg)
 subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
 receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
 assert auth['etat']=='actif' and json.loads((receipt.parent/'recette.json').read_text())['delai']==BUDGET
 cg=Path('/sys/fs/cgroup')/Path('/proc/self/cgroup').read_text().strip().split('::',1)[1].lstrip('/')
 limits={name:(cg/name).read_text().strip() for name in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits['memory.max']=='34359738368' and limits['memory.high']=='max' and limits['memory.swap.max']=='0'
 quota,period=limits['cpu.max'].split();assert quota!='max' and 0<int(quota)<=8*int(period)
 save(P,'authority-path.json',dict(path=str(receipt),initial=end-BUDGET,deadline=end,limits=limits))
 assert not T.exists() and not T.is_symlink()
 tmp=short_tmp();save(P,'short-tmp.json',dict(path=str(tmp),uid=tmp.stat().st_uid,device=tmp.stat().st_dev,inode=tmp.stat().st_ino))
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for name in ['cache','config','data','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS=str(min(8,int(os.environ.get('CARGO_BUILD_JOBS','8')))),CARGO_TERM_COLOR='never',RUST_TEST_THREADS='1',RUST_BACKTRACE='1',
  TMPDIR=str(tmp),XDG_CACHE_HOME=str(P/'cache'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=LIB,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824')
 for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
 assert not Path(env['XDG_STATE_HOME']).is_relative_to(Path(env['RAG3WEAVER_STATE_DIR'])) and not Path(env['RAG3WEAVER_STATE_DIR']).is_relative_to(Path(env['XDG_STATE_HOME']))
 save(P,'profile.json',{k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'})
 args=[sys.executable,str(C/'scripts/cargo_garde.py'),'test','--locked','--offline','--no-default-features','--features',','.join(FEATURES),'--lib','--no-run','--message-format=json']
 rc,text=run('compile',args,env,end,900)
 if rc:first_red('compilation failed before any judge');raise RuntimeError('compile refused')
 artifacts=[]
 for line in text.splitlines():
  try:row=json.loads(line)
  except ValueError:continue
  if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='rag3weaver' and row.get('profile',{}).get('test') and row.get('executable'):artifacts.append(row)
 assert len(artifacts)==1 and sorted(artifacts[0]['features'])==FEATURES
 binary=Path(artifacts[0]['executable']);assert binary.resolve()==binary and binary.is_relative_to(T) and binary.is_file()
 with binary.open('rb') as stream:assert stream.read(4)==b'\x7fELF'
 assert (T/'.arbre-du-target').read_text().strip()==str(C)
 inventory=[]
 for label,options in [('inventory',[]),('ignored',['--ignored'])]:
  rc,listed=run(label,[str(binary),'--list','--format','terse',*options],env,end,20);assert rc==0
  names=[line[:-6] for line in listed.splitlines() if line.startswith(PREFIX) and line.endswith(': test')]
  if label=='inventory':inventory=names;assert len(names)==len(EXACTS) and set(names)=={PREFIX+n for n in EXACTS}
  else:assert not names
 copied=P/'unit-tests';shutil.copy2(binary,copied);assert sha(binary)==sha(copied)
 ledger=dict(source=APP,pilot=cfg['pilot'],features=FEATURES,binary=str(binary),copy=str(copied),sha256=sha(binary),size=binary.stat().st_size,inventory=inventory,native_runtime=False)
 save(P,'ledger-before-tests.json',ledger)
 # Agent copies ledger + ELF to the other host, hashes both copies, then publishes this receipt.
 pause_end=min(end-30,time.monotonic()+120)
 while not (P/'ledger-published.json').exists():
  assert time.monotonic()<pause_end,'ledger not published before bounded pause';time.sleep(.2)
 publication=json.loads((P/'ledger-published.json').read_text());assert publication['source']==APP and publication['elf_sha256']==ledger['sha256'] and publication['ledger_sha256']==sha(P/'ledger-before-tests.json') and publication['two_hosts_rehashed'] is True
 guard(cfg,end)
 for i,name in enumerate(EXACTS):
  rc,text=run('exact-'+str(i),[str(binary),PREFIX+name,'--exact','--nocapture'],env,end,30)
  if rc:first_red('fixture refused: '+name);raise RuntimeError(name)
  assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',text)
 guard(cfg,end);assert sha(binary)==sha(copied)==ledger['sha256']
 save(P,'driver-result.json',dict(status='CPU_VERTEX_JUDGES_PASSED',source=APP,pilot=cfg['pilot'],exact_count=len(EXACTS),elf_sha256=ledger['sha256'],deadline=end,credentials_read=False,cloud_contact=False,native_runtime=False),end=end)

def quiet(owned):
 alive=[];unknown=[];found=[];roots=[W,T,Q]
 if (P/'short-tmp.json').exists():roots.append(Path(json.loads((P/'short-tmp.json').read_text())['path']))
 for row in owned:
  proc=Path('/proc')/str(row['pid'])
  try:
   fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
   if fields[19]==row['debut'] and fields[0]!='Z':alive.append(row)
  except (FileNotFoundError,ProcessLookupError):pass
  except OSError as error:unknown.append(dict(owned=True,pid=row['pid'],error=repr(error)))
 for proc in Path('/proc').iterdir():
  if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
  try:
   if proc.stat().st_uid!=os.getuid():continue
   fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
   if fields[0]=='Z':continue
   values=[os.readlink(proc/'exe'),os.readlink(proc/'cwd')]
   for fd in (proc/'fd').iterdir():
    try:values.append(os.readlink(fd))
    except (FileNotFoundError,ProcessLookupError):pass
   for value in values:
    if any(value==str(root) or value.startswith(str(root)+'/') for root in roots):found.append(dict(pid=proc.name,start=fields[19],path=value))
  except (FileNotFoundError,ProcessLookupError):pass
  except OSError as error:unknown.append(dict(owned=False,pid=proc.name,error=repr(error)))
 return dict(owned_alive=alive,unknown=unknown,found=found,observed=time.monotonic())

def compose():
 import importlib.util
 cfg=json.loads((Q/'configuration.json').read_text())
 spec=importlib.util.spec_from_file_location('common',Q/'confiner_validation.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 parent=Path('/proc')/str(os.getppid());expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s',str(OUTER)+'s',sys.executable,str(Q/'qualify.py'),'--compose']
 assert os.readlink(parent/'exe')==cfg['timeout'] and [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]==expected
 assert common.poste_tenu(common.processus())
 result=dict(status='RED',returncode=1)
 try:
  guard(cfg)
  command=[sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai',str(BUDGET),'--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver']
  rc=subprocess.run(command,cwd='/var/tmp').returncode
  authority_path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(authority_path.read_text());scope=json.loads((authority_path.parent/'verdict.json').read_text());end=auth['echeance']
  assert rc==0 and auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
  assert json.loads((authority_path.parent/'recette.json').read_text())['delai']==BUDGET
  driver=json.loads((P/'driver-result.json').read_text());assert driver['status']=='CPU_VERTEX_JUDGES_PASSED' and driver['exact_count']==len(EXACTS) and not (P/'first-red.json').exists()
  ledger=json.loads((P/'ledger-before-tests.json').read_text());assert sha(ledger['binary'])==sha(ledger['copy'])==ledger['sha256']==driver['elf_sha256']
  observed=quiet(json.loads((authority_path.parent/'enfants.json').read_text()));save(P,'quiescence-final.json',observed)
  assert not observed['owned_alive'] and not observed['found'] and not any(x['owned'] for x in observed['unknown'])
  guard(cfg);assert time.monotonic()<end
  result=dict(status='CPU_VERTEX_CHECKED',returncode=0,source=APP,pilot=cfg['pilot'],scope=scope,authority_terminated=True,bus_absent=True,deadline=end,elapsed=time.monotonic()-(end-BUDGET),native_runtime=False,cloud_contact=False,real_oauth_qualified=False)
 except BaseException as error:
  first_red(repr(error));result=dict(status='RED',returncode=1,error=repr(error))
 save(P,'composed-result.json',result,end=result.get('deadline'));print(json.dumps(result));return result['returncode']

def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 control=Path(__file__).resolve().parents[4]
 assert git(control,'rev-parse','HEAD')==args.pilot and not git(control,'status','--porcelain')
 assert not any(x.exists() or x.is_symlink() for x in [S,T])
 for path in [P,Q]:assert not path.is_symlink() and (not path.exists() or (path.is_dir() and path.resolve()==path and path.stat().st_uid==os.getuid()))
 if P.exists():assert not any(P.iterdir()),'prepared proof root must still be empty'
 assert git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain')
 Q.mkdir(mode=0o700,exist_ok=True);P.mkdir(mode=0o700,exist_ok=True)
 copies={}
 for name,source in [('qualify.py',Path(__file__)),('poste',C/'scripts/poste'),('confiner_validation.py',C/'scripts/confiner_validation.py')]:
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(source)
  else:shutil.copy2(source,dest)
  copies[name]=sha(dest)
 timeout=str(Path(shutil.which('timeout')).resolve())
 cfg=dict(app=APP,pilot=args.pilot,control=str(control),copies=copies,timeout=timeout,timeout_sha256=sha(timeout))
 if (Q/'configuration.json').exists():assert json.loads((Q/'configuration.json').read_text())==cfg
 else:save(Q,'configuration.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=True,cargo=False,authority=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
 env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 cmd=[str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s',str(OUTER)+'s',sys.executable,str(Q/'qualify.py'),'--compose']
 save(Q,'launch.json',dict(command=cmd,allocation=args.allocation))
 with (P/'outer.log').open('x') as output:rc=subprocess.run(cmd,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT).returncode
 return rc # Parent does nothing after bounded composition except relay the exit status.
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--pilot');parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--allocation');parser.add_argument('--driver',action='store_true');parser.add_argument('--compose',action='store_true');args=parser.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:
   if P.exists():first_red(repr(error))
   raise
 else:sys.exit(launch(args))
