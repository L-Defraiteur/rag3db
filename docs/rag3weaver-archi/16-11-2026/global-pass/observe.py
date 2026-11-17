"""Owned process observations. Selected fields only; no cleanup attestation."""
from pathlib import Path
import json,os,time
import cargo_stop,evidence,policy
from settings import *

class StrictOps(cargo_stop.ProcOps):
 def snapshot(self,pid,keys):
  row=super().snapshot(pid,keys);raw=(Path('/proc')/str(pid)/'environ').read_bytes()
  fields=[v.split(b'=',1)[0] for v in raw.split(b'\0') if b'=' in v]
  row['old_engine_present']=b'RAG3WEAVER_MOTEUR_ANCIEN' in fields
  row['duplicate_selected']=any(fields.count(os.fsencode(k))>1 for k in keys)
  return row

class Observer:
 def __init__(self,config,cgroup,end,latch):
  self.config=config;self.cgroup=cgroup;self.end=end;self.latch=latch;self.phase='preflight';self.seen={};self.cargo=[];self.runtimes=[];self.frames={};self.allowed={};self.source_roles={};self.unknown=[];self.protected=[];self.image_sha={}
  self.root=Path(config['sdk_root']);self.rustlib=Path(config['cargo_exe']).parent.parent/'lib'
  (P/'observations').mkdir(mode=0o700)
 def executable(self,path,manifest,native,label,ld=None):self.allowed[str(Path(path))]=dict(sha256=evidence.sha(path),manifest=str(manifest),native=native,label=label,ld=ld)
 def source(self,cwd,role):self.source_roles[str(Path(cwd))]=role
 def identity(self,proc):
  fields=(proc/'stat').read_text().rsplit(') ',1)[1].split();exe=os.readlink(proc/'exe');st=(proc/'exe').stat();key=(st.st_dev,st.st_ino,st.st_size,st.st_mtime_ns)
  if key not in self.image_sha:self.image_sha[key]=evidence.sha(proc/'exe')
  row=dict(pid=int(proc.name),starttime=fields[19],uid=proc.stat().st_uid,cgroup=(proc/'cgroup').read_text().strip().split('::',1)[1],exe=exe,exe_sha256=self.image_sha[key],state=fields[0],argv=[os.fsdecode(v) for v in (proc/'cmdline').read_bytes().split(b'\0') if v],cwd=os.readlink(proc/'cwd'),ppid=int(fields[1]))
  raw=(proc/'environ').read_bytes()
  field=next((v.split(b'=',1)[1] for v in raw.split(b'\0') if v.startswith(b'RAG3WEAVER_PASSE_E2E=')),None)
  if field:row['e2e_pass']=os.fsdecode(field)
  after=(proc/'stat').read_text().rsplit(') ',1)[1].split();policy.require(fields[19]==after[19] and fields[1]==after[1],'process identity changed during capture')
  return row
 def ancestors(self,row):
  result=[];pid=row['ppid']
  while pid>1 and pid!=os.getpid() and pid not in [r['pid'] for r in result]:
   item=self.identity(Path('/proc')/str(pid));result.append(item);pid=item['ppid']
  return result
 def runtime_ld(self,base=None):
  return ':'.join([str(T/'debug/deps'),str(T/'debug'),str(self.rustlib/'rustlib/x86_64-unknown-linux-gnu/lib'),str(self.rustlib),base or str(self.root)])
 def cargo_profile(self,row,ancestors):
  role=self.source_roles.get(row['cwd']);manifest=C/'tty' if self.phase=='tty' else C;ld=str(self.root)
  if role is not None:manifest=Path(role['manifest']);ld=role['ld']
  elif self.phase=='canonical':
   names=[r['argv'] for r in ancestors]
   guarded=any(any(a.endswith('/cargo_garde.py') for a in args) for args in names)
   e2e=any(any(a.endswith('/run_e2e.sh') for a in args) for args in names)
   in_test=any(r['exe'] in self.allowed for r in ancestors)
   if in_test:ld=self.runtime_ld(str(self.root)+':/usr/local/cuda/lib64' if e2e else None)
   elif row['argv'][1:2]==['metadata'] and '--all-features' in row['argv'] and not guarded:ld=''
   elif e2e:ld=str(self.root)+':/usr/local/cuda/lib64'
  # Rustup prepends its lib directory; raw direct Cargo in the D6 shim is given this declared value too.
  ld=':'.join([str(self.rustlib),*[x for x in ld.split(':') if x and x!=str(self.rustlib)]])
  return policy.profile(R.parent/'rag3db-libs',manifest,ld)
 def runtime_snapshot(self,proc,before,expected,path):
  value=dict(before=before,phase=self.phase,expected=expected)
  try:
   value['profile']=policy.selected((proc/'environ').read_bytes(),expected)
   value['maps']=(proc/'maps').read_text();value['after']=self.identity(proc)
  except BaseException as error:
   value['error']=repr(error);evidence.save(path,value);raise RuntimeError('runtime coherent snapshot refused') from error
  evidence.save(path,value)
  policy.require(all(before[k]==value['after'][k] for k in ['pid','starttime','uid','cgroup','exe','exe_sha256','argv','cwd']),'runtime identity changed during env/maps capture')
  policy.validate_profile(value['profile'])
  return value
 def tick(self,cleanup=False):
  if cleanup:raise RuntimeError('cleanup may not perform SDK/profile qualification')
  policy.require(time.monotonic()<self.end,'observer deadline')
  for proc in Path('/proc').iterdir():
   if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
   group=None
   try:
    if proc.stat().st_uid!=os.getuid():continue
    group=(proc/'cgroup').read_text().strip().split('::',1)[1]
    if group!=self.cgroup:continue
    fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[0]=='Z':continue
    row=self.identity(proc)
   except (FileNotFoundError,ProcessLookupError):continue
   except (OSError,ValueError) as error:
    # No profile claim can follow an attributable unreadable process.
    if group==self.cgroup:
     evidence.save(P/'observations'/('unknown-'+proc.name+'-'+str(time.monotonic_ns())+'.json'),dict(pid=int(proc.name),error=repr(error)))
     raise RuntimeError('owned process snapshot refused') from error
    continue
   key=(row['pid'],row['starttime'],row['exe_sha256']);self.frames[key]=row
   if row['exe']==self.config['cargo_exe'] and key not in self.seen:
    ancestors=self.ancestors(row);expected=self.cargo_profile(row,ancestors)
    path=P/'observations'/('cargo-'+str(row['pid'])+'-'+row['starttime']+'-'+self.phase+'.json')
    try:attempt=cargo_stop.observe(row['pid'],row,expected,path,self.end,StrictOps())
    except BaseException as error:raise RuntimeError('owned Cargo coherent capture refused') from error
    if attempt['status']=='NON_CARGO':continue
    policy.require(not attempt.get('old_engine_present') and not attempt.get('duplicate_selected'),'old engine/duplicate selected Cargo field')
    policy.require(attempt['argv_rawlen']>0 and attempt['argv'] and all(attempt['argv']),'empty Cargo argv')
    self.seen[key]=str(path);self.cargo.append(dict(path=str(path),phase=self.phase,cwd=row['cwd'],argv=attempt['argv'],profile=expected))
   if row['exe'] in self.allowed and key not in self.seen:
    entry=self.allowed[row['exe']];policy.require(row['exe_sha256']==entry['sha256'],'runtime ELF changed')
    expected=policy.profile(R.parent/'rag3db-libs',Path(entry['manifest']),entry.get('ld') or str(self.root))
    ancestors=self.ancestors(row)
    if any(r['exe']==self.config['cargo_exe'] for r in ancestors):expected=policy.profile(R.parent/'rag3db-libs',Path(entry['manifest']),self.runtime_ld(str(self.root)+':/usr/local/cuda/lib64' if any(any(a.endswith('/run_e2e.sh') for a in r['argv']) for r in ancestors) else None))
    elif self.phase=='canonical' and entry['label']=='e2e':expected=policy.profile(R.parent/'rag3db-libs',Path(entry['manifest']),str(self.root)+':/usr/local/cuda/lib64')
    path=P/'observations'/('runtime-'+str(row['pid'])+'-'+row['starttime']+'-'+str(time.monotonic_ns())+'.json')
    value=self.runtime_snapshot(proc,row,expected,path);native=[line for line in value['maps'].splitlines() if 'librag3db.so' in line]
    if entry['native'] and not native:continue
    if entry['native']:policy.require(all(str(self.root/'librag3db.so') in line for line in native),'runtime SDK maps differ')
    self.seen[key]=str(path);self.runtimes.append(dict(path=str(path),label=entry['label'],phase=self.phase,argv=row['argv'],pid=row['pid'],start=row['starttime']))
 def complete(self):
  policy.require(self.cargo,'no actual Cargo profile was observed')
  policy.require({'host','tty','large','e2e','nav'}<={r['label'] for r in self.runtimes},'one required actual runtime/profile was not observed')
  canonical=[r for r in self.runtimes if r['phase']=='canonical' and '--list' not in r['argv']]
  policy.require(any(r['label']=='large' and not r['argv'][1:] for r in canonical),'unfiltered actual lib runtime SDK profile not observed')
  policy.require(any(r['label']=='nav' for r in canonical),'actual NAV runtime SDK profile not observed')
  for suite in policy.SUITES:policy.require(any(r['label']=='e2e' and suite+'::' in r['argv'] for r in canonical),'actual suite runtime SDK profile not observed: '+suite)
  evidence.save(P/'owned-observations.json',dict(cargo=self.cargo,runtimes=self.runtimes,processes=list(self.frames.values())))
