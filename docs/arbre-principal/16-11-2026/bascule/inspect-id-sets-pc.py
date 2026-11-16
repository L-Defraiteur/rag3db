#!/usr/bin/env python3
"""One rustc build and immutable readers over new private cold copies, PC only."""
from pathlib import Path
import hashlib,importlib.util,json,os,platform,stat,subprocess,time
HERE=Path(__file__).resolve().parent
HOME=Path.home();ROOT=HOME/'.cache/rag3weaver-build/bascule-live-20261116/id-sets-pc-01'
DEPS=HOME/'.cache/rag3weaver-build/target-cx-optimiseur-bascule-16/release/deps'
RUSTC=HOME/'.rustup/toolchains/1.98.1-x86_64-unknown-linux-gnu/bin/rustc'
QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PINS={RUSTC:'859254978c0a0402c32f949f6de0d99aee73be8d15f45aac00ae1448aac51e74',DEPS/'liblucivy_core-37418cf8e9ef4b85.rlib':'437d48a3439ae92c50574c7acebb09318df2291d794274044277b98ad1c47791',DEPS/'libld_lucivy-46539b3d4bc9bc0b.rlib':'36e98e35cebf488989debe94b6f0e7064939d0b000eca125d70db35a1f2bb09a',DEPS/'lucivy_core-37418cf8e9ef4b85.d':'1bc48547eb6290a8534938b11f7e66e1efedfb4c0d0ef5df048e88484eb2688f',QUERY:'48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616',LIB/'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101',HERE/'snapshot-files.rs':'21528070fc29dd55519b9ebaa57dba64710ea96d05bf4172a63d9fc616a9982b',HERE/'compare-live-ids.py':'c89b119cc568da91663d9b8557052e65cac58f7915e5aee1e80d49ac574bb60a',HERE/'inspect-fts.py':'b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8'}
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def check_pins():
 for path,expected in PINS.items():
  if digest(path)!=expected:raise ValueError('pin mismatch '+str(path))
def save(name,value):
 with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
def command(args,name,env=None):
 remaining=END-time.monotonic()
 if remaining<=5:raise ValueError('deadline before '+name)
 result=subprocess.run([str(a) for a in args],capture_output=True,timeout=min(90,remaining-5),env=env)
 (ROOT/(name+'.stdout')).write_bytes(result.stdout);(ROOT/(name+'.stderr')).write_bytes(result.stderr)
 save(name+'-process.json',dict(argv=[str(a) for a in args],rc=result.returncode))
 return result
def main():
 check_pins();ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 save('pins.json',{str(p):v for p,v in PINS.items()});save('platform.json',dict(host=platform.node(),machine=platform.machine(),qualification='PC_ONLY'))
 spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
 spec=importlib.util.spec_from_file_location('compare_ids',HERE/'compare-live-ids.py');ids=importlib.util.module_from_spec(spec);spec.loader.exec_module(ids)
 version=command([RUSTC,'--version','--verbose'],'rustc-version')
 if version.returncode or b'1.98.1' not in version.stdout:raise ValueError('compiler version mismatch')
 binary=ROOT/'snapshot-files'
 result=command([RUSTC,'--edition=2021',HERE/'snapshot-files.rs','--crate-name','snapshot_files','-C','opt-level=1','-L','dependency='+str(DEPS),'--extern','lucivy_core='+str(DEPS/'liblucivy_core-37418cf8e9ef4b85.rlib'),'--extern','ld_lucivy='+str(DEPS/'libld_lucivy-46539b3d4bc9bc0b.rlib'),'-o',binary],'compile')
 check_pins()
 if result.returncode:raise ValueError('rustc compilation failed')
 elf=digest(binary);save('elf.json',dict(path=str(binary),sha256=elf))
 fixture=ROOT/'fixtures';fixture.mkdir();corrupt=fixture/'corrupt';corrupt.mkdir();(corrupt/'_shard_config.json').write_text(json.dumps(dict(fields=[dict(name='body',type='text',stored=True)],shards=1)));shard=corrupt/'shard_0';shard.mkdir();(shard/'meta.json').write_text('broken')
 before=fs.file_inventory(fixture)
 for name,path in [('absent',fixture/'absent'),('corrupt',corrupt)]:
  result=command([binary,path],'fixture-'+name)
  if result.returncode!=1 or b'SNAPSHOT_REFUSED' not in result.stderr:raise ValueError('fixture failed to refuse normally '+name)
  if before!=fs.file_inventory(fixture):raise ValueError('immutable fixture mutated '+name)
 save('fixtures.json',dict(absent='REFUSED_UNCHANGED',corrupt='REFUSED_UNCHANGED',isa='PC_EXECUTED_NO_TRANSFER_CLAIM'))
 env=dict(os.environ,LD_LIBRARY_PATH=str(LIB))
 for base in ['code-principal','code-lourd']:
  source=HOME/'.cache/rag3weaver-build/bascule-live-20261116/immutable-pc-01'/base
  expected_path=source.parent/(base+'-source-before.json')
  expected_sha={'code-principal':'c6284ab801ef844b77ce0fea678f2588cd91c86020deb536743cc0fd5127cf0a','code-lourd':'367e926ee95153f7f9cabd7c532cf4f0bc55d55c9884122b387178b553bcaabc'}[base]
  if digest(expected_path)!=expected_sha:raise ValueError('cold inventory pin mismatch')
  expected=json.loads(expected_path.read_text())
  before=fs.file_inventory(source);save(base+'-source-before.json',before)
  if before!=expected:raise ValueError('private cold input changed')
  copy=ROOT/base
  result=command(['cp','-a','--reflink=always','--',source,copy],base+'-copy')
  if result.returncode or before!=fs.file_inventory(copy):raise ValueError('copy mismatch '+base)
  fts=copy/(base+'.rag3db.fts');save(base+'-fts.json',fs.inspect(fts))
  queries=["MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'fts_generation:' OR m._key = 'code_scope_form' RETURN m._key,m._value ORDER BY m._key", "MATCH (n:File) RETURN n._org,n._project,count(*)", "MATCH (n:Scope) RETURN n._org,n._project,count(*)"]
  first_error=None
  try:
   result=command([QUERY,copy/(base+'.rag3db'),*queries],base+'-native',env)
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('native copy metadata refused '+base)
   generation={}
   for line in result.stdout.decode().splitlines():
    if line.startswith('ROW\tfts_generation:'):
     key,value=line[4:].split('|',1)
     if key in generation:raise ValueError('duplicate generation metadata')
     generation[key]=ids.decimal(value)
   config=json.loads((ROOT/(base+'-fts.json')).read_text())
   comparison=[dict(index=i['index_key'],database=generation.get('fts_generation:'+i['index_key']),disk=i['generation_file']) for i in config['indexes']]
   save(base+'-generation-comparison.json',comparison)
   if any(i['database'] is None or i['disk'] is None or i['database']!=i['disk'] for i in comparison):raise ValueError('generation mismatch')
   for table in ['File','Scope']:
    result=command([binary,fts/('Lucivy_'+table),'--live-ids'],base+'-'+table+'-fts-ids')
    if result.returncode:raise ValueError('immutable live IDs refused '+base+'/'+table)
    fts_ids=ids.parse_fts(result.stdout.decode())
    graph=command([QUERY,copy/(base+'.rag3db'),'MATCH (n:'+table+') RETURN OFFSET(id(n))'],base+'-'+table+'-graph-ids',env)
    if graph.returncode:raise ValueError('graph offsets refused '+base+'/'+table)
    comparison=ids.compare(fts_ids,ids.parse_graph(graph.stdout.decode()))
    save(base+'-'+table+'-set-comparison.json',comparison)
    if not comparison['equal']:raise ValueError('ID sets differ '+base+'/'+table)
  except Exception as error:first_error=error
  try:
   after=fs.file_inventory(copy);save(base+'-copy-after.json',after)
   source_after=fs.file_inventory(source);save(base+'-source-after.json',source_after)
   old={r['path']:r for r in before};new={r['path']:r for r in after}
   allowed=base+'.rag3db.readers';path=copy/allowed;created=allowed not in old and allowed in new
   if path.exists() or path.is_symlink():
    info=path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid!=1000 or info.st_size!=0:raise ValueError('invalid private readers sidecar')
   if created:
    row=new.pop(allowed)
    if row['sha256']!=hashlib.sha256(b'').hexdigest():raise ValueError('invalid private sidecar hash')
   save(base+'-sidecar.json',dict(path=allowed,created=created))
   if old!=new or before!=source_after:raise ValueError('input changed '+base)
  except Exception as audit_error:
   save(base+'-audit-error.json',dict(error=str(audit_error)))
   if first_error is None:first_error=audit_error
  if first_error:raise first_error
 check_pins()
 if digest(binary)!=elf:raise ValueError('reader ELF changed')
 if time.monotonic()>=END:raise ValueError('deadline after hashes')
 save('verdict.json',dict(verdict='ID_SETS_EQUAL_ON_COLD_COPIES',host='PC_ONLY',live_qualified=False,maintenance_qualified=False,elapsed=time.monotonic()-START))
START=time.monotonic();END=START+580
if __name__=='__main__':
 try:main()
 except Exception as error:
  if ROOT.is_dir() and not (ROOT/'verdict.json').exists():save('verdict.json',dict(verdict='RED',error=str(error)))
  raise
