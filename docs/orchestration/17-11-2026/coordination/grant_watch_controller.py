"""One owner for watcher and driver, under the existing common whole deadline.

No network, service policy or signal-based cleanup. Unexpected child exits and
refusals latch first-red; the common scope performs attributed cleanup on red.
"""
from contextlib import ExitStack
from pathlib import Path
import hashlib,json,os,stat,subprocess,tempfile,time
import grant_watch_actor as actor
import provider_health_transport as strict


def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def read_owned(path):
 path=Path(path)
 if not path.is_absolute() or path.resolve()!=path or '.env' in path.parts or '.vault' in path.parts:raise ValueError('ambiguous/protected controller artifact')
 with path.open('rb') as stream:
  info=os.fstat(stream.fileno())
  if not stat.S_ISREG(info.st_mode) or info.st_uid!=os.getuid():raise ValueError('controller artifact is not regular/owned')
  raw=stream.read(1048577)
  if len(raw)>1048576:raise ValueError('controller artifact too large')
 return strict.decode(raw),hashlib.sha256(raw).hexdigest()


def publish(path,value):
 path=Path(path);stage=None
 try:
  with tempfile.NamedTemporaryFile(mode='w',dir=path.parent,prefix='.'+path.name+'.',delete=False) as stream:
   stage=Path(stream.name);json.dump(value,stream,indent=2,allow_nan=False);stream.write('\n');stream.flush();os.fsync(stream.fileno())
  os.link(stage,path)
 finally:
  if stage is not None:stage.unlink(missing_ok=True)


def first_red(config,reason):
 try:publish(config['paths']['first_red'],dict(reason=str(reason),observed=time.monotonic(),actor='GRANT_WATCH_CONTROLLER'))
 except FileExistsError:pass


def check_scope(config):
 now=time.monotonic();end=strict.finite(config['common_end_monotonic'])
 if now>=end:raise ValueError('original common deadline expired')
 if Path(config['paths']['first_red']).exists():raise ValueError('first refusal already exists')
 authority,_=read_owned(config['authority']['path']);expected=config['authority']
 if authority.get('etat')!='actif' or authority.get('pid')!=expected['pid'] or authority.get('debut')!=expected['start'] or authority.get('cgroup')!=expected['cgroup'] or authority.get('script_sha256')!=expected['script_sha256'] or authority.get('echeance')!=end:raise ValueError('common authority changed')
 return now


def _identity(config,pid,start):
 actual=actor.basic_process(pid)
 if actual['start']!=start or actual['uid']!=config['authority']['uid'] or actual['cgroup']!=config['authority']['cgroup'] or actual['state'] not in ['R','S','D','I']:raise ValueError('owned controller identity not live/exact')
 return actual


def check_armed(config,config_sha256,child):
 check_scope(config)
 if child.poll() is not None:raise ValueError('producer exited, including unexpected zero')
 receipt,receipt_sha=read_owned(config['paths']['armed']);actual=actor.process_snapshot(child.pid)
 actor.validate_armed(receipt,config,config_sha256,actual,expected_pid=child.pid)
 heartbeat,_=read_owned(config['paths']['heartbeat']);actor.validate_heartbeat(receipt,config,heartbeat,time.monotonic())
 if child.poll() is not None:raise ValueError('producer exited during independent ARMED verification')
 check_scope(config)
 return receipt,receipt_sha,actual


def read_admission(path,expected_sha256=None):
 """Fresh OS/loop observations from the controller's immutable admission."""
 admission,admission_sha=read_owned(path)
 if expected_sha256 is not None and admission_sha!=expected_sha256:raise ValueError('producer admission SHA changed')
 if admission.get('kind')!='INDEPENDENT_PRODUCER_ADMISSION':raise ValueError('producer admission not from controller')
 config,config_sha=read_owned(admission['config_path'])
 if config_sha!=admission['config_sha256']:raise ValueError('producer config SHA changed after admission')
 root=Path(admission['config_path']).parent
 if Path(path).parent!=root or any(Path(value).parent!=(Path(config['prepared_path']).parent if key=='grant' else root) for key,value in config['paths'].items()):raise ValueError('producer admission outside private job')
 prepared,prepared_sha=read_owned(config['prepared_path'])
 if prepared_sha!=config['prepared_sha256'] or prepared_sha!=admission['prepared_sha256'] or prepared.get('pilot')!=config['pilot']:raise ValueError('producer prepared/source binding changed')
 for key in ['actor_nonce','ready_nonce','pilot','allocation']:
  if admission.get(key)!=config.get(key):raise ValueError('producer admission binding changed '+key)
 check_scope(config)
 receipt,receipt_sha=read_owned(config['paths']['armed'])
 if receipt_sha!=admission['armed_sha256'] or admission['armed_path']!=config['paths']['armed']:raise ValueError('producer ARMED changed')
 actual=actor.process_snapshot(admission['expected_pid'])
 actor.validate_armed(receipt,config,config_sha,actual,expected_pid=admission['expected_pid'])
 heartbeat,_=read_owned(config['paths']['heartbeat']);actor.validate_heartbeat(receipt,config,heartbeat,time.monotonic())
 check_scope(config)
 return dict(admission=admission,admission_sha256=admission_sha,config=config,config_sha256=config_sha,receipt=receipt,receipt_sha256=receipt_sha,actual=actual,heartbeat=heartbeat,observed_monotonic=time.monotonic())

def supervise(config_path,config_sha256,driver_argv):
 config=None
 try:
  config,actual_sha=read_owned(config_path)
  if actual_sha!=config_sha256:raise ValueError('external controller config SHA differs')
  root=Path(config_path).parent
  if any(Path(path).parent!=(Path(config['prepared_path']).parent if key=='grant' else root) or Path(path).name in ['.env','.vault'] for key,path in config['paths'].items()):raise ValueError('controller output outside its private root')
  now=check_scope(config)
  if not strict.finite(config['issued_monotonic'])<=now<strict.finite(config['arm_deadline_monotonic'])<=config['common_end_monotonic']:raise ValueError('ARMED phase lacks original budget')
  if config['controller']['pid']!=os.getpid():raise ValueError('controller PID is not this process')
  _identity(config,os.getpid(),config['controller']['start'])
  _identity(config,config['authority']['pid'],config['authority']['start'])
  prepared,prepared_sha=read_owned(config['prepared_path'])
  if prepared_sha!=config['prepared_sha256'] or prepared.get('pilot')!=config['pilot']:raise ValueError('prepared source/allocation differs')
  script=Path(config['actor']['script_path']);policy=Path(config['policy']['path'])
  for path,expected in [(script,config['actor']['script_sha256']),(policy,config['policy']['sha256'])]:
   if path.parent!=Path(config['prepared_path']).parent or path.is_symlink() or not path.is_file() or path.stat().st_uid!=os.getuid() or sha(path)!=expected:raise ValueError('pinned producer/policy source differs')
  prefix=[config['actor']['exe'],'-B',str(script),'--watch',str(config_path)]
  if config['actor']['argv_prefix']!=prefix:raise ValueError('actor argv is not the exact admitted command')
  for path in config['paths'].values():
   if Path(path).exists():raise ValueError('producer artifacts already exist before spawn')
  with ExitStack() as files:
   out=files.enter_context((root/'watcher.stdout').open('xb'));err=files.enter_context((root/'watcher.stderr').open('xb'))
   child=subprocess.Popen(prefix+[config_sha256],stdin=subprocess.DEVNULL,stdout=out,stderr=err,cwd=root)
   while True:
    now=check_scope(config)
    if child.poll() is not None:raise ValueError('producer exited before ARMED')
    if now>=config['arm_deadline_monotonic']:raise ValueError('producer never armed before its fixed cutoff')
    if Path(config['paths']['armed']).exists() and Path(config['paths']['heartbeat']).exists():
     receipt,receipt_sha,actual=check_armed(config,config_sha256,child);break
    time.sleep(min(.05,config['arm_deadline_monotonic']-now))
   admission=dict(kind='INDEPENDENT_PRODUCER_ADMISSION',config_path=str(config_path),config_sha256=config_sha256,armed_path=config['paths']['armed'],armed_sha256=receipt_sha,expected_pid=child.pid,process=actual,actor_nonce=config['actor_nonce'],ready_nonce=config['ready_nonce'],pilot=config['pilot'],allocation=config['allocation'],prepared_sha256=config['prepared_sha256'])
   publish(root/'watcher-admission.json',admission)
   check_armed(config,config_sha256,child)  # Recheck after receipt fsync, before any DB driver.
   driver_out=files.enter_context((root/'controller-driver.stdout').open('xb'));driver_err=files.enter_context((root/'controller-driver.stderr').open('xb'))
   driver=subprocess.Popen(driver_argv,stdin=subprocess.DEVNULL,stdout=driver_out,stderr=driver_err,cwd='/var/tmp')
   last_full=time.monotonic()
   while True:
    now=check_scope(config)
    if child.poll() is not None:raise ValueError('producer disappeared while driver was active')
    heartbeat,_=read_owned(config['paths']['heartbeat']);actor.validate_heartbeat(receipt,config,heartbeat,now)
    if now-last_full>=1.:
     check_armed(config,config_sha256,child);last_full=now
    rc=driver.poll()
    if rc is not None:break
    time.sleep(min(.05,config['common_end_monotonic']-now))
   if rc!=0:raise ValueError('controlled driver refused: '+str(rc))
   check_armed(config,config_sha256,child)
   publish(config['paths']['release'],dict(kind='DRIVER_FINISHED',config_sha256=config_sha256,actor_nonce=config['actor_nonce'],ready_nonce=config['ready_nonce'],pilot=config['pilot'],allocation=config['allocation']))
   release_due=min(time.monotonic()+2.,config['common_end_monotonic'])
   while child.poll() is None:
    now=check_scope(config)
    if now>=release_due:raise ValueError('producer release not reaped inside original deadline')
    time.sleep(min(.02,release_due-now))
   check_scope(config)
   if child.returncode!=0:raise ValueError('producer refused terminal release')
   terminal,_=read_owned(config['paths']['result'])
   if terminal.get('kind')!='RELEASED' or terminal.get('config_sha256')!=config_sha256 or terminal.get('actor_nonce')!=config['actor_nonce']:raise ValueError('producer terminal release not attested')
   check_scope(config)
   return 0
 except BaseException as error:
  if config is not None:
   try:first_red(config,error)
   except BaseException as evidence_error:error.add_note('first-red publication failed: '+type(evidence_error).__name__)
  raise_or_print=type(error).__name__+': '+str(error)
  print(raise_or_print,flush=True)
  return 1
