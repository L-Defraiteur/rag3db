#!/usr/bin/env python3
"""Preflight the PC reader locally, then decode a new private portable cold copy."""
from pathlib import Path
import hashlib,importlib.util,json,platform,subprocess,time
HERE=Path(__file__).resolve().parent;HOME=Path.home()
ROOT=HOME/'.cache/rag3weaver-build/bascule-live-20261116/immutable-portable-01'
SOURCE=HOME/'.cache/rag3weaver-build/bascule-live-20261116/cold-inspection-02/copy'
TOOLS=HOME/'.cache/rag3weaver-build/bascule-live-20261116/immutable-portable-tools-01'
BINARY=TOOLS/'snapshot-files';ELF='248222d2f76e43465730d0f2c1da5f0ce7101196c3440c609ac0094257fd36f1'
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def save(name,value):
 with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
def command(args,name):
 remaining=END-time.monotonic()
 if remaining<=5:raise ValueError('deadline before '+name)
 r=subprocess.run([str(a) for a in args],capture_output=True,timeout=min(90,remaining-5))
 (ROOT/(name+'.stdout')).write_bytes(r.stdout);(ROOT/(name+'.stderr')).write_bytes(r.stderr)
 save(name+'-process.json',dict(argv=[str(a) for a in args],rc=r.returncode))
 return r

def main():
 if digest(BINARY)!=ELF:raise ValueError('ELF pin mismatch')
 if digest(HERE/'inspect-fts.py')!='b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8':raise ValueError('helper pin mismatch')
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 save('provenance.json',dict(elf_sha256=ELF,build_host='luciepc',pc_proof_manifest_sha256='170026a346c35c2f91a326c1d6fa796039ad0184422a9278af52763ec50cfa12',host=platform.node(),machine=platform.machine(),qualification='LOCAL_ISA_NOT_YET_QUALIFIED'))
 spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
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
  for index in sorted(fts.iterdir()):
   result=command([BINARY,index],index.name)
   if result.returncode or b'IMMUTABLE_SNAPSHOT' not in result.stdout:raise ValueError('snapshot refused '+index.name)
 except Exception as error:first_error=error
 after=fs.file_inventory(copy);save('copy-after.json',after)
 source_after=fs.file_inventory(SOURCE);save('source-after.json',source_after)
 if before!=after or before!=source_after:raise ValueError('private input or copy changed')
 if first_error:raise first_error
 if digest(BINARY)!=ELF:raise ValueError('ELF changed')
 if time.monotonic()>=END:raise ValueError('deadline after hashes')
 save('verdict.json',dict(verdict='PORTABLE_IMMUTABLE_COLD_COPY_DECODED',isa='LOCAL_FIXTURES_AND_REAL_DECODE_EXECUTED',live_qualified=False,maintenance_qualified=False,elapsed=time.monotonic()-START))
START=time.monotonic();END=START+580
if __name__=='__main__':
 try:main()
 except Exception as error:
  if ROOT.is_dir() and not (ROOT/'verdict.json').exists():save('verdict.json',dict(verdict='RED',error=str(error)))
  raise
