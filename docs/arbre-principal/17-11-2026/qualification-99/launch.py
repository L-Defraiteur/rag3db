#!/usr/bin/env python3
"""Proposed metadata seal / poste envelope / post-Common composition.

None of these entries is authorized by publication. No service, DB or cleanup.
"""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,stat,subprocess,sys,time

APP='99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
ROOT=Path('/home/lucied/.cache/rag3weaver-build')
Q=ROOT/'qualification-cli99-control-r1';P=ROOT/'qualification-cli99-pc-r1'
S=ROOT/'qualification-cli99-scopes-r1';T=ROOT/'target-arbre-principal-qualif'

def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def read(path):
 path=Path(path)
 if any(s in ['.env','.vault'] for s in path.parts) or path.resolve()!=path or path.is_symlink() or not stat.S_ISREG(path.stat().st_mode) or path.stat().st_uid!=os.getuid():raise ValueError('owned regular unambiguous metadata required')
 return json.loads(path.read_text())

def save_new(path,value):
 with Path(path).open('x') as stream:json.dump(value,stream,indent=2,allow_nan=False);stream.write('\n');stream.flush();os.fsync(stream.fileno())

def git(root,*args):return subprocess.check_output(['/usr/bin/git','-C',str(root),*args],text=True).strip()

def guard(cfg,config_sha):
 if sha(Q/'prepared.json')!=config_sha or cfg['app']!=APP or cfg['allocation'].startswith('PENDING'):raise ValueError('prepared source/allocation not frozen')
 for root,head in [(Path(cfg['source_root']),APP),(Path(cfg['pilot_root']),cfg['pilot'])]:
  if git(root,'rev-parse','HEAD')!=head or git(root,'status','--porcelain'):raise ValueError('source/pilot not exact and clean')
 for path,digest in cfg['pins'].items():
  path=Path(path)
  if any(s in ['.env','.vault'] for s in path.parts) or path.resolve()!=path or path.is_symlink() or not path.is_file() or sha(path)!=digest:raise ValueError('prepared input differs')
 for name,digest in cfg['packet'].items():
  path=Q/name
  if path.resolve()!=path or path.is_symlink() or not path.is_file() or path.stat().st_uid!=os.getuid() or sha(path)!=digest:raise ValueError('copied packet differs')

def prepare(allocation):
 source=Path(__file__).resolve().parent;pilot=source.parents[3]
 cfg=read(source/'config-proposal.json')
 if os.uname().nodename.lower()!='luciepc' or not allocation or allocation.startswith('PENDING') or any(path.exists() or path.is_symlink() for path in [P,Q,S,T]):raise ValueError('PC/new paths/proposed exact allocation required')
 if git(pilot,'status','--porcelain') or git(Path(cfg['source_root']),'rev-parse','HEAD')!=APP or git(Path(cfg['source_root']),'status','--porcelain'):raise ValueError('clean exact source/pilot required before seal')
 cfg.update(allocation=allocation,kind='PREPARED_NOT_EXECUTED',pilot_root=str(pilot),pilot=git(pilot,'rev-parse','HEAD'),launcher_sha256=sha(source/'launch.py'),driver_sha256=sha(source/'build_private_cli.py'))
 Q.mkdir(mode=0o700);packet={}
 for name in ['launch.py','build_private_cli.py']:
  raw=(source/name).read_bytes();(Q/name).write_bytes(raw);os.chmod(Q/name,0o600);packet[name]=sha(Q/name)
 for name,item in cfg['helper_origins'].items():
  raw=subprocess.check_output(['/usr/bin/git','-C',str(pilot),'show',item['ref']+':'+item['path']])
  if hashlib.sha256(raw).hexdigest()!=item['sha256']:raise ValueError('helper Git pin differs')
  (Q/name).write_bytes(raw);os.chmod(Q/name,0o600);packet[name]=sha(Q/name)
 cfg['packet']=packet;save_new(Q/'prepared.json',cfg);os.chmod(Q/'prepared.json',0o600)
 digest=sha(Q/'prepared.json');guard(cfg,digest)
 print(json.dumps(dict(prepared_sha256=digest,build_started=False,DB_opened=False)))

def compose(config_sha,allocation):
 cfg=read(Q/'prepared.json');guard(cfg,config_sha)
 if cfg['allocation']!=allocation or sha(__file__)!=cfg['launcher_sha256']:raise ValueError('wrong frozen launcher/allocation')
 spec=importlib.util.spec_from_file_location('common',Path(cfg['source_root'])/'extension/rag3weaver/scripts/confiner_validation.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','2398s','/usr/bin/python3.14','-B',str(Q/'launch.py'),'--compose',config_sha,'--allocation',allocation]
 parent=Path('/proc')/str(os.getppid())
 if os.readlink(parent/'exe')!=cfg['timeout'] or [os.fsdecode(x) for x in (parent/'cmdline').read_bytes().split(b'\0') if x]!=expected or not common.poste_tenu(common.processus()):raise ValueError('real outer/poste absent')
 started=time.monotonic();save_new(Q/'poste-entry.json',dict(observed_monotonic=started,allocation=allocation))
 sys.path.insert(0,str(Q));import grant_watch_actor as frames
 authority=None;record=None
 def check():
  if authority is None or time.monotonic()>=authority['echeance'] or time.monotonic()-started>=2400 or (P/'first-red.json').exists():raise ValueError('deadline/first red forbids composition')
 try:
  if any(path.exists() for path in [P,S,T]):raise ValueError('source/evidence/target not fresh after poste')
  guard(cfg,config_sha)
  argv=['/usr/bin/python3.14',str(Path(cfg['source_root'])/'extension/rag3weaver/scripts/confiner_validation.py'),'lancer','--delai','2330','--preuves',str(S),'--','/usr/bin/python3.14','-B',str(Q/'build_private_cli.py'),'--config',str(Q/'prepared.json'),'--config-sha256',config_sha]
  rc=subprocess.run(argv,cwd='/var/tmp',env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1')).returncode
  ref=read(P/'authority-reference.json');path=Path(ref['path'])
  if not path.is_relative_to(S) or path.name!='autorite.json':raise ValueError('authority reference outside this scope')
  authority=read(path);verdict=read(path.parent/'verdict.json')
  if rc or authority.get('etat')!='termine' or authority.get('pid')!=ref['pid'] or authority.get('debut')!=ref['start'] or authority.get('cgroup')!=ref['cgroup'] or authority.get('echeance')!=ref['deadline'] or verdict.get('verdict')!='VERT' or type(verdict.get('code')) is not int or verdict['code']!=0 or verdict.get('restes')!=[] or verdict.get('nettoye') is not True or Path(verdict['prive']).exists():raise ValueError('Common not green/terminated/owned-quiet/bus-absent')
  result=read(P/'result.json')
  if result.get('app')!=APP or result.get('common_end')!=authority['echeance'] or result.get('status')!='BUILT_AND_STATIC_PAIR_CHECKED_ONLY' or any(result.get(key) is not False for key in ['SDK_runtime_qualified','backend_invoked','DB_opened','provider_contact','reader_DB_qualified','live_authorized']):raise ValueError('driver result exceeds admitted static-only scope')
  ledger=read(P/'ledger-before-first-invocation.json')
  import build_private_cli as driver
  if ledger.get('app')!=APP or ledger.get('config_sha256')!=config_sha or ledger.get('features')!=driver.FEATURES or set(ledger.get('artifacts',{}))!=set(driver.NAMES)|{'typed-reader','ids','query'}:raise ValueError('post-Common ledger incomplete/foreign')
  for item in ledger['artifacts'].values():
   if sha(item['path'])!=item['sha256'] or sha(item['origin'])!=item['sha256']:raise ValueError('post-Common artifact hash differs')
  guard(cfg,config_sha);check()
  record=dict(status='BUILD_STATIC_PAIR_OBSERVED',source=APP,prepared_sha256=config_sha,common_deadline=authority['echeance'],scope=verdict,backend_runtime_qualified=False,reader_DB_qualified=False,live_authorized=False)
  frames.write_frame(Q/'composed.json',record,guard=check);check()
  return 0
 except BaseException as error:
  first=P/'first-red.json' if P.is_dir() else Q/'first-red.json'
  try:frames.write_frame(first,dict(reason=str(error),observed=time.monotonic()))
  except FileExistsError:pass
  refusal=dict(status='RED',error=str(error),live_authorized=False)
  target=Q/'composed.json'
  if target.exists():
   if record is None or read(target)!=record:raise ValueError('foreign composed result cannot be replaced') from error
   frames.write_frame(target,refusal,replace=True)
  else:frames.write_frame(target,refusal)
  return 1

def launch(config_sha,allocation):
 cfg=read(Q/'prepared.json');guard(cfg,config_sha)
 if cfg['allocation']!=allocation or sha(__file__)!=cfg['launcher_sha256'] or 'RAG3WEAVER_VALIDATION_RECU' in os.environ or any(path.exists() or path.is_symlink() for path in [P,S,T,Q/'outer.log',Q/'composed.json']):raise ValueError('exact fresh unconsumed allocation required')
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',PYTHONDONTWRITEBYTECODE='1')
 with (Q/'outer.log').open('x') as out:return subprocess.run([str(Path(cfg['source_root'])/'extension/rag3weaver/scripts/poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','2398s','/usr/bin/python3.14','-B',str(Q/'launch.py'),'--compose',config_sha,'--allocation',allocation],env=env,cwd='/var/tmp',stdout=out,stderr=subprocess.STDOUT).returncode

if __name__=='__main__':
 ap=argparse.ArgumentParser();mode=ap.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare',action='store_true');mode.add_argument('--launch');mode.add_argument('--compose');ap.add_argument('--allocation',required=True);args=ap.parse_args()
 if args.prepare:prepare(args.allocation)
 elif args.launch:sys.exit(launch(args.launch,args.allocation))
 else:sys.exit(compose(args.compose,args.allocation))
