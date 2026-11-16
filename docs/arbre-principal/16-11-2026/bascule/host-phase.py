#!/usr/bin/env python3
"""Private live-host phases, to run only after G's FINI. Never opens/closes a host."""
from pathlib import Path
import argparse,hashlib,json,re,time,urllib.request,urllib.error
SOURCE='927941a67a5805450d49f1e04b39fdd542a6059b'
BASES={
 'code-rag3db-depot':('/home/lucied/.config/rag3weaver/code-rag3db/templates/backends/code/rag3db.json','8ebbb2e34b582207276980bb0a84457aeaf6e3b5daaf8893f6a3aa2488a2cc77'),
 'code-principal':('/home/lucied/.config/rag3weaver/code-principal/templates/backends/code/backend.json','329a51c5f88595074e2eecebcffccea1aad5be2dce1cfa5d3cdbba50abf20fec'),
 'code-lourd':('/home/lucied/.config/rag3weaver/code-lourd/templates/backends/code/backend.json','3e46a86a510cefe0f37cdaef6012f7ab4fa9257e30617b97f2a20c8f66eb7420'),
}

def save(root,name,value):
 with (root/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)

def archive_http_error(root,route,error):
 # Le corps seul est conservé octet pour octet ; jamais les en-têtes d'authentification.
 root.chmod(0o700)
 try:raw=error.read(1048577)
 finally:error.close()
 save(root,'http-refusal.json',dict(route=route,status=error.code,body_hex=raw[:1048576].hex(),body_length_read=len(raw),truncated=len(raw)>1048576))
 raise RuntimeError('host refused '+route+' status '+str(error.code)) from None

def maintenance_payload(result):
 assert isinstance(result,dict) and isinstance(result.get('token'),str),'missing maintenance token'
 ids=result.get('orphans');live=result.get('live_docs')
 assert isinstance(live,int) and not isinstance(live,bool) and live>=0,'invalid live count'
 assert isinstance(ids,list) and all(isinstance(v,int) and not isinstance(v,bool) and v>=0 for v in ids),'invalid orphan ids'
 assert len(set(ids))==len(ids),'duplicate orphan ids'
 assert len(ids)*100<=live,'purge exceeds the fixed 1 percent bound'
 return dict(token=result['token'],purge_ids=ids,compact=True,max_purge_ratio=.01)

def find_journal(value):
 paths=set()
 def walk(v):
  if isinstance(v,dict):
   if isinstance(v.get('journal'),str):paths.add(v['journal'])
   for child in v.values():walk(child)
  elif isinstance(v,list):
   for child in v:walk(child)
  elif isinstance(v,str):
   if v.lstrip().startswith(('{','[')):
    try:walk(json.loads(v))
    except (ValueError,TypeError):pass
   paths.update(re.findall(r'(?:^|\n)journal : (\S+\.out)',v))
 walk(value)
 assert len(paths)==1,'index did not return exactly one journal; busy is not completion'
 path=Path(paths.pop());assert path.is_absolute() and path.suffix=='.out' and not any(x in path.parts for x in ['.vault','.env']), 'invalid index journal'
 return path

def main():
 ap=argparse.ArgumentParser();ap.add_argument('action',choices=['maintenance','index','diagnostic']);ap.add_argument('base',choices=BASES);ap.add_argument('--evidence',required=True);ap.add_argument('--fini-proof',required=True);ap.add_argument('--seconds',type=int,default=7200);ap.add_argument('--table',action='append',default=[])
 args=ap.parse_args();proof=Path(args.fini_proof);assert proof.is_file(),'G FINI evidence absent'
 manifest,expected=BASES[args.base];m=Path(manifest);assert hashlib.sha256(m.read_bytes()).hexdigest()==expected,'live declaration changed; inspect instead of rewriting'
 root=Path(args.evidence);root.mkdir(parents=True,exist_ok=False);end=time.monotonic()+args.seconds
 save(root,'phase.json',dict(source=SOURCE,action=args.action,base=args.base,manifest=manifest,manifest_sha256=expected,g_fini_proof=str(proof),g_fini_sha256=hashlib.sha256(proof.read_bytes()).hexdigest(),started_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),deadline=end,qualification='RUNNING'))
 # Auth is read from the local host's protected files, never printed or copied.
 state=Path.home()/'.local/share/rag3weaver';address=json.loads((state/'daemon.json').read_text())['address'];token=(state/'daemon.token').read_text().strip();session=None
 def post(route,body):
  assert route in ['/session','/call','/backend/maintain','/backend/fts-live-ids'],'service open/close operations belong to G'
  remaining=end-time.monotonic();assert remaining>0,'phase deadline'
  headers={'Content-Type':'application/json','Authorization':'Bearer '+token}
  if session:headers['X-Rag3weaver-Session']=session
  request=urllib.request.Request('http://'+address+route,data=json.dumps(body).encode(),headers=headers)
  try:
   with urllib.request.urlopen(request,timeout=min(600,remaining)) as response:result=json.load(response)
  except urllib.error.HTTPError as error:
   archive_http_error(root,route,error)
  assert result.get('ok') is True or (route=='/session' and isinstance(result.get('session'),str)),result.get('error','host operation refused')
  return result
 session=post('/session',dict(client='migration-16nov',keys=['dev']))['session']
 try:
  if args.action=='diagnostic':
   assert args.base=='code-rag3db-depot' and args.table==['File'],'diagnostic is File on portable only'
   before=post('/backend/fts-live-ids',dict(manifest=manifest,table='File'))
   save(root,'File-diagnostic-live-ids.json',before)
  elif args.action=='maintenance':
   assert args.table and all(re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*',t) for t in args.table),'explicit table list required'
   for table in args.table:
    before=post('/backend/maintain',dict(manifest=manifest,table=table,dry_run=True));save(root,table+'-dry-run.json',before)
    payload=maintenance_payload(before['result'])
    result=post('/backend/maintain',dict(payload,manifest=manifest,table=table));save(root,table+'-maintain.json',result)
  else:
   result=post('/call',dict(manifest=manifest,name='index',arguments=dict(confirm=True)));save(root,'index-call.json',result)
   journal=find_journal(result);save(root,'index-journal.json',dict(path=str(journal)))
   with (root/'progress.jsonl').open('x') as progress:
    while True:
     assert time.monotonic()<end,'index wait deadline; G must inspect the hosted producer'
     lines=journal.read_text().splitlines();last=lines[-1] if lines else ''
     progress.write(json.dumps(dict(observed=time.monotonic(),lines=len(lines),last=last),ensure_ascii=False)+'\n');progress.flush()
     if last.startswith('indexation échouée') or last.startswith('indexation arrêtée'):raise RuntimeError(last)
     if last.startswith('indexation terminée'):break
     time.sleep(min(5,max(0,end-time.monotonic())))
   save(root,'index-completion.json',dict(last=last,journal=str(journal),debt_qualified=False))
  save(root,'verdict.json',dict(verdict='PHASE_COMPLETED',base=args.base,action=args.action,finished=time.monotonic(),warning='This phase alone does not qualify debt, parity, or hosted-process quiescence.'))
 except BaseException as error:
  save(root,'verdict.json',dict(verdict='RED',base=args.base,action=args.action,error=str(error),hosted_producer_quiescence='NOT_ATTESTED'));raise
if __name__=='__main__':main()
