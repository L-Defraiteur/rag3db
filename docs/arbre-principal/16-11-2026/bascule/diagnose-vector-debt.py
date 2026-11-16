#!/usr/bin/env python3
"""Counts and exact-match evidence on two private copies; no embeddings, writer or service."""
from pathlib import Path
import hashlib,importlib.util,json,os,re,subprocess,time
HOME=Path.home();HERE=Path(__file__).resolve().parent;DATA=HOME/'.cache/rag3weaver-build/bascule-live-20261116'
ROOT=DATA/'vector-debt-diagnostic-01'
OLD=DATA/'fresh-sets-01/code-principal/copy'
NEW=DATA/'migration-rehearsal-service-corrective-02/code-principal/post-copy'
QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc'
LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
def digest(path):
 h=hashlib.sha256()
 with path.open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def main():
 if digest(QUERY)!='48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616' or digest(LIB/'librag3db.so')!='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101':raise ValueError('native pin mismatch')
 if digest(HERE/'inspect-fts.py')!='b846b0991999040c04f298747eda27aa07a4aa9ad0c277f8b7dc485eb24465a8':raise ValueError('inventory helper mismatch')
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 spec=importlib.util.spec_from_file_location('inspect_fts',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
 before={name:fs.file_inventory(path) for name,path in [('old',OLD),('new',NEW)]}
 for name,rows in before.items():save(name+'-before.json',rows)
 env=dict(os.environ,LD_LIBRARY_PATH=str(LIB));start=time.monotonic();end=start+280;observations={};first_error=None
 debt="(n.embedding IS NULL OR n._embed_hash IS NULL OR n._embed_hash <> n._text_hash)"
 exact="o._parent_field=n._parent_field AND o._index=n._index AND o._text_hash=n._text_hash AND coalesce(o._title,'')=coalesce(n._title,'')"
 valid="o.embedding IS NOT NULL AND o._embed_hash=o._text_hash"
 pattern='MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) MATCH (o:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(s) WHERE '+debt+' AND '+exact
 queries=[('old-chunks',OLD,'MATCH (c:Scope_Chunk) RETURN count(c)'),('old-debt',OLD,'MATCH (n:Scope_Chunk) WHERE '+debt+' RETURN count(n)'),('new-old-chunks',NEW,'MATCH (c:Scope_Chunk) RETURN count(c)'),('new-old-debt',NEW,'MATCH (n:Scope_Chunk) WHERE '+debt+' RETURN count(n)'),('new-debt',NEW,'MATCH (n:ScopeContent_Chunk) WHERE '+debt+' RETURN count(n)'),('new-debt-any-old-exact',NEW,pattern+' RETURN count(DISTINCT n)'),('new-debt-any-old-exact-valid',NEW,pattern+' AND '+valid+' RETURN count(DISTINCT n)')]
 save('queries.json',[dict(name=name,base=str(root/'code-principal.rag3db'),query=sql) for name,root,sql in queries])
 try:
  for name,root,sql in queries:
   remaining=end-time.monotonic()
   if remaining<=5:raise ValueError('diagnostic deadline')
   try:result=subprocess.run([str(QUERY),str(root/'code-principal.rag3db'),sql],capture_output=True,env=env,timeout=min(60,remaining-5))
   except subprocess.TimeoutExpired as error:
    (ROOT/(name+'.stdout')).write_bytes(error.stdout or b'');(ROOT/(name+'.stderr')).write_bytes(error.stderr or b'');save(name+'-process.json',dict(rc=None,timeout=True));raise
   (ROOT/(name+'.stdout')).write_bytes(result.stdout);(ROOT/(name+'.stderr')).write_bytes(result.stderr);save(name+'-process.json',dict(rc=result.returncode))
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('RO query refused '+name)
   match=re.fullmatch(r'BEGIN_QUERY\t0\nROW\t([0-9]+)\nEND_QUERY\t1\n',result.stdout.decode())
   if not match:raise ValueError('invalid scalar output '+name)
   observations[name]=int(match[1]);save(name+'-value.json',dict(count=int(match[1])))
 except Exception as error:first_error=error
 for name,root in [('old',OLD),('new',NEW)]:
  after=fs.file_inventory(root);save(name+'-after.json',after)
  if after!=before[name]:
   save(name+'-audit-error.json',dict(error='private input changed'))
   if first_error is None:first_error=ValueError('private input changed')
 save('observations.json',observations)
 if first_error:save('verdict.json',dict(verdict='RED',error=str(first_error)));raise first_error
 save('verdict.json',dict(verdict='READ_ONLY_VECTOR_DEBT_FACTS',elapsed=time.monotonic()-start,live_qualified=False,migration_qualified=False))
if __name__=='__main__':main()
