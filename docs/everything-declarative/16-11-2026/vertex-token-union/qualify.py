#!/usr/bin/env python3
"""Proposed full canonical union qualification; no OAuth/cloud calls."""
import union_cache as vertex_cache
import argparse,hashlib,json,os,re,shutil,stat,subprocess,sys,time
from pathlib import Path
APP='b472035a4e2944a9d90727817c95290cc0fba1ea'
BASE='b0a845bb4877328eada37951c174e3d208986630'
R=Path.home()/'.cache/rag3weaver-build'
W=Path.home()/'git_workspaces/rag3db-cx-ed-vertex-token-union-r1'
C=W/'extension/rag3weaver'
P=R/'cx-ed-vertex-token-union-v1';Q=R/'cx-ed-vertex-token-union-controller-v1';S=R/'cx-ed-vertex-token-union-scope-v1'
T=R/'target-cx-ed-stable-tie-identity' # The one existing session target, known red fe40 provenance.
PREFIX='openai_llm::vertex_refresh_tests::'
POSITIVE=['fixture_token_source_changes_its_cached_token_only_after_invalidation','a_fixed_api_bearer_is_not_refreshed_after_unauthorized','vertex_never_refreshes_or_replays_after_a_stream_fragment']
RED='vertex_rejected_token_is_refreshed_once_before_opening_a_stream'
PERSISTENT='vertex_persistent_unauthorized_is_named_after_one_refresh_only'
EXACTS=[*POSITIVE,RED,PERSISTENT,'declared_vertex_connector_retains_injected_source_and_refreshes','dynamic_vertex_auth_debug_never_discloses_a_token']
FEATURES=['rag3db-native','burn-embedder','burn-ocr','code','daemon','postgres','openai-llm','anthropic-llm'];BUDGET=5330;OUTER=5398
PASS_SHA='2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
LIB='rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value,end=None):
 with (root/name).open('x') as stream:
  if end is not None and time.monotonic()>=end:raise RuntimeError('deadline after open before publication')
  json.dump(value,stream,indent=2);stream.flush();os.fsync(stream.fileno())
FAILED = re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.', re.M)

def first_red(reason):
    try:
        with (P / 'first-red.json').open('x') as f:
            json.dump(dict(reason=reason, observed=time.monotonic()), f)
            f.flush(); os.fsync(f.fileno())
    except FileExistsError:
        pass

def active_logs(name):
    paths = [P / (name + '.log')]
    if name == 'official-pass':
        paths += [P / 'official' / (item + '.log') for item in
                  ['temoins', 'e2e_code', 'e2e_lectures', 'check_features',
                   'preuve', 'lib', 'navigateur', 'resume']]
    return paths

def watch_logs(name):
    # Only files produced by this invocation, never cached template evidence.
    for log in active_logs(name):
        if not log.exists():
            continue
        with log.open('rb') as stream:
            before = os.fstat(stream.fileno())
            data = stream.read()
            after = os.fstat(stream.fileno())
        text = data.decode(errors='replace')
        match = FAILED.search(text)
        if match is None and log.name in ['check_features.log', 'resume.log']:
            match = re.search(r'^(?:CHECK_FEATURES )?ROUGE\b.*$', text, re.M)
        if match:
            evidence = dict(invocation=name, path=str(log), observed=time.monotonic(),
                            device=before.st_dev, inode=before.st_ino, size_before=before.st_size,
                            size_after=after.st_size, mtime_ns=after.st_mtime_ns,
                            bytes_read=len(data), sha256=hashlib.sha256(data).hexdigest(),
                            matched=match.group(), meaning='active invocation output')
            with (P / 'watcher-first-match.json').open('x') as stream:
                json.dump(evidence, stream, indent=2)
                stream.flush(); os.fsync(stream.fileno())
            first_red('active log failure: ' + str(log))
            raise AssertionError('first red in active log ' + str(log))

def guard(cfg,end=None):
 for root,head in [(W,APP),(Path(cfg['control']),cfg['pilot'])]:
  assert git(root,'rev-parse','HEAD')==head and not git(root,'status','--porcelain')
  for name in ['rebase-merge','rebase-apply']:assert not Path(git(root,'rev-parse','--path-format=absolute','--git-path',name)).exists()
 assert git(C/'codeparsers','rev-parse','HEAD')==git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2]
 assert not git(C/'codeparsers','status','--porcelain')
 for name,value in cfg['copies'].items():assert sha(Q/name)==value
 assert sha(cfg['timeout'])==cfg['timeout_sha256']
 assert sha(cfg['bash'])==cfg['bash_sha256']
 assert sha(R/'passe_epinglee.sh')==sha(W/'tools/poste/passe_epinglee.sh')==PASS_SHA
 assert git(W,'merge-base',APP,'origin/master')==BASE
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
   watch_logs(label)
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

def library_summary(text):
 rows=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text,re.M)
 assert len(rows)==1,rows
 passed,failed,ignored,measured,filtered=map(int,rows[0]);assert passed>0 and failed==0
 return dict(passed=passed,failed=failed,ignored=ignored,measured=measured,filtered=filtered)

def driver():
 cfg=json.loads((Q/'configuration.json').read_text());guard(cfg)
 subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
 receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());end=auth['echeance']
 assert auth['etat']=='actif' and json.loads((receipt.parent/'recette.json').read_text())['delai']==BUDGET
 allocation=json.loads((Q/'launch.json').read_text());assert allocation['resource_ack'] and 0<=time.monotonic()-allocation['resource_ack_recorded']<=120
 cg=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={name:(cg/name).read_text().strip() for name in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits['memory.max']=='34359738368' and limits['memory.high']=='max' and limits['memory.swap.max']=='0'
 quota,period=limits['cpu.max'].split();assert quota!='max' and int(quota)==5*int(period)
 save(P,'authority-path.json',dict(path=str(receipt),initial=end-BUDGET,deadline=end,limits=limits))
 before=vertex_cache.capture(T);save(P,'cache-after-poste.json',before);assert before==cfg['cache_contract']
 old_scope=Path(before['authority']['path']).parent
 observed=quiet(json.loads((old_scope/'enfants.json').read_text()));save(P,'cache-quiet-after-poste.json',observed)
 assert not observed['owned_alive'] and not observed['found'] and not any(x['owned'] or x.get('attributed',False) for x in observed['unknown'])
 tmp=short_tmp();save(P,'short-tmp.json',dict(path=str(tmp),uid=tmp.stat().st_uid,device=tmp.stat().st_dev,inode=tmp.stat().st_ino))
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for name in ['cache','config','data','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env.update(CARGO_MANIFEST_DIR=str(C),CARGO_TARGET_DIR=str(T),CIBLE=T.name,CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS=str(min(8,int(os.environ.get('CARGO_BUILD_JOBS','8')))),CARGO_TERM_COLOR='never',RUST_TEST_THREADS='8',
  TMPDIR=str(tmp),XDG_CACHE_HOME=str(P/'cache'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=LIB,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_TARGET_MAX_GO='32',RAG3WEAVER_PAQUETS_REACT=str(R/'I-paquets-react'),RAG3WEAVER_EXIGER_NAVIGATEUR='1',SUITE_DU_TEMOIN='lib')
 for suffix in ['','_EMBEDDER','_RERANKER','_OCR']:env['RAG3WEAVER_BURN_DEVICE'+suffix]='cpu'
 # HOME stays the real cache/worktree root; an absent explicit file prevents ADC fallback.
 forbidden=P/'credentials-not-authorized.json';assert not forbidden.exists() and not forbidden.is_symlink();env['GOOGLE_APPLICATION_CREDENTIALS']=str(forbidden)
 assert not Path(env['XDG_STATE_HOME']).is_relative_to(Path(env['RAG3WEAVER_STATE_DIR'])) and not Path(env['RAG3WEAVER_STATE_DIR']).is_relative_to(Path(env['XDG_STATE_HOME']))
 save(P,'profile.json',{k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'})
 guard_argv=['bash','-c','. "$1"; garder_le_target "$2" "$3"','vertex-cache-guard',str(Q/'garde_target.sh'),str(T)]
 assert sha(Q/'garde_target.sh')==sha(vertex_cache.OLD/'extension/rag3weaver/scripts/garde_target.sh')
 guard(cfg,end)
 for label,owner,reuse in [('cache-old-owner',vertex_cache.OLD/'extension/rag3weaver','0'),('cache-adopt',C,'1')]:
  rc,_=run(label,guard_argv+[str(owner)],dict(env,RAG3WEAVER_TARGET_REPRENDRE=reuse),end,60)
  if rc:first_red(label+' refused');raise RuntimeError('shared target guard refused')
  assert (T/'.arbre-du-target').read_text().strip()==str(owner)
 save(P,'cache-adopted.json',dict(target=str(T),owner=str(C),real_shared_guard_calls=2,python_invalidation=False,prior_cpu_green_preserved=True))
 required=subprocess.run(['bash',str(W/'tools/poste/required_suites.sh'),BASE,APP],cwd=W,capture_output=True,text=True,timeout=15)
 save(P,'required-suites.json',dict(rc=required.returncode,stdout=required.stdout,stderr=required.stderr));assert required.returncode==0 and required.stdout==cfg['required_suites']
 # No SUITES override: the unchanged canonical runs lectures + code + any required additions.
 assert 'SUITES' not in env
 rc,_=run('official-pass',['bash',str(R/'passe_epinglee.sh'),W.name,str(P.relative_to(R)/'official'),'vertex_refresh_tests'],env,end,5100)
 if rc:first_red('canonical returned '+str(rc));raise RuntimeError('canonical refused')
 assert (P/'official/resume.log').read_text().splitlines()[-1]=='VERT'
 witness=library_summary((P/'official/temoins.log').read_text());assert witness['passed']==7 and witness['failed']==witness['ignored']==0
 assert re.search(r'^CHECK_FEATURES VERT 8/8$',(P/'official/check_features.log').read_text(),re.M)
 large=(P/'official/lib.log').read_text();lib=library_summary(large);assert lib['filtered']==0
 nav=library_summary((P/'official/navigateur.log').read_text());assert nav['ignored']==0
 suites={}
 for name in sorted({'e2e_code','e2e_lectures'}|{line.split()[0] for line in required.stdout.splitlines() if line.strip()}):
  suites[name]=library_summary((P/('official/'+name+'.log')).read_text());assert suites[name]['ignored']==suites[name]['filtered']==0
 binaries=re.findall(r'Running unittests src/lib\.rs \(([^\n]+)\)',large);assert len(binaries)==1
 binary=Path(binaries[0]);assert binary.resolve()==binary and binary.is_file() and binary.is_relative_to(T)
 copied=P/'large-unit-tests';shutil.copy2(binary,copied);assert sha(binary)==sha(copied)
 save(P,'ledger-after-canonical.json',dict(source=APP,pilot=cfg['pilot'],binary=str(binary),copy=str(copied),sha256=sha(binary),size=binary.stat().st_size,lib=lib,witness=witness,nav=nav,suites=suites,features=FEATURES,summary_sha256=sha(P/'official/resume.log'),meaning='post canonical attestation only; not an ELF ledger before first witness',maps_main_qualified=False))
 assert not forbidden.exists() and not (P/'first-red.json').exists() and (T/'.arbre-du-target').read_text().strip()==str(C)
 guard(cfg,end);save(P,'driver-result.json',dict(status='VERTEX_UNION_CANONICAL_PASSED',source=APP,pilot=cfg['pilot'],deadline=end,lib=lib,witness=witness,nav=nav,suites=suites,oauth_qualified=False,cloud_qualified=False),end=end)

def ancestor_identities():
 result={};pid=os.getpid()
 while pid>1 and pid not in result:
  fields=(Path('/proc')/str(pid)/'stat').read_text().rsplit(') ',1)[1].split()
  result[pid]=fields[19];pid=int(fields[1])
 return result

def poste_fd_allowed(proc,role,value,start,ancestors,cfg):
 if not role.startswith('fd:') or value!=str(Q/'poste') or ancestors.get(int(proc.name))!=start:return False
 if proc.stat().st_uid!=os.getuid() or os.readlink(proc/'exe')!=cfg['bash'] or sha(proc/'exe')!=cfg['bash_sha256']:return False
 argv=[os.fsdecode(v) for v in (proc/'cmdline').read_bytes().split(b'\0') if v]
 expected=[str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s',str(OUTER)+'s',sys.executable,str(Q/'qualify.py'),'--compose']
 if argv[1:]!=expected:return False
 fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
 return fields[19]==start and fields[0]!='Z' and proc.stat().st_uid==os.getuid()

def quiet(owned):
 cfg=json.loads((Q/'configuration.json').read_text());ancestors=ancestor_identities()
 alive=[];unknown=[];found=[];exemptions=[];roots=[W,T,Q]+[Path(cfg[k]) for k in ['old_source','old_controller','old_control_root'] if k in cfg]
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
  attributed=False
  try:
   if proc.stat().st_uid!=os.getuid():continue
   fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
   if fields[0]=='Z':continue
   def observe(role,value):
    nonlocal attributed
    if any(value==str(root) or value.startswith(str(root)+'/') for root in roots):
     if poste_fd_allowed(proc,role,value,fields[19],ancestors,cfg):
      exemptions.append(dict(pid=proc.name,start=fields[19],role=role,path=value,meaning='verified ancestor Bash poste owns this allocation'));return
     attributed=True;found.append(dict(pid=proc.name,start=fields[19],role=role,path=value))
   for role in ['exe','cwd']:observe(role,os.readlink(proc/role))
   for fd in (proc/'fd').iterdir():
    try:value=os.readlink(fd)
    except (FileNotFoundError,ProcessLookupError):continue
    observe('fd:'+fd.name,value)
  except (FileNotFoundError,ProcessLookupError):pass
  except OSError as error:unknown.append(dict(owned=False,attributed=attributed,pid=proc.name,error=repr(error)))
 return dict(owned_alive=alive,unknown=unknown,found=found,exemptions=exemptions,observed=time.monotonic())

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
  driver=json.loads((P/'driver-result.json').read_text());assert driver['status']=='VERTEX_UNION_CANONICAL_PASSED' and not (P/'first-red.json').exists()
  ledger=json.loads((P/'ledger-after-canonical.json').read_text());assert sha(ledger['binary'])==sha(ledger['copy'])==ledger['sha256'];assert sha(P/'official/resume.log')==ledger['summary_sha256']
  observed=quiet(json.loads((authority_path.parent/'enfants.json').read_text()));save(P,'quiescence-final.json',observed)
  assert not observed['owned_alive'] and not observed['found'] and not any(x['owned'] or x.get('attributed',False) for x in observed['unknown'])
  guard(cfg);assert time.monotonic()<end
  result=dict(status='VERTEX_UNION_CANONICAL_CHECKED',returncode=0,source=APP,pilot=cfg['pilot'],scope=scope,authority_terminated=True,bus_absent=True,deadline=end,elapsed=time.monotonic()-(end-BUDGET),sdk_pin_verified=True,maps_main_qualified=False,cloud_qualified=False,real_oauth_qualified=False)
 except BaseException as error:
  first_red(repr(error));result=dict(status='RED',returncode=1,error=repr(error))
 save(P,'composed-result.json',result,end=result.get('deadline'));print(json.dumps(result));return result['returncode']

def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 control=Path(__file__).resolve().parents[4]
 assert git(control,'rev-parse','HEAD')==args.pilot and not git(control,'status','--porcelain')
 assert not S.exists() and not S.is_symlink()
 for path in [P,Q]:assert not path.is_symlink() and (not path.exists() or (path.is_dir() and path.resolve()==path and path.stat().st_uid==os.getuid()))
 if P.exists():assert not any(P.iterdir()),'prepared proof root must still be empty'
 assert git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain')
 Q.mkdir(mode=0o700,exist_ok=True);P.mkdir(mode=0o700,exist_ok=True)
 copies={}
 for name,source in [('qualify.py',Path(__file__)),('union_cache.py',Path(__file__).with_name('union_cache.py')),('poste',C/'scripts/poste'),('confiner_validation.py',C/'scripts/confiner_validation.py'),('garde_target.sh',C/'scripts/garde_target.sh')]:
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(source)
  else:shutil.copy2(source,dest)
  copies[name]=sha(dest)
 timeout=str(Path(shutil.which('timeout')).resolve())
 bash=str(Path(shutil.which('bash')).resolve())
 required=subprocess.check_output(['bash',str(W/'tools/poste/required_suites.sh'),BASE,APP],cwd=W,text=True)
 oldcfg=json.loads((vertex_cache.OLD_Q/'configuration.json').read_text())
 cfg=dict(app=APP,pilot=args.pilot,control=str(control),copies=copies,timeout=timeout,timeout_sha256=sha(timeout),bash=bash,bash_sha256=sha(bash),cache_contract=vertex_cache.capture(T),old_source=str(vertex_cache.OLD),old_controller=str(vertex_cache.OLD_Q),old_control_root=oldcfg['control'],required_suites=required)
 if (Q/'configuration.json').exists():assert json.loads((Q/'configuration.json').read_text())==cfg
 else:save(Q,'configuration.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=True,cargo=False,authority=False)));return 0
 assert args.allocation and args.resource_ack and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
 env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 cmd=[str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s',str(OUTER)+'s',sys.executable,str(Q/'qualify.py'),'--compose']
 save(Q,'launch.json',dict(command=cmd,allocation=args.allocation,resource_ack=args.resource_ack,resource_ack_recorded=time.monotonic(),meaning='fresh Embar/C7 operator reference, not an automated lease proof'))
 with (P/'outer.log').open('x') as output:rc=subprocess.run(cmd,cwd='/var/tmp',env=env,stdout=output,stderr=subprocess.STDOUT).returncode
 return rc # Parent does nothing after bounded composition except relay the exit status.
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('--pilot');parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--allocation');parser.add_argument('--resource-ack');parser.add_argument('--driver',action='store_true');parser.add_argument('--compose',action='store_true');args=parser.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:
   if P.exists():first_red(repr(error))
   raise
 else:sys.exit(launch(args))
