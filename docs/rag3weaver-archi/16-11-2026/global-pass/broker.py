"""Owned local Cargo admission for D6 and the documentary fixture, never a host executor."""
from pathlib import Path
import hashlib,json,os,re,shutil,socket,struct,threading,time
import evidence,policy
from settings import *

class Broker:
 def __init__(self,context):
  self.ctx=context;self.mode='d6';self.cwd=None;self.target=None;self.review=False;self.completed=[];self.sessions={};self.error=None;self.socket=None;self.thread=None;self.stop=threading.Event();self.serial=0;self.active_connection=None
 def bind(self):
  root=F/'private-controller';root.mkdir(mode=0o700);path=root/'cargo.sock'
  policy.require(len(os.fsencode(path))<=107 and not path.exists(),'broker path not fresh/too long')
  self.socket=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);self.socket.bind(str(path));os.chmod(path,0o600);self.socket.listen(8);self.socket.settimeout(.1)
  self.thread=threading.Thread(target=self.serve,daemon=True);self.thread.start();self.ctx.probes.append(self.monitor)
 def monitor(self):
  if self.error:raise RuntimeError('private Cargo broker failed: '+self.error)
  policy.require(not self.thread or self.thread.is_alive() or self.stop.is_set(),'private broker thread stopped unexpectedly')
  for value in list(self.sessions.values()):
   paths=[]
   if value.get('compile_log'):paths.append(Path(value['compile_log']))
   if value.get('test_log') and value['stage']!='agent-red':paths.append(Path(value['test_log']))
   if not value.get('done'):self.ctx.active_failure(paths)
 def serve(self):
  while not self.stop.is_set():
   try:connection,_=self.socket.accept()
   except socket.timeout:continue
   except OSError:
    if self.stop.is_set():break
    raise
   with connection:
    self.active_connection=connection
    try:
     connection.settimeout(900);peer=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
     request=json.loads(connection.makefile('rb').readline(2*1024*1024));response=self.handle(request,peer)
     connection.sendall((json.dumps(dict(ok=True,**response))+'\n').encode())
    except BaseException as error:
     self.error=repr(error);self.ctx.fail('private Cargo refusal',dict(error=self.error,request=request if 'request' in locals() else None))
     try:connection.sendall((json.dumps(dict(ok=False,error=self.error))+'\n').encode())
     except OSError:pass
    finally:self.active_connection=None
 def identity(self,request,peer):
  policy.require(peer[0]==request['pid'] and peer[1]==os.getuid(),'Unix peer identity differs')
  proc=Path('/proc')/str(peer[0]);row=self.ctx.observer.identity(proc)
  policy.require(row['cgroup']==self.ctx.cgroup and row['cwd']==request['cwd'] and row['exe']==str(Path(self.ctx.config['tools']['python3']['path'])),'shim not an owned exact interpreter/cwd')
  policy.require(any(a==str(Q/'bin/cargo') for a in row['argv']),'unexpected private Cargo entry')
  return row
 def handle(self,request,peer):
  self.ctx.tick();row=self.identity(request,peer);cwd=Path(request['cwd']);target=Path(request['target'])
  policy.require(cwd.resolve()==cwd and target.resolve()==target and target.is_relative_to(TASKS),'unconfined source/target')
  if request['kind']=='admit':
   args=request['argv']
   if self.mode=='d6':
    policy.require(cwd==self.cwd and target==self.target,'another D6 source/target')
    stage='review' if self.review else ('agent-red' if not self.completed else 'agent-green')
    policy.require((not self.review and len(self.completed)<2) or (self.review and len(self.completed)==2),'extra D6 Cargo test')
    expected=policy.REVIEW_NAME if self.review else policy.D6_NAME;policy.require(args==policy.d6_command(expected),'wrong D6 exact argv')
    build,test=policy.split_test(args);source_head=self.ctx.git(cwd,'rev-parse','HEAD');policy.require(source_head==policy.APP,'task source HEAD differs')
   else:
    policy.require(self.mode=='documentary' and cwd.is_relative_to(F) and target==policy.source_target(TASKS,'petit-depot'),'unrecognized fixture Cargo source/target')
    import tomllib
    manifest=tomllib.loads((cwd/'Cargo.toml').read_text());policy.require(manifest['package']['name']=='sommes' and not manifest.get('dependencies'),'not the declared documentary toy crate')
    policy.require(args in [['test'],['test','--lib'],['test','--locked','--offline'],['test','--lib','--locked','--offline']],'unsupported documentary support command')
    stage='documentary-'+str(self.serial);expected=None;build=['test','--lib','--locked','--offline','--no-run','--message-format=json-render-diagnostics'];test=['--nocapture','--test-threads=1'];source_head=self.ctx.git(cwd,'rev-parse','HEAD')
   self.serial+=1;identifier='task-'+str(self.serial)+'-'+row['starttime']
   source=evidence.source_state(cwd,lambda *args:self.ctx.git(cwd,*args))
   if self.mode=='d6':
    policy.require(not self.ctx.git(cwd,'diff','--cached','--name-only'),'staged D6 source is not admitted')
    policy.require(not any(v.get('untracked') for v in source['files'].values()),'untracked D6 source is not admitted')
    policy.require(self.ctx.git(cwd,'diff','HEAD','--name-only').splitlines()==['extension/rag3weaver/src/backend.rs'],'D6 source delta has undeclared paths')
    if stage in ['agent-red','agent-green']:policy.red_source((cwd/'src/backend.rs').read_text())
   patch=self.ctx.git(cwd,'diff','--binary');record=dict(identity=identifier,peer=row,cwd=str(cwd),target=str(target),argv=args,build_argv=build,test_argv=test,stage=stage,name=expected,source_head=source_head,source_state=source,patch_sha256=hashlib.sha256(patch.encode()).hexdigest(),done=False)
   if stage in ['agent-red','agent-green']:
    record['assertion_line']=policy.red_source((cwd/'src/backend.rs').read_text());record['judge_sha256']=hashlib.sha256(policy.D6_RED_TEST.encode()).hexdigest()
    if stage=='agent-red':record['red_line']=record['assertion_line']
    else:policy.require(record['judge_sha256']==self.completed[0]['judge_sha256'],'green judge differs from original red judge')
   self.sessions[identifier]=record;self.ctx.save(identifier+'-admission.json',record)
   ld=str(Path(self.ctx.config['cargo_exe']).parent.parent/'lib')+':'+str(self.ctx.config['sdk_root'])
   self.ctx.observer.source(cwd,dict(manifest=str(cwd),ld=ld))
   return dict(identity=identifier,cargo=self.ctx.config['cargo_exe'],guard=str(C/'scripts/garde_target.sh'),build_argv=build,test_argv=test,ld=ld)
  value=self.sessions[request['identity']];policy.require(value['peer']['pid']==row['pid'] and value['peer']['starttime']==row['starttime'] and value['cwd']==request['cwd'] and value['argv']==request['argv'],'admission identity changed')
  for name in ['compile_log','test_log']:
   if name in request:
    path=Path(request[name]);policy.require(path.resolve()==path and path.is_relative_to(F),'fixture log outside private scratch');value[name]=str(path)
  if request['kind'] in ['begin','test-begin']:return {}
  if request['kind']=='compile-failed':raise RuntimeError('real fixture Cargo compilation failed before test')
  if request['kind']=='built':
   if value['stage'] in ['agent-red','agent-green']:
    policy.red_source((cwd/'src/backend.rs').read_text());policy.require(value['judge_sha256']==hashlib.sha256(policy.D6_RED_TEST.encode()).hexdigest(),'agent judge changed before ledger')
   policy.require(any(row['cwd']==str(cwd) and row['argv'][1:]==value['build_argv'] for row in self.ctx.observer.cargo),'actual task Cargo compilation SDK frame missing')
   text=Path(value['compile_log']).read_text();rows=[]
   for line in text.splitlines():
    try:item=json.loads(line)
    except ValueError:continue
    if item.get('reason')=='compiler-artifact' and item.get('profile',{}).get('test') and item.get('executable'):rows.append(item)
   policy.require(len(rows)==1,'one actual fixture test ELF required');artifact=rows[0]
   policy.require(artifact['manifest_path']==str(cwd/'Cargo.toml'),'fixture artifact from another manifest')
   if value['name']:policy.require(sorted(artifact['features'])==policy.feature_closure(cwd/'Cargo.toml',policy.D6_FEATURES,False),'D6 feature closure differs')
   else:policy.require(not artifact['features'],'documentary toy unexpectedly has features')
   binary=Path(artifact['executable']);policy.require(binary.is_relative_to(target) and binary.resolve()==binary,'task ELF target differs')
   policy.require(evidence.source_state(cwd,lambda *args:self.ctx.git(cwd,*args))==value['source_state'],'complete source/index changed during Cargo')
   copy=P/(value['identity']+'-elf');shutil.copy2(binary,copy);sha=evidence.sha(copy);policy.require(sha==evidence.sha(binary),'task ELF changed during copy')
   self.ctx.observer.executable(binary,cwd,value['stage'].startswith('agent') or value['stage']=='review',value['stage'],ld=str(Path(self.ctx.config['cargo_exe']).parent.parent/'lib')+':'+str(self.ctx.config['sdk_root']))
   item=dict(binary=str(binary),copy=str(copy),sha256=sha,size=copy.stat().st_size,features=artifact['features'],cargo_artifact=artifact)
   ledger=dict(source=policy.APP,pilot=self.ctx.config['pilot'],stage=value['stage'],source_head=value['source_head'],source_patch_sha256=value['patch_sha256'],source_state=value['source_state'],created=time.monotonic(),artifacts={'task':item})
   self.ctx.published(value['identity'],ledger);value['binary']=str(binary);value['runtime_permitted']=time.monotonic();self.ctx.save(value['identity']+'-runtime-permitted.json',value)
   return dict(binary=str(binary))
  if request['kind']=='tested':
   if value['stage'] in ['agent-red','agent-green']:
    policy.red_source((cwd/'src/backend.rs').read_text());policy.require(value['judge_sha256']==hashlib.sha256(policy.D6_RED_TEST.encode()).hexdigest(),'agent judge changed before result')
   policy.require(evidence.source_state(cwd,lambda *args:self.ctx.git(cwd,*args))==value['source_state'],'source/index changed during test')
   text=Path(value['test_log']).read_text();expected_red=value['stage']=='agent-red'
   if value['name']:
    result=policy.judged(text,value['name'],expected_red,value.get('red_line'));policy.require(request['code']==(101 if expected_red else 0),'real Rust test code differs')
   else:
    policy.require(request['code']==0 and not re.search(r'^test result: FAILED|^test .*FAILED$',text,re.M),'documentary support command failed')
    result=dict(support_only=True,not_a_qualification_judge=True)
   copy=P/(value['identity']+'-test.log');shutil.copy2(value['test_log'],copy)
   shutil.copy2(value['compile_log'],P/(value['identity']+'-build.jsonl'))
   value.update(done=True,result=result,code=request['code'],finished=time.monotonic());self.ctx.save(value['identity']+'-result.json',value)
   if value['name']:self.completed.append(value)
   return {}
  raise RuntimeError('unknown broker protocol operation')
 def close(self):
  self.stop.set()
  if self.socket:self.socket.close()
  if self.active_connection:
   try:self.active_connection.shutdown(socket.SHUT_RDWR)
   except OSError:pass
  limit=min(self.ctx.end,self.ctx.phase_deadline,time.monotonic()+2)
  if self.thread:self.thread.join(timeout=max(0,limit-time.monotonic()))
  self.ctx.save('broker-close.json',dict(observed=time.monotonic(),deadline=limit,thread_alive=bool(self.thread and self.thread.is_alive())))
  policy.require(not self.thread or not self.thread.is_alive(),'broker thread did not stop')
