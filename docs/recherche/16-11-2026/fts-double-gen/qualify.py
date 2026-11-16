#!/usr/bin/env python3
"""Continue only the direct generations witness, under the original deadline."""
import argparse,hashlib,importlib.util,json,math,os,re,shutil,signal,subprocess,sys,time
from pathlib import Path
from classifier import classify_classic,FAILED,GEN
R=Path.home()/'.cache/rag3weaver-build';Q=R/'cx-control-recherche-fts-double-gen-v1';P=R/'cx-proofs-recherche-fts-double-gen-v1';S=R/'cx-scope-recherche-fts-double-gen-v1'
ORIGINAL_START=1189011.616250876;ORIGINAL_END=1190161.616250876
PREVIOUS=R/'cx-proofs-recherche-fts-double-v1';ARCHIVE=R/'cx-archive-recherche-fts-double-v1'
MANIFEST='1a2e39e4b8cc3ee521c81ac57b6e9c6a2a62d7d0397e1b6684c3007a21519324'
ELF_SHA='268229be9fd21290e369337cc52993bf5be42ea0b1eee17fe078180c581c147c'
APP='3ee1e117cc0c9ff7e9de1f08263c26cb4e907c79';OLD_PILOT='4d5265fc11151dd78c69355f26d214075b66ab8d'
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(root,name,value):
 with (root/name).open('x') as f:json.dump(value,f,indent=2)
def module(name):
 spec=importlib.util.spec_from_file_location(name,Q/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def guard(cfg):
 assert time.monotonic()<ORIGINAL_END
 assert git(cfg['control'],'rev-parse','HEAD')==cfg['pilot'] and not git(cfg['control'],'status','--porcelain')
 for n,h in cfg['copies'].items():assert sha(Q/n)==h
 assert sha(cfg['timeout'])==cfg['timeout_sha256']
 base=module('base_controller');old=json.loads((base.Q/'prepared.json').read_text());base.guard(old)
 assert old['pilot']==OLD_PILOT and sha(base.Q/'prepared.json')=='941d245702027039c7977599bf3fd960c964a1cba0af17b70d89f2af856d0751'
 assert (base.T/'.arbre-du-target').read_text().strip()==str(Path(old['source_root'])/'extension/rag3weaver')
 assert sha(ARCHIVE/'manifest.json')==MANIFEST;manifest=json.loads((ARCHIVE/'manifest.json').read_text())
 names=['ledger.json','exact-0.stdout','exact-0.stderr','exact-0.command.json','exact-0.result.json','inventory.stdout','main-sdk.json','first-red.json','composed.json']
 for n in names:assert sha(PREVIOUS/n)==sha(ARCHIVE/'proofs'/n)==manifest['proofs/'+n]['sha256']
 ledger=json.loads((PREVIOUS/'ledger.json').read_text());assert ledger['app']==APP and ledger['pilot']==OLD_PILOT and ledger['elf_sha256']==ELF_SHA
 for path in [ledger['source_executable'],ledger['copied_executable']]:assert sha(path)==ELF_SHA
 authpath=Path(json.loads((PREVIOUS/'authority-path.json').read_text())['path']);auth=json.loads(authpath.read_text());scope=json.loads((authpath.parent/'verdict.json').read_text())
 assert auth['etat']=='termine' and auth['echeance']==ORIGINAL_END and scope['verdict']=='ROUGE' and scope['code']==1 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 assert sha(authpath)==manifest['scope/validation-e0s_22uo/autorite.json']['sha256'] and sha(authpath.parent/'verdict.json')==manifest['scope/validation-e0s_22uo/verdict.json']['sha256']
 classified=classify_classic((PREVIOUS/'exact-0.stdout').read_text(),(PREVIOUS/'exact-0.stderr').read_text(),json.loads((PREVIOUS/'exact-0.command.json').read_text()),json.loads((PREVIOUS/'exact-0.result.json').read_text()),ledger,(PREVIOUS/'inventory.stdout').read_text())
 return base,old,ledger,classified

def first_red(error):
 try:save(P,'first-red.json',dict(reason=repr(error),observed=time.monotonic(),original_start=ORIGINAL_START,original_end=ORIGINAL_END))
 except FileExistsError:pass

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());base,old,ledger,classified=guard(cfg);common=module('confiner_validation');auth=common.autorite(common.processus());end=auth['echeance']
 assert end<=ORIGINAL_END and end-time.monotonic()>30
 cg=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits=={'memory.max':'34359738368','memory.high':'max','memory.swap.max':'0','cpu.max':'500000 100000'}
 save(P,'authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,original_start=ORIGINAL_START,original_end=ORIGINAL_END,limits=limits))
 save(P,'fresh-poste-quiet.json',base.cache_quiet(old));save(P,'classic-raw-classification.json',classified)
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 expected=json.loads((PREVIOUS/'profile.json').read_text())['expected']
 for key,value in expected.items():
  if key not in ['DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU']:env[key]=value
 for n in ['tmp','config','data','cache','xdg-state','state-fixture','databases']:(P/n).mkdir(mode=0o700)
 env.update(TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3WEAVER_FTS_DOUBLE_ROOT=str(P/'databases'))
 save(P,'profile.json',dict(expected={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'},services_required=[],cargo=False,gdb=False))
 elf=ledger['copied_executable'];save(P,'ledger.json',dict(app=APP,pilot=cfg['pilot'],previous_ledger_sha256=sha(PREVIOUS/'ledger.json'),elf=elf,elf_sha256=ELF_SHA,names=[GEN],ignored=0,original_start=ORIGINAL_START,original_end=ORIGINAL_END,published_before_runtime=time.monotonic()))
 assert sha(elf)==sha(ledger['source_executable'])==ELF_SHA
 deadline=min(end-15,time.monotonic()+60);assert deadline>time.monotonic()
 argv=[elf,GEN,'--exact','--nocapture'];save(P,'gen.command.json',dict(argv=argv,started=time.monotonic(),deadline=deadline))
 save(P,'runtime-marker.json',dict(ledger_sha256=sha(P/'ledger.json'),observed=time.monotonic()))
 with (P/'gen.stdout').open('xb') as out,(P/'gen.stderr').open('xb') as err:
  child=subprocess.Popen(argv,cwd=P,env=env,stdout=out,stderr=err)
  native_captured=False;proc=Path('/proc')/str(child.pid);stamp=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19]
  save(P,'gen-process.json',dict(pid=child.pid,start=stamp,exe_sha256=sha(proc/'exe')))
  try:
   while True:
    if not native_captured:
     maps=(proc/'maps').read_text()
     if str(base.LIB/'librag3db.so') in maps:
      actual={}
      for field in (proc/'environ').read_bytes().split(b'\0'):
       if b'=' in field:
        key,value=field.split(b'=',1);key=os.fsdecode(key)
        if key in env and key!='RAG3WEAVER_VALIDATION_JETON':actual[key]=os.fsdecode(value)
      expected_env={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'}
      save(P,'gen-actual-profile.json',dict(pid=child.pid,start=stamp,expected=expected_env,actual=actual,maps=maps,exe_sha256=sha(proc/'exe')))
      assert actual==expected_env and sha(proc/'exe')==ELF_SHA and (proc/'stat').read_text().rsplit(') ',1)[1].split()[19]==stamp
      sdk=module('sdk_contract').validate_sdk_maps(maps,ledger,str(base.LIB),sha);save(P,'gen-sdk.json',sdk);native_captured=True
    raw=(P/'gen.stdout').read_bytes()+(P/'gen.stderr').read_bytes()
    if FAILED.search(raw.decode(errors='replace')):raise RuntimeError('first FAILED/panic in generations witness')
    if time.monotonic()>=deadline:raise TimeoutError('GEN exact')
    if child.poll() is not None:break
    time.sleep(.05)
   rc=child.wait();save(P,'gen.result.json',dict(rc=rc,finished=time.monotonic()));assert rc==0
  except BaseException as error:
   first_red(error)
   if child.poll() is None:child.terminate()
   raise
 assert native_captured
 output=(P/'gen.stdout').read_text();assert GEN+' ... ok' in output and re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored;',output)
 guard(cfg);assert time.monotonic()<end-5
 save(P,'driver-result.json',dict(status='GEN_PASSED',deadline=end,original_start=ORIGINAL_START,original_end=ORIGINAL_END,elapsed_from_original=time.monotonic()-ORIGINAL_START,classic_rerun=False,cargo=False,gdb=False))
def compose():
 cfg=json.loads((Q/'prepared.json').read_text());window=json.loads((Q/'remaining-after-poste.json').read_text());common=module('confiner_validation');parent=Path('/proc')/str(os.getppid())
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s',str(window['soft_timeout'])+'s',sys.executable,str(Q/'qualify.py'),'--compose']
 assert os.readlink(parent/'exe')==cfg['timeout'] and [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]==expected and common.poste_tenu(common.processus())
 parent_start=float((parent/'stat').read_text().rsplit(') ',1)[1].split()[19])/os.sysconf('SC_CLK_TCK')
 assert parent_start+window['soft_timeout']+2<=ORIGINAL_END
 P.mkdir(mode=0o700);save(P,'outer-supervision.json',dict(observed=time.monotonic(),parent_start=parent_start,soft_timeout=window['soft_timeout'],kill_after=2,original_start=ORIGINAL_START,original_end=ORIGINAL_END,original_budget_preserved=True))
 def stop(sig,frame):first_red('outer absolute deadline');raise RuntimeError('original deadline stopped continuation')
 signal.signal(signal.SIGTERM,stop);record=dict(status='RED')
 try:
  guard(cfg);duration=math.floor(ORIGINAL_END-time.monotonic())-25;assert duration>60
  save(P,'new-common-window.json',dict(duration=duration,original_start=ORIGINAL_START,original_end=ORIGINAL_END))
  rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai',str(duration),'--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver'],cwd='/var/tmp').returncode
  authority=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(authority.read_text());scope=json.loads((authority.parent/'verdict.json').read_text());base,old,ledger,classified=guard(cfg)
  quiet=base.owned_quiet(authority.parent);save(P,'owned-after-scope.json',quiet);save(P,'cache-quiet-after-scope.json',base.cache_quiet(old))
  record.update(code=rc,scope=scope,authority_state=auth['etat'],owned=quiet,original_start=ORIGINAL_START,original_end=ORIGINAL_END)
  assert auth['echeance']<=ORIGINAL_END and auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
  assert rc==0 and not quiet['alive'] and not quiet['unknown'] and not (P/'first-red.json').exists()
  assert json.loads((P/'driver-result.json').read_text())['status']=='GEN_PASSED'
  guard(cfg);record.update(status='GEN_PASSED_WITH_PRIOR_CLASSIC_RAW',deadline=auth['echeance'],historical_scope_still_red=True)
 except BaseException as error:first_red(error);record.update(status='RED',error=repr(error))
 with (P/'composed.json').open('x') as f:
  if record['status']!='RED' and time.monotonic()>=min(record['deadline'],ORIGINAL_END):record=dict(status='RED',error='absolute deadline before publication')
  json.dump(record,f,indent=2)
 print(json.dumps(record));return 0 if record['status']!='RED' else 1

def admit():
 cfg=json.loads((Q/'prepared.json').read_text())
 now=time.monotonic();remaining=math.floor(ORIGINAL_END-now)-5;assert remaining>90
 save(Q,'remaining-after-poste.json',dict(observed=now,soft_timeout=remaining-2,hardcap=remaining,original_start=ORIGINAL_START,original_end=ORIGINAL_END))
 os.execv(cfg['timeout'],[cfg['timeout'],'--signal=TERM','--kill-after=2s',str(remaining-2)+'s',sys.executable,str(Q/'qualify.py'),'--compose'])
def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 root=Path(__file__).resolve().parents[4];assert git(root,'rev-parse','HEAD')==args.pilot and not git(root,'status','--porcelain')
 assert not any(x.exists() or x.is_symlink() for x in [P,S]);assert time.monotonic()<ORIGINAL_END
 Q.mkdir(mode=0o700,exist_ok=True);assert Q.resolve()==Q
 sources={'qualify.py':Path(__file__),'classifier.py':Path(__file__).with_name('classifier.py'),'sdk_contract.py':root/'docs/recherche/16-11-2026/fts-double/sdk_contract.py','base_controller.py':root/'docs/recherche/16-11-2026/fts-double/qualify.py','confiner_validation.py':root/'extension/rag3weaver/scripts/confiner_validation.py','poste':root/'extension/rag3weaver/scripts/poste'}
 for name,path in sources.items():
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(path)
  else:shutil.copy2(path,dest)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(control=str(root),pilot=args.pilot,copies={n:sha(Q/n) for n in sources},timeout=timeout,timeout_sha256=sha(timeout))
 if (Q/'prepared.json').exists():assert json.loads((Q/'prepared.json').read_text())==cfg
 else:save(Q,'prepared.json',cfg)
 guard(cfg)
 print(json.dumps(dict(prepared=str(Q/'prepared.json'),sha256=sha(Q/'prepared.json'),native_execution=False,cargo=False,original_end=ORIGINAL_END)))
 if args.prepare_only:return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ
 save(Q,'allocation.json',dict(reference=args.allocation))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ}
 env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 with (Q/'launch.log').open('x') as out:result=subprocess.run([str(Q/'poste'),'lourd',sys.executable,str(Q/'qualify.py'),'--admit'],cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--pilot');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--allocation');ap.add_argument('--admit',action='store_true');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.admit:sys.exit(admit())
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:first_red(error);raise
 else:sys.exit(launch(args))
