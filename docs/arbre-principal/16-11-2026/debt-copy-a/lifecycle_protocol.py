"""Strict public CLI framing/correlation; local observation, not a DB permit."""
import hashlib,json

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
