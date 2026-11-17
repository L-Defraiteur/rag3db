#!/usr/bin/env python3
"""Proposed full union/D6 qualification. Preparation never authorizes execution."""
from pathlib import Path
import importlib.util,json,math,os,re,shutil,stat,subprocess,sys,time
import attester,cache,cache_state,controllers,d6,evidence,observe,policy,runner
from settings import *

def configure():
 for name in ['R','W','C','T','P','Q','REL']:setattr(controllers,name,globals()[name])
 controllers.OLD_W=ARCHIVE742;controllers.OUTER_SECONDS=policy.TERM

def cfg():return json.loads((Q/'configuration.json').read_text())

def guard(config):
 cache.source_clean(W,policy.APP);cache.source_clean(Path(config['control']),config['pilot'])
 policy.require(cache.git(W,'merge-base',policy.APP,'origin/master')==policy.BASE,'union base changed')
 for name,digest in config['copies'].items():policy.require(evidence.sha(Q/name)==digest,'controller copy changed: '+name)
 for name,item in config['tools'].items():policy.require(evidence.sha(item['path'])==item['sha256'],'tool changed: '+name)
 for name,digest in policy.SDK_SHA.items():policy.require(evidence.sha(Path(config['sdk_root'])/name)==digest,'SDK changed: '+name)
 policy.require(evidence.sha(R/'passe_epinglee.sh')==evidence.sha(W/'tools/poste/passe_epinglee.sh')==policy.PASS_SHA,'canonical script changed')
 for name,digest in config['preparation'].items():policy.require(evidence.sha(PREP/name)==digest,'Git/T0 preparation changed')
 cache.source742.verify_archive(json.loads((PREP/'source742.json').read_text()))
 policy.require(cache.previous()==config['previous'],'positive proof changed')
 cache_state.require_identity(T,config['T0']['identity'])
 policy.require(cache.outside_active()==json.loads((PREP/'outside-active.json').read_text()),'old session cache changed outside admitted children')

def fresh(config,old_group,owned_group=None):
 launch=json.loads((Q/'launch.json').read_text());roles=controllers.policy(Path(config['control']),config['pilot'],launch['identity'])
 value=controllers.scan(roles,[T,W,Path(config['control']),ARCHIVE742,TASKS],old_group,owned_cgroup=owned_group)
 policy.require(not value['found'] and not value['ancestor_errors'],'cache/source access or incomplete ancestry')
 policy.require(not any(r.get('attributed') or r.get('reasons') for r in value['unknown']),'attributed UNKNOWN')
 policy.require(len(value['controllers'])==3 and {r['role'] for r in value['controllers']}=={'launch','poste','watchdog'},'three exact controllers missing')
 return value

def prepare(pilot):
 policy.require(os.uname().nodename.lower()=='luciepc' and re.fullmatch('[0-9a-f]{40}',pilot),'PC/exact pilot required')
 control=Path(__file__).resolve().parents[4];cache.source_clean(control,pilot);cache.source_clean(W,policy.APP)
 policy.require(not P.exists() and not Q.exists() and not F.exists(),'new proof/controller/runtime required')
 policy.require(not list(R.glob('e2e-[0-9]*')),'foreign canonical disk parent exists; no cleanup authorized')
 previous=cache.previous();cache.owned_gone(previous['authority']);T0=json.loads((PREP/'T0-before-git.json').read_text());cache.check_t0(T0)
 P.mkdir(mode=0o700);Q.mkdir(mode=0o700)
 copies={};here=Path(__file__).parent
 for origin in [*sorted(here.glob('*.py')),here/'provenance.json',C/'scripts/confiner_validation.py']:
  dest=Q/origin.name;shutil.copy2(origin,dest);copies[dest.name]=evidence.sha(dest)
 (Q/'bin').mkdir(mode=0o700)
 wrapper=Q/'bin/cargo';wrapper.write_text('#!/usr/bin/python3\nimport sys\nfrom pathlib import Path\nsys.dont_write_bytecode=True\nsys.path.insert(0,str(Path(__file__).resolve().parent.parent))\nfrom cargo_shim import main\nsys.exit(main(sys.argv[1:]))\n');wrapper.chmod(0o755);copies['bin/cargo']=evidence.sha(wrapper)
 tools={}
 for name in ['python3','bash','timeout','git','rustup','zstd']:
  file=Path(shutil.which(name)).resolve();tools[name]=dict(path=str(file),sha256=evidence.sha(file))
 cargo=Path(subprocess.check_output(['rustup','which','cargo'],cwd=C,text=True).strip()).resolve();tools['cargo']=dict(path=str(cargo),sha256=evidence.sha(cargo))
 config=dict(app=policy.APP,base=policy.BASE,pilot=pilot,control=str(control),copies=copies,tools=tools,cargo_exe=str(cargo),sdk_root=str(R.parent/'rag3db-libs'/policy.LIB),previous=previous,T0=T0,
  preparation={p.name:evidence.sha(p) for p in sorted(PREP.glob('*.json'))},budget=dict(common=policy.BUDGET,term=policy.TERM,kill=15,physical=policy.HARD,final=policy.FINAL),mode='mesure',quota='800%',minimum_fils=8)
 evidence.save(Q/'configuration.json',config);guard(config)
 print(json.dumps(dict(prepared=True,source=policy.APP,pilot=pilot,configuration_sha256=evidence.sha(Q/'configuration.json'),cargo=False,authority=False,proof_empty=not list(P.iterdir()))))

def launch(pilot):
 control=Path(__file__).resolve().parents[4]
 policy.require(sys.orig_argv==['python3',str(control/REL),'--launch',pilot],'exact unflagged launcher required')
 return subprocess.run(controllers.commands(control,pilot)['outer'],cwd='/var/tmp').returncode

def arm(pilot):
 observed=time.monotonic();policy.require(os.getcwd()=='/var/tmp' and 'RAG3WEAVER_VALIDATION_RECU' not in os.environ,'fresh external launch only')
 config=cfg();policy.require(config['pilot']==pilot,'pilot pin differs');guard(config);cache.check_t0(config['T0'])
 actual=controllers.commands(Path(config['control']),pilot);watchdog=controllers.snapshot(Path('/proc')/str(os.getppid()));policy.require(watchdog['stable'] and watchdog['argv']==actual['outer'],'not already under the exact OS watchdog')
 identity=controllers.snapshot(Path('/proc')/str(watchdog['ppid']));policy.require(identity['stable'] and identity['argv']==['python3',str(Path(config['control'])/REL),'--launch',pilot],'launcher identity changed')
 allocation=os.environ['RAG3WEAVER_C7_ALLOCATION'];policy.require(allocation.strip() and not list(P.iterdir()) and not (Q/'launch.json').exists() and not F.exists(),'used allocation/state')
 user_runtime=Path('/run/user')/str(os.getuid());bus=user_runtime/'bus';policy.require(bus.resolve()==bus and bus.is_socket() and bus.stat().st_uid==os.getuid(),'user bus unavailable for scope entry')
 env={k:os.environ[k] for k in ['HOME','USER','LOGNAME','PATH','LANG','LC_ALL','CARGO_HOME','RUSTUP_HOME'] if k in os.environ}
 env.update(POSTE_MEM_MAX='32G',RAG3WEAVER_BUILD_MEMORY_HIGH='0',POSTE_CPU_QUOTA='800%',DBUS_SESSION_BUS_ADDRESS='unix:path='+str(bus),XDG_RUNTIME_DIR=str(user_runtime))
 fd=os.open(P/'supervision.log',os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
 try:os.dup2(fd,1);os.dup2(fd,2)
 finally:os.close(fd)
 evidence.save(Q/'launch.json',dict(identity=identity,allocation=allocation,worker_observed_after_watchdog=observed,whole_start=observed,soft_end=observed+policy.TERM,hard_end=observed+policy.HARD,
  clock_semantics='worker observation after physical watchdog; OS timeout can stop earlier',outer_seconds=policy.TERM))
 policy.require(time.monotonic()<observed+policy.TERM,'armed allocation deadline')
 os.execve(actual['poste'][0],actual['poste'],env)

def compose(pilot):
 config=cfg();policy.require(config['pilot']==pilot,'pilot pin differs')
 spec=importlib.util.spec_from_file_location('common',Q/'confiner_validation.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 policy.require(common.poste_tenu(common.processus()),'true poste required');policy.jobs(os.environ['POSTE_FILS'])
 launch_record=json.loads((Q/'launch.json').read_text());outcome=dict(status='RED',not_a_master_permission=True)
 try:
  guard(config);cache.check_t0(config['T0']);cache.owned_gone(config['previous']['authority'])
  evidence.save(P/'fresh-after-poste-before-guard.json',fresh(config,config['previous']['cgroup']))
  remaining=min(policy.BUDGET,math.floor(launch_record['soft_end']-time.monotonic())-15);policy.require(remaining>120,'insufficient original remaining budget')
  evidence.save(P/'common-budget.json',dict(seconds=remaining,observed=time.monotonic(),soft=launch_record['soft_end'],hard=launch_record['hard_end']))
  # The actual exporter runs once before the driver; profiles later distinguish each script's export.
  env=dict(os.environ,RAG3DB_LIB=policy.LIB,RAG3DB_LIBS_DIR=str(R.parent/'rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',CARGO_MANIFEST_DIR=str(C));env.pop('LD_LIBRARY_PATH',None)
  command=[sys.executable,str(Q/'confiner_validation.py'),'lancer','--mode','mesure','--delai',str(remaining),'--preuves',str(P/'supervision'),'--','bash','-c','. "$1"; exporter_la_lib || exit 2; shift; exec "$@"','driver-sdk',str(C/'scripts/lib_du_magasin.sh'),sys.executable,str(Q/'qualify.py'),'--driver',pilot]
  code=subprocess.run(command,cwd='/var/tmp',env=env).returncode;started=time.monotonic()
  receipt=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(receipt.read_text());scope=json.loads((receipt.parent/'verdict.json').read_text())
  end=min(started+policy.FINAL,launch_record['soft_end']);policy.require(started<auth['echeance'] and started<end,'common exceeded its cap')
  policy.require(code==0 and auth['etat']=='termine' and scope['verdict']=='VERT' and scope['code']==0 and not scope['erreur'] and not scope['restes'] and scope['nettoye'] and not Path(scope['prive']).exists(),'scope not clean VERT')
  result=json.loads((P/'driver-result.json').read_text());policy.require(result['status']=='GREEN_GLOBAL_D6_AND_CANONICAL' and not (P/'first-red.json').exists(),'driver not fully green')
  owned=cache.owned_gone(receipt);scan=fresh(config,auth['cgroup']);evidence.save(P/'final-quiescence.json',dict(owned=owned,scan=scan))
  guard(config);check_artifacts(json.loads((P/'global-ledger.json').read_text()),live=True)
  policy.require(time.monotonic()<end,'final hashes exceeded cap')
  outcome.update(status='GREEN_GLOBAL_D6_AND_CANONICAL',source=policy.APP,pilot=pilot,scope=scope,deadline=end,final_started=started,common_deadline=auth['echeance'],soft_end=launch_record['soft_end'],hard_end=launch_record['hard_end'],worker_observed_after_watchdog=launch_record['whole_start'],elapsed_recorded=time.monotonic()-launch_record['whole_start'])
 except BaseException as error:
  latch('compose refused',repr(error));outcome.update(error=repr(error))
 outcome=evidence.finish(P/'result.json',outcome);print(json.dumps(outcome));return 0 if outcome['status']=='GREEN_GLOBAL_D6_AND_CANONICAL' else 1

def latch(reason,payload=None):
 try:evidence.save(P/'first-red.json',dict(reason=reason,payload=payload,observed=time.monotonic()))
 except FileExistsError:pass

def check_artifacts(ledger,live=False):
 policy.require(ledger['source']==policy.APP and set(ledger['artifacts'])=={'tty','host','large','e2e','nav'},'wrong global artifact set')
 for label,item in ledger['artifacts'].items():
  policy.require(evidence.sha(item['copy'])==item['sha256'],'published ELF copy changed')
  if live and label!='tty':policy.require(evidence.sha(item['binary'])==item['sha256'],'canonical executed/rebuilt a different ELF')

def driver(pilot):
 config=cfg();policy.require(config['pilot']==pilot,'pilot pin differs');guard(config)
 subprocess.run([sys.executable,str(Q/'confiner_validation.py'),'verifier'],check=True)
 receipt=Path(os.environ['RAG3WEAVER_VALIDATION_RECU']);auth=json.loads(receipt.read_text());launch_record=json.loads((Q/'launch.json').read_text());common_budget=json.loads((receipt.parent/'recette.json').read_text())['delai']
 policy.require(receipt.parent.parent==P/'supervision','authority outside this fresh proof scope')
 policy.require(auth['etat']=='actif' and 120<common_budget<=policy.BUDGET and auth['echeance']<=launch_record['soft_end'],'invalid fresh common authority')
 cgroup=(Path('/proc/self/cgroup').read_text().strip().split('::',1)[1]);policy.require(cgroup==auth['cgroup'],'driver cgroup differs')
 g=Path('/sys/fs/cgroup'+cgroup);limits={k:(g/k).read_text().strip() for k in ['memory.max','memory.high','cpu.max']}
 policy.require(limits['memory.max']=='34359738368' and limits['memory.high']=='max','32GiB/HIGHmax required');quota,period=limits['cpu.max'].split();policy.require(int(quota)==8*int(period),'800 percent CPU required');policy.jobs(os.environ['POSTE_FILS'])
 end=min(auth['echeance'],launch_record['soft_end']);evidence.save(P/'authority-path.json',dict(path=str(receipt),common_started=auth['echeance']-common_budget,deadline=end,limits=limits))
 evidence.save(P/'fresh-driver-before-guard.json',fresh(config,config['previous']['cgroup'],cgroup));cache.check_t0(config['T0'])
 policy.require(not F.exists() and not TASKS.exists(),'private runtime/task roots not fresh')
 F.mkdir(mode=0o700);(F/'tmp').mkdir(mode=0o700);(F/'runtime').mkdir(mode=0o700);TASKS.mkdir(mode=0o700)
 policy.require(subprocess.check_output(['findmnt','-n','-o','FSTYPE','-T',str(F)],text=True).strip() not in ['tmpfs','ramfs',''],'scratch must be on disk')
 context=runner.Runner(config,end,cgroup);context.env=policy.private_environment(os.environ,F/'runtime',F/'tmp',TASKS,os.environ['POSTE_FILS'])
 for path in {Path(v) for k,v in context.env.items() if k.endswith('_DIR') or k.startswith('XDG_')}:
  if path.is_relative_to(F):path.mkdir(mode=0o700,parents=True,exist_ok=True)
 expected=policy.profile(R.parent/'rag3db-libs',C,str(config['sdk_root']));selected=policy.selected(b''.join(os.fsencode(k)+b'='+os.fsencode(v)+b'\0' for k,v in os.environ.items()),expected);policy.validate_profile(selected)
 context.env.update(expected,CARGO_TARGET_DIR=str(T),RAG3WEAVER_TARGET_REPRENDRE='1');context.save('driver-sdk-profile.json',selected)
 context.observer=observe.Observer(config,cgroup,end,context.fail);context.start_phase('preflight',policy.CAPS['preflight'])
 before=cache_state.inventory(T,None);context.save('guard-before.json',before)
 context.run('initial-common-guard',['bash','-c','. "$1"; garder_le_target "$2" "$3" --offline --locked','source-guard',str(C/'scripts/garde_target.sh'),str(T),str(C)],120)
 context.save('guard-after.json',cache_state.inventory(T,None));cache_state.require_identity(T,config['T0']['identity'])
 context.start_phase('tty',policy.CAPS['tty']);context.compile('tty',['--bin','rag3weaver-tty'],['vue'],240,manifest=C/'tty',kind='build');context.tick()
 context.start_phase('builds',policy.CAPS['builds'])
 context.compile('host',['--bin','rag3weaver-backend'],policy.FEATURES,900,kind='build')
 for label,selection,features in [('large',['--lib'],policy.FEATURES),('e2e',['--test','e2e'],policy.FEATURES),('nav',['--lib'],[*policy.FEATURES,'navigateur'])]:
  item=context.compile(label,selection,features,900);attester.attest(C,policy.APP,Path(item['copy']),item['artifact'],item['builds'],P,label,context.phase_deadline);context.inventory(label)
 context.tick();ledger=dict(source=policy.APP,pilot=pilot,created=time.monotonic(),artifacts=context.artifacts,inventories=context.inventories)
 check_artifacts(ledger);context.start_phase('publication',90);context.published('global',ledger);context.save('runtime-permitted.json',dict(ledger_sha256=evidence.sha(P/'global-ledger.json'),observed=time.monotonic()))
 context.start_phase('d6',policy.CAPS['d6']);service=__import__('broker').Broker(context);service.bind()
 try:
  # These are paths only; the controller never reads the service-account file.
  context.env.update(GOOGLE_APPLICATION_CREDENTIALS=str(Path.home()/'git_workspaces/rag3db/.vault/vertex-sa.json'),GOOGLE_CLOUD_PROJECT='lr-hub-472010',GOOGLE_CLOUD_LOCATION='global',RAG3WEAVER_EXIGER_GEMINI='1')
  d6.run(context,service);context.tick()
  service.mode='documentary';context.start_phase('canonical',policy.CAPS['canonical']);context.acknowledge('canonical')
  template=json.loads((C/'templates/backends/code/backend.json').read_text());decl=policy.sandbox(template,Q,F/'private-controller',Path(config['sdk_root']),os.environ['POSTE_FILS'])['workspace']['sandbox'];evidence.save(Q/'documentary-sandbox.json',decl)
  env=dict(context.env,CIBLE=str(T.relative_to(R)),SUITE_DU_TEMOIN='lib',SUITES_EN_PLUS=' '.join(policy.SUITES[2:]),RAG3WEAVER_DOCUMENTATION_SANDBOX_FILE=str(Q/'documentary-sandbox.json'),RAG3WEAVER_TARGET_REPRENDRE='1',RAG3WEAVER_EXIGER_NAVIGATEUR='1',RAG3WEAVER_PAQUETS_REACT=str(R/'paquets-react'))
  env.pop('LD_LIBRARY_PATH',None);official=P/'official';policy.require(not official.exists(),'canonical log root reused')
  extras=[official/(s+'.log') for s in ['temoins',*policy.SUITES,'check_features','preuve','lib','navigateur','resume']]
  context.run('canonical',[str(R/'passe_epinglee.sh'),W.name,str(official.relative_to(R)),'daemon_host::teammate_tests'],policy.CAPS['canonical'],env=env,extra_logs=extras)
  # The NAV selectors are source-derived before classification, never inferred from successful RAW.
  nav_leaves=[]
  for file in (C/'src').rglob('*.rs'):
   lines=file.read_text().splitlines()
   for i,line in enumerate(lines):
    if 'feature = "navigateur"), ignore' in line:
     nav_leaves+=re.findall(r'fn ([a-z0-9_]+)\(', '\n'.join(lines[i:i+5]))
  nav_names=[n for n in context.inventories['nav']['names'] if any(leaf in n for leaf in nav_leaves)]
  required=subprocess.check_output(['bash',str(W/'tools/poste/required_suites.sh'),policy.BASE,policy.APP],cwd=W,text=True).splitlines();policy.require(not required,'additional required suite needs explicit frozen recipe')
  classification=evidence.canonical(official,context.inventories,nav_names,policy.SUITES);context.save('canonical-independent-classification.json',classification)
  proof=(official/'preuve.log').read_text();policy.require(re.search(r'(Checking|Compiling) rag3weaver v0\.1\.0 \('+re.escape(str(C))+r'\)',proof),'proof not from actual union source')
  context.tick();policy.require(all(value.get('done') for value in service.sessions.values()),'unjudged fixture Cargo invocation remains')
  check_artifacts(ledger,live=True);guard(config);context.observer.complete()
  context.save('driver-result.json',dict(status='GREEN_GLOBAL_D6_AND_CANONICAL',source=policy.APP,pilot=pilot,finished=time.monotonic(),deadline=end,d6=json.loads((P/'d6-result.json').read_text()),ledger_sha256=evidence.sha(P/'global-ledger.json'),publications=context.ledgers,canonical=dict(path=str(official),summary_sha256=evidence.sha(official/'resume.log')),not_a_master_permission=True))
 finally:service.close()
 return 0

def main():
 configure();mode,pilot=sys.argv[1:]
 if mode=='--launch':return launch(pilot)
 if mode=='--arm':arm(pilot);return 97
 if mode=='--freeze':
  cache.source_clean(Path(__file__).resolve().parents[4],pilot);cache.freeze_and_transition();return 0
 if mode=='--prepare':prepare(pilot);return 0
 if mode=='--admit':return compose(pilot)
 if mode=='--driver':
  try:return driver(pilot)
  except BaseException as error:latch('driver refused',repr(error));raise
 raise RuntimeError('explicit known mode required')

if __name__=='__main__':sys.exit(main())
