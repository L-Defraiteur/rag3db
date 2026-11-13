"""Coherent selected-profile observation of an owned Cargo; no raw env published."""
from pathlib import Path
import hashlib,json,os,signal,time

class ProcOps:
 def open(self,pid):return os.pidfd_open(pid)
 def close(self,fd):os.close(fd)
 def signal(self,fd,sig):signal.pidfd_send_signal(fd,sig)
 def now(self):return time.monotonic()
 def sleep(self):time.sleep(.002)
 def identity(self,pid):
  p=Path('/proc')/str(pid);s=(p/'stat').read_text().rsplit(') ',1)[1].split()
  with (p/'exe').open('rb') as f:sha=hashlib.file_digest(f,'sha256').hexdigest()
  tracer=next(line.split(':',1)[1].strip() for line in (p/'status').read_text().splitlines() if line.startswith('TracerPid:'))
  return dict(pid=pid,starttime=s[19],uid=p.stat().st_uid,cgroup=(p/'cgroup').read_text().strip().split('::',1)[1],exe=os.readlink(p/'exe'),exe_sha256=sha,state=s[0],tracer_pid=int(tracer))
 def threads(self,pid):
  result={}
  for t in (Path('/proc')/str(pid)/'task').iterdir():
   s=(t/'stat').read_text().rsplit(') ',1)[1].split();result[t.name]=dict(starttime=s[19],state=s[0])
  return result
 def snapshot(self,pid,keys):
  p=Path('/proc')/str(pid);data=(p/'environ').read_bytes();raw=dict(v.split(b'=',1) for v in data.split(b'\0') if b'=' in v)
  argvraw=(p/'cmdline').read_bytes();parts=argvraw.split(b'\0')
  if parts and parts[-1]==b'':parts.pop() # preserve real empty argv, remove only terminal NUL
  return dict(rawlen=len(data),argv_rawlen=len(argvraw),argv=[os.fsdecode(v) for v in parts],present={k:os.fsencode(k) in raw for k in keys},actual={k:os.fsdecode(raw[os.fsencode(k)]) if os.fsencode(k) in raw else None for k in keys})

def save_once(path,record):
 with Path(path).open('x') as f:json.dump(record,f,indent=2)

def same_owner(identity,expected):
 return all(identity[k]==expected[k] for k in ['pid','starttime','uid','cgroup'])

def observe(pid,expected,profile,path,deadline,ops=None):
 """Return OBSERVED_CARGO or NON_CARGO. Every failure refuses qualification."""
 ops=ops or ProcOps();path=Path(path);record=dict(pid=pid,expected_identity=expected,expected=profile,expected_presence={k:v is not None for k,v in profile.items()},observed=ops.now(),status='UNQUALIFIED');fd=None;initiated=False;primary=None
 try:
  fd=ops.open(pid)
  before=ops.identity(pid);record['before']=before
  assert same_owner(before,expected),'identity/ownership changed after pidfd_open'
  assert before['state'] not in ['T','t'] and before['tracer_pid']==0,'already stopped or traced; refuse without resume'
  if before['exe']!=expected['exe']:
   record['status']='NON_CARGO';save_once(path,record);return record
  # Identity/UID/CG were established AFTER pidfd_open and BEFORE this signal.
  ops.signal(fd,signal.SIGSTOP);initiated=True
  limit=min(deadline,ops.now()+1.0)
  while True:
   assert ops.now()<limit,'STOP deadline'
   current=ops.identity(pid);assert same_owner(current,expected),'identity changed while stopping'
   assert current['tracer_pid']==0,'tracer appeared while stopping'
   first=ops.threads(pid);second=ops.threads(pid)
   if first and first==second and all(v['state']=='T' for v in first.values()):break
   ops.sleep()
  current=ops.identity(pid) # Re-read exe only after group STOP is effective.
  assert same_owner(current,expected) and current['tracer_pid']==0,'identity changed at effective STOP'
  record['stopped']=current;record['threads_before']=second
  snapshot=ops.snapshot(pid,list(profile));record.update(snapshot)
  after=ops.identity(pid);tasks_after=ops.threads(pid);record['after']=after;record['threads_after']=tasks_after
  record['diff']={k:dict(expected=v,actual=snapshot['actual'][k]) for k,v in profile.items() if snapshot['actual'][k]!=v}
  record['status']='OBSERVED_CARGO' if current['exe']==expected['exe'] else 'NON_CARGO'
  # Immutable evidence always precedes qualification assertions.
  save_once(path,record)
  assert same_owner(after,expected) and after['tracer_pid']==0,'identity/tracer changed during capture'
  assert all(current[k]==after[k] for k in ['exe','exe_sha256','starttime','uid','cgroup']),'exec/identity changed during capture'
  assert tasks_after==second and all(v['state']=='T' for v in tasks_after.values()),'thread set not stopped and stable'
  assert ops.now()<limit,'capture deadline'
  if record['status']=='NON_CARGO':return record
  assert current['exe_sha256']==expected['exe_sha256'],'Cargo hash differs'
  assert snapshot['rawlen']>0 and snapshot['present']==record['expected_presence'],'empty or incorrect Cargo field presence for phase'
  assert not record['diff'],'Cargo selected profile differs'
  return record
 except BaseException as error:
  primary=error
  if not path.exists():
   record.update(error=repr(error));save_once(path,record)
  raise
 finally:
  if fd is not None:
   resume=dict(initiated=initiated,pidfd=fd,observed=ops.now())
   try:
    if initiated:ops.signal(fd,signal.SIGCONT);resume['continued']=True
   except BaseException as error:
    resume['error']=repr(error)
    if primary is None:raise
   finally:
    try:save_once(path.with_suffix('.resume.json'),resume)
    finally:ops.close(fd)
