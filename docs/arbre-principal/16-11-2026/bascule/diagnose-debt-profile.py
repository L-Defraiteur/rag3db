#!/usr/bin/env python3
"""Sanitized entity configuration and equality-only debt probes, private copies only."""
from pathlib import Path
import hashlib,importlib.util,json,os,re,subprocess,time
HOME=Path.home();HERE=Path(__file__).resolve().parent;DATA=HOME/'.cache/rag3weaver-build/bascule-live-20261116'
ROOT=DATA/'vector-debt-profile-01';OLD=DATA/'fresh-sets-01/code-principal/copy';NEW=DATA/'migration-rehearsal-service-corrective-02/code-principal/post-copy'
QUERY=HOME/'.cache/optimiseur-notes/bascule-16nov-927/read-only-query-v2-luciepc';LIB=HOME/'.cache/rag3db-libs/rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF'
def digest(path):
 h=hashlib.sha256()
 with Path(path).open('rb') as stream:
  for chunk in iter(lambda:stream.read(1048576),b''):h.update(chunk)
 return h.hexdigest()
def rows(text):
 lines=text.splitlines()
 if len(lines)<2 or lines[0]!='BEGIN_QUERY\t0' or not re.fullmatch(r'END_QUERY\t[0-9]+',lines[-1]):raise ValueError('incomplete native output')
 result=[]
 for line in lines[1:-1]:
  if not line.startswith('ROW\t'):raise ValueError('unexpected row')
  result.append(line[4:])
 if len(result)!=int(lines[-1].split('\t')[1]):raise ValueError('native row count mismatch')
 return result

def profile(config):
 fields=config.get('fields',{});derived=config.get('derived') or {};render=derived.get('render',{})
 return dict(chunking=config.get('chunking'),source_lines=config.get('source_lines'),signals=config.get('signals'),search_index=config.get('search_index'),chunked=config.get('chunked'),field_names=sorted(fields),content_fields=sorted(k for k,v in fields.items() if v.get('is_content')),title_fields=sorted(k for k,v in fields.items() if v.get('is_title')),derived_from=derived.get('from'),derived_identity=derived.get('identity'),render_hashes={k:hashlib.sha256(json.dumps(v,ensure_ascii=False,sort_keys=True).encode()).hexdigest() for k,v in render.items()})

def main():
 if digest(QUERY)!='48ae2328a29676fa69154cb9c67d9125d4d04cf92b029bd2b2a09981dea95616' or digest(LIB/'librag3db.so')!='b7e9fc75a3d2de04c72680b0309e8bb1df9eaadb5211ebbacf39716c6d0a2101':raise ValueError('native pins')
 ROOT.mkdir(mode=0o700,parents=True,exist_ok=False)
 def save(name,value):
  with (ROOT/name).open('x') as stream:json.dump(value,stream,ensure_ascii=False,indent=2)
 spec=importlib.util.spec_from_file_location('inventory',HERE/'inspect-fts.py');fs=importlib.util.module_from_spec(spec);spec.loader.exec_module(fs)
 before={k:fs.file_inventory(p) for k,p in [('old',OLD),('new',NEW)]}
 for k,v in before.items():save(k+'-before.json',v)
 debt='(n.embedding__granite_278m IS NULL OR n._embed_hash__granite_278m IS NULL OR n._embed_hash__granite_278m <> n._text_hash)'
 pattern='MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) MATCH (o:Scope_Chunk)-[:Scope_CHUNKED_FROM]->(s) WHERE '+debt+" AND coalesce(o._parent_field,'')=coalesce(n._parent_field,'') AND coalesce(o._index,-1)=coalesce(n._index,-1)"
 queries=[('old-scope',OLD,"MATCH (m:_catalog_meta {_key:'entity_config:Scope'}) RETURN m._value",True),('new-scope',NEW,"MATCH (m:_catalog_meta {_key:'entity_config:Scope'}) RETURN m._value",True),('new-content',NEW,"MATCH (m:_catalog_meta {_key:'entity_config:ScopeContent'}) RETURN m._value",True),('old-parent-chunking-debt',OLD,"MATCH (s:Scope) WHERE s._chunked_hash IS NULL OR s._chunked_hash <> s._content_hash RETURN count(s)",False),('cohort-parent-chunking-debt',NEW,'MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE '+debt+' AND (s._chunked_hash IS NULL OR s._chunked_hash <> s._content_hash) RETURN count(DISTINCT s),count(DISTINCT n)',False),('cohort-metadata',NEW,'MATCH (n:ScopeContent_Chunk)-[:ScopeContent_CHUNKED_FROM]->(v:ScopeContent)-[:ScopeContent_DERIVED_FROM]->(s:Scope) WHERE '+debt+' RETURN s.language,s.scope_type,n._parent_field,count(DISTINCT n)',False),('same-rank-text',NEW,pattern+" AND coalesce(o._text,'')=coalesce(n._text,'') RETURN count(DISTINCT n)",False),('same-rank-title',NEW,pattern+" AND coalesce(o._title,'')=coalesce(n._title,'') RETURN count(DISTINCT n)",False)]
 save('queries.json',[dict(name=n,source=str(p),query=q,configuration=c) for n,p,q,c in queries]);start=time.monotonic();end=start+110;env=dict(os.environ,LD_LIBRARY_PATH=str(LIB));first_error=None;observations={}
 try:
  for name,path,sql,configuration in queries:
   remaining=end-time.monotonic()
   if remaining<=5:raise ValueError('profile deadline')
   result=subprocess.run([str(QUERY),str(path/'code-principal.rag3db'),sql],capture_output=True,env=env,timeout=min(60,remaining-5))
   (ROOT/(name+'.stdout')).write_bytes(result.stdout);(ROOT/(name+'.stderr')).write_bytes(result.stderr)
   save(name+'-process.json',dict(rc=result.returncode))
   if result.returncode or b'59ac9c86bc2b3529b590a7462ab7051e3ad26b8a' not in result.stderr:raise ValueError('profile query refused '+name)
   values=rows(result.stdout.decode())
   if configuration:
    if len(values)!=1:raise ValueError('missing entity config')
    observations[name]=profile(json.loads(values[0]))
   else:observations[name]=values
 except Exception as error:first_error=error
 for k,path in [('old',OLD),('new',NEW)]:
  after=fs.file_inventory(path);save(k+'-after.json',after)
  if after!=before[k] and first_error is None:first_error=ValueError('private source changed')
 save('observations.json',observations)
 if first_error:save('verdict.json',dict(verdict='RED',error=str(first_error)));raise first_error
 save('verdict.json',dict(verdict='READ_ONLY_DEBT_PROFILE',elapsed=time.monotonic()-start,migration_qualified=False))
if __name__=='__main__':main()
