"""Private-bus consumer of fresh G management evidence plus independent local OS checks."""
from pathlib import Path
import hashlib,json,math,os,secrets,time
SERVICE='rag3weaver-daemon.service';SOCKET='rag3weaver-daemon.socket'
CLI_SHA='0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288'
SOURCE_SHA='927941a67a5805450d49f1e04b39fdd542a6059b'
def digest(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def parse_unit_stdout(text,unit):
 fields={}
 for line in text.splitlines():
  if not line:continue
  key,separator,value=line.partition('=')
  if not separator or key in fields:raise ValueError('invalid or duplicate systemctl field')
  fields[key]=value
 required={'LoadState':'loaded','ActiveState':'inactive','SubState':'dead'}
 if unit==SERVICE:required['MainPID']='0'
 if any(fields.get(k)!=v for k,v in required.items()):raise ValueError('unit not loaded inactive dead/PID0')
 return {k:fields[k] for k in required}

def validate_reply(request,reply,request_sha,now,raw_units):
 if reply.get('schema_version')!=1 or reply.get('verdict')!='VERT':raise ValueError('G guard not green')
 for key in ['stage','nonce','phase','entered_monotonic','runner']:
  if reply.get(key)!=request[key]:raise ValueError('G response does not bind '+key)
 if reply.get('request_sha256')!=request_sha or reply.get('common_receipt_sha256')!=request['common_receipt']['sha256']:raise ValueError('G request/receipt hash mismatch')
 times=[request['entered_monotonic'],request['requested_monotonic'],reply.get('observed_monotonic'),now]
 if any(type(t) not in (int,float) or not math.isfinite(t) for t in times) or times!=sorted(times) or now-times[2]>30:raise ValueError('G response outside phase or TTL30')
 for unit,kind in [(SERVICE,'units_service'),(SOCKET,'units_socket')]:
  parsed=parse_unit_stdout(raw_units.get(kind,''),unit);declared=reply.get('units',{}).get(unit,{})
  if any(declared.get(k)!=v for k,v in parsed.items()):raise ValueError('G JSON differs from raw unit output')
 if reply.get('previous_host')!=dict(pid=request['expected_host']['pid'],absent=True):raise ValueError('previous host not absent')
 expected=request['expected_host']
 if reply.get('executable')!=dict(path=expected['exe_path'],sha256=expected['exe_sha256'],source_sha=expected['source_sha']):raise ValueError('G executable/source mismatch')
 if reply.get('port_7970')!=dict(rc=0,listeners=[]):raise ValueError('G observed API listener')
 refs=reply.get('base_refs')
 if not isinstance(refs,dict) or refs.get('fd')!=[] or refs.get('maps')!=[]:raise ValueError('G observed base references')
 if not isinstance(reply.get('protected_processes'),list):raise ValueError('missing explicit protected-process baseline')
 if not isinstance(reply.get('raw_files'),list) or not reply['raw_files']:raise ValueError('missing G raw evidence')
 return reply

def normalized_cgroup(value):
 if not isinstance(value,str) or '\n' in value:raise ValueError('invalid cgroup representation')
 if value.startswith('0::'):value=value[3:]
 if not value.startswith('/') or '..' in Path(value).parts:raise ValueError('ambiguous cgroup path')
 return value

def identity(pid):
 proc=Path('/proc')/str(pid);stat=(proc/'stat').read_text();tail=stat.rsplit(')',1)[1].split()
 return dict(pid=int(pid),start=tail[19],cgroup=(proc/'cgroup').read_text().split('0::',1)[1].strip(),uid=proc.stat().st_uid,comm=(proc/'comm').read_text().strip(),state=tail[0])
def refers_to_base(path,bases):
 path=path.removesuffix(' (deleted)')
 for base in bases:
  b=str(base);checkpoint=str(Path(base).with_suffix('.checkpoints'))
  if path==b or path.startswith(b+'.') or path.startswith(b+'/') or path==checkpoint or path.startswith(checkpoint+'/'):return True
 return False

def main_host(exe,argv):
 if len(argv)<2 or argv[1]!='daemon' or not (Path(exe).name=='rag3weaver-backend' or Path(argv[0]).name=='rag3weaver-backend'):return False
 if '--address' not in argv:return True
 position=argv.index('--address')+1
 return position>=len(argv) or argv[position].rsplit(':',1)[-1]=='7970'

def tcp_listeners(text):
 lines=text.splitlines()
 if not lines or 'local_address' not in lines[0]:raise ValueError('invalid TCP table')
 result=[]
 for line in lines[1:]:
  fields=line.split()
  if len(fields)<4 or ':' not in fields[1]:raise ValueError('incomplete TCP row')
  if fields[3]=='0A' and int(fields[1].rsplit(':',1)[1],16)==7970:result.append(fields[1])
 return result

def local_observation(request,reply):
 previous=request['expected_host']['pid'];bases=[Path(request['canonical_base']),Path(request['target_base'])]
 observation=dict(previous_pid_exists=(Path('/proc')/str(previous)).exists(),listeners=[],hosts=[],fd=[],maps=[],errors=[],protected_unknown=[],excluded_other_uid=[])
 for file in [Path('/proc/net/tcp'),Path('/proc/net/tcp6')]:
  if file.exists():observation['listeners'].extend(tcp_listeners(file.read_text()))
 baseline={(int(p['pid']),str(p['start']),int(p['uid']),p['comm']) for p in reply['protected_processes']}
 for proc in Path('/proc').iterdir():
  if not proc.name.isdigit():continue
  try:
   who=identity(int(proc.name))
   if who['state']=='Z':continue
   if who['uid']!=os.getuid():
    observation['excluded_other_uid'].append({k:who[k] for k in ['pid','start','uid','comm']});continue
   key=(who['pid'],who['start'],who['uid'],who['comm'])
   try:
    argv=[os.fsdecode(v) for v in (proc/'cmdline').read_bytes().split(b'\0') if v]
    exe=os.readlink(proc/'exe')
    if main_host(exe,argv):observation['hosts'].append(dict(pid=who['pid'],start=who['start'],exe=exe,argv=argv))
    for fd in (proc/'fd').iterdir():
     try:link=os.readlink(fd)
     except FileNotFoundError:continue
     if refers_to_base(link,bases):observation['fd'].append(dict(pid=who['pid'],start=who['start'],fd=fd.name,path=link))
    for line in (proc/'maps').read_text().splitlines():
     pieces=line.split(maxsplit=5)
     if len(pieces)==6 and refers_to_base(pieces[5],bases):observation['maps'].append(dict(pid=who['pid'],start=who['start'],line=line))
   except PermissionError as error:
    if key not in baseline:observation['errors'].append(dict(pid=who['pid'],start=who['start'],error=str(error)))
    else:observation['protected_unknown'].append({k:who[k] for k in ['pid','start','uid','comm']})
   after=identity(who['pid'])
   if after['start']!=who['start']:observation['errors'].append(dict(pid=who['pid'],error='process identity changed'))
  except (FileNotFoundError,ProcessLookupError):continue
  except Exception as error:observation['errors'].append(dict(pid=int(proc.name),error=str(error)))
 return observation

def validate_local(observation):
 if observation['previous_pid_exists'] or any(observation[k] for k in ['listeners','hosts','fd','maps','errors']):raise ValueError('local cold-state observation refused')

def validate_target_mode(stage,target_kind,base,canonical):
 if target_kind=='live':
  if stage!='live-'+Path(canonical).stem+'-01' or Path(base)!=Path(canonical) or Path(canonical).parent!=Path.home()/'.local/share/rag3weaver' or Path(canonical).stem not in ['code-principal','code-lourd']:raise ValueError('live guard target/stage mismatch')
 elif target_kind!='rehearsal' or stage!='service-guard-corrective-02' or Path(base)==Path(canonical):raise ValueError('rehearsal guard target/stage mismatch')

def fetch_phase_guard(root,phase,entered,base,canonical,source,common,end,stage='service-guard-corrective-02',target_kind='rehearsal'):
 validate_target_mode(stage,target_kind,base,canonical)
 # All authority checks run inside the real Common descendant/cgroup; no user bus access.
 authority=common.autorite(common.processus());receipt=Path(os.environ[common.RECU]);runner=identity(os.getpid())
 guard=root/'guard';guard.mkdir(mode=0o700,exist_ok=True)
 request=dict(schema_version=1,stage=stage,base=Path(canonical).stem,target_kind=target_kind,nonce=secrets.token_hex(16),phase=phase,entered_monotonic=entered,requested_monotonic=time.monotonic(),runner={k:runner[k] for k in ['pid','start','cgroup']},common_receipt=dict(path=str(receipt),sha256=digest(receipt),pid=authority['pid'],start=authority['debut'],cgroup=authority['cgroup'],deadline=authority['echeance']),canonical_base=str(canonical),target_base=str(base),source_snapshot=str(source),expected_host=dict(pid=3761669,exe_path=str(Path.home()/'.local/bin/rag3weaver-backend'),exe_sha256=CLI_SHA,source_sha=SOURCE_SHA),reply_path=str(guard/(phase+'-response.json')),reply_sha_path=str(guard/(phase+'-response.sha256')))
 path=guard/(phase+'-request.json')
 with path.open('x') as stream:json.dump(request,stream,indent=2)
 request_sha=digest(path);reply_path=Path(request['reply_path']);sha_path=Path(request['reply_sha_path']);deadline=min(end-30,time.monotonic()+30)
 while not sha_path.exists():
  if time.monotonic()>=deadline:raise TimeoutError('G phase guard response deadline')
  time.sleep(.05)
 expected=sha_path.read_text().strip()
 if len(expected)!=64 or digest(reply_path)!=expected:raise ValueError('G response hash mismatch')
 reply=json.loads(reply_path.read_text());raw_units={}
 for raw in reply.get('raw_files',[]):
  raw_path=Path(raw['path'])
  if any(p in ['.env','.vault'] for p in raw_path.parts) or digest(raw_path)!=raw['sha256']:raise ValueError('G raw file hash mismatch')
  if raw.get('kind') in ['units_service','units_socket']:
   if raw['kind'] in raw_units:raise ValueError('duplicate G unit stdout')
   raw_units[raw['kind']]=raw_path.read_text()
 validate_reply(request,reply,request_sha,time.monotonic(),raw_units)
 observed=identity(reply['observer']['pid'])
 if any(observed[k]!=reply['observer'][k] for k in ['pid','start','uid']) or observed['cgroup']!=normalized_cgroup(reply['observer']['cgroup']) or observed['uid']!=1000 or common.sous(observed['cgroup'],authority['cgroup']):raise ValueError('G observer identity/role mismatch')
 observation=local_observation(request,reply)
 with (guard/(phase+'-local-os.json')).open('x') as stream:json.dump(observation,stream,indent=2)
 validate_local(observation)
 common.autorite(common.processus())
 return dict(request=request,reply=reply,request_sha256=request_sha,response_sha256=expected,raw_units=raw_units,reply_path=str(reply_path))

def final_phase_check(binding,common):
 validate_reply(binding['request'],binding['reply'],binding['request_sha256'],time.monotonic(),binding['raw_units'])
 if digest(binding['reply_path'])!=binding['response_sha256']:raise ValueError('G response changed')
 for raw in binding['reply']['raw_files']:
  if digest(raw['path'])!=raw['sha256']:raise ValueError('G raw evidence changed')
 observer=identity(binding['reply']['observer']['pid'])
 if any(observer[k]!=binding['reply']['observer'][k] for k in ['pid','start','uid']) or observer['cgroup']!=normalized_cgroup(binding['reply']['observer']['cgroup']):raise ValueError('G observer changed before Popen')
 observation=local_observation(binding['request'],binding['reply'])
 path=Path(binding['reply_path']).parent/(binding['request']['phase']+'-final-local-os.json')
 with path.open('x') as stream:json.dump(observation,stream,indent=2)
 validate_local(observation)
 validate_reply(binding['request'],binding['reply'],binding['request_sha256'],time.monotonic(),binding['raw_units'])
 common.autorite(common.processus())
