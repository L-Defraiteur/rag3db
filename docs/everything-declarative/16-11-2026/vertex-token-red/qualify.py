#!/usr/bin/env python3
"""Prepare/run one CPU-only red witness, never patch production or retry a red."""
import argparse,hashlib,json,os,re,shutil,stat,subprocess,sys,time
from pathlib import Path
APP='fe40b640eb0cc54645cbd552aa13b065ed1d2fa7'
BASE='df2a5789888d16247f0dad42dcdaa3c4ceea2094'
R=Path.home()/'.cache/rag3weaver-build'
W=Path.home()/'git_workspaces/rag3db-cx-ed-vertex-token-refresh-r3'
C=W/'extension/rag3weaver'
P=R/'cx-ed-vertex-token-red-v1';Q=R/'cx-ed-vertex-token-controller-v1';S=R/'cx-ed-vertex-token-scope-v1'
T=R/'target-cx-ed-stable-tie-identity' # Session target, removed after TIES; must now start absent.
PREFIX='openai_llm::vertex_refresh_tests::'
POSITIVE=['fixture_token_source_changes_its_cached_token_only_after_invalidation','a_fixed_api_bearer_is_not_refreshed_after_unauthorized','vertex_never_refreshes_or_replays_after_a_stream_fragment']
RED='vertex_rejected_token_is_refreshed_once_before_opening_a_stream'
PERSISTENT='vertex_persistent_unauthorized_is_named_after_one_refresh_only'
FEATURES=['daemon','openai-llm'];BUDGET=1200
LIB='rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value):
 with (root/name).open('x') as stream:json.dump(value,stream,indent=2);stream.flush();os.fsync(stream.fileno())
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
  if label=='inventory':inventory=names;assert set(names)=={PREFIX+n for n in [*POSITIVE,RED,PERSISTENT]}
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
 for i,name in enumerate(POSITIVE):
  rc,text=run('positive-'+str(i),[str(binary),PREFIX+name,'--exact','--nocapture'],env,end,30)
  if rc:first_red('positive fixture refused: '+name);raise RuntimeError(name)
  assert re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',text)
 rc,text=run('red-refresh',[str(binary),PREFIX+RED,'--exact','--nocapture'],env,end,30)
 # A red is not a green validation; preserve its cause and stop without a correction/retry.
 first_red('refresh witness returned '+str(rc))
 expected=(rc!=0 and re.search(r'test result: FAILED\. 0 passed; 1 failed; 0 ignored;',text) and 'Vertex must refresh one rejected token before retrying' in text and 'HTTP 401' in text)
 guard(cfg,end);assert sha(binary)==sha(copied)==ledger['sha256']
 save(P,'red-observation.json',dict(status='EXPECTED_TRANSPORT_RED_OBSERVED' if expected else 'UNEXPECTED_RESULT',source=APP,pilot=cfg['pilot'],test=PREFIX+RED,rc=rc,elf_sha256=ledger['sha256'],production_fixed=False,declared_connection_qualified=False,persistent_401_judge_played=False))
 raise RuntimeError('STOP after observed red; no patch/retry authorised by this pilot')

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
 cfg=dict(app=APP,pilot=args.pilot,control=str(control),copies=copies)
 if (Q/'configuration.json').exists():assert json.loads((Q/'configuration.json').read_text())==cfg
 else:save(Q,'configuration.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=True,cargo=False,authority=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
 env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 cmd=[str(Q/'poste'),'lourd',sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai',str(BUDGET),'--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver']
 save(Q,'launch.json',dict(command=cmd,allocation=args.allocation))
 with (P/'outer.log').open('x') as output:rc=subprocess.run(cmd,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT).returncode
 print(json.dumps(dict(status='RED_VALIDATION_ENDED',rc=rc,proofs=str(P))));return rc or 1
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--pilot');parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--allocation');parser.add_argument('--driver',action='store_true');args=parser.parse_args()
 if args.driver:
  try:driver()
  except BaseException as error:
   if P.exists():first_red(repr(error))
   raise
 else:sys.exit(launch(args))
