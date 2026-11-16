#!/usr/bin/env python3
"""Short negative fixture probe only; no Cargo/GDB/database/service operation."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,re,shutil,signal,stat,subprocess,sys,time
R=Path.home()/'.cache/rag3weaver-build';Q=R/'cx-control-recherche-fts-fixture-root-red-v1';P=R/'cx-proofs-recherche-fts-fixture-root-red-v1';S=R/'cx-scope-recherche-fts-fixture-root-red-v1'
SOURCE=R/'cx-wt-recherche-fts-generation-guard-source-v1';APP='bc9526408ea3a48320b4a27e7644075dcf7307b0';C=SOURCE/'extension/rag3weaver'
A=R/'cx-archive-recherche-fts-generation-guard-v1';MANIFEST='57fbd0280d3c46452366eb546ad3d7dedac484051e60ea859b59b2e60e9deb3c';ELF=A/'proofs/unit-tests';ELF_SHA='b23414d3742f7d8f1b2a230d72b2d91df187926375c1389f90b05ad277458c19'
NAME='code_sync::fts_guard_double_tests::generations_sync_removes_a_committed_fts_ghost'
LIB=R.parent/'rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def save(root,name,value):
 with (root/name).open('x') as f:json.dump(value,f,indent=2)
def module(name):
 spec=importlib.util.spec_from_file_location(name,Q/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
def guard(cfg):
 assert git(SOURCE,'rev-parse','HEAD')==APP and not git(SOURCE,'status','--porcelain')
 assert git(cfg['control'],'rev-parse','HEAD')==cfg['pilot'] and not git(cfg['control'],'status','--porcelain')
 assert all(path.resolve()==path for path in [SOURCE,C,A,ELF,Q])
 assert stat.S_ISREG(ELF.lstat().st_mode) and ELF.stat().st_uid==os.getuid()
 assert sha(A/'manifest.json')==MANIFEST and sha(ELF)==ELF_SHA
 manifest=json.loads((A/'manifest.json').read_text());ledger=json.loads((A/'proofs/ledger.json').read_text())
 assert ledger['app']==APP and ledger['elf_sha256']==ELF_SHA and ledger['names'].count(NAME)==1
 for name in ['proofs/unit-tests','proofs/ledger.json','proofs/main-sdk.json']:assert sha(A/name)==manifest[name]['sha256']
 for name,h in SO.items():assert sha(LIB/name)==h
 for name,h in cfg['copies'].items():assert sha(Q/name)==h
 assert sha(cfg['timeout'])==cfg['timeout_sha256']
def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);common=module('confiner_validation');auth=common.autorite(common.processus());end=auth['echeance'];receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
 assert json.loads((receipt.parent/'recette.json').read_text())['delai']==90
 cg=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']};assert limits=={'memory.max':'34359738368','memory.high':'max','memory.swap.max':'0','cpu.max':'500000 100000'}
 save(P,'authority-path.json',dict(path=str(receipt),deadline=end,limits=limits))
 base=module('base_controller');save(P,'quiet-before.json',base.cache_quiet({'source_root':str(SOURCE)}))
 for n in ['home','tmp','cache','config','data','xdg-state','state-fixture']:(P/n).mkdir(mode=0o700)
 env={k:os.environ[k] for k in ['USER','PATH','LANG','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 env.update(HOME=str(P/'home'),TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),CARGO_MANIFEST_DIR=str(C),RAG3DB_LIBRARY_DIR=str(LIB),RAG3DB_INCLUDE_DIR=str(LIB),RAG3DB_LIB=LIB.name,RAG3DB_LIBS_DIR=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',LD_LIBRARY_PATH=str(LIB),RUST_TEST_THREADS='1',RUST_BACKTRACE='0')
 env.pop('RAG3WEAVER_FTS_DOUBLE_ROOT',None);assert 'RAG3WEAVER_FTS_DOUBLE_ROOT' not in env
 copied=P/'unit-tests';shutil.copy2(ELF,copied);assert sha(copied)==ELF_SHA
 save(P,'ledger-before-test.json',dict(app=APP,pilot=cfg['pilot'],elf_sha256=ELF_SHA,previous_ledger_sha256=sha(A/'proofs/ledger.json'),sdk=SO,expected_native_exit=101,expected_failure='racine privée DOUBLE before fixture/Database',profile={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'},new_maps_attestation=False))
 argv=[str(copied),NAME,'--exact','--nocapture'];save(P,'command.json',dict(argv=argv,cwd=str(C),timeout=min(15,end-time.monotonic()-15),ledger_sha256=sha(P/'ledger-before-test.json')))
 with (P/'native.stdout').open('xb') as out,(P/'native.stderr').open('xb') as err:result=subprocess.run(argv,cwd=C,env=env,stdout=out,stderr=err,timeout=min(15,end-time.monotonic()-15))
 save(P,'native-result.json',dict(rc=result.returncode,finished=time.monotonic()))
 output=(P/'native.stdout').read_text();error=(P/'native.stderr').read_text()
 assert result.returncode==101 and NAME in output and 'test result: FAILED. 0 passed; 1 failed; 0 ignored;' in output
 assert 'panicked at src/code_sync_fts_guard_double_tests.rs:' in error and 'racine privée DOUBLE' in error
 assert not any(x in error for x in ['FTS_DOUBLE_TRACE','FTS_DOUBLE_BEFORE',"modèle d'embarquement"])
 assert not list(P.rglob('*.rag3db')) and not (P/'databases').exists();guard(cfg);assert time.monotonic()<end-5
 save(P,'negative-probe.json',dict(status='ROOT_ABSENT_RED_REPRODUCED',native_rc=101,source_test_green=False,root_explicitly_absent=True,database_before_failure=False,services_called=False))
def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=module('confiner_validation');parent=Path('/proc')/str(os.getppid());expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','118s',sys.executable,str(Q/'qualify.py'),'--compose'];assert os.readlink(parent/'exe')==cfg['timeout'] and [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]==expected and common.poste_tenu(common.processus())
 P.mkdir(mode=0o700);record=dict(status='REFUSED')
 def stop(sig,frame):raise RuntimeError('outer negative-probe deadline')
 signal.signal(signal.SIGTERM,stop)
 try:
  guard(cfg);rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','90','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text());base=module('base_controller');quiet=base.owned_quiet(path.parent);save(P,'owned-after.json',quiet);save(P,'quiet-after.json',base.cache_quiet({'source_root':str(SOURCE)}))
  assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and auth['etat']=='termine' and not Path(scope['prive']).exists() and not quiet['alive'] and not quiet['unknown']
  assert json.loads((P/'negative-probe.json').read_text())['status']=='ROOT_ABSENT_RED_REPRODUCED';guard(cfg);assert sha(P/'unit-tests')==ELF_SHA
  record=dict(status='ROOT_ABSENT_RED_REPRODUCED',native_rc=101,source_test_green=False,scope=scope,owned=quiet,authority_state=auth['etat'],deadline=auth['echeance'])
 except BaseException as error:record=dict(status='REFUSED',error=repr(error),source_test_green=False)
 with (P/'composed.json').open('x') as f:
  if record['status']=='ROOT_ABSENT_RED_REPRODUCED' and time.monotonic()>=record['deadline']:record=dict(status='REFUSED',error='deadline before publication',source_test_green=False)
  json.dump(record,f,indent=2)
 print(json.dumps(record));return 0 if record['status']=='ROOT_ABSENT_RED_REPRODUCED' else 1

def launch(args):
 root=Path(__file__).resolve().parents[4];assert os.uname().nodename.lower()=='luciepc' and git(root,'rev-parse','HEAD')==args.pilot and not git(root,'status','--porcelain') and not any(p.exists() or p.is_symlink() for p in [P,S])
 Q.mkdir(mode=0o700,exist_ok=True);assert Q.resolve()==Q and Q.stat().st_uid==os.getuid();sources={'qualify.py':Path(__file__),'base_controller.py':root/'docs/recherche/16-11-2026/fts-generation-guard/qualify.py','confiner_validation.py':C/'scripts/confiner_validation.py','poste':C/'scripts/poste'}
 for n,path in sources.items():
  dest=Q/n
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(path)
  else:shutil.copy2(path,dest)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(control=str(root),pilot=args.pilot,copies={n:sha(Q/n) for n in sources},timeout=timeout,timeout_sha256=sha(timeout))
 if (Q/'prepared.json').exists():assert json.loads((Q/'prepared.json').read_text())==cfg
 else:save(Q,'prepared.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=str(Q/'prepared.json'),sha256=sha(Q/'prepared.json'),native_execution=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 save(Q,'allocation.json',dict(reference=args.allocation,negative_probe_only=True))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 with (Q/'launch.log').open('x') as out:result=subprocess.run([str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','118s',sys.executable,str(Q/'qualify.py'),'--compose'],cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--pilot');ap.add_argument('--allocation');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:driver()
 else:sys.exit(launch(args))
