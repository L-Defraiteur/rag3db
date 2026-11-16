#!/usr/bin/env python3
"""Fixed portable cold-copy inventory; no live path or service is opened."""
from pathlib import Path
import hashlib,importlib.util,json,os,subprocess,sys
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
HOME=Path.home()
SOURCE=HOME/'.local/share/rag3weaver/instantanes/avant-59ac9c86b-20261116-927/code-rag3db-depot'
ROOT=HOME/'.cache/rag3weaver-build/bascule-live-20261116/cold-inspection-01'
QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
PINS={QUERY:'43cffb082a2487e9f33ffa62b5a246081e5c458683f4e6b7de50e46d60c01329',LIB/'librag3db.so':'b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101'}
def digest(p):
 h=hashlib.sha256()
 with p.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def save(name,value):
 with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
def query(base,queries,name,env):
 result=subprocess.run([str(QUERY),str(base),*queries],stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=90)
 (ROOT/(name+'.stdout')).write_bytes(result.stdout);(ROOT/(name+'.stderr')).write_bytes(result.stderr)
 return result

def main():
 for path,expected in PINS.items():
  if digest(path)!=expected:raise ValueError('tool pin mismatch: '+str(path))
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 save('pins.json',{str(p):v for p,v in PINS.items()})
 env=dict(os.environ,LD_LIBRARY_PATH=str(LIB))
 # Refusal fixtures precede opening the cold copy. No native writer is used.
 fixture=ROOT/'fixtures';fixture.mkdir();bad=fixture/'corrupt.rag3db';bad.write_bytes(b'invalid private fixture')
 before=fs.file_inventory(fixture)
 for name,path in [('absent',fixture/'absent.rag3db'),('corrupt',bad)]:
  result=query(path,['RETURN 1'],name,env)
  if result.returncode==0:raise ValueError('native invalid fixture accepted: '+name)
  if fs.file_inventory(fixture)!=before:raise ValueError('native refusal mutated fixture: '+name)
 save('fixtures.json',dict(absent='REFUSED_UNCHANGED',corrupt='REFUSED_UNCHANGED'))
 original=fs.file_inventory(SOURCE);save('source-before.json',original)
 copy=ROOT/'copy'
 subprocess.run(['cp','-a','--reflink=always','--',str(SOURCE),str(copy)],check=True,timeout=90)
 if fs.file_inventory(copy)!=original or fs.file_inventory(SOURCE)!=original:raise ValueError('copy mismatch or source changed')
 save('fts-inventory.json',fs.inspect(copy/'code-rag3db-depot.rag3db.fts'))
 queries=["CALL show_tables() RETURN *", "MATCH (m:_catalog_meta) WHERE m._key STARTS WITH 'fts_' OR m._key = 'code_scope_form' RETURN m._key,m._value ORDER BY m._key", "CALL table_info('File') RETURN *", "CALL table_info('Scope') RETURN *", "MATCH (n:File) RETURN n._org,n._project,count(*)", "MATCH (n:Scope) RETURN n._org,n._project,count(*)"]
 save('queries.json',queries)
 result=query(copy/'code-rag3db-depot.rag3db',queries,'native-inventory',env)
 after=fs.file_inventory(copy);save('copy-after.json',after)
 source_after=fs.file_inventory(SOURCE);save('source-after.json',source_after)
 if result.returncode!=0:raise ValueError('native inventory refused; inspect raw logs')
 if b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('native BUILD_ID mismatch')
 if after!=original or source_after!=original:raise ValueError('native inspection mutated input or copy')
 save('verdict.json',dict(verdict='READ_ONLY_INVENTORY_COMPLETE',lucivy_decode_qualified=False,source=str(SOURCE),copy=str(copy),files=len(original)))
if __name__=='__main__':
 try:main()
 except Exception as error:
  if ROOT.is_dir() and not (ROOT/'verdict.json').exists():save('verdict.json',dict(verdict='RED',error=str(error)))
  raise
