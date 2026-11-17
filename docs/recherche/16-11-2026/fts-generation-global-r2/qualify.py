#!/usr/bin/env python3
"""Proposed default canonical pass. Exact C7 GO plus a fresh Embar receipt required."""
from pathlib import Path
import uuid
global_owner=None # Test injection only; real import follows admitted source pins.
import lifecycle_protocol as ready_protocol
import lifecycle_provider_observation as owner_observation
import argparse,hashlib,importlib.util,json,math,os,re,shutil,signal,socket,stat,subprocess,sys,time,urllib.request
R=Path.home()/'.cache/rag3weaver-build';Q=R/'cx-control-recherche-fts-generation-global-v5';P=R/'cx-proofs-recherche-fts-generation-global-v5';S=R/'cx-scope-recherche-fts-generation-global-v5'
SHORT_TMP=R/'t5'
APP='5af94c3f983fcbdeca213b2cff100846b7922cdb';BASE='99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
W=Path.home()/'git_workspaces/rag3db-cx-recherche-fts-generation-source-doc-r3';OLD=R/'cx-wt-recherche-fts-fixture-root-source-v1';OLD_SHA='84ddeb88b7452278b0d697fad817c7e44f5fd3d3';T=R/'cx-target-recherche-fts-counts-freshness'
ARCHIVE=R/'cx-archive-recherche-fts-fixture-root-green-v1';MANIFEST='ee0e1cde90660c9e210c60e3902f1c4ae3809990fca51fd722b92761fc1d101d'
PIN='2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d'
LIB=R.parent/'rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
SO={'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101','libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561'}
WITNESS='code_sync::fts_guard_double_tests::generations_sync_removes_a_committed_fts_ghost'
REQUIRED=['e2e_code_sync','e2e_lectures','e2e_code','e2e_contrat_transactions','e2e_tx_ligne_a_ligne','e2e_tx_merge_puis_copy','e2e_tx_par_paquet_arret','e2e_tx_vecteurs_apres_rollback']
FAILED=re.compile(r'^test .* \.\.\. FAILED$|^\s*FAILED\s*$|^test result: FAILED\.',re.M)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(root,name,obj):
 with (root/name).open('x') as f:json.dump(obj,f,indent=2);f.flush();os.fsync(f.fileno())
def git(root,*args):return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
def module(name):
 spec=importlib.util.spec_from_file_location(name,Q/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
def first_red(error):
 try:save(P,'first-red.json',dict(reason=repr(error),observed=time.monotonic()))
 except FileExistsError:pass
def canonical_overrides():
 return dict(CIBLE=T.name,SUITE_DU_TEMOIN='lib',SUITES_EN_PLUS=' '.join(REQUIRED[3:]),RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_TARGET_MAX_GO='32',RAG3DB_LIB=LIB.name,RAG3DB_LIBS_DIR=str(LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',CARGO_BUILD_JOBS='8',CARGO_TARGET_DIR=str(T),CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',RUST_TEST_THREADS='8',RUST_BACKTRACE='1')
def validate_services(grant,now,end,context=None):
 assert grant.get('app')==APP
 assert re.fullmatch('[0-9a-f]{40}',grant.get('pilot','')) and grant.get('scope') and re.fullmatch('[0-9a-f]{64}',grant.get('ready_sha256',''))
 if context is not None:
  assert set(context)=={'app','pilot','scope','ready_sha256','c7_reference'}
  for key,value in context.items():assert grant.get(key)==value
 assert grant.get('c7_reference') and grant.get('embar_reference') and grant.get('restart_authorized') is False
 for key in ['observed_monotonic','valid_until_monotonic']:assert type(grant.get(key)) in [int,float] and math.isfinite(grant[key])
 assert 0<=now-grant['observed_monotonic']<=120 and grant['valid_until_monotonic']>=end and now<end
 providers=grant['providers'];assert len(providers)==2
 expected={'127.0.0.1:7878':('granite-278m',768),'127.0.0.1:7879':('bge-m3',1024)}
 assert {row['address'] for row in providers}==set(expected)
 for row in providers:
  identity=row['identity'];assert set(identity)=={'service','modele','dim','factice','dual','sparse','precision','executable','max_chars_per_text'}
  assert (identity['modele'],identity['dim'])==expected[row['address']] and identity['service']=='rag3weaver-embeddings' and identity['factice'] is False
  assert type(identity['dual']) is bool and type(identity['sparse']) is bool and type(identity['max_chars_per_text']) is int and identity['max_chars_per_text']>0 and identity['precision'] and identity['executable']
  assert type(row['uid']) is int and row['uid']==os.getuid() and isinstance(row['exe'],str) and row['exe'].startswith('/')
  assert type(row['pid']) is int and row['pid']>1 and re.fullmatch('[0-9]+',row['start']) and re.fullmatch('[0-9a-f]{64}',row['exe_sha256'])
 pg=grant['postgres'];assert pg['host']=='127.0.0.1' and pg['port']==5433 and pg['database']=='rag3weaver_test' and pg['ready'] is True and pg['private_test_only'] is True and pg['exclusive_reservation'] is True and pg['reservation_reference']
 return grant

def cache_contract():
 assert T.resolve()==T and T.is_dir() and T.stat().st_uid==os.getuid();marker=T/'.arbre-du-target';m=marker.lstat()
 assert stat.S_ISREG(m.st_mode) and m.st_uid==os.getuid() and marker.read_text().strip()==str(OLD/'extension/rag3weaver')
 assert git(OLD,'rev-parse','HEAD')==OLD_SHA and not git(OLD,'status','--porcelain') and sha(ARCHIVE/'manifest.json')==MANIFEST
 manifest=json.loads((ARCHIVE/'manifest.json').read_text());selected={}
 for name in ['proofs/ledger.json','proofs/unit-tests','proofs/composed.json','proofs/authority-path.json','scope/authority-public.json','scope/verdict.json']:
  h=sha(ARCHIVE/name);assert h==manifest[name]['sha256'];selected[name]=h
 live=Path(json.loads((ARCHIVE/'proofs/authority-path.json').read_text())['path']).parent
 assert sha(live/'verdict.json')==selected['scope/verdict.json']
 private=json.loads((live/'autorite.json').read_text());auth={k:private[k] for k in ['etat','echeance','cgroup']};scope=json.loads((live/'verdict.json').read_text())
 assert auth==json.loads((ARCHIVE/'scope/authority-public.json').read_text()) and auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists()
 ledger=json.loads((ARCHIVE/'proofs/ledger.json').read_text());assert ledger['app']==OLD_SHA and ledger['elf_sha256']=='23da1279965f9a882373f0c185e6ec635b7bd0419ea6abd595ce903e100b9461'
 for key in ['source_executable','copied_executable']:assert sha(ledger[key])==ledger['elf_sha256']
 return dict(marker=dict(dev=m.st_dev,ino=m.st_ino,mode=m.st_mode,uid=m.st_uid,value=marker.read_text()),selected=selected,old=OLD_SHA,authority=auth,scope=scope)
def tmp_socket_lengths(root):
 source=(W/'extension/rag3weaver/src/pont_navigateur/temoins_relai.rs').read_text()
 prefix=re.search(r'format!\("([^"\n]+)", std::process::id\(\), N.fetch_add',source).group(1)
 suffix=re.search(r'let sock = d.join\("([^"\n]+)"\).join\("([^"\n]+)"\)',source)
 assert prefix.count('{}')==2 and suffix is not None
 return [len(os.fsencode(root/prefix.format(pid,n)/suffix[1]/suffix[2])) for pid,n in [(1,0),(9999999,0),(2**32-1,2**64-1)]]
def tmp_state():
 assert SHORT_TMP.resolve()==SHORT_TMP and not SHORT_TMP.is_symlink()
 meta=SHORT_TMP.lstat();assert stat.S_ISDIR(meta.st_mode) and meta.st_uid==os.getuid() and stat.S_IMODE(meta.st_mode)==0o700 and meta.st_dev==R.stat().st_dev
 sizes=tmp_socket_lengths(SHORT_TMP);assert all(n<=107 for n in sizes)
 return dict(path=str(SHORT_TMP),dev=meta.st_dev,ino=meta.st_ino,uid=meta.st_uid,mode=stat.S_IMODE(meta.st_mode),socket_lengths=sizes)
def create_private_tmp():
 assert SHORT_TMP.resolve()==SHORT_TMP
 assert all(n<=107 for n in tmp_socket_lengths(SHORT_TMP))
 SHORT_TMP.mkdir(mode=0o700) # Exclusive: no reuse/repair or cleanup of a prior lot.
 save(P,'tmp-profile.json',tmp_state())
def private_tmp_guard():
 receipt=P/'tmp-profile.json'
 if receipt.exists():assert tmp_state()==json.loads(receipt.read_text())
 else:assert not SHORT_TMP.exists() and not SHORT_TMP.is_symlink(),'private TMP already exists without this scope receipt'

def guard(cfg):
 private_tmp_guard()
 for root,value in [(W,APP),(Path(cfg['control']),cfg['pilot'])]:assert git(root,'rev-parse','HEAD')==value and not git(root,'status','--porcelain')
 assert git(W,'rev-parse','origin/master')==BASE,'published master moved; review a new union before execution'
 assert git(W,'merge-base','HEAD','origin/master')==BASE,'master changed; review before any new pass'
 assert (T/'.arbre-du-target').read_text().strip() in [str(OLD/'extension/rag3weaver'),str(W/'extension/rag3weaver')]
 for n,h in cfg['copies'].items():assert sha(Q/n)==h
 assert sha(cfg['timeout'])==cfg['timeout_sha256'] and sha(R/'passe_epinglee.sh')==PIN and sha(Q/'passe_epinglee.sh')==PIN
 for n,h in SO.items():assert sha(LIB/n)==h
 assert sha(W/'tools/poste/required_suites.txt')==cfg['required_table_sha256']
 assert sha(Q/'confiner_validation.py')=='6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea'
 assert sha(Q/'lifecycle_protocol.py')=='c67f072ba0134f97be6150547d80b8a23174937f2c964b0bdc30bec5e0309ec0'

def provider_stable(row,hash_exe=False):
 p=Path('/proc')/str(row['pid'])
 if hash_exe:
  actual=owner_observation.process_snapshot(row['pid'])
  assert actual['start']==row['start'] and actual['uid']==row['uid']==os.getuid() and actual['exe']==row['exe'] and actual['exe_sha256']==row['exe_sha256']
 else:
  fields=(p/'stat').read_text().rsplit(') ',1)[1].split()
  assert fields[19]==row['start'] and fields[0] in ['R','S','D','I'] and p.stat().st_uid==row['uid']==os.getuid() and os.readlink(p/'exe')==row['exe']
def owner_pins(grant,ready):
 pins=grant['owner_pins']
 canonical=json.dumps(pins,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
 assert hashlib.sha256(canonical).hexdigest()==grant['owner_pins_sha256']
 admitted=pins.get('admitted_monotonic');observed=grant['observed_monotonic']
 assert type(admitted) in [int,float] and type(admitted) is type(observed) and math.isfinite(admitted) and admitted==observed
 for key,value in [('app',APP),('pilot',ready['pilot']),('scope',ready['scope']),('nonce',ready['nonce']),('ready_path',str(P/'READY_FOR_SERVICES.json')),('ready_sha256',sha(P/'READY_FOR_SERVICES.json')),('c7_reference',ready['c7_reference'])]:assert pins[key]==value
 for key,name in [('protocol_sha256','lifecycle_protocol.py'),('process_module_sha256','lifecycle_provider_observation.py'),('owner_sha256','global_services_owner.py'),('pg_receipt_sha256','pg_receipt.py'),('pg_identity_sql_sha256','identity.sql'),('pg_postcheck_sql_sha256','backend-postcheck.sql'),('transport_sha256','provider_health_transport.py')]:assert pins[key]==sha(Q/name)
 assert len(pins['providers'])==2 and {v['address'] for v in pins['providers']}=={v['address'] for v in grant['providers']}
 for pinned in pins['providers']:
  row=next(v for v in grant['providers'] if v['address']==pinned['address'])
  assert row['identity']==pinned['identity']
  for key in ['pid','uid','start','exe','exe_sha256']:assert row[key]==pinned['process'][key]
 return pins

def health(grant,label,end,*,ready,admission):
 pins=owner_pins(grant,ready)
 owner=global_owner if global_owner is not None else module('global_services_owner')
 result=[];lease_end=min(end,grant['valid_until_monotonic'])
 for provider in pins['providers']:
  evidence=dict(address=provider['address'],admission=admission);failure=None
  try:
   snapshot=ready_protocol.read_phase_state(ready);now=time.monotonic()
   if admission:ready_protocol.validate_ready_phase(ready,now,snapshot)
   else:ready_protocol.validate_phase_authority(ready,now,snapshot)
   assert now<lease_end<=ready['deadline']
   data=owner.health_observation(ready,pins,provider,evidence,admission=admission,lease_end=lease_end)
   snapshot=ready_protocol.read_phase_state(ready);now=time.monotonic()
   if admission:ready_protocol.validate_ready_phase(ready,now,snapshot)
   else:ready_protocol.validate_phase_authority(ready,now,snapshot)
   assert now<lease_end
   result.append(data)
  except BaseException as error:
   failure=error;evidence['consumer_refusal_type']=type(error).__name__;raise
  finally:
   try:save(P,label+'-'+provider['address'].rsplit(':',1)[1]+'.json',evidence)
   except BaseException as publication_error:
    if failure is not None:raise failure from publication_error
    raise
 return result

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);common=module('confiner_validation');auth=common.autorite(common.processus());end=auth['echeance'];receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU'])
 assert json.loads((receipt.parent/'recette.json').read_text())['delai']==5900
 cg=Path('/sys/fs/cgroup')/auth['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']}
 assert limits=={'memory.max':'34359738368','memory.high':'max','memory.swap.max':'0','cpu.max':'500000 100000'}
 save(P,'authority-path.json',dict(path=str(receipt),deadline=end,initial=end-5900,limits=limits))
 create_private_tmp()
 base=module('base_controller');base.OLD=OLD
 contract=cache_contract();save(P,'cache-after-poste.json',contract);assert contract==cfg['cache_contract'];save(P,'cache-quiet-after-poste.json',base.cache_quiet({'source_root':str(W)}))
 C=W/'extension/rag3weaver';env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 for n in ['config','data','cache','xdg-state','state-fixture']:(P/n).mkdir(mode=0o700)
 env.pop('RAG3WEAVER_FTS_DOUBLE_ROOT',None);env.update(canonical_overrides());env.update(TMPDIR=str(SHORT_TMP),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'))
 def run_guard(label,owner,resume):
  deadline=min(end-15,time.monotonic()+120);assert deadline>time.monotonic();phase_env=dict(env,RAG3WEAVER_TARGET_REPRENDRE=resume)
  argv=['bash','-c','. "$1"; garder_le_target "$2" "$3"','target-guard',str(Q/'garde_target.sh'),str(T),str(owner)]
  save(P,label+'.command.json',dict(argv=argv,deadline=deadline))
  with (P/(label+'.log')).open('x') as out:result=subprocess.run(argv,cwd=owner,env=phase_env,stdout=out,stderr=subprocess.STDOUT,timeout=deadline-time.monotonic())
  save(P,label+'.result.json',dict(rc=result.returncode,finished=time.monotonic()));assert result.returncode==0
 run_guard('old-owner',OLD/'extension/rag3weaver','0');run_guard('adopt-union',C,'1');assert (T/'.arbre-du-target').read_text().strip()==str(C)
 required=subprocess.check_output(['bash',str(W/'tools/poste/required_suites.sh'),BASE,APP],cwd=W,text=True);assert required=='e2e_code_sync extension/rag3weaver/src/catalog.rs\n'
 save(P,'required-suites.json',dict(base=BASE,app=APP,result=required,table_sha256=cfg['required_table_sha256']))
 phase=dict(job='lifecycle-provider',purpose='fts-global-models-and-pg',pilot=cfg['pilot'],allocation=json.loads((Q/'allocation.json').read_text())['reference'],authority=dict(path=str(receipt),pid=auth['pid'],start=auth['debut'],cgroup=auth['cgroup'],uid=os.getuid(),script_sha256=auth['script_sha256']),first_red_path=str(P/'first-red.json'))
 ready=dict(c7_reference=phase['allocation'],deadline=end,app=APP,pilot=cfg['pilot'],scope=str(receipt.parent),nonce=uuid.uuid4().hex,phase=phase,requested=['127.0.0.1:7878','127.0.0.1:7879','127.0.0.1:5433/rag3weaver_test'],receipt=str(Q/'SERVICES_GO.json'),service_operations_authorized=False,embedding_calls_authorized_only_by_canonical=True,**ready_protocol.ready_bounds(time.monotonic(),end))
 save(P,'READY_FOR_SERVICES.pending.json',ready)
 assert time.monotonic()<ready['grant_deadline_monotonic'];os.link(P/'READY_FOR_SERVICES.pending.json',P/'READY_FOR_SERVICES.json')
 while not (Q/'SERVICES_GO.json').exists():
  snapshot=ready_protocol.read_phase_state(ready);ready_protocol.validate_ready_phase(ready,time.monotonic(),snapshot);time.sleep(.2)
 grant_path=Q/'SERVICES_GO.json';m=grant_path.lstat();assert not grant_path.is_symlink() and stat.S_ISREG(m.st_mode) and m.st_uid==os.getuid() and m.st_size<=65536
 context=dict(app=APP,pilot=cfg['pilot'],scope=str(receipt.parent),ready_sha256=sha(P/'READY_FOR_SERVICES.json'),c7_reference=phase['allocation'])
 grant=validate_services(ready_protocol.decode(grant_path.read_bytes()),time.monotonic(),end,context)
 assert grant['nonce']==ready['nonce'] and ready['ready_observed_monotonic']<=grant['observed_monotonic']
 snapshot=ready_protocol.read_phase_state(ready);ready_protocol.validate_ready_phase(ready,time.monotonic(),snapshot)
 save(P,'services-receipt.json',grant)
 snapshot=ready_protocol.read_phase_state(ready);ready_protocol.validate_ready_phase(ready,time.monotonic(),snapshot)
 health(grant,'services-before',end,ready=ready,admission=True)
 snapshot=ready_protocol.read_phase_state(ready);ready_protocol.validate_ready_phase(ready,time.monotonic(),snapshot)
 # No second PostgreSQL contact: the owner validates the exact SQL-RO/topology/
 # exclusive-reservation artefact before the grant and again before each health.
 snapshot=ready_protocol.read_phase_state(ready);ready_protocol.validate_phase_authority(ready,time.monotonic(),snapshot)
 guard(cfg);official=P/'official';official.mkdir();argv=['bash',str(Q/'passe_epinglee.sh'),W.name,str(official.relative_to(R)),WITNESS]
 save(P,'canonical-ledger.json',dict(app=APP,pilot=cfg['pilot'],base=BASE,argv=argv,script_sha256=PIN,cache_owner=str(C),services_receipt_sha256=sha(P/'services-receipt.json'),expected_before_script_environment={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'},required_suites=required,sdk=SO,compiled_elf_attested_before_tests=False))
 save(P,'runtime-marker.json',dict(observed=time.monotonic(),ledger_sha256=sha(P/'canonical-ledger.json')))
 with (P/'canonical.log').open('x') as out:
  child=subprocess.Popen(argv,cwd=C,env=env,stdout=out,stderr=subprocess.STDOUT);last_pid_scan=0
  try:
   while True:
    assert time.monotonic()<end-30 and time.monotonic()<grant['valid_until_monotonic']
    for name in ['temoins',*REQUIRED,'check_features','preuve','lib','navigateur','resume']:
     log=official/(name+'.log')
     if not log.exists():continue
     with log.open('rb') as stream:before=os.fstat(stream.fileno());raw=stream.read();after=os.fstat(stream.fileno())
     text=raw.decode(errors='replace');match=FAILED.search(text)
     if match is None and name in ['check_features','resume']:match=re.search(r'^(?:CHECK_FEATURES )?ROUGE\b.*$',text,re.M)
     if match:
      save(P,'watcher-first-match.json',dict(path=str(log),observed=time.monotonic(),dev=before.st_dev,inode=before.st_ino,bytes_read=len(raw),size_after=after.st_size,sha256=hashlib.sha256(raw).hexdigest(),matched=match.group()));raise RuntimeError('first failure in active log '+name)
    if time.monotonic()-last_pid_scan>=1:
     for row in grant['providers']:provider_stable(row)
     last_pid_scan=time.monotonic()
    if child.poll() is not None:break
    time.sleep(.1)
   rc=child.wait();save(P,'canonical-process.json',dict(rc=rc,finished=time.monotonic()));assert rc==0
  except BaseException as error:
   first_red(error)
   if child.poll() is None:child.terminate()
   raise
 summary=(official/'resume.log').read_text();assert summary.splitlines()[-1]=='VERT' and (official/'base').read_text().strip()==BASE
 assert 'suites réduites' not in summary and all('== '+name in summary for name in REQUIRED)
 for name in ['temoins',*REQUIRED,'lib','navigateur']:assert re.search(r'test result: ok\.',(official/(name+'.log')).read_text(errors='replace'))
 assert (official/'check_features.log').read_text().splitlines()[-1]=='CHECK_FEATURES VERT 8/8'
 assert 'preuve : rag3weaver revérifié' in summary and '== témoins navigateur' in summary
 health(grant,'services-after',end,ready=ready,admission=False);guard(cfg);assert time.monotonic()<end-10
 save(P,'driver-result.json',dict(status='CANONICAL_PASSED',app=APP,deadline=end,services_unchanged=True,canonical_summary=str(official/'resume.log'),baseline_old_master_qualified=False))
def outer_command(cfg):return [str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','5998s',sys.executable,str(Q/'qualify.py'),'--compose']
def run_outer(cfg,env):
 with (Q/'launch.log').open('x') as out:result=subprocess.run(outer_command(cfg),cwd='/var/tmp',env=env,stdout=out,stderr=subprocess.STDOUT)
 return result.returncode

def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=module('confiner_validation');parent=Path('/proc')/str(os.getppid())
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','5998s',sys.executable,str(Q/'qualify.py'),'--compose']
 assert os.readlink(parent/'exe')==cfg['timeout'] and [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]==expected and common.poste_tenu(common.processus())
 P.mkdir(mode=0o700);save(P,'outer-supervision.json',dict(observed=time.monotonic(),soft_timeout=5998,kill_after=2,hardcap=6000,common_budget=5900))
 def stop(sig,frame):first_red('outer supervision signal');raise RuntimeError('outer stopped canonical qualification')
 signal.signal(signal.SIGTERM,stop);record=dict(status='RED');rc=1
 try:
  guard(cfg);rc=subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'lancer','--delai','5900','--preuves',str(S),'--',sys.executable,str(Q/'qualify.py'),'--driver'],cwd='/var/tmp').returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text());base=module('base_controller');base.OLD=OLD;quiet=base.owned_quiet(path.parent)
  save(P,'owned-after-scope.json',quiet);save(P,'cache-quiet-after-scope.json',base.cache_quiet({'source_root':str(W)}));save(P,'tmp-quiet-after-scope.json',base.cache_quiet({'source_root':str(SHORT_TMP)}));record.update(code=rc,scope=scope,owned=quiet,authority_state=auth['etat'])
  assert rc==0 and scope['verdict']=='VERT' and scope['code']==0 and not scope['restes'] and scope['nettoye'] and auth['etat']=='termine' and not Path(scope['prive']).exists()
  assert not quiet['alive'] and not quiet['unknown'] and not (P/'first-red.json').exists()
  assert json.loads((P/'driver-result.json').read_text())['status']=='CANONICAL_PASSED';guard(cfg)
  record.update(status='CANONICAL_PASSED',deadline=auth['echeance'],app=APP,master_integration_authorized=False)
 except BaseException as error:first_red(error);record.update(status='RED',code=rc,error=repr(error))
 with (P/'composed.json').open('x') as f:
  if record['status']=='CANONICAL_PASSED' and time.monotonic()>=record['deadline']:record=dict(status='RED',error='deadline immediately before publication')
  json.dump(record,f,indent=2)
 print(json.dumps(record));return 0 if record['status']=='CANONICAL_PASSED' else 1

def launch(args):
 assert os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',args.pilot)
 control=Path(__file__).resolve().parents[4];assert git(control,'rev-parse','HEAD')==args.pilot and not git(control,'status','--porcelain')
 assert git(W,'rev-parse','HEAD')==APP and not git(W,'status','--porcelain')
 cp=W/'extension/rag3weaver/codeparsers';expected=git(W,'ls-tree',APP,'extension/rag3weaver/codeparsers').split()[2];assert (cp/'Cargo.toml').is_file() and git(cp,'rev-parse','HEAD')==expected and not git(cp,'status','--porcelain')
 assert not any(p.exists() or p.is_symlink() for p in [P,S])
 contract=cache_contract();Q.mkdir(mode=0o700,exist_ok=True);assert Q.resolve()==Q and Q.stat().st_uid==os.getuid()
 sources={'qualify.py':Path(__file__),'lifecycle_protocol.py':Path(__file__).parent/'lifecycle_protocol.py','services-contract.json':Path(__file__).parent/'services-contract.json','global_services_owner.py':Path(__file__).parent/'global_services_owner.py','provider_health_transport.py':Path(__file__).parent/'provider_health_transport.py','pg_receipt.py':Path(__file__).parent/'pg_receipt.py','identity.sql':Path(__file__).parent/'identity.sql','backend-postcheck.sql':Path(__file__).parent/'backend-postcheck.sql','lifecycle_grant_producer.py':Path(__file__).parent/'lifecycle_grant_producer.py','lifecycle_provider_observation.py':Path(__file__).parent/'lifecycle_provider_observation.py','base_controller.py':control/'docs/recherche/16-11-2026/fts-fixture-root-green/qualify.py','passe_epinglee.sh':W/'tools/poste/passe_epinglee.sh'}
 for name in ['confiner_validation.py','poste','garde_target.sh']:sources[name]=W/'extension/rag3weaver/scripts'/name
 for name,path in sources.items():
  dest=Q/name
  if dest.exists():assert not dest.is_symlink() and sha(dest)==sha(path)
  else:shutil.copy2(path,dest)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(control=str(control),pilot=args.pilot,copies={n:sha(Q/n) for n in sources},timeout=timeout,timeout_sha256=sha(timeout),cache_contract=contract,required_table_sha256=sha(W/'tools/poste/required_suites.txt'))
 if (Q/'prepared.json').exists():assert json.loads((Q/'prepared.json').read_text())==cfg
 else:save(Q,'prepared.json',cfg)
 guard(cfg)
 if args.prepare_only:print(json.dumps(dict(prepared=str(Q/'prepared.json'),sha256=sha(Q/'prepared.json'),cargo=False,native_execution=False,authority_created=False)));return 0
 assert args.allocation and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ and not (Q/'SERVICES_GO.json').exists()
 runtime=Path('/run/user')/str(os.getuid());bus=runtime/'bus';before=[p.lstat() for p in [runtime,bus]]
 assert runtime.resolve()==runtime and bus.resolve()==bus and stat.S_ISDIR(before[0].st_mode) and stat.S_ISSOCK(before[1].st_mode) and all(m.st_uid==os.getuid() for m in before)
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
 env.update(XDG_RUNTIME_DIR=str(runtime),DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus),POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 subprocess.run(['busctl','--user','--timeout=5','call','org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus.Peer','Ping'],env=env,check=True)
 for path,old in zip([runtime,bus],before):
  new=path.lstat();assert (old.st_dev,old.st_ino,old.st_mode,old.st_uid)==(new.st_dev,new.st_ino,new.st_mode,new.st_uid)
 save(Q,'allocation.json',dict(reference=args.allocation,services_grant_still_required=True));return run_outer(cfg,env)
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--pilot');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');args=ap.parse_args()
 if args.compose:sys.exit(compose())
 if args.driver:
  try:driver()
  except BaseException as error:first_red(error);raise
 else:sys.exit(launch(args))
