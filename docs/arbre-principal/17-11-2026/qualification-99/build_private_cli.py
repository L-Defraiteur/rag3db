#!/usr/bin/env python3
"""Proposed PC build and static pair check ONLY, under an exact Common GO.

No backend invocation, database, provider, install, service or target cleanup.
Configuration and the copied helpers must be frozen/reviewed before execution.
"""
from pathlib import Path
import argparse,hashlib,importlib.util,io,json,os,shutil,stat,subprocess,sys,time

APP='99fef92ce6ee9a0007eaf99c7c3a6a3789061c57'
FEATURES=['code','daemon','default','openai-llm','rag3db-native']
NAMES=['rag3weaver-backend','rag3db-check-lib']
ROOT=Path('/home/lucied/.cache/rag3weaver-build')
TARGET=ROOT/'target-arbre-principal-qualif'

def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def load(path,expected):
 path=Path(path)
 if any(s in ['.env','.vault'] for s in path.parts):raise ValueError('protected configuration refused')
 if path.resolve()!=path or path.is_symlink() or not stat.S_ISREG(path.stat().st_mode) or path.stat().st_uid!=os.getuid() or sha(path)!=expected:raise ValueError('owned regular pinned input required: '+str(path))
 return json.loads(path.read_text())

def select_artifacts(log,crate):
 artifacts={name:[] for name in NAMES}
 for line in log.splitlines():
  event=json.loads(line)
  if event.get('reason')=='compiler-message' and event.get('message',{}).get('level')=='error':raise ValueError('compiler diagnostic is an error')
  if event.get('reason')=='compiler-artifact' and event.get('target',{}).get('name') in NAMES and event.get('executable'):
   name=event['target']['name']
   if event['target']['kind']!=['bin'] or event['profile']['test'] is not False or event['profile'].get('opt_level')!='3' or event['profile'].get('debug_assertions') is not False or sorted(event['features'])!=FEATURES or Path(event['manifest_path'])!=crate/'Cargo.toml' or Path(event['target']['src_path'])!=crate/'src/bin'/(name+'.rs'):raise ValueError('actual Cargo bin/features/source/release profile differs')
   artifacts[name].append(event)
 if any(len(events)!=1 for events in artifacts.values()):raise ValueError('Cargo executable artifacts must each be unique')
 return {name:events[0] for name,events in artifacts.items()}

def main(config_path,config_sha):
 cfg=load(config_path,config_sha);p=Path(cfg['evidence']);w=Path(cfg['source_root']);q=Path(config_path).parent
 if set(cfg['readers'])!={'typed-reader','ids','query'}:raise ValueError('exact existing reader set required')
 if cfg['app']!=APP or not cfg['allocation'] or cfg['allocation'].startswith('PENDING'):raise ValueError('exact source/allocation not frozen')
 if os.uname().nodename.lower()!='luciepc' or Path(cfg['target'])!=TARGET or p.parent!=ROOT or not p.name.startswith('qualification-cli99-'):raise ValueError('unexpected host/evidence/unique session target')
 if any(x.resolve()!=x for x in [w,p,q,TARGET]) or p.exists() or TARGET.exists():raise ValueError('fresh canonical evidence/target required; no adoption or cleanup')
 required=[w/'extension/rag3weaver/scripts'/name for name in ['confiner_validation.py','poste','cargo_magasin.sh','cargo_garde.py','lib_du_magasin.sh','garde_target.sh']]+[q/name for name in ['grant_watch_actor.py','provider_health_transport.py','typed_rows.py']]
 if any(str(path) not in cfg['pins'] for path in required):raise ValueError('Common/Cargo/export/guard/framing helper pins missing')
 for path,digest in cfg['pins'].items():
  path=Path(path)
  if any(s in ['.env','.vault'] for s in path.parts) or path.resolve()!=path or path.is_symlink() or not path.is_file() or sha(path)!=digest:raise ValueError('source/tool/SDK pin differs: '+str(path))
  info=path.stat();expected=cfg['file_metadata'].get(str(path))
  if expected is not None and (info.st_uid!=expected['uid'] or stat.S_IMODE(info.st_mode)!=expected['mode'] or info.st_size!=expected['size']):raise ValueError('pinned tool/reader/SDK UID/mode/size differs')
  if (path.is_relative_to(q) or path.is_relative_to(w)) and info.st_uid!=os.getuid():raise ValueError('foreign source/helper owner')
 if sha(__file__)!=cfg['driver_sha256']:raise ValueError('driver is not the copied frozen source')
 def git(*args):return subprocess.check_output([cfg['git'],'-C',str(w),*args],text=True).strip()
 if git('rev-parse','HEAD')!=APP or git('status','--porcelain'):raise ValueError('app worktree not exact and clean')
 parser=w/'extension/rag3weaver/codeparsers'
 if git('submodule','status','--recursive').startswith(('-','+','U')) or subprocess.check_output([cfg['git'],'-C',str(parser),'rev-parse','HEAD'],text=True).strip()!='bd133850f89d6b12a3ca2b2f6ce238785ed37e3a' or subprocess.check_output([cfg['git'],'-C',str(parser),'status','--porcelain'],text=True):raise ValueError('parser checkout not exact and clean')
 spec=importlib.util.spec_from_file_location('common',w/'extension/rag3weaver/scripts/confiner_validation.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 authority=common.autorite(common.processus());end=authority['echeance']
 sys.path.insert(0,str(q));import grant_watch_actor as frames
 p.mkdir(mode=0o700);first=p/'first-red.json'
 def check():
  if time.monotonic()>=end or first.exists():raise ValueError('original Common deadline/first red forbids continuation')
  current=common.autorite(common.processus())
  if any(current[k]!=authority[k] for k in ['pid','debut','cgroup','script_sha256','echeance']):raise ValueError('Common authority changed')
  if time.monotonic()>=end or first.exists():raise ValueError('deadline/first red changed during check')
 def save(name,value):frames.write_frame(p/name,value,guard=check);check()
 env={k:os.environ[k] for k in ['HOME','USER','LANG','PATH','DBUS_SESSION_BUS_ADDRESS','XDG_RUNTIME_DIR','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 sdk=Path(cfg['sdk']);crate=w/'extension/rag3weaver'
 env.update(RAG3DB_LIB=sdk.name,RAG3DB_LIBS_DIR=str(sdk.parent),CARGO_TARGET_DIR=str(TARGET),CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='8',RAG3WEAVER_TARGET_MAX_GO='32',RUSTFLAGS='-C link-args=-Wl,-rpath,'+str(sdk))
 env.update(PATH=str(Path.home()/'.cargo/bin')+':/usr/bin:/bin',RUSTUP_TOOLCHAIN=cfg['rustup_toolchain'],RUSTC=cfg['rustc'])
 for name,expected_path in cfg['path_tools'].items():
  found=shutil.which(name,path=env['PATH'])
  if not found or str(Path(found).resolve())!=expected_path:raise ValueError('PATH tool resolves differently: '+name)
 for variable,name in [('TMPDIR','tmp'),('XDG_CONFIG_HOME','config'),('XDG_DATA_HOME','data'),('XDG_CACHE_HOME','cache'),('XDG_STATE_HOME','xdg-state'),('RAG3WEAVER_STATE_DIR','state-fixture')]:
  directory=p/name;directory.mkdir(mode=0o700);env[variable]=str(directory)
 def run(argv,label,cap,environment):
  check();limit=min(float(cap),end-time.monotonic()-30)
  if limit<=0:raise ValueError('insufficient original budget for '+label)
  position=0;pending=b''
  with (p/(label+'.stdout')).open('xb') as out,(p/(label+'.stderr')).open('xb') as err:
   actual=[cfg['timeout'],'--signal=TERM','--kill-after=1s',str(limit)+'s',*argv]
   child=subprocess.Popen(actual,cwd=crate,env=environment,stdin=subprocess.DEVNULL,stdout=out,stderr=err)
   deadline=time.monotonic()+limit
   while child.poll() is None:
    check()
    if label=='build':
     with (p/'build.stdout').open('rb') as stream:stream.seek(position);chunk=stream.read(1048576);position+=len(chunk)
     pending+=chunk;lines=pending.split(b'\n');pending=lines.pop()
     for line in lines:
      event=json.loads(line)
      if event.get('reason')=='compiler-message' and event.get('message',{}).get('level')=='error':raise ValueError('first compiler error; Common performs attributed cleanup')
     if len(pending)>1048576:raise ValueError('oversized/incomplete Cargo event')
    if time.monotonic()>=deadline:raise ValueError('phase expired: '+label)
    if TARGET.exists():
     usage=subprocess.check_output([cfg['du'],'-sb',str(TARGET)],text=True)
     if int(usage.split()[0])>32*1024**3:raise ValueError('unique target exceeds 32GiB; preserve, never purge')
    time.sleep(.2)
   rc=child.returncode
  save(label+'-process.json',dict(argv=actual,rc=rc,deadline=deadline))
  if rc!=0:raise ValueError(label+' refused: '+str(rc))
  check()
 try:
  save('source-before-build.json',dict(app=APP,config_sha256=config_sha,allocation=cfg['allocation'],pins=cfg['pins'],features_expected=FEATURES,target=str(TARGET),common_end=end,sdk_runtime_loaded=False))
  save('configured-build-profile.json',dict(kind='CONFIGURED_NOT_PROCESS_OBSERVED',values={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'}))
  argv=[cfg['bash'],'scripts/cargo_magasin.sh','build','--offline','--locked','--release','--bin',NAMES[0],'--bin',NAMES[1],'--features','rag3db-native,code,daemon,openai-llm','--message-format=json-render-diagnostics','--jobs','8']
  run(argv,'build',cfg['caps']['build'],env)
  artifacts=select_artifacts((p/'build.stdout').read_text(),crate)
  copied={};destination=p/'artifacts';destination.mkdir(mode=0o700)
  for name,event in artifacts.items():
   origin=Path(event['executable'])
   if origin.resolve()!=origin or origin!=TARGET/'release'/name or not stat.S_ISREG(origin.stat().st_mode) or origin.stat().st_uid!=os.getuid():raise ValueError('unexpected compiler output')
   before=sha(origin);target=destination/name;shutil.copy2(origin,target)
   if sha(target)!=before or sha(origin)!=before:raise ValueError('copied ELF differs')
   copied[name]=dict(path=str(target),origin=str(origin),sha256=before,artifact=event)
  for name,item in cfg['readers'].items():
   origin=Path(item['path']);before=sha(origin)
   if origin.is_symlink() or not origin.is_file() or origin.stat().st_uid!=os.getuid() or before!=item['sha256']:raise ValueError('existing reader differs')
   target=destination/name;shutil.copy2(origin,target)
   if sha(target)!=before or sha(origin)!=before:raise ValueError('reader copy differs')
   copied[name]=dict(path=str(target),origin=str(origin),sha256=before,runtime_qualification_carried=item['scope'])
  # All executables are copied/hashed and ledger persisted BEFORE any invocation.
  save('ledger-before-first-invocation.json',dict(app=APP,artifacts=copied,features=FEATURES,config_sha256=config_sha,SDK_runtime_qualified=False))
  runtime_env=dict(env,LD_LIBRARY_PATH=str(sdk),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',RAYON_NUM_THREADS='4')
  save('configured-invocation-profile.json',dict(kind='CONFIGURED_NOT_PROCESS_OBSERVED',values={k:v for k,v in runtime_env.items() if k!='RAG3WEAVER_VALIDATION_JETON'}))
  run([copied['rag3db-check-lib']['path'],copied['rag3weaver-backend']['path'],str(sdk/'librag3db.so')],'static-pair',cfg['caps']['static_pair'],runtime_env)
  run([copied['typed-reader']['path'],'--self-test'],'typed-framing',cfg['caps']['reader_self_test'],runtime_env)
  spec=importlib.util.spec_from_file_location('typed_rows',q/'typed_rows.py');typed=importlib.util.module_from_spec(spec);spec.loader.exec_module(typed)
  decoded=[];typed.consume(io.BytesIO((p/'typed-framing.stdout').read_bytes()),decoded.append)
  if decoded!=[[('STRING',b'a\0|\nb'),('FLOAT',1.25),('STRING',b'')]]:raise ValueError('native typed framing/golden content differs')
  for item in copied.values():
   if sha(item['path'])!=item['sha256'] or sha(item['origin'])!=item['sha256']:raise ValueError('final ELF/reader hash differs')
  for path,digest in cfg['pins'].items():
   if sha(path)!=digest:raise ValueError('final source/tool/SDK hash differs')
  if git('rev-parse','HEAD')!=APP or git('status','--porcelain'):raise ValueError('final source differs')
  check();record=dict(status='BUILT_AND_STATIC_PAIR_CHECKED_ONLY',app=APP,common_end=end,SDK_runtime_qualified=False,backend_invoked=False,DB_opened=False,provider_contact=False,reader_DB_qualified=False,live_authorized=False)
  frames.write_frame(p/'result.json',record,guard=check);check()
  return 0
 except BaseException as error:
  try:frames.write_frame(first,dict(reason=str(error),observed=time.monotonic()))
  except FileExistsError:pass
  frames.write_frame(p/'result.json',dict(status='RED',error=str(error),live_authorized=False),replace=(p/'result.json').exists())
  return 1

if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--config',required=True);ap.add_argument('--config-sha256',required=True);args=ap.parse_args();sys.exit(main(args.config,args.config_sha256))
