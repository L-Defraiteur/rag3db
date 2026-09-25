"""Drive manifest-generated rag3weaver tools; all selection/search runs in the engine."""
import argparse,json,os,subprocess,time,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3];P=ROOT/'experiments/mtga/data';B=ROOT/'experiments/mtga/backend'
p=argparse.ArgumentParser();p.add_argument('--manifest',type=Path,default=B/'backend.json');p.add_argument('--ingest',action='store_true');p.add_argument('--request',type=Path);p.add_argument('--output',type=Path);p.add_argument('--verify',action='store_true');p.add_argument('--ingest-relations',action='store_true');p.add_argument('--ingest-catalog-links',action='store_true');p.add_argument('--ingest-catalog',action='store_true');args=p.parse_args()
env=dict(os.environ,RAG3WEAVER_RENDER_TEMPLATES=str(B/'render'),LD_LIBRARY_PATH=str(ROOT/'build/lecteurs-csv/src'),RAG3DB_BUFFER_POOL_SIZE=os.environ.get('RAG3DB_BUFFER_POOL_SIZE','16106127360'),RAG3DB_MAX_DB_SIZE='68719476736')
with (P/'engine-backend.log').open('a') as err:
 proc=subprocess.Popen([str(ROOT/'extension/rag3weaver/target/debug/rag3weaver-backend'),str(args.manifest.resolve())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=err,text=True,env=env)
 def call(name,arguments):
  try:
   proc.stdin.write(json.dumps({'op':'call','name':name,'arguments':arguments})+'\n');proc.stdin.flush();line=proc.stdout.readline()
  except BrokenPipeError:
   line=''
  if not line:
   detail='\n'.join((P/'engine-backend.log').read_text(errors='replace').splitlines()[-12:])
   raise RuntimeError(f'backend exited while calling {name}:\n{detail}')
  reply=json.loads(line)
  if not reply['ok']:raise RuntimeError(str(reply)[-2400:])
  return reply['result']
 try:
  if args.ingest:
   rows=list(map(json.loads,(P/'engine-collection.jsonl').open()));reports=[];start=time.time()
   for i in range(0,len(rows),512):
    report=call('ingest_cards',{'records':rows[i:i+512]});reports.append(report)
    print(f'ingested {min(i+512,len(rows))}/{len(rows)} elapsed={time.time()-start:.1f}s',flush=True)
    (P/'engine-ingestion-progress.json').write_text(json.dumps({'completed_records':min(i+512,len(rows)),'expected_records':len(rows),'reports':reports},indent=2))
  if args.ingest_relations:
   reports=[];start=time.time()
   for entity in ['Ability','Mechanic','Deck','DeckEntry']:
    rows=list(map(json.loads,(P/f'engine-{entity}.jsonl').open()))
    for i in range(0,len(rows),512):
     report=call('ingest_'+entity.lower(),{'records':rows[i:i+512]});reports.append({'entity':entity,'offset':i,'report':report})
     print(f'{entity} {min(i+512,len(rows))}/{len(rows)} elapsed={time.time()-start:.1f}s',flush=True)
   for relation,links in json.loads((P/'engine-links.json').read_text()).items():
    for i in range(0,len(links),512):
     report=call('link_'+relation.lower(),{'links':links[i:i+512]});reports.append({'relation':relation,'offset':i,'report':report})
     print(f'{relation} {min(i+512,len(links))}/{len(links)} elapsed={time.time()-start:.1f}s',flush=True)
   (P/'engine-relations-ingestion.json').write_text(json.dumps(reports,indent=2))
  if args.ingest_catalog:
   reports=[];start=time.time()
   for entity,tool in [('CatalogCard','ingest_catalog'),('CatalogAbility','ingest_catalogability'),('WildcardInventory','ingest_wildcardinventory')]:
    rows=list(map(json.loads,(P/f'engine-{entity}.jsonl').open()))
    for i in range(0,len(rows),512):
     report=call(tool,{'records':rows[i:i+512]});reports.append({'entity':entity,'offset':i,'report':report})
     print(f'{entity} {min(i+512,len(rows))}/{len(rows)} elapsed={time.time()-start:.1f}s',flush=True)
  if args.ingest_catalog or args.ingest_catalog_links:
   if not args.ingest_catalog: reports=[];start=time.time()
   for relation,links in json.loads((P/'engine-catalog-links.json').read_text()).items():
    for i in range(0,len(links),512):
     report=call('link_'+relation.lower(),{'links':links[i:i+512]});reports.append({'relation':relation,'offset':i,'report':report})
     print(f'{relation} {min(i+512,len(links))}/{len(links)} elapsed={time.time()-start:.1f}s',flush=True)
   (P/'engine-catalog-ingestion.json').write_text(json.dumps(reports,indent=2))
  if args.verify:
   target=P/'engine-results';target.mkdir(exist_ok=True)
   for query in sorted((B/'queries').glob('*.json')):
    request=json.loads(query.read_text());reply=call(request['name'],request['arguments'])
    (target/query.name).write_text(json.dumps(reply,ensure_ascii=False,indent=2)+'\n')
    hits=reply.get('result',[])
    print(f'{query.stem}: {len(hits) if isinstance(hits,list) else "object"} results',flush=True)
  if args.request:
   request=json.loads(args.request.read_text());reply=call(request['name'],request['arguments'])
   if args.output:args.output.write_text(json.dumps(reply,ensure_ascii=False,indent=2)+'\n')
   else:print(json.dumps(reply,ensure_ascii=False))
 finally:
  already_failing=sys.exc_info()[0] is not None
  try:proc.stdin.close()
  except BrokenPipeError:pass
  try:code=proc.wait(timeout=120)
  except subprocess.TimeoutExpired:proc.kill();proc.wait();raise
  if code and not already_failing:raise RuntimeError(f'backend exit {code}')
