#!/usr/bin/env python3
"""Fresh G cold snapshot to a new private read-only ID comparison, PC only."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,subprocess,time
HOME=Path.home();HERE=Path(__file__).resolve().parent
BINARY=HOME/'.cache/rag3weaver-build/bascule-live-20261116/id-sets-pc-01/snapshot-files'
QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PINS={BINARY:'d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7',QUERY:'48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616',LIB/'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',HERE/'inspect-fts.py':'b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8',HERE/'compare-live-ids.py':'c89b119cc568da91663d9b8557052e65cac58f7915e5aee1e80d49ac574bb60a'}
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def check_pins():
 for path,expected in PINS.items():
  if digest(path)!=expected:raise ValueError('pin mismatch '+str(path))
def load(name,file):
 spec=importlib.util.spec_from_file_location(name,HERE/file);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--base',choices=['code-principal','code-lourd'],required=True);ap.add_argument('--snapshot-root',required=True);ap.add_argument('--receipt',required=True);ap.add_argument('--receipt-sha',required=True);args=ap.parse_args()
 check_pins();receipt=Path(args.receipt)
 if digest(receipt)!=args.receipt_sha:raise ValueError('G receipt pin mismatch')
 snapshot=Path(args.snapshot_root).resolve(strict=True);allowed=(HOME/'.local/share/rag3weaver/instantanes').resolve(strict=True)
 if snapshot==allowed or not snapshot.is_relative_to(allowed):raise ValueError('input is not a snapshot root')
 source=snapshot/args.base
 root=HOME/'.cache/rag3weaver-build/bascule-live-20261116/fresh-sets-01'/args.base;root.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (root/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 def command(argv,name,env=None):
  remaining=end-time.monotonic()
  if remaining<=5:raise ValueError('deadline before '+name)
  result=subprocess.run([str(a) for a in argv],capture_output=True,timeout=min(90,remaining-5),env=env)
  (root/(name+'.stdout')).write_bytes(result.stdout);(root/(name+'.stderr')).write_bytes(result.stderr)
  save(name+'-process.json',dict(argv=[str(a) for a in argv],rc=result.returncode));return result
 start=time.monotonic();end=start+580
 save('provenance.json',dict(base=args.base,snapshot_root=str(snapshot),source=str(source),g_receipt=str(receipt),g_receipt_sha256=args.receipt_sha,pins={str(p):v for p,v in PINS.items()},qualification='FRESH_DIAGNOSTIC_ONLY'))
 try:
  fs=load('inspect_fts','inspect-fts.py');ids=load('compare_ids','compare-live-ids.py')
  before=fs.file_inventory(source);save('source-before.json',before)
  copy=root/'copy';result=command(['cp','-a','--reflink=always','--',source,copy],'copy')
  if result.returncode or before!=fs.file_inventory(copy):raise ValueError('copy mismatch')
  fts=copy/(args.base+'.rag3db.fts');config=fs.inspect(fts);save('fts-inventory.json',config)
  env=dict(os.environ,LD_LIBRARY_PATH=str(LIB));first_error=None
  try:
   result=command([QUERY,copy/(args.base+'.rag3db'),"MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'fts_generation:' OR m._key = 'code_scope_form' RETURN m._key,m._value ORDER BY m._key",'CALL show_tables() RETURN name','MATCH (n:File) RETURN n._org,n._project,count(*)','MATCH (n:Scope) RETURN n._org,n._project,count(*)'],'native-metadata',env)
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('native metadata refused')
   generations={}
   for line in result.stdout.decode().splitlines():
    if line.startswith('ROW\tfts_generation:'):
     key,value=line[4:].split('|',1)
     if key in generations:raise ValueError('duplicate generation')
     generations[key]=ids.decimal(value)
   comparison=[dict(index=i['index_key'],database=generations.get('fts_generation:'+i['index_key']),disk=i['generation_file']) for i in config['indexes']]
   save('generation-comparison.json',comparison)
   if any(i['database'] is None or i['disk'] is None or i['database']!=i['disk'] for i in comparison):raise ValueError('generation mismatch')
   for table in ['File','Scope']:
    result=command([BINARY,fts/('Lucivy_'+table),'--live-ids'],table+'-fts-ids')
    if result.returncode:raise ValueError('immutable IDs refused '+table)
    fts_ids=ids.parse_fts(result.stdout.decode())
    graph=command([QUERY,copy/(args.base+'.rag3db'),'MATCH (n:'+table+') RETURN OFFSET(id(n))'],table+'-graph-ids',env)
    if graph.returncode:raise ValueError('graph IDs refused '+table)
    comparison=ids.compare(fts_ids,ids.parse_graph(graph.stdout.decode()));save(table+'-set-comparison.json',comparison)
    if not comparison['equal']:raise ValueError('ID sets differ '+table)
  except Exception as error:first_error=error
  after=fs.file_inventory(copy);source_after=fs.file_inventory(source);save('copy-after.json',after);save('source-after.json',source_after)
  old={r['path']:r for r in before};new={r['path']:r for r in after};name=args.base+'.rag3db.readers';path=copy/name;created=name not in old and name in new
  if path.exists() or path.is_symlink():
   import stat
   info=path.lstat()
   if not stat.S_ISREG(info.st_mode) or info.st_uid!=1000 or info.st_size!=0:raise ValueError('invalid readers sidecar')
  if created:
   row=new.pop(name)
   if row['sha256']!=hashlib.sha256(b'').hexdigest():raise ValueError('nonempty readers')
  save('sidecar.json',dict(path=name,created=created))
  if old!=new or before!=source_after:raise ValueError('input changed')
  if first_error:raise first_error
  check_pins()
  if digest(receipt)!=args.receipt_sha or time.monotonic()>=end:raise ValueError('receipt changed or deadline')
  save('verdict.json',dict(verdict='FRESH_ID_SETS_EQUAL',base=args.base,elapsed=time.monotonic()-start,purge_qualified=False,directions_qualified=False,cli_authorized=False))
 except Exception as error:save('verdict.json',dict(verdict='RED',error=str(error)));raise
if __name__=='__main__':main()
