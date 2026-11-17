#!/usr/bin/env python3
"""PROPOSAL: public writer lifecycle on a NEW debt-zero FIN fixture only."""
from pathlib import Path
import argparse,hashlib,json,os,shutil,signal,subprocess,sys,time,uuid
import finish_result as f
import lifecycle_protocol as protocol
import grant_watch_actor as watch_actor
import grant_watch_controller as watch_control
a=f.a;q=a.q;R=a.R
P=R/'lifecycle-copy-v9';Q=R/'lifecycle-control-v9';S=R/'lifecycle-scopes-v9'
FIN=R/'debt-fin-ro-result-v1';SOURCE=FIN/'copy';BASE=P/'copy/code-principal.rag3db'
FIN_MANIFEST=R/'debt-fin-ro-proofs-01/proof-manifest.json'
FIN_MANIFEST_SHA='05661b1fddc82c1e66d21f218d0c8cdfd12b69d8ef16432a2c55c3b127a9462a'
FIN_REFS=['copy-after.json','fts-observed.json','vectors-observed.json','sources-after.json','composed.json']
a.FILES=dict(a.FILES,**{n:a.PREFIX+n for n in ['lifecycle.py','lifecycle_protocol.py','lifecycle.json','lifecycle.mmd','lifecycle-provider-go.schema.json','lifecycle-ready.schema.json','lifecycle_grant_producer.py','lifecycle_provider_observation.py','lifecycle_health_policy.py','lifecycle-provider-expectations.json']})
a.FILES['provider_health_transport.py']='docs/orchestration/16-11-2026/coordination/provider_health_transport.py'
for name in ['grant_watch_actor.py','grant_watch_controller.py']:a.FILES[name]='docs/orchestration/17-11-2026/coordination/'+name
f.Q=Q;f.carried.Q=Q;a.Q=Q;a.P=P;a.S=S;a.BASE=BASE;q.Q=Q
def save(name,value):q.save(P/name,value)
def postwriter_scope(end):
 if (P/'first-red.json').exists():raise ValueError('first red forbids post-writer admission')
 q.require_time(end,0)
 if (P/'first-red.json').exists():raise ValueError('first red during post-writer observation')
def postwriter_inventory(before,after,before_dirs,after_dirs,cfg,profile,nonce,end):
 """Physical admission only; logical/FTS/vector/source judges remain mandatory."""
 postwriter_scope(end)
 try:
  def read(name):return watch_control.read_owned(P/name)[0]
  context=read('prewriter-context.json')
  expected_context=dict(database=str(BASE),pilot=cfg['pilot'],prepared_sha256=q.sha(Q/'prepared.json'),nonce=nonce,deadline=end,cgroup=cfg['active_cgroup'],profile=profile,inventory_sha256=q.sha(P/'copy-before-writer.json'),directories_sha256=q.sha(P/'copy-directories-before-writer.json'))
  if context!=expected_context or read('copy-before-writer.json')!=before or read('copy-directories-before-writer.json')!=before_dirs:raise ValueError('pre-writer baseline/context substituted or belongs to another cycle')
  identity=read('writer-process.json');closed=read('writer-closed.json');publication=read('response-publication.json');shutdown=read('shutdown-request.json');runtime=read('writer-runtime-profile.json')
  if type(identity.get('pid')) is not int or identity['pid']<=0 or not isinstance(identity.get('start'),str) or not identity['start'].isdigit() or identity.get('exe')!=str(q.CLI) or identity.get('exe_sha256')!=q.PINS[q.CLI]:raise ValueError('writer identity not the pinned executable/process')
  if any(record.get('process')!=identity for record in [closed,publication,shutdown]):raise ValueError('writer/publication/shutdown process binding differs')
  if closed.get('shutdown_acknowledged') is not True or closed.get('state')!='CLOSED' or type(closed.get('rc')) is not int or closed['rc']!=0:raise ValueError('same writer shutdown/return unqualified')
  if runtime.get('pid')!=identity['pid'] or type(runtime.get('pid')) is not int or runtime.get('start')!=identity['start'] or runtime.get('start_after')!=identity['start'] or runtime.get('exe_sha256')!=identity['exe_sha256'] or runtime.get('exe_after')!=identity['exe']:raise ValueError('writer runtime identity differs')
  expected_cgroup='0::'+cfg['active_cgroup']+'\n'
  if runtime.get('cgroup')!=expected_cgroup or runtime.get('cgroup_after')!=expected_cgroup or runtime.get('expected')!=profile or runtime.get('actual')!=profile or runtime.get('diff')!={}:raise ValueError('writer runtime profile/cgroup differs')
  maps=runtime.get('maps')
  if not isinstance(maps,str) or any(str(q.LIB/name) not in {line.split()[-1] for line in maps.splitlines() if line.split()} for name in ['librag3db.so','libvector.rag3db_extension']):raise ValueError('actual writer SDK maps absent')
  if publication.get('database')!=str(BASE) or publication.get('manifest_sha256')!=cfg['files']['lifecycle.json'] or publication.get('graph_sha256')!=cfg['files']['lifecycle.mmd']:raise ValueError('publication database/source differs')
  if publication.get('request')!=dict(op='call',name='observer_cycle_vie',arguments=dict(nonce=nonce)) or shutdown.get('request')!={'op':'shutdown'} or shutdown.get('publication_sha256')!=q.sha(P/'response-publication.json'):raise ValueError('request/publication-before-shutdown binding differs')
  path=P/'writer.stdout'
  if path.is_symlink() or not path.is_file() or path.stat().st_uid!=os.getuid():raise ValueError('writer response is not owned/regular')
  raw=path.read_bytes()
  if len(raw)>1048576 or not raw.endswith(b'\n') or len(raw.splitlines(keepends=True))!=2:raise ValueError('writer response framing differs')
  call,ack=raw.splitlines(keepends=True);session=protocol.Session(nonce)
  observed=session.call_response(call,lambda value:None)
  if observed!=publication.get('observation'):raise ValueError('persisted public response differs from raw frame')
  session.shutdown_request();session.closed_response(ack);result=session.finish(closed['rc'])
  if any(closed.get(key)!=value for key,value in result.items()):raise ValueError('closed nonce/run/state differs from raw session')
  def inventory(rows,directories=False):
   if not isinstance(rows,list):raise ValueError('inventory must be complete list')
   result={};keys={'path','mode','uid'} if directories else {'path','size','mode','sha256'}
   for row in rows:
    if not isinstance(row,dict) or set(row)!=keys:raise ValueError('inventory row shape differs')
    name=row['path']
    if not isinstance(name,str) or not name or Path(name).is_absolute() or any(x in ['.','..','.env','.vault'] for x in name.split('/')) or name in result:raise ValueError('duplicate/ambiguous/protected inventory path')
    if type(row['mode']) is not int or not 0<=row['mode']<=4095:raise ValueError('inventory mode invalid')
    if directories:
     if type(row['uid']) is not int or row['uid']!=os.getuid():raise ValueError('foreign directory')
    elif type(row['size']) is not int or row['size']<0 or not isinstance(row['sha256'],str) or len(row['sha256'])!=64 or any(x not in '0123456789abcdef' for x in row['sha256']):raise ValueError('inventory size/SHA invalid')
    result[name]=row
   return result
  old=inventory(before);new=inventory(after)
  if inventory(before_dirs,True)!=inventory(after_dirs,True):raise ValueError('post-writer directories differ')
  name=BASE.name
  if name not in old or name not in new or new[name]['size']<=0 or new[name]['mode']!=old[name]['mode']:raise ValueError('main database absent/empty/mode changed')
  for suffix in ['.wal','.ecrivain']:
   if name+suffix in new:raise ValueError('writer WAL/mark remains after acknowledged shutdown')
   old.pop(name+suffix,None)
  old.pop(name);new.pop(name)
  readers=name+'.readers'
  if readers not in old and readers in new:
   if new[readers]!=dict(path=readers,size=0,mode=420,sha256=hashlib.sha256(b'').hexdigest()):raise ValueError('new readers file not empty/canonical')
   new.pop(readers)
  if old!=new:raise ValueError('unrelated post-writer files changed/added/removed')
  postwriter_scope(end)
  return dict(physical_delta_admitted=True,logical_integrity_qualified=False)
 except (OSError,KeyError,TypeError) as error:raise ValueError('post-writer evidence absent/malformed: '+type(error).__name__) from error
def finalize_composed(record):
 path=P/'composed.json'
 if path.exists():raise FileExistsError('refuse to replace an existing composed result')
 def positive_guard():
  if (P/'first-red.json').exists():raise ValueError('first red forbids positive finalization')
  deadline=record['deadline']
  if not isinstance(deadline,(int,float)) or not float('-inf')<deadline<float('inf') or time.monotonic()>=deadline:raise ValueError('lifecycle final publication deadline expired')
 try:
  if record['status']!='LIFECYCLE_FIXTURE_OBSERVED':
   watch_actor.write_frame(path,record);return 1
  positive_guard()
  watch_actor.write_frame(path,record,guard=positive_guard)
  # Includes file fsync, exclusive link, directory fsync and staging cleanup.
  positive_guard()
  return 0
 except BaseException as error:
  a.red(error)
  refusal=dict(status='RED',error=str(error),live_authorized=False)
  if path.exists():
   # Only reclassify our own complete positive frame, never a foreign result.
   expected=json.dumps(record,sort_keys=True,separators=(',',':'),allow_nan=False).encode()
   if path.read_bytes()!=expected:raise RuntimeError('foreign composed result cannot be replaced') from error
   watch_actor.write_frame(path,refusal,replace=True)
  else:watch_actor.write_frame(path,refusal)
  return 1
def publish_ready(ready):
 state=require_producer()
 if ready['nonce']!=state['config']['ready_nonce'] or ready['phase'].get('producer_watcher')!=producer_descriptor(state):raise ValueError('READY must bind the independently admitted live producer')
 # The observation is sampled before this write; no fsync-completion time claimed.
 pending=P/'READY_FOR_PROVIDER_COORDINATION.pending.json';public=P/'READY_FOR_PROVIDER_COORDINATION.json'
 q.save(pending,ready);q.require_time(ready['grant_deadline_monotonic'],0)
 os.link(pending,public) # Exclusive publication of a complete, fsynced immutable file.
def health_context(ready,grant,ready_sha256,admission):
 policy=Q/'lifecycle_health_policy.py'
 return dict(address='127.0.0.1:7878',expected_process=grant['owner_pins']['expected_process'],gate_module_path=str(policy),gate_module_sha256=q.sha(policy),ready_path=str(P/'READY_FOR_PROVIDER_COORDINATION.json'),ready_sha256=ready_sha256,nonce=ready['nonce'],grant=grant,admission=admission)
def producer_descriptor(state):
 return dict(admission_path=str(P/'watcher-admission.json'),admission_sha256=state['admission_sha256'])
def scope_reference(authority):
 return dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],pid=authority['pid'],start=authority['debut'],uid=os.getuid(),cgroup=authority['cgroup'],script_sha256=authority['script_sha256'])
def install_action_guard(cfg,authority):
 expected_authority=scope_reference(authority);expected_end=authority['echeance'];admission_path=P/'watcher-admission.json';expected_admission=q.sha(admission_path);prepared_sha=q.sha(Q/'prepared.json');allocation=protocol.decode((Q/'allocation.json').read_bytes())['reference']
 def action_guard():
  try:
   state=watch_control.read_admission(admission_path,expected_admission);config=state['config']
   if config['prepared_path']!=str(Q/'prepared.json') or config['prepared_sha256']!=prepared_sha or config['pilot']!=cfg['pilot'] or config['allocation']!=allocation:raise ValueError('producer source/allocation is not this exact job')
   if config['authority']!=expected_authority or config['common_end_monotonic']!=expected_end:raise ValueError('producer belongs to another authority/deadline')
   if config['paths']['ready']!=str(P/'READY_FOR_PROVIDER_COORDINATION.json') or config['paths']['grant']!=str(Q/'PROVIDER_GO.json'):raise ValueError('producer READY/grant targets another job')
   if config['actor']['script_path']!=str(Q/'grant_watch_actor.py') or config['actor']['script_sha256']!=cfg['files']['grant_watch_actor.py'] or config['actor']['exe']!=cfg['python'] or config['actor']['exe_sha256']!=cfg['python_sha256']:raise ValueError('producer executable/source differs from prepared role')
   own=watch_actor.python_role_snapshot(os.getpid(),str(Q/'lifecycle.py'))
   if own['ppid']!=config['controller']['pid'] or own['exe']!=cfg['python'] or own['exe_sha256']!=cfg['python_sha256'] or own['argv']!=[cfg['python'],'-B',str(Q/'lifecycle.py'),'--driver'] or own['script_sha256']!=cfg['files']['lifecycle.py'] or own['uid']!=os.getuid() or own['cgroup']!=expected_authority['cgroup'] or own['state'] not in ['R','S','D','I']:raise ValueError('DB driver not the exact owned source/interpreter/argv role')
   return state
  except BaseException as error:
   a.red(error)
   raise RuntimeError('producer admission refused before action') from error
 q.ACTION_GUARD=action_guard
 return action_guard()
def require_producer():
 if q.ACTION_GUARD is None:raise ValueError('producer has not been independently admitted before READY/action')
 return q.ACTION_GUARD()
def active_scope():
 common=q.loadmod('confiner_validation');authority=common.autorite(common.processus());end=authority['echeance']
 if json.loads((Path(os.environ['RAG3WEAVER_VALIDATION_RECU']).parent/'recette.json').read_text())['delai']!=830:raise ValueError('lifecycle budget differs')
 cg=Path('/sys/fs/cgroup')/authority['cgroup'].lstrip('/');limits={n:(cg/n).read_text().strip() for n in ['memory.max','memory.high','memory.swap.max','cpu.max']};quota,period=limits['cpu.max'].split()
 if limits['memory.max']!='34359738368' or limits['memory.high']!='max' or limits['memory.swap.max']!='0' or quota=='max' or not 0<int(quota)<=8*int(period):raise ValueError('lifecycle resource contract differs')
 return authority,end,limits
def controller():
 cfg=protocol.decode((Q/'prepared.json').read_bytes());guard(cfg);a.bind();authority,end,limits=active_scope();own=watch_actor.python_role_snapshot(os.getpid(),str(Q/'lifecycle.py'))
 expected_argv=[cfg['python'],'-B',str(Q/'lifecycle.py'),'--controller']
 if own['exe']!=cfg['python'] or own['exe_sha256']!=cfg['python_sha256'] or own['argv']!=expected_argv or own['script_sha256']!=cfg['files']['lifecycle.py'] or own['cgroup']!=authority['cgroup'] or own['uid']!=os.getuid():raise ValueError('controller OS role is not exact')
 save('controller-authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,limits=limits,process=own))
 now=time.monotonic();arm_due=min(now+10.,end-500.)
 if arm_due<=now:raise ValueError('no original budget for producer admission')
 config_path=P/'watcher-config.json';paths=dict(ready=str(P/'READY_FOR_PROVIDER_COORDINATION.json'),grant=str(Q/'PROVIDER_GO.json'),armed=str(P/'watcher-armed.json'),heartbeat=str(P/'watcher-heartbeat.json'),grant_ack=str(P/'watcher-grant.json'),result=str(P/'watcher-result.json'),release=str(P/'watcher-release.json'),first_red=str(P/'first-red.json'),evidence=str(P/'watcher-owner-evidence.json'))
 config=dict(clock=protocol.READY_CLOCK,pilot=cfg['pilot'],allocation=protocol.decode((Q/'allocation.json').read_bytes())['reference'],prepared_path=str(Q/'prepared.json'),prepared_sha256=q.sha(Q/'prepared.json'),actor_nonce=uuid.uuid4().hex,ready_nonce=uuid.uuid4().hex,issued_monotonic=now,arm_deadline_monotonic=arm_due,common_end_monotonic=end,controller=dict(pid=own['pid'],start=own['start']),authority=scope_reference(authority),actor=dict(script_path=str(Q/'grant_watch_actor.py'),script_sha256=cfg['files']['grant_watch_actor.py'],exe=cfg['python'],exe_sha256=cfg['python_sha256'],argv_prefix=[cfg['python'],'-B',str(Q/'grant_watch_actor.py'),'--watch',str(config_path)]),paths=paths,policy=dict(path=str(Q/'lifecycle_health_policy.py'),sha256=cfg['files']['lifecycle_health_policy.py']),policy_context=dict(expected_provider_path=str(Q/'lifecycle-provider-expectations.json'),expected_provider_sha256=cfg['files']['lifecycle-provider-expectations.json']))
 q.save(config_path,config)
 return watch_control.supervise(config_path,q.sha(config_path),[cfg['python'],'-B',str(Q/'lifecycle.py'),'--driver'])
def guard(cfg):
 f.guard(cfg)
 if protocol.decode((Q/'lifecycle.json').read_bytes()).get('database')!=str(BASE):raise ValueError('manifest must target this exact fresh fixture')
 if q.sha(cfg['python'])!=cfg['python_sha256'] or str(Path(cfg['python']).resolve())!=cfg['python']:raise ValueError('prepared Python interpreter differs')
 if q.sha(FIN_MANIFEST)!=FIN_MANIFEST_SHA or set(cfg['fin_refs'])!=set(FIN_REFS):raise ValueError('FIN evidence reference set differs')
 rows={x['name']:x['sha256'] for x in json.loads(FIN_MANIFEST.read_text())}
 for name,h in cfg['fin_refs'].items():
  if rows.get('client/'+name)!=h or q.sha(FIN/name)!=h:raise ValueError('FIN fixture reference differs')
 if json.loads((FIN/'composed.json').read_text())['status']!='FIN_RO_OBSERVED':raise ValueError('not a qualified observable FIN fixture')
def sources(fs):return dict(previous=f.sources(fs),fin_files=fs.file_inventory(SOURCE),fin_dirs=q.directories(SOURCE))
def quiet_source():
 previous=q.BASE
 try:q.BASE=SOURCE/BASE.name;return q.copy_quiet()
 finally:q.BASE=previous
def full_vector_digest(reader):
 return reader.digest('MATCH (n:ScopeContent_Chunk) RETURN n._uuid,n.embedding__granite_278m,n._embed_hash__granite_278m,n._text_hash,n._embed_claim')
def no_fts_rebuild(reader,label):
 rows=reader.collect("MATCH (m:_catalog_meta) WHERE m._key='fts_promesse' OR m._key STARTS WITH 'fts_pending:' RETURN m._key,m._value")
 values={}
 for row in rows:
  key,value=map(a.audit.text,row)
  if key in values:raise ValueError('duplicate FTS pending/promise metadata')
  values[key]=value
 nonempty=[key for key,value in values.items() if (bool(value) if key=='fts_promesse' else bool(value.strip()))]
 save(label,dict(keys=sorted(values),nonempty=nonempty))
 if nonempty:raise ValueError('FTS promise/pending present; no background rebuild authorized')

def writer(cfg,env,profile,grant,end,nonce):
 session=protocol.Session(nonce);captured=False;sent=False;consumed=0
 request=dict(op='call',name='observer_cycle_vie',arguments=dict(nonce=nonce))
 save('request-before-writer.json',dict(request=request,nonce_echo='public SearchMeta.query; no request_id echo claimed'))
 deadline=min(end-500,time.monotonic()+180,grant['valid_until_monotonic']-20);q.require_time(deadline)
 with (P/'writer.stdout').open('xb') as out,(P/'writer.stderr').open('xb') as err:
  child=subprocess.Popen([str(q.CLI),str(Q/'lifecycle.json')],stdin=subprocess.PIPE,stdout=out,stderr=err,env=env,cwd=P)
  identity=q.process_identity(child.pid);save('writer-process.json',identity)
  if identity['exe_sha256']!=q.PINS[q.CLI]:raise ValueError('writer ELF differs')
  q.require_time(deadline,0)
  child.stdin.write(json.dumps(request).encode()+b'\n');child.stdin.flush()
  def publish(record):
   save('response-publication.json',dict(observation=record,process=identity,manifest_sha256=q.sha(Q/'lifecycle.json'),graph_sha256=q.sha(Q/'lifecycle.mmd'),database=str(BASE),request=request))
   # Re-read the persisted local publication before enabling the next transport step.
   if json.loads((P/'response-publication.json').read_text())['observation']!=record:raise ValueError('local publication differs')
  while True:
   q.require_time(deadline,0)
   raw=(P/'writer.stdout').read_bytes()
   if len(raw)>1048576:raise ValueError('oversized writer protocol')
   complete=raw.splitlines(keepends=True)
   for line in complete[consumed:]:
    if not line.endswith(b'\n'):break
    if session.state=='CALL_SENT':session.call_response(line,publish)
    elif session.state=='SHUTDOWN_SENT':session.closed_response(line)
    else:raise ValueError('unexpected extra writer response')
    consumed+=1
   stderr=(P/'writer.stderr').read_text(errors='replace')
   if 'backend est rouvert' in stderr or 'doit être rouverte' in stderr:raise ValueError('automatic reopen or failed checkpoint')
   rc=child.poll()
   if rc is not None:break
   proc=Path('/proc')/str(child.pid)
   try:
    fields=(proc/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[19]!=identity['start'] or os.readlink(proc/'exe')!=identity['exe']:raise ValueError('writer identity changed')
    maps=(proc/'maps').read_text()
    if not captured and str(q.LIB/'librag3db.so') in maps and str(q.LIB/'libvector.rag3db_extension') in maps:
     actual={}
     for value in (proc/'environ').read_bytes().split(b'\0'):
      if b'=' in value:
       key,val=value.split(b'=',1);key=os.fsdecode(key)
       if key in profile:actual[key]=os.fsdecode(val)
     record=dict(pid=child.pid,start=identity['start'],expected=profile,actual=actual,diff={k:dict(expected=v,actual=actual.get(k)) for k,v in profile.items() if actual.get(k)!=v},maps=maps,exe_sha256=q.sha(proc/'exe'),cgroup=(proc/'cgroup').read_text())
     record['start_after']=(proc/'stat').read_text().rsplit(') ',1)[1].split()[19];record['cgroup_after']=(proc/'cgroup').read_text();record['exe_after']=os.readlink(proc/'exe')
     save('writer-runtime-profile.json',record)
     if actual!=profile or record['exe_sha256']!=q.PINS[q.CLI] or record['start_after']!=identity['start'] or record['exe_after']!=identity['exe'] or record['cgroup_after']!=record['cgroup'] or ('0::'+cfg['active_cgroup']+'\n')!=record['cgroup']:raise ValueError('actual writer profile/exe/cgroup differs')
     captured=True
   except (FileNotFoundError,ProcessLookupError):
    if child.poll() is None:raise
    continue # Next iteration consumes the final complete response before judging rc.
   if session.state=='PUBLISHED' and captured and not sent:
    q.require_time(deadline,0)
    shutdown=session.shutdown_request();save('shutdown-request.json',dict(process=identity,request=json.loads(shutdown),publication_sha256=q.sha(P/'response-publication.json')))
    child.stdin.write(shutdown);child.stdin.flush();child.stdin.close();sent=True
   time.sleep(.05)
  rc=child.wait()
 if not captured or raw and not raw.endswith(b'\n') or consumed!=len(raw.splitlines()):raise ValueError('writer profile or protocol framing unqualified')
 result=session.finish(rc);q.require_time(deadline,0);q.save(P/'writer-closed.json',dict(process=identity,**result,shutdown_acknowledged=True),deadline=deadline)
 return result

def driver():
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg);a.bind();authority,end,limits=active_scope();cfg['active_cgroup']=authority['cgroup'];producer_state=install_action_guard(cfg,authority)
 save('authority-path.json',dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],deadline=end,limits=limits,fixture_only=True))
 fs=q.loadmod('inspect-fts');save('source-before-quiet.json',quiet_source());initial=sources(fs)
 if initial['fin_files']!=json.loads((FIN/'copy-after.json').read_text()) or initial['previous']!=json.loads((FIN/'sources-after.json').read_text()):raise ValueError('FIN or previous sources changed')
 save('sources-before.json',initial)
 for name in ['tmp','config','data','cache','xdg-state','state-fixture']:(P/name).mkdir(mode=0o700)
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','LC_ALL','DBUS_SESSION_BUS_ADDRESS','RAG3WEAVER_VALIDATION_RECU','RAG3WEAVER_VALIDATION_JETON'] if k in os.environ}
 env.update(LD_LIBRARY_PATH=str(q.LIB),TMPDIR=str(P/'tmp'),XDG_CONFIG_HOME=str(P/'config'),XDG_DATA_HOME=str(P/'data'),XDG_CACHE_HOME=str(P/'cache'),XDG_STATE_HOME=str(P/'xdg-state'),RAG3WEAVER_STATE_DIR=str(P/'state-fixture'),RAG3DB_LIB=q.LIB.name,RAG3DB_LIBS_DIR=str(q.LIB.parent),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4',RAG3WEAVER_EXEC_THREADS='4',RAYON_NUM_THREADS='4',RAG3WEAVER_SERVICE_EMBED='127.0.0.1:7878')
 profile={k:v for k,v in env.items() if k!='RAG3WEAVER_VALIDATION_JETON'};save('profile-before-db-provider.json',profile)
 q.require_time(end);cap=min(120,end-time.monotonic()-15);argv=['cp','-a','--reflink=always','--',str(SOURCE),str(P/'copy')]
 save('copy-command.json',dict(argv=argv,cap=cap));result=None;timeout=None
 try:result=subprocess.run(argv,capture_output=True,env=env,timeout=cap)
 except subprocess.TimeoutExpired as error:timeout=error
 (P/'copy.stdout').write_bytes((timeout.stdout or b'') if timeout else result.stdout);(P/'copy.stderr').write_bytes((timeout.stderr or b'') if timeout else result.stderr)
 save('copy-process.json',dict(rc=None if timeout else result.returncode,timeout=timeout is not None))
 if timeout or result.returncode:raise ValueError('fixture clone refused')
 if fs.file_inventory(BASE.parent)!=initial['fin_files'] or q.directories(BASE.parent)!=initial['fin_dirs'] or sources(fs)!=initial:raise ValueError('fixture clone differs')
 save('copy-before-db.json',initial['fin_files']);reader=q.Reader(a.BINARY,end,env,cfg['timeout'])
 if reader.collect('MATCH (n:ScopeContent_Chunk) WHERE '+a.audit.DEBT+' RETURN n._uuid'):raise ValueError('fixture has debt BEFORE writer; no payment authorized')
 no_fts_rebuild(reader,'fts-pending-before.json')
 pre_fts=q.fts(reader,fs,env,end,json.loads((a.OLD/'graph-before.json').read_text())['offsets'],'before');save('fts-before.json',pre_fts)
 if pre_fts!=json.loads((FIN/'fts-observed.json').read_text()):raise ValueError('FTS preflight generations/IDs/files differ')
 vectors_before=full_vector_digest(reader);save('vectors-bits-before.json',vectors_before)
 expected=json.loads((FIN/'vectors-observed.json').read_text())
 if vectors_before['count']!=expected['total_vectors_checked'] or expected['debt']!=0:raise ValueError('fixture vector count differs')
 before_writer=fs.file_inventory(BASE.parent);a.stable_readers(initial['fin_files'],before_writer)
 save('copy-before-writer.json',before_writer);save('copy-directories-before-writer.json',q.directories(BASE.parent))
 save('copy-before-writer-quiet.json',q.copy_quiet());producer_state=require_producer();nonce=producer_state['config']['ready_nonce']
 postwriter_scope(end)
 save('prewriter-context.json',dict(database=str(BASE),pilot=cfg['pilot'],prepared_sha256=q.sha(Q/'prepared.json'),nonce=nonce,deadline=end,cgroup=cfg['active_cgroup'],profile=profile,inventory_sha256=q.sha(P/'copy-before-writer.json'),directories_sha256=q.sha(P/'copy-directories-before-writer.json')))
 postwriter_scope(end)
 phase_identity=dict(job='lifecycle-provider',pilot=cfg['pilot'],allocation=json.loads((Q/'allocation.json').read_text())['reference'],authority=dict(path=os.environ['RAG3WEAVER_VALIDATION_RECU'],pid=authority['pid'],start=authority['debut'],cgroup=authority['cgroup'],uid=os.getuid(),script_sha256=authority['script_sha256']),first_red_path=str(P/'first-red.json'),producer_watcher=producer_descriptor(producer_state))
 ready=dict(nonce=nonce,deadline=end,phase=phase_identity,no_embedding_calls_authorized=True,expected_embedding_calculations=0,health_required=True,max_chars=0,manifest_sha256=q.sha(Q/'lifecycle.json'),graph_sha256=q.sha(Q/'lifecycle.mmd'),fixture_vector_digest_sha256=q.sha(P/'vectors-bits-before.json'),**protocol.ready_bounds(time.monotonic(),end))
 publish_ready(ready)
 while not (Q/'PROVIDER_GO.json').exists():
  q.require_time(ready['grant_deadline_monotonic'],0)
  time.sleep(.2)
 path=Q/'PROVIDER_GO.json';info=path.lstat()
 import stat
 if path.is_symlink() or not stat.S_ISREG(info.st_mode) or info.st_uid!=os.getuid():raise ValueError('invalid provider coordination file')
 grant=protocol.decode(path.read_bytes());q.validate_grant(grant,time.monotonic(),0)
 ready_sha256=q.sha(P/'READY_FOR_PROVIDER_COORDINATION.json');phase_state=protocol.read_phase_state(ready)
 protocol.validate_grant_phase(ready,grant,time.monotonic(),phase_state,ready_sha256)
 save('provider-coordination.json',grant)
 phase_state=protocol.read_phase_state(ready);protocol.validate_grant_phase(ready,grant,time.monotonic(),phase_state,ready_sha256)
 q.health(grant,'provider-before',min(end,grant['valid_until_monotonic'],ready['grant_deadline_monotonic']),context=health_context(ready,grant,ready_sha256,True))
 if sources(fs)!=initial or fs.file_inventory(BASE.parent)!=before_writer:raise ValueError('sources or fixture changed before writer')
 guard(cfg);save('copy-just-before-writer-quiet.json',q.copy_quiet());save('source-just-before-writer-quiet.json',quiet_source());phase_state=protocol.read_phase_state(ready);protocol.validate_phase_authority(ready,time.monotonic(),phase_state);q.require_time(end,500);writer(cfg,env,profile,grant,end,nonce);q.health(grant,'provider-after',min(end,grant['valid_until_monotonic']),context=health_context(ready,grant,ready_sha256,False))
 # No open in recovery mode. Refuse a new/residual WAL before any cold read.
 postwriter_scope(end)
 after_writer=fs.file_inventory(BASE.parent);save('copy-after-shutdown.json',after_writer)
 after_dirs=q.directories(BASE.parent);save('copy-directories-after-shutdown.json',after_dirs)
 physical=postwriter_inventory(before_writer,after_writer,initial['fin_dirs'],after_dirs,cfg,profile,nonce,end)
 save('postwriter-physical-admission.json',physical);postwriter_scope(end)
 save('copy-after-shutdown-quiet.json',q.copy_quiet())
 vectors_after=full_vector_digest(reader);save('vectors-bits-after.json',vectors_after)
 if not a.same_graph_publication(vectors_before,vectors_after):raise ValueError('vector bits/markers/claims changed on debt-zero fixture')
 cohort=expected['covered'];graph=a.audit.snapshot(reader,cohort);reference=json.loads((a.OLD/'graph-before.json').read_text());save('graph-publications.json',dict(observed=a.published_graph(graph),reference=a.published_graph(reference)))
 if not a.same_graph_publication(graph,reference):raise ValueError('logical graph/schema/config/generation changed')
 fts=q.fts(reader,fs,env,end,graph['offsets'],'after');save('fts-after.json',fts)
 if fts!=json.loads((FIN/'fts-observed.json').read_text()):raise ValueError('FTS files/IDs/generations changed')
 final=a.audit.finished(reader,cohort);save('vectors-final.json',final);no_fts_rebuild(reader,'fts-pending-after.json')
 after_reads=fs.file_inventory(BASE.parent)
 if sources(fs)!=initial or after_reads!=after_writer or q.directories(BASE.parent)!=initial['fin_dirs']:raise ValueError('sources or cold read-only fixture mutated')
 save('sources-after.json',initial);save('copy-after.json',after_reads);guard(cfg);q.require_time(end,5)
 save('driver-result.json',dict(status='LIFECYCLE_FIXTURE_JUDGES_PASSED',clean_shutdown_observed=True,fixture_only=True,durability_qualified=False,live_authorized=False))

def compose():
 cfg=json.loads((Q/'prepared.json').read_text());common=q.loadmod('confiner_validation');parent=Path('/proc')/str(os.getppid());expected=[cfg['timeout'],'--signal=TERM','--kill-after=2s','898s',cfg['python'],'-B',str(Q/'lifecycle.py'),'--compose']
 if os.readlink(parent/'exe')!=cfg['timeout'] or [os.fsdecode(v) for v in (parent/'cmdline').read_bytes().split(b'\0') if v]!=expected or not common.poste_tenu(common.processus()):raise ValueError('real lifecycle outer/poste absent')
 P.mkdir(mode=0o700);a.bind();record=dict(status='RED')
 def stop(sig,frame):a.red('lifecycle outer deadline');raise RuntimeError('lifecycle stopped')
 signal.signal(signal.SIGTERM,stop)
 try:
  guard(cfg);rc=subprocess.run([cfg['python'],str(Q/'confiner_validation.py'),'lancer','--delai','830','--preuves',str(S),'--',cfg['python'],'-B',str(Q/'lifecycle.py'),'--controller'],cwd='/var/tmp',env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1')).returncode
  path=Path(json.loads((P/'authority-path.json').read_text())['path']);auth=json.loads(path.read_text());scope=json.loads((path.parent/'verdict.json').read_text())
  if rc or scope['verdict']!='VERT' or scope['code'] or scope['restes'] or not scope['nettoye'] or auth['etat']!='termine' or Path(scope['prive']).exists():raise ValueError('lifecycle scope not green quiet')
  for row in json.loads((path.parent/'enfants.json').read_text()):
   try:
    fields=(Path('/proc')/str(row['pid'])/'stat').read_text().rsplit(') ',1)[1].split()
    if fields[19]==row['debut'] and fields[0]!='Z':raise ValueError('owned process remains')
   except (FileNotFoundError,ProcessLookupError):pass
  if (P/'first-red.json').exists() or json.loads((P/'driver-result.json').read_text())['status']!='LIFECYCLE_FIXTURE_JUDGES_PASSED':raise ValueError('lifecycle driver refused')
  fs=q.loadmod('inspect-fts')
  initial=json.loads((P/'sources-before.json').read_text())
  if sources(fs)!=initial or fs.file_inventory(BASE.parent)!=json.loads((P/'copy-after.json').read_text()) or q.directories(BASE.parent)!=initial['fin_dirs']:raise ValueError('sources or final fixture changed')
  save('copy-final-quiet.json',q.copy_quiet());save('source-final-quiet.json',quiet_source());guard(cfg);q.require_time(auth['echeance'],0)
  record=dict(status='LIFECYCLE_FIXTURE_OBSERVED',deadline=auth['echeance'],scope=scope,clean_shutdown_observed=True,fixture_only=True,durability_qualified=False,live_authorized=False,old_copya_verdict='RED',old_audit_verdict='RED',independent_review_pending=True)
 except BaseException as error:a.red(error);record=dict(status='RED',error=str(error),live_authorized=False)
 return finalize_composed(record)
def prepare(root,pilot):
 if os.uname().nodename.lower()!='luciepc' or q.git(root,'rev-parse','HEAD')!=pilot or q.git(root,'status','--porcelain') or any(x.exists() for x in [P,Q,S]):raise ValueError('exact clean lifecycle source/new P-Q-S required')
 if any(x.resolve()!=x for x in [P,Q,S,SOURCE]):raise ValueError('canonical lifecycle paths required')
 Q.mkdir(mode=0o700)
 for name,path in a.FILES.items():shutil.copy2(root/path,Q/name)
 timeout=str(Path(shutil.which('timeout')).resolve());cfg=dict(source_root=str(root),pilot=pilot,files={n:q.sha(Q/n) for n in a.FILES},timeout=timeout,timeout_sha=q.sha(timeout),old_proofs={n:q.sha(a.OLD/n) for n in a.OLD_REFERENCES},audit_refs={n:q.sha(f.carried.PRIOR/n) for n in f.carried.REFERENCES},fin_refs={n:q.sha(FIN/n) for n in FIN_REFS})
 cfg['python']=str(Path(sys.executable).resolve());cfg['python_sha256']=q.sha(cfg['python']);cfg['carry']=f.carry(cfg);q.save(Q/'prepared.json',cfg);guard(cfg);print(json.dumps(dict(prepared_sha256=q.sha(Q/'prepared.json'),database_opened=False,provider_contact=False)))
def launch(pilot,allocation):
 cfg=json.loads((Q/'prepared.json').read_text());guard(cfg)
 if pilot!=cfg['pilot'] or not allocation or P.exists() or S.exists() or (Q/'PROVIDER_GO.json').exists() or 'RAG3WEAVER_VALIDATION_RECU' in os.environ:raise ValueError('exact new lifecycle allocation and fresh provider grant required')
 q.save(Q/'allocation.json',dict(reference=allocation,fixture_only=True,budget_proposed=900,live_authorized=False))
 env={k:os.environ[k] for k in ['HOME','USER','PATH','LANG','XDG_RUNTIME_DIR','DBUS_SESSION_BUS_ADDRESS'] if k in os.environ};env.update(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',RAG3WEAVER_BUILD_MEMORY_HIGH='0',CARGO_BUILD_JOBS='8')
 with (Q/'outer.log').open('x') as out:return subprocess.run([str(Q/'poste'),'lourd',cfg['timeout'],'--signal=TERM','--kill-after=2s','898s',cfg['python'],'-B',str(Q/'lifecycle.py'),'--compose'],env=env,cwd='/var/tmp',stdout=out,stderr=subprocess.STDOUT).returncode
if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--prepare');ap.add_argument('--launch');ap.add_argument('--allocation');ap.add_argument('--compose',action='store_true');ap.add_argument('--driver',action='store_true');ap.add_argument('--controller',action='store_true');args=ap.parse_args()
 if args.prepare:prepare(Path(__file__).resolve().parents[4],args.prepare)
 elif args.launch:sys.exit(launch(args.launch,args.allocation))
 elif args.compose:sys.exit(compose())
 elif args.controller:
  try:rc=controller()
  except BaseException as error:a.red(error);raise
  sys.exit(rc)
 elif args.driver:
  try:driver()
  except BaseException as error:a.red(error);raise
 else:ap.error('explicit mode required')
