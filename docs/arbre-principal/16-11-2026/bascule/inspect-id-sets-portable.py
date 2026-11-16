#!/usr/bin/env python3
"""Preflight the PC reader locally, then decode a new private portable cold copy."""
from pathlib import Path
import hashlib,importlib.util,json,os,platform,subprocess,time
HERE=Path(__file__).resolve().parent;HOME=Path.home()
ROOT=HOME/'.cache/rag3weaver-build/bascule-live-20261116/id-sets-portable-01'
SOURCE=HOME/'.cache/rag3weaver-build/bascule-live-20261116/cold-inspection-02/copy'
TOOLS=HOME/'.cache/rag3weaver-build/bascule-live-20261116/id-set-tools-portable-01'
BINARY=TOOLS/'snapshot-files';ELF='d0dd13e6d403e614038b251f413637d5f6d02a98ba1c10509388d0be1f5e15c7'
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def save(name,value):
 with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
def command(args,name,env=None):
 remaining=END-time.monotonic()
 if remaining<=5:raise ValueError('deadline before '+name)
 r=subprocess.run([str(a) for a in args],capture_output=True,timeout=min(90,remaining-5),env=env)
 (ROOT/(name+'.stdout')).write_bytes(r.stdout);(ROOT/(name+'.stderr')).write_bytes(r.stderr)
 save(name+'-process.json',dict(argv=[str(a) for a in args],rc=r.returncode))
 return r

QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'

def main():
 if digest(BINARY)!=ELF:raise ValueError('ELF pin mismatch')
 if digest(HERE/'inspect-fts.py')!='b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8':raise ValueError('helper pin mismatch')
 if digest(QUERY)!='43cffb082a2487e9f33ffa62b5a246081e5c458683f4e6b7de50e46d60c01329' or digest(LIB/'librag3db.so')!='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101':raise ValueError('native tool pin mismatch')
 if digest(HERE/'compare-live-ids.py')!='c89b119cc568da91663d9b8557052e65cac58f7915e5aee1e80d49ac574bb60a':raise ValueError('comparator pin mismatch')
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 save('provenance.json',dict(elf_sha256=ELF,build_host='luciepc',pc_proof_scope='validation-0_vhpecn',host=platform.node(),machine=platform.machine(),qualification='LOCAL_ISA_NOT_YET_QUALIFIED'))
 spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
 spec=importlib.util.spec_from_file_location('compare_ids',HERE/'compare-live-ids.py');ids=importlib.util.module_from_spec(spec);spec.loader.exec_module(ids)
 fixture=ROOT/'fixtures';fixture.mkdir();bad=fixture/'corrupt';bad.mkdir();(bad/'_shard_config.json').write_text(json.dumps(dict(fields=[dict(name='body',type='text',stored=True)],shards=1)));shard=bad/'shard_0';shard.mkdir();(shard/'meta.json').write_text('broken')
 before=fs.file_inventory(fixture)
 for name,path in [('absent',fixture/'absent'),('corrupt',bad)]:
  result=command([BINARY,path],'fixture-'+name)
  if result.returncode!=1 or b'SNAPSHOT_REFUSED' not in result.stderr:raise ValueError('fixture or local ISA refused '+name)
  if fs.file_inventory(fixture)!=before:raise ValueError('fixture mutated '+name)
 save('fixtures.json',dict(absent='REFUSED_UNCHANGED',corrupt='REFUSED_UNCHANGED',isa='LOCAL_FIXTURES_EXECUTED'))
 manifest=SOURCE.parent/'source-before.json'
 if digest(manifest)!='71377e1da485835454b9c7d8317c2069d5f948563dad4d268569f37c6d3583a2':raise ValueError('cold inventory pin mismatch')
 expected=json.loads(manifest.read_text())
 before=fs.file_inventory(SOURCE);save('source-before.json',before)
 if before!=expected:raise ValueError('private input no longer equals qualified cold inventory')
 copy=ROOT/'copy';result=command(['cp','-a','--reflink=always','--',SOURCE,copy],'copy')
 if result.returncode or before!=fs.file_inventory(copy):raise ValueError('copy mismatch')
 fts=copy/'code-rag3db-depot.rag3db.fts';save('fts-inventory.json',fs.inspect(fts))
 first_error=None
 try:
  env=dict(os.environ,LD_LIBRARY_PATH=str(LIB))
  meta=command([QUERY,copy/'code-rag3db-depot.rag3db',"MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'fts_generation:' RETURN m._key,m._value ORDER BY m._key"],'native-generations',env)
  if meta.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in meta.stderr:raise ValueError('native generation refused')
  generations={}
  for line in meta.stdout.decode().splitlines():
   if line.startswith('ROW\tfts_generation:'):
    key,value=line[4:].split('|',1)
    if key in generations:raise ValueError('duplicate generation')
    generations[key]=ids.decimal(value)
  config=json.loads((ROOT/'fts-inventory.json').read_text())
  comparison=[dict(index=i['index_key'],database=generations.get('fts_generation:'+i['index_key']),disk=i['generation_file']) for i in config['indexes']]
  save('generation-comparison.json',comparison)
  if any(i['database'] is None or i['disk'] is None or i['database']!=i['disk'] for i in comparison):raise ValueError('generation mismatch')
  for table in ['File','Scope']:
   result=command([BINARY,fts/('Lucivy_'+table),'--live-ids'],table+'-fts-ids')
   if result.returncode:raise ValueError('FTS live IDs refused '+table)
   fts_ids=ids.parse_fts(result.stdout.decode())
   graph=command([QUERY,copy/'code-rag3db-depot.rag3db','MATCH (n:'+table+') RETURN OFFSET(id(n))'],table+'-graph-ids',env)
   if graph.returncode:raise ValueError('graph offsets refused '+table)
   comparison=ids.compare(fts_ids,ids.parse_graph(graph.stdout.decode()))
   save(table+'-set-comparison.json',comparison)
   if not comparison['equal']:raise ValueError('ID sets differ '+table)
 except Exception as error:first_error=error
 after=fs.file_inventory(copy);save('copy-after.json',after)
 source_after=fs.file_inventory(SOURCE);save('source-after.json',source_after)
 if before!=after or before!=source_after:raise ValueError('private input or copy changed')
 if first_error:raise first_error
 if digest(BINARY)!=ELF:raise ValueError('ELF changed')
 if time.monotonic()>=END:raise ValueError('deadline after hashes')
 save('verdict.json',dict(verdict='ID_SETS_EQUAL_ON_PORTABLE_COLD_COPY',isa='LOCAL_FIXTURES_AND_REAL_DECODE_EXECUTED',live_qualified=False,maintenance_qualified=False,elapsed=time.monotonic()-START))
START=time.monotonic();END=START+580
if __name__=='__main__':
 try:main()
 except Exception as error:
  if ROOT.is_dir() and not (ROOT/'verdict.json').exists():save('verdict.json',dict(verdict='RED',error=str(error)))
  raise
