#!/usr/bin/env python3
"""Official writer CLI, gated by fresh proofs and a cold PC API; never operates services."""
from pathlib import Path
import argparse,collections,hashlib,importlib.util,json,os,re,signal,stat,subprocess,time,uuid
HOME=Path.home();HERE=Path(__file__).resolve().parent
CLI=HOME/'.local/bin/rag3weaver-backend';QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PINS={CLI:'0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288',QUERY:'48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616',LIB/'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',LIB/'libvector.rag3db_extension':'1669d6392ec8067fcf92ae2ccc972fd7cd3086629fb82d1f5d254e90d72d7561',HERE/'confiner_validation.py':'6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea',HERE/'cold-service-guard.py':'3ba491472cf42efecb54e33fb1012eca997fdd5d7d1d0dc90f0e3ecddceb75c2',HERE/'inspect-fts.py':'b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8'}
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def check_pins():
 for path,expected in PINS.items():
  if digest(path)!=expected:raise ValueError('pin mismatch '+str(path))
def inventory_live(fs,base):
 paths=[base,*sorted(base.parent.glob(base.name+'.*'))]
 checkpoint=base.with_suffix('.checkpoints')
 if checkpoint.exists():paths.append(checkpoint)
 rows=[]
 for path in paths:
  info=path.lstat()
  if stat.S_ISREG(info.st_mode):rows.append(dict(path=path.name,size=info.st_size,mode=stat.S_IMODE(info.st_mode),sha256=digest(path)))
  elif stat.S_ISDIR(info.st_mode):
   rows.extend(dict(row,path=path.name+'/'+row['path']) for row in fs.file_inventory(path))
  else:raise ValueError('unexpected owned file type '+str(path))
 return sorted(rows,key=lambda row:row['path'])

STORED_GENERATIONS_FORM='code-scope-v1/content/generations'

def normalize_inventory(rows):
 if len({r['path'] for r in rows})!=len(rows):raise ValueError('duplicate inventory paths')
 return sorted(rows,key=lambda row:row['path'])

def mirror_pairs(rows):
 out=[]
 for row in rows:
  values=row.split('|')
  if len(values)!=2:raise ValueError('invalid mirror pair')
  for value in values:uuid.UUID(value)
  out.append(tuple(values))
 return collections.Counter(out)

def model_columns(storage,slug):
 if not re.fullmatch('[a-z0-9_]+',slug) or '__' in slug:raise ValueError('invalid model slug')
 if storage=='legacy':return 'embedding','_embed_hash'
 if storage=='suffixed':return 'embedding__'+slug,'_embed_hash__'+slug
 raise ValueError('unknown model storage')

def one_query_rows(text):
 lines=text.splitlines()
 if len(lines)<2 or lines[0]!='BEGIN_QUERY\t0' or not re.fullmatch(r'END_QUERY\t[0-9]+',lines[-1]):raise ValueError('incomplete query observation')
 rows=[]
 for line in lines[1:-1]:
  if not line.startswith('ROW\t'):raise ValueError('unexpected query output')
  rows.append(line[4:])
 if int(lines[-1].split('\t')[1])!=len(rows):raise ValueError('query count mismatch')
 return rows

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--base',choices=['code-principal','code-lourd'],required=True);ap.add_argument('--target-kind',choices=['rehearsal','live'],required=True);ap.add_argument('--rehearsal-verdict-sha');ap.add_argument('--sets-verdict-sha',required=True);ap.add_argument('--directions-verdict-sha',required=True);ap.add_argument('--source-inventory-sha',required=True);args=ap.parse_args()
 check_pins();data=HOME/'.cache/rag3weaver-build/bascule-live-20261116';sets=data/'fresh-sets-01'/args.base;directions=data/'fresh-directions-01'/args.base
 for path,expected in [(sets/'verdict.json',args.sets_verdict_sha),(directions/'verdict.json',args.directions_verdict_sha),(sets/'source-before.json',args.source_inventory_sha)]:
  if digest(path)!=expected:raise ValueError('fresh gate pin mismatch')
 if json.loads((sets/'verdict.json').read_text()).get('verdict')!='FRESH_ID_SETS_EQUAL' or json.loads((directions/'verdict.json').read_text()).get('verdict')!='FRESH_18_DIRECTIONS_EQUAL':raise ValueError('fresh gates not green')
 provenance=json.loads((sets/'provenance.json').read_text());source=Path(provenance['source']).resolve(strict=True)
 if digest(Path(provenance['g_receipt']))!=provenance['g_receipt_sha256']:raise ValueError('G receipt changed')
 g_receipt=json.loads(Path(provenance['g_receipt']).read_text())
 expected=normalize_inventory(json.loads((sets/'source-before.json').read_text()))
 if normalize_inventory(json.loads((directions/'source-before.json').read_text()))!=expected:raise ValueError('gates refer to different snapshots')
 base=HOME/'.local/share/rag3weaver'/(args.base+'.rag3db')
 if Path(g_receipt.get('canonical_base',''))!=base or not Path(g_receipt.get('units_inactive','')).is_file() or g_receipt.get('retention_hours_minimum',0)<72:raise ValueError('G cold receipt contract failed')
 if args.target_kind=='live':raise ValueError('live runner awaits versioned RET_v2 attestation gate')
 if digest(Path(g_receipt['config']))!=g_receipt['config_sha256']:raise ValueError('live configuration changed')
 if args.target_kind=='live':
  rehearsal=data/'migration-rehearsal-01/code-principal/verdict.json'
  if not args.rehearsal_verdict_sha or digest(rehearsal)!=args.rehearsal_verdict_sha:raise ValueError('rehearsal proof missing or changed')
  rehearsal_record=json.loads(rehearsal.read_text())
  if rehearsal_record.get('verdict')!='REHEARSAL_OFFICIAL_CLI_GREEN':raise ValueError('official rehearsal not green')
 if base.resolve(strict=True)!=base or not stat.S_ISREG(base.lstat().st_mode):raise ValueError('target base identity/path invalid')
 root=data/('migration-rehearsal-service-corrective-02' if args.target_kind=='rehearsal' else 'migration-offline-01')/args.base;root.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (root/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 start=time.monotonic();end=start+(1180 if args.target_kind=='rehearsal' else 3500)
 spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
 spec=importlib.util.spec_from_file_location('common',HERE/'confiner_validation.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
 spec=importlib.util.spec_from_file_location('cold_guard',HERE/'cold-service-guard.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
 common.autorite(common.processus());writer_started=False
 env=dict(os.environ)
 for key in ['RAG3WEAVER_MOTEUR_ANCIEN','RAG3WEAVER_EMBED_SERVICE']:env.pop(key,None)
 profile=dict(LD_LIBRARY_PATH=str(LIB),RAG3DB_LIB=str(LIB),RAG3DB_LIBS_DIR=str(HOME/'.cache/rag3db-libs'),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4')
 for name,key in [('tmp','TMPDIR'),('xdg-config','XDG_CONFIG_HOME'),('xdg-data','XDG_DATA_HOME'),('xdg-cache','XDG_CACHE_HOME'),('xdg-state','XDG_STATE_HOME'),('state-fixture','RAG3WEAVER_STATE_DIR')]:
  path=root/name;path.mkdir();profile[key]=str(path)
 env.update(profile);save('profile.json',profile)
 def run(argv,name,cap,writer=False):
  nonlocal writer_started
  binding=None
  if writer:
   target_before=inventory_live(fs,base);canonical_before=inventory_live(fs,canonical)
   if normalize_inventory(fs.file_inventory(source))!=expected:raise ValueError('snapshot changed before management request')
   if args.target_kind=='rehearsal' and canonical_before!=expected:raise ValueError('canonical live changed before rehearsal')
   binding=guard.fetch_phase_guard(root,name,start,base,canonical,source,common,end)
   if inventory_live(fs,base)!=target_before or inventory_live(fs,canonical)!=canonical_before or normalize_inventory(fs.file_inventory(source))!=expected:raise ValueError('source/target changed during management guard')
   guard.final_phase_check(binding,common)
   save(name+'-management-guard.json',dict(response_path=binding['reply_path'],response_sha256=binding['response_sha256'],request_sha256=binding['request_sha256'],nonce=binding['request']['nonce'],observed_monotonic=binding['reply']['observed_monotonic']))
  remaining=end-time.monotonic();timeout=min(cap,remaining-30)
  if timeout<=0:raise ValueError('deadline before '+name)
  with (root/(name+'.stdout')).open('xb') as out,(root/(name+'.stderr')).open('xb') as err:
   if binding:guard.validate_reply(binding['request'],binding['reply'],binding['request_sha256'],time.monotonic(),binding['raw_units'])
   process=subprocess.Popen([str(a) for a in argv],stdout=out,stderr=err,env=env,start_new_session=True)
   if writer:writer_started=True
   began=time.monotonic();captured=False;pid_start=None
   try:
    if binding:guard.validate_reply(binding['request'],binding['reply'],binding['request_sha256'],time.monotonic(),binding['raw_units'])
    while process.poll() is None:
     if time.monotonic()-began>=timeout:raise TimeoutError(name+' phase deadline')
     if writer and not captured:
      try:
       proc=Path('/proc')/str(process.pid);current=(proc/'stat').read_text().rsplit(')',1)[1].split()[19]
       if pid_start is None:pid_start=current
       if current!=pid_start:raise ValueError('writer PID start changed')
       maps=[line for line in (proc/'maps').read_text().splitlines() if str(LIB) in line and ('librag3db.so' in line or 'libvector.rag3db_extension' in line)]
       if any(str(LIB/'librag3db.so') in line for line in maps) and any(str(LIB/'libvector.rag3db_extension') in line for line in maps):
        if digest(proc/'exe')!=PINS[CLI]:raise ValueError('writer executable changed')
        save(name+'-runtime.json',dict(pid=process.pid,start=pid_start,exe_sha256=PINS[CLI],maps=maps));captured=True
      except FileNotFoundError:pass
     time.sleep(.05)
    rc=process.wait();save(name+'-process.json',dict(argv=[str(a) for a in argv],rc=rc,elapsed=time.monotonic()-began,runtime_maps_captured=captured))
    if rc:raise ValueError(name+' refused rc='+str(rc))
    if writer and not captured:raise ValueError('writer native runtime not observed '+name)
   except BaseException as error:
    if process.poll() is None:process.kill()
    rc=process.wait();save(name+'-failure.json',dict(error=str(error),rc_after_stop=rc,pid=process.pid,start=pid_start));raise
  return (root/(name+'.stdout')).read_text()
 def query(path,sql,name):
  text=run([QUERY,path,sql],name,120)
  if '59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in (root/(name+'.stderr')).read_text():raise ValueError('query BUILD_ID mismatch')
  return one_query_rows(text)
 try:
  authority=common.autorite(common.processus());save('common-entry.json',dict(pid=authority['pid'],start=authority['debut'],cgroup=authority['cgroup'],deadline=authority['echeance'],private_bus_unchanged=True))
  if normalize_inventory(fs.file_inventory(source))!=expected:raise ValueError('snapshot changed before CLI')
  before=inventory_live(fs,base);save('live-before.json',before)
  if before!=expected:raise ValueError('live base changed since fresh snapshot')
  canonical=base
  if args.target_kind=='rehearsal':
   target=root/'target'
   subprocess.run(['cp','-a','--reflink=always','--',str(source),str(target)],check=True,timeout=min(90,max(1,end-time.monotonic()-30)))
   if normalize_inventory(fs.file_inventory(target))!=expected:raise ValueError('rehearsal copy mismatch')
   base=target/canonical.name
   if base.resolve(strict=True)!=base:raise ValueError('rehearsal target alias')
   save('target.json',dict(target_kind=args.target_kind,canonical_live=str(canonical),writer_target=str(base)))
  # All physical prechecks occur before invoking the first writer.
  form_rows=one_query_rows((sets/'native-metadata.stdout').read_text().split('BEGIN_QUERY\t1')[0])
  if any(row.startswith('code_scope_form|') for row in form_rows):raise ValueError('source already has a versioned form')
  save('gates.json',dict(sets_sha256=args.sets_verdict_sha,directions_sha256=args.directions_verdict_sha,source_inventory_sha256=args.source_inventory_sha,purge='NOT_NEEDED_FRESH_ZERO_DIFFERENCES',compaction='DEFERRED',old_422='RED_UNCHANGED'))
  run([CLI,'migration','versions','--base',base],'versions',900 if args.target_kind=='rehearsal' else 1800,True)
  generation=run([CLI,'migration','generations','--base',base],'generations',120 if args.target_kind=='rehearsal' else 180,True)
  if not re.search(r'form_after:\s*"generations"',generation) or not re.search(r'read_rank:\s*Some\(\s*1,?\s*\)',generation):raise ValueError('generation report contract failed')
  save('management-status.json',dict(source='fresh G nonce attestation immediately before each writer, local OS rechecked',private_bus_unchanged=True))
  after=inventory_live(fs,base);save('live-after-writers.json',after)
  if normalize_inventory(fs.file_inventory(source))!=expected:raise ValueError('return snapshot changed')
  copy=root/'post-copy';copy.mkdir()
  paths=[base,*sorted(base.parent.glob(base.name+'.*'))]
  if base.with_suffix('.checkpoints').exists():paths.append(base.with_suffix('.checkpoints'))
  result=subprocess.run(['cp','-a','--reflink=always','--',*[str(p) for p in paths],str(copy)],capture_output=True,timeout=min(90,max(1,end-time.monotonic()-30)))
  if result.returncode or normalize_inventory(fs.file_inventory(copy))!=after:raise ValueError('post migration copy mismatch')
  copied=copy/base.name
  if query(copied,"MATCH (m:_catalog_meta {_key:'code_scope_form'}) RETURN m._value",'post-form')!=[STORED_GENERATIONS_FORM]:raise ValueError('stored form is not generations')
  if query(copied,"MATCH (g:_generation) RETURN g.couche,g.rang",'post-generation')!=['backend|0']:raise ValueError('published generation differs from backend0')
  table_rows=query(copied,'CALL show_tables() RETURN name,type','post-table-inventory')
  tables=[];schema_queries=[]
  for row in table_rows:
   pieces=row.split('|')
   if len(pieces)!=2 or not re.fullmatch('[A-Za-z_][A-Za-z0-9_]*',pieces[0]) or pieces[1] not in ['NODE','REL']:raise ValueError('invalid post table inventory')
   name,kind=pieces;tables.append(dict(name=name,kind=kind))
   schema_queries.append("CALL table_info('"+name+"') RETURN *")
   if kind=='REL':schema_queries.append("CALL show_connection('"+name+"') RETURN *")
   schema_queries.append('MATCH (n:`'+name+'`) RETURN count(*)' if kind=='NODE' else 'MATCH ()-[r:`'+name+'`]->() RETURN count(*)')
  if len({t['name'] for t in tables})!=len(tables):raise ValueError('duplicate post table names')
  save('post-table-inventory.json',tables);save('post-schema-queries.json',schema_queries)
  metadata=run([QUERY,copied,*schema_queries],'post-schema-and-raw-counts',300)
  if len(re.findall(r'^BEGIN_QUERY\t[0-9]+$',metadata,re.M))!=len(schema_queries):raise ValueError('incomplete post schema inventory')
  counts={}
  for table in ['File','Scope']:
   raw=query(copied,'MATCH (n:'+table+') RETURN count(*)','post-'+table+'-raw')
   visible=query(copied,"MATCH (n:"+table+") WHERE n._couche='backend' AND n._depuis <= 1 AND (n._jusqu_a IS NULL OR n._jusqu_a > 1) RETURN count(*)",'post-'+table+'-visible')
   if len(raw)!=1 or len(visible)!=1 or not raw[0].isdigit() or not visible[0].isdigit():raise ValueError('invalid post count')
   expected_count=json.loads((sets/(table+'-set-comparison.json')).read_text())['graph_count']
   counts[table]=dict(raw=int(raw[0]),visible=int(visible[0]),before=expected_count)
   if int(raw[0])!=expected_count or int(visible[0])!=expected_count:raise ValueError('cold occurrence count changed '+table)
  save('post-counts.json',counts)
  links=query(copied,'MATCH (v:ScopeContent)-[r:ScopeContent_DERIVED_FROM]->(s:Scope) RETURN count(r),count(DISTINCT s)','content-occurrence-links')
  if links!=[str(counts['Scope']['raw'])+'|'+str(counts['Scope']['raw'])]:raise ValueError('content occurrence link counts differ')
  if not (copy/(base.name+'.fts')/'Lucivy_Scope').is_dir():raise ValueError('legacy Scope FTS directory missing')
  if not query(copied,"CALL table_info('Scope_Chunk') RETURN name",'legacy-chunk-columns'):raise ValueError('legacy chunks missing')
  forward=query(copied,'MATCH (a:Scope)-[:CONSUMES]->(b:Scope) RETURN a._uuid,b._uuid','mirror-forward')
  reverse=query(copied,'MATCH (b:Scope)-[:CONSUMED_BY]->(a:Scope) RETURN a._uuid,b._uuid','mirror-reverse')
  f=mirror_pairs(forward);r=mirror_pairs(reverse);save('mirror.json',dict(forward=len(forward),reverse=len(reverse),missing_reverse=sum((f-r).values()),missing_forward=sum((r-f).values()),comparison='endpoints_with_multiplicity'))
  if f!=r:raise ValueError('mirror pairs differ')
  model_rows=query(copied,"MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'embedding_model:' RETURN m._key,m._value",'models')
  debt={}
  if not model_rows:raise ValueError('no registered embedding model')
  for row in model_rows:
   key,value=row.split('|',1);model=json.loads(value);slug=key.removeprefix('embedding_model:')
   column,marker=model_columns(model.get('storage'),slug)
   rows=query(copied,'MATCH (c:ScopeContent_Chunk) WHERE c.'+column+' IS NULL OR c.'+marker+' IS NULL OR c.'+marker+' <> c._text_hash RETURN count(*)','debt-'+slug)
   if len(rows)!=1 or not rows[0].isdigit():raise ValueError('invalid vector debt count')
   debt[slug]=int(rows[0])
  save('vector-debt.json',debt)
  if any(debt.values()):raise ValueError('transferred content has vector debt')
  post_copy_after=normalize_inventory(fs.file_inventory(copy));save('post-copy-after.json',post_copy_after)
  if post_copy_after!=after:raise ValueError('post diagnostic copy mutated')
  if inventory_live(fs,base)!=after or normalize_inventory(fs.file_inventory(source))!=expected:raise ValueError('closed source changed during post queries')
  if args.target_kind=='rehearsal':
   canonical_after=inventory_live(fs,canonical);save('canonical-live-after.json',canonical_after)
   if canonical_after!=expected:raise ValueError('rehearsal touched live base')
  if args.target_kind=='live' and digest(Path(g_receipt['retour']))!=g_receipt['retour_sha256']:raise ValueError('return script changed')
  check_pins();common.autorite(common.processus())
  if time.monotonic()>=end:raise ValueError('deadline after final hashes')
  save('verdict.json',dict(verdict='REHEARSAL_OFFICIAL_CLI_GREEN' if args.target_kind=='rehearsal' else 'OFFLINE_GENERATIONS_AND_COLD_GATES_GREEN',base=args.base,elapsed=time.monotonic()-start,index_qualified=False,oracle_qualified=False,service_reopened=False))
 except BaseException as error:
  try:
   snapshot_after=normalize_inventory(fs.file_inventory(source));save('failure-snapshot-after.json',snapshot_after)
   if args.target_kind=='rehearsal':
    canonical_after=inventory_live(fs,Path(g_receipt['canonical_base']));save('failure-canonical-live-after.json',canonical_after)
    save('failure-source-audit.json',dict(snapshot_unchanged=snapshot_after==expected,canonical_live_unchanged=canonical_after==expected))
  except Exception as audit_error:save('failure-source-audit-error.json',dict(error=str(audit_error)))
  save('verdict.json',dict(verdict='RED',error=str(error),target_kind=args.target_kind,target_may_be_partially_migrated=writer_started,retry_authorized=False));raise
if __name__=='__main__':main()
