"""Strict public CLI framing/correlation; local observation, not a DB permit."""
import hashlib,json,math,os,re,stat,time
from pathlib import Path

READY_CLOCK='CLOCK_MONOTONIC@luciepc'
READY_CAP_SECONDS=120
POST_WRITER_RESERVE_SECONDS=500

def _clock(value,name):
 if type(value) not in [int,float] or not math.isfinite(value) or value<0:raise ValueError('invalid finite monotonic '+name)
 return value

def ready_bounds(ready_observed_monotonic,scope_deadline_monotonic):
 """Observation BEFORE publication, not a claimed fsync completion timestamp."""
 observed=_clock(ready_observed_monotonic,'READY observation');end=_clock(scope_deadline_monotonic,'scope deadline')
 deadline=min(observed+READY_CAP_SECONDS,end-POST_WRITER_RESERVE_SECONDS)
 if deadline<=observed:raise ValueError('READY deadline has no reserve')
 return dict(clock=READY_CLOCK,ready_observed_monotonic=observed,grant_deadline_monotonic=deadline)

def _descriptor(ready):
 if not isinstance(ready,dict):raise ValueError('READY is not an object')
 phase=ready.get('phase');nonce=ready.get('nonce')
 if not isinstance(nonce,str) or not nonce or not isinstance(phase,dict) or phase.get('job')!='lifecycle-provider':raise ValueError('READY phase/nonce absent')
 if not isinstance(phase.get('pilot'),str) or not re.fullmatch('[0-9a-f]{40}',phase['pilot']) or not isinstance(phase.get('allocation'),str) or not phase['allocation'].strip():raise ValueError('READY phase provenance absent')
 expected=phase.get('authority')
 if not isinstance(expected,dict) or type(expected.get('pid')) is not int or expected['pid']<=1 or type(expected.get('uid')) is not int or expected['uid']<0:raise ValueError('READY authority identity absent')
 if not isinstance(expected.get('start'),str) or not expected['start'].isdigit() or not isinstance(expected.get('cgroup'),str) or not expected['cgroup'].startswith('/') or not isinstance(expected.get('script_sha256'),str) or not re.fullmatch('[0-9a-f]{64}',expected['script_sha256']):raise ValueError('READY authority provenance invalid')
 for value in [expected.get('path'),phase.get('first_red_path')]:
  if not isinstance(value,str) or not value.startswith('/'):raise ValueError('READY authority/first-red path invalid')
 return phase,expected

def validate_phase_authority(ready,now,phase):
 """Scope authority gate, also usable after timely admission during its lease."""
 if not isinstance(ready,dict) or not isinstance(phase,dict):raise ValueError('READY/phase snapshot must be objects')
 now=_clock(now,'current observation');end=_clock(ready.get('deadline'),'scope deadline');observed=_clock(ready.get('ready_observed_monotonic'),'READY observation');deadline=_clock(ready.get('grant_deadline_monotonic'),'grant deadline')
 if ready.get('clock')!=READY_CLOCK or not observed<deadline<=min(observed+READY_CAP_SECONDS,end-POST_WRITER_RESERVE_SECONDS) or now<observed or now>=end:raise ValueError('READY clock/domain/scope deadline invalid')
 _,expected=_descriptor(ready)
 if phase.get('clock')!=READY_CLOCK or phase.get('snapshot_stable') is not True or phase.get('first_red_exists') is not False:raise ValueError('READY authority changed or first refusal exists')
 seen=_clock(phase.get('snapshot_observed_monotonic'),'phase observation')
 if not observed<=seen<=now:raise ValueError('phase observation must precede current clock sample')
 current=phase.get('authority',{});process=phase.get('process',{})
 if not isinstance(current,dict) or not isinstance(process,dict):raise ValueError('authority/process snapshots must be objects')
 if current.get('etat')!='actif' or type(current.get('pid')) is not int or current.get('pid')!=expected['pid'] or current.get('debut')!=expected['start'] or current.get('cgroup')!=expected['cgroup'] or current.get('script_sha256')!=expected['script_sha256'] or _clock(current.get('echeance'),'authority deadline')!=end:raise ValueError('READY authority not active/exact')
 if type(phase.get('authority_uid')) is not int or phase['authority_uid']!=expected['uid'] or type(process.get('uid')) is not int or process['uid']!=expected['uid'] or process.get('start')!=expected['start'] or process.get('cgroup')!=expected['cgroup'] or process.get('state') not in ['R','S','D','I','W','P']:raise ValueError('READY process identity unknown or changed')
 return phase

def validate_ready_phase(ready,now,phase):
 """Admission must be inside the published absolute window, never a new TTL."""
 validate_phase_authority(ready,now,phase)
 if now>=ready['grant_deadline_monotonic']:raise ValueError('READY grant deadline expired')
 return phase

def validate_grant_phase(ready,grant,now,phase,ready_sha256):
 validate_ready_phase(ready,now,phase)
 if not isinstance(grant,dict):raise ValueError('grant must be an object')
 if not isinstance(ready_sha256,str) or not re.fullmatch('[0-9a-f]{64}',ready_sha256) or grant.get('nonce')!=ready['nonce'] or grant.get('ready_sha256')!=ready_sha256 or grant.get('no_embedding_calls_authorized') is not True:raise ValueError('grant nonce/READY binding invalid')
 observed=_clock(grant.get('observed_monotonic'),'grant observation');until=_clock(grant.get('valid_until_monotonic'),'grant lease')
 if not ready['ready_observed_monotonic']<=observed<=now or observed>=ready['grant_deadline_monotonic'] or not now<until<=ready['deadline']:raise ValueError('grant admission deadline/lease invalid')
 return grant

def read_phase_state(ready):
 """RO snapshot on the PC; never return/log the authority's private token."""
 if os.uname().nodename.lower()!='luciepc':raise ValueError('READY clock must be observed on luciepc')
 descriptor,expected=_descriptor(ready);path=Path(expected['path']);red=Path(descriptor['first_red_path'])
 def refusal_exists():
  try:red.lstat();return True
  except FileNotFoundError:return False
 def authority():
  info=path.lstat()
  if not stat.S_ISREG(info.st_mode) or info.st_uid!=expected['uid']:raise ValueError('authority file not regular/owned')
  raw=decode(path.read_bytes());value={k:raw.get(k) for k in ['etat','pid','debut','cgroup','echeance','script_sha256']}
  return value,info.st_uid,(info.st_dev,info.st_ino,info.st_uid,info.st_mode)
 def process():
  root=Path('/proc')/str(expected['pid']);info=root.stat();fields=(root/'stat').read_text().rsplit(') ',1)[1].split();groups=(root/'cgroup').read_text().splitlines();groups=[line[3:] for line in groups if line.startswith('0::')]
  if len(groups)!=1:raise ValueError('authority cgroup unknown')
  return dict(start=fields[19],cgroup=groups[0],uid=info.st_uid,state=fields[0])
 refused_before=refusal_exists();first,uid,identity=authority();before=process();last,last_uid,last_identity=authority();after=process();refused_after=refusal_exists()
 stable=first==last and uid==last_uid and identity==last_identity and all(before[k]==after[k] for k in ['start','cgroup','uid'])
 return dict(clock=READY_CLOCK,authority=last,authority_uid=last_uid,process=after,first_red_exists=refused_before or refused_after,snapshot_stable=stable,snapshot_observed_monotonic=time.monotonic())

def decode(raw):
 def nonfinite(value):raise ValueError('nonfinite JSON scalar')
 def unique(pairs):
  value={}
  for key,item in pairs:
   if key in value:raise ValueError('duplicate JSON key')
   value[key]=item
  return value
 return json.loads(raw,object_pairs_hook=unique,parse_constant=nonfinite)

def public_call(raw,nonce):
 response=decode(raw)
 if not isinstance(response,dict) or response.get('ok') is not True or response.get('mustReopen'):raise ValueError('call refused or must reopen')
 result=response.get('result')
 if not isinstance(result,dict) or not isinstance(result.get('run'),str) or not result['run'].strip():raise ValueError('public run identity absent')
 validation=result.get('validation');delivery=result.get('delivery')
 if not isinstance(validation,dict) or set(validation)!=set(['accepted','errors','warnings']) or validation.get('accepted') is not True or validation.get('errors')!=[] or validation.get('warnings')!=[]:raise ValueError('tool validation not clean')
 if not isinstance(delivery,dict) or set(delivery)!=set(['ok','results']) or delivery.get('ok') is not True or delivery.get('results')!=[]:raise ValueError('unexpected tool delivery')
 meta=result.get('result')
 if isinstance(meta,str):meta=decode(meta)
 if not isinstance(meta,dict) or meta.get('target')!='Scope' or meta.get('query')!=nonce or meta.get('consistency')!='strict':raise ValueError('public metadata not bound to request')
 if 'pending_count' in meta or type(meta.get('pendingCount')) is not int or meta['pendingCount']!=0 or meta.get('partial') is not False or meta.get('signals')!=[] or meta.get('warnings',[])!=[]:raise ValueError('public metadata incomplete or partial')
 for key in ['vectorCount','bm25Count','sparseCount','fusedCount','rerankedCount','searchTimeMs']:
  if type(meta.get(key)) is not int or meta[key]!=0:raise ValueError('unexpected candidate/search metadata')
 return dict(kind='LOCAL_TRANSPORT_OBSERVATION_NOT_DATABASE_TOKEN',nonce=nonce,run=result['run'],raw_sha256=hashlib.sha256(raw).hexdigest(),metadata=meta)

class Session:
 def __init__(self,nonce):
  if not isinstance(nonce,str) or not nonce:raise ValueError('request nonce absent')
  self.nonce=nonce;self.state='CALL_SENT';self.publication=None
 def call_response(self,raw,publish):
  if self.state!='CALL_SENT':raise ValueError('unsolicited or duplicate response')
  publication=public_call(raw,self.nonce)
  publish(publication) # Must persist/fsync successfully BEFORE allowing shutdown.
  self.publication=publication;self.state='PUBLISHED';return publication
 def shutdown_request(self):
  if self.state!='PUBLISHED':raise ValueError('shutdown before persisted local publication')
  self.state='SHUTDOWN_SENT';return b'{"op":"shutdown"}\n'
 def closed_response(self,raw):
  response=decode(raw)
  if self.state!='SHUTDOWN_SENT' or not isinstance(response,dict) or set(response)!=set(['ok','result']) or response.get('ok') is not True or not isinstance(response.get('result'),dict) or set(response['result'])!={'closed'} or response['result']['closed'] is not True:raise ValueError('shutdown not acknowledged')
  self.state='CLOSED'
 def finish(self,rc):
  if self.state!='CLOSED' or type(rc) is not int or rc!=0:raise ValueError('same process not cleanly closed')
  return dict(state=self.state,rc=rc,run=self.publication['run'],nonce=self.nonce)
